//! The SurrealDB engine (Cloud mode, 5.3) and the account calls around it.
//!
//! **One database for every congregation** of a server (decision D-46): the server's owner applies
//! `surreal/schema.surql` once, with their own tools. The app never has owner or root credentials:
//! it signs up and signs in as a **record user** (`DEFINE ACCESS account ... TYPE RECORD`), with
//! the auth key derived from the account password (6.2) as the password. The schema's permissions
//! keep each congregation's rows to its members.
//!
//! The app talks plain HTTP: `POST /signup`, `POST /signin` and `POST /rpc` (method `query`, with
//! bound variables, so no value is ever pasted into SurrealQL). A write's version check, the next
//! change number and the write run in one transaction; a wrong version `THROW`s `conflict:<n>`.
//! Binary values travel as base64 text.

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD as B64;
use serde_json::{json, Value};

use crate::{DbError, Header, KeyringEntry, NewRecord, Record, RecordId, Write};

/// The schema version this app speaks (`meta:schema` on the server).
pub const SCHEMA_VERSION: i64 = 3;
/// The access method of accounts.
const ACCESS: &str = "account";
/// The access method that uses an invite (it signs in and joins in one step).
const JOIN: &str = "join";

/// A SurrealDB server: its address, namespace and database.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Server {
    /// `https://db.example.org` (no trailing slash).
    pub url: String,
    /// The namespace (default `theo`).
    pub namespace: String,
    /// The database (default `theo`).
    pub database: String,
}

impl Server {
    /// A server at `url`, with the default namespace and database (`theo` / `theo`).
    pub fn new(url: &str) -> Self {
        Server { url: url.trim().trim_end_matches('/').to_string(), namespace: "theo".into(), database: "theo".into() }
    }

    /// The origin used in the account salt (6.2): the address in lower case.
    pub fn origin(&self) -> String {
        self.url.to_lowercase()
    }
}

/// A signed-in account on a server. The token lives in memory only: it is never stored.
#[derive(Clone, PartialEq, Eq)]
pub struct Session {
    /// The server it is signed in to.
    pub server: Server,
    token: String,
}

impl std::fmt::Debug for Session {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Session({}, token: ..)", self.server.url)
    }
}

/// What a server keeps of an account for the account itself (all encrypted by the app, hex).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct AccountData {
    /// The personal keys, wrapped by the account key.
    pub keys: Option<String>,
    /// The public part of the personal keys.
    pub public: Option<String>,
    /// The display name, encrypted with the account key.
    pub profile: Option<String>,
}

/// An invite of a congregation, as its managers see it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InviteState {
    /// The invite's id (SHA-256 of its proof).
    pub id: String,
    /// The username of the account that used it.
    pub used_by: Option<String>,
    /// Past its expiry date.
    pub expired: bool,
    /// When it expires (RFC 3339, UTC).
    pub expires: String,
}

/// An invite the signed-in account has used (to finish joining).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UsedInvite {
    /// The congregation id (32 hex digits).
    pub congregation: String,
    /// `"full"` or `"member"`.
    pub kind: String,
    /// The sealed bundle (hex), until the new member deletes it.
    pub bundle: Option<String>,
}

/// One line of the server's audit log (R-227): metadata only.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AuditEntry {
    /// When (RFC 3339, UTC).
    pub at: String,
    /// The username of who did it (none: the server itself).
    pub actor: Option<String>,
    /// `create`, `update`, `delete` or `use`.
    pub action: String,
    /// `record`, `member`, `invite`, `password`, `key` or `grant`.
    pub what: String,
    /// Which one: `collection:record id`, a username, an invite kind or a key version.
    pub target: Option<String>,
}

/// A grant of the signed-in account (`sealed` in hex).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GrantRow {
    /// The key domain.
    pub domain: u8,
    /// The domain's key version.
    pub key_version: u32,
    /// The grant (hex).
    pub sealed: String,
}

/// One congregation of the signed-in account.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Membership {
    /// The congregation id (32 hex digits).
    pub congregation: String,
    /// `"owner"` or `"member"`.
    pub role: String,
    /// The name, encrypted with the account's key (hex), as the app stored it.
    pub label: String,
    /// The collections a member may write (R-241); managers write everything.
    pub writes: Vec<String>,
    /// This account holds a grant of the root key (opens without the congregation password).
    pub root_grant: bool,
}

/// A member of a congregation, as its managers see it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Member {
    /// The account's username.
    pub username: String,
    /// `"owner"`, `"full"` or `"member"`.
    pub role: String,
    /// The collections a member may write (R-241).
    pub writes: Vec<String>,
    /// The public part of the member's personal keys (hex), to make grants to them.
    pub public: Option<String>,
}

fn strings(v: Option<&Value>) -> Vec<String> {
    v.and_then(Value::as_array).map(|a| a.iter().filter_map(Value::as_str).map(String::from).collect()).unwrap_or_default()
}

fn storage(e: impl std::fmt::Display) -> DbError {
    DbError::Storage(e.to_string())
}

fn b64(bytes: &[u8]) -> String {
    B64.encode(bytes)
}

fn unb64(v: &Value) -> Result<Vec<u8>, DbError> {
    B64.decode(v.as_str().unwrap_or_default()).map_err(storage)
}

/// The token in a sign-in or sign-up answer: `"token": "<jwt>"` or `"token": {"access": ..}`.
fn token_of(body: &str) -> Option<String> {
    let v: Value = serde_json::from_str(body).ok()?;
    let t = v.get("token")?;
    t.as_str().map(String::from).or_else(|| t.get("access").and_then(Value::as_str).map(String::from))
}

async fn access(server: &Server, path: &str, username: &str, auth_key_hex: &str) -> Result<Session, DbError> {
    let body = json!({ "NS": server.namespace, "DB": server.database, "AC": ACCESS, "user": username, "pass": auth_key_hex }).to_string();
    post_access(server, path, body).await
}

async fn post_access(server: &Server, path: &str, body: String) -> Result<Session, DbError> {
    let headers = [("Accept", "application/json".to_string()), ("Content-Type", "application/json".to_string())];
    let (status, text) = crate::http::post(&format!("{}/{path}", server.url), &headers, body).await?;
    match token_of(&text) {
        Some(token) if status < 300 => Ok(Session { server: server.clone(), token }),
        _ if path == "signup" => Err(DbError::Auth("This username is taken, or the server refused the account.".into())),
        _ => Err(DbError::Auth("Username or password is not correct.".into())),
    }
}

/// Creates an account on the server (`SIGNUP`) and signs it in. `auth_key_hex` is the auth key
/// derived from the account password, never the password.
pub async fn sign_up(server: &Server, username: &str, auth_key_hex: &str) -> Result<Session, DbError> {
    access(server, "signup", username, auth_key_hex).await
}

/// Signs in (`SIGNIN`).
pub async fn sign_in(server: &Server, username: &str, auth_key_hex: &str) -> Result<Session, DbError> {
    access(server, "signin", username, auth_key_hex).await
}

/// Uses an invite (5.6): the server checks the account and the invite's `proof`, makes the
/// membership with the invite's role and spends the invite. Afterwards, sign in again as usual:
/// the congregation is then in [`Session::memberships`].
pub async fn join(server: &Server, username: &str, auth_key_hex: &str, proof: &str) -> Result<(), DbError> {
    let body = json!({ "NS": server.namespace, "DB": server.database, "AC": JOIN, "user": username, "pass": auth_key_hex, "proof": proof }).to_string();
    post_access(server, "signin", body)
        .await
        .map(|_| ())
        .map_err(|_| DbError::Auth("This invite cannot be used: it was used already, it expired or was cancelled, or you are already a member. Ask an administrator for a new one.".into()))
}

/// The text of a failed statement ("An error occurred: conflict:3").
fn error_text(statement: &Value) -> String {
    match statement.get("result") {
        Some(Value::String(s)) => s.clone(),
        Some(other) => other.to_string(),
        None => statement.get("detail").and_then(Value::as_str).unwrap_or("unknown error").to_string(),
    }
}

/// Reads `conflict:<n>` out of an error message.
fn conflict_in(message: &str) -> Option<u64> {
    let i = message.find("conflict:")?;
    let digits: String = message[i + 9..].chars().take_while(char::is_ascii_digit).collect();
    digits.parse().ok()
}

/// Turns the answer of an RPC `query` into the results of its statements, or the first error.
fn parse_rpc(status: u16, text: &str) -> Result<Vec<Value>, DbError> {
    if status == 401 || status == 403 {
        return Err(DbError::Auth("You are signed out of the server. Sign in again.".into()));
    }
    let v: Value = serde_json::from_str(text).map_err(|_| DbError::Storage(format!("The server answered {status}: {}", text.chars().take(200).collect::<String>())))?;
    if let Some(e) = v.get("error") {
        let message = e.get("message").and_then(Value::as_str).unwrap_or("unknown error").to_string();
        if let Some(n) = conflict_in(&message) {
            return Err(DbError::Conflict { current: n });
        }
        return Err(DbError::Storage(message));
    }
    let statements = v.get("result").and_then(Value::as_array).cloned().unwrap_or_default();
    // A failed transaction marks every statement as failed; the THROW carries the reason.
    let errors: Vec<String> = statements.iter().filter(|s| s.get("status").and_then(Value::as_str) == Some("ERR")).map(error_text).collect();
    if let Some(n) = errors.iter().find_map(|e| conflict_in(e)) {
        return Err(DbError::Conflict { current: n });
    }
    if errors.iter().any(|e| e.contains("notfound")) {
        return Err(DbError::NotFound);
    }
    if errors.iter().any(|e| e.contains("denied")) {
        return Err(DbError::Denied);
    }
    if let Some(e) = errors.iter().find(|e| !e.contains("not executed")).or(errors.first()) {
        return Err(DbError::Storage(e.clone()));
    }
    Ok(statements.into_iter().map(|s| s.get("result").cloned().unwrap_or(Value::Null)).collect())
}

impl Session {
    /// Runs SurrealQL with bound variables; returns each statement's result.
    pub(crate) async fn query(&self, sql: &str, vars: Value) -> Result<Vec<Value>, DbError> {
        let body = json!({ "id": 1, "method": "query", "params": [sql, vars] }).to_string();
        let headers = [
            ("Accept", "application/json".to_string()),
            ("Content-Type", "application/json".to_string()),
            ("Authorization", format!("Bearer {}", self.token)),
            ("Surreal-NS", self.server.namespace.clone()),
            ("Surreal-DB", self.server.database.clone()),
        ];
        let (status, text) = crate::http::post(&format!("{}/rpc", self.server.url), &headers, body).await?;
        parse_rpc(status, &text)
    }

    /// Runs SurrealQL as this account (tests: to try what a client could send by hand).
    #[doc(hidden)]
    pub async fn query_raw(&self, sql: &str) -> Result<Vec<Value>, DbError> {
        self.query(sql, json!({})).await
    }

    /// The server's schema version (0 when it has none).
    pub async fn schema_version(&self) -> Result<i64, DbError> {
        let r = self.query("SELECT VALUE version FROM meta:schema;", json!({})).await?;
        Ok(r.first().and_then(|v| v.get(0)).and_then(Value::as_i64).unwrap_or(0))
    }

    /// Refuses a server without the schema, or with one this app does not know.
    pub async fn check_schema(&self) -> Result<(), DbError> {
        match self.schema_version().await? {
            SCHEMA_VERSION => Ok(()),
            0 => Err(DbError::Storage("This server is not set up for Theo Manager yet: its owner must apply surreal/schema.surql once.".into())),
            v if v > SCHEMA_VERSION => Err(DbError::Storage("This server uses a newer Theo Manager. Update the app.".into())),
            _ => Err(DbError::Storage("This server uses an older Theo Manager schema: its owner must apply the new surreal/schema.surql.".into())),
        }
    }

    /// Creates a congregation owned by this account, with its owner membership and its change
    /// counter. `label` is the name, already encrypted by the app.
    pub async fn create_congregation(&self, id: RecordId, label: &str) -> Result<(), DbError> {
        let sql = "BEGIN TRANSACTION;
            LET $cg = type::record('congregation', $c);
            CREATE $cg;
            CREATE membership SET account = $auth.id, congregation = $cg, role = 'owner', label = $label;
            CREATE type::record('counter', $c) SET congregation = $cg, seq = 0;
            COMMIT TRANSACTION;";
        self.query(sql, json!({ "c": id.to_hex(), "label": label })).await.map(|_| ())
    }

    /// The congregations of this account.
    pub async fn memberships(&self) -> Result<Vec<Membership>, DbError> {
        let sql = "SELECT record::id(congregation) AS c, role, label, writes,
                array::len(SELECT VALUE id FROM grant WHERE account = $auth.id AND congregation = $parent.congregation AND domain = 0) > 0 AS root
            FROM membership WHERE account = $auth.id;";
        let r = self.query(sql, json!({})).await?;
        let rows = r.first().and_then(Value::as_array).cloned().unwrap_or_default();
        Ok(rows
            .iter()
            .filter_map(|m| {
                Some(Membership {
                    congregation: m.get("c")?.as_str()?.to_string(),
                    role: m.get("role")?.as_str()?.to_string(),
                    label: m.get("label").and_then(Value::as_str).unwrap_or_default().to_string(),
                    writes: strings(m.get("writes")),
                    root_grant: m.get("root").and_then(Value::as_bool).unwrap_or(false),
                })
            })
            .collect())
    }

    /// Changes this account's (encrypted) name of a congregation.
    pub async fn set_label(&self, congregation: RecordId, label: &str) -> Result<(), DbError> {
        let sql = "UPDATE membership SET label = $label WHERE account = $auth.id AND congregation = type::record('congregation', $c);";
        self.query(sql, json!({ "c": congregation.to_hex(), "label": label })).await.map(|_| ())
    }

    /// Leaves a congregation: removes this account's membership (its data stays for the others).
    pub async fn leave(&self, congregation: RecordId) -> Result<(), DbError> {
        let sql = "DELETE membership WHERE account = $auth.id AND congregation = type::record('congregation', $c);";
        self.query(sql, json!({ "c": congregation.to_hex() })).await.map(|_| ())
    }

    /// Deletes a congregation and all its data (its owner only).
    pub async fn delete_congregation(&self, congregation: RecordId) -> Result<(), DbError> {
        // The audit lines go last but one: deleting the rest writes some.
        let sql = "BEGIN TRANSACTION;
            LET $cg = type::record('congregation', $c);
            IF (SELECT VALUE owner FROM $cg)[0] != $auth.id { THROW 'denied'; };
            DELETE item WHERE congregation = $cg;
            DELETE keyring WHERE congregation = $cg;
            DELETE vault_header WHERE congregation = $cg;
            DELETE counter WHERE congregation = $cg;
            DELETE grant WHERE congregation = $cg;
            DELETE invite WHERE congregation = $cg;
            DELETE membership WHERE congregation = $cg;
            DELETE audit WHERE congregation = $cg;
            DELETE $cg;
            COMMIT TRANSACTION;";
        self.query(sql, json!({ "c": congregation.to_hex() })).await.map(|_| ())
    }
}

impl Session {
    /// The account's own encrypted data.
    pub async fn account_data(&self) -> Result<AccountData, DbError> {
        let r = self.query("SELECT keys, public, profile FROM $auth.id;", json!({})).await?;
        let row = rows(&r, 0).into_iter().next().unwrap_or(Value::Null);
        let s = |k: &str| row.get(k).and_then(Value::as_str).map(String::from);
        Ok(AccountData { keys: s("keys"), public: s("public"), profile: s("profile") })
    }

    /// Stores the account's wrapped personal keys and their public part.
    pub async fn set_personal_keys(&self, wrapped: &str, public: &str) -> Result<(), DbError> {
        let sql = format!("LET $w = (UPDATE $auth.id SET keys = $k, public = $p RETURN VALUE id); {WRITTEN}");
        self.query(&sql, json!({ "k": wrapped, "p": public })).await.map(|_| ())
    }

    /// Stores the account's encrypted display name.
    pub async fn set_profile(&self, sealed: &str) -> Result<(), DbError> {
        let sql = format!("LET $w = (UPDATE $auth.id SET profile = $n RETURN VALUE id); {WRITTEN}");
        self.query(&sql, json!({ "n": sealed })).await.map(|_| ())
    }

    /// Creates an invite (owner and full-access members): `id` is the token's id
    /// (`theo_crypto::invite::InviteToken::id`), `kind` `"full"` or `"member"`, `bundle` the
    /// sealed bundle (hex).
    /// `writes`: the collections the new member may write (R-241; ignored for full access).
    pub async fn create_invite(&self, congregation: RecordId, id: &str, kind: &str, bundle: &str, writes: &[String], days: u32) -> Result<(), DbError> {
        let sql = format!(
            "LET $w = (CREATE type::record('invite', $id) SET congregation = type::record('congregation', $c), kind = $kind, bundle = $bundle,
                writes = $writes, expires = time::now() + type::duration(string::concat(<string> $days, 'd')) RETURN VALUE id);
            {WRITTEN}"
        );
        self.query(&sql, json!({ "c": congregation.to_hex(), "id": id, "kind": kind, "bundle": bundle, "writes": writes, "days": days })).await.map(|_| ())
    }

    /// The members of a congregation (owner and full-access members).
    pub async fn members(&self, congregation: RecordId) -> Result<Vec<Member>, DbError> {
        let sql = "SELECT account.username AS username, role, writes, account.public AS public FROM membership WHERE congregation = type::record('congregation', $c);";
        let r = self.query(sql, json!({ "c": congregation.to_hex() })).await?;
        Ok(rows(&r, 0)
            .iter()
            .filter_map(|m| {
                Some(Member {
                    username: m.get("username")?.as_str()?.to_string(),
                    role: m.get("role")?.as_str()?.to_string(),
                    writes: strings(m.get("writes")),
                    public: m.get("public").and_then(Value::as_str).map(String::from),
                })
            })
            .collect())
    }

    /// Sets what a member may write (managers only; R-241).
    pub async fn set_member_writes(&self, congregation: RecordId, username: &str, writes: &[String]) -> Result<(), DbError> {
        let sql = format!(
            "LET $w = (UPDATE membership SET writes = $writes WHERE congregation = type::record('congregation', $c) AND account.username = $u RETURN VALUE writes);
            {WRITTEN}
            IF $w[0] != $writes {{ THROW 'denied'; }};"
        );
        self.query(&sql, json!({ "c": congregation.to_hex(), "u": username, "writes": writes })).await.map(|_| ())
    }

    /// Changes a member's role between `"member"` and `"full"` (the owner only).
    pub async fn set_member_role(&self, congregation: RecordId, username: &str, role: &str) -> Result<(), DbError> {
        let sql = "LET $w = (UPDATE membership SET role = $r WHERE congregation = type::record('congregation', $c) AND account.username = $u AND role != 'owner' RETURN VALUE role);
            IF $w[0] != $r { THROW 'denied'; };";
        self.query(sql, json!({ "c": congregation.to_hex(), "u": username, "r": role })).await.map(|_| ())
    }

    /// Removes a member (managers only; never the owner): their membership and their grants go.
    /// The server denies them at once; the app then rotates the keys they held (R-246).
    pub async fn remove_member(&self, congregation: RecordId, username: &str) -> Result<(), DbError> {
        let sql = format!(
            "BEGIN TRANSACTION;
            LET $cg = type::record('congregation', $c);
            LET $acc = (SELECT VALUE id FROM account WHERE username = $u)[0];
            IF $acc = NONE {{ THROW 'notfound'; }};
            LET $w = (DELETE membership WHERE congregation = $cg AND account = $acc AND role != 'owner' RETURN BEFORE);
            {WRITTEN}
            DELETE grant WHERE congregation = $cg AND account = $acc;
            COMMIT TRANSACTION;"
        );
        self.query(&sql, json!({ "c": congregation.to_hex(), "u": username })).await.map(|_| ())
    }

    /// Stores a grant for another member (managers only), by username.
    pub async fn put_grant_for(&self, congregation: RecordId, username: &str, domain: u8, key_version: u32, sealed: &str) -> Result<(), DbError> {
        let sql = format!(
            "LET $acc = (SELECT VALUE id FROM account WHERE username = $u)[0];
            IF $acc = NONE {{ THROW 'notfound'; }};
            LET $w = (UPSERT type::record('grant', [$c, record::id($acc), $d, $v]) SET congregation = type::record('congregation', $c), account = $acc,
                domain = $d, key_version = $v, sealed = $s RETURN VALUE id);
            {WRITTEN}"
        );
        self.query(&sql, json!({ "c": congregation.to_hex(), "u": username, "d": domain, "v": key_version, "s": sealed })).await.map(|_| ())
    }

    /// Who holds which domain versions in a congregation (managers only): (username, domain,
    /// key version). Domain 0 is the root key.
    pub async fn member_grants(&self, congregation: RecordId) -> Result<Vec<(String, u8, u32)>, DbError> {
        let sql = "SELECT account.username AS u, domain, key_version FROM grant WHERE congregation = type::record('congregation', $c);";
        let r = self.query(sql, json!({ "c": congregation.to_hex() })).await?;
        Ok(rows(&r, 0)
            .iter()
            .filter_map(|g| {
                Some((g.get("u")?.as_str()?.to_string(), u8::try_from(g.get("domain")?.as_u64()?).ok()?, u32::try_from(g.get("key_version")?.as_u64()?).ok()?))
            })
            .collect())
    }

    /// Cancels an invite (owner and full-access members). An unknown one is fine.
    pub async fn revoke_invite(&self, id: &str) -> Result<(), DbError> {
        self.query("DELETE type::record('invite', $id);", json!({ "id": id })).await.map(|_| ())
    }

    /// The invites of a congregation (owner and full-access members).
    pub async fn invites(&self, congregation: RecordId) -> Result<Vec<InviteState>, DbError> {
        let sql = "SELECT record::id(id) AS id, used_by.username AS used_by, expires < time::now() AS expired, <string> expires AS expires FROM invite WHERE congregation = type::record('congregation', $c);";
        let r = self.query(sql, json!({ "c": congregation.to_hex() })).await?;
        Ok(rows(&r, 0)
            .iter()
            .filter_map(|v| {
                Some(InviteState {
                    id: v.get("id")?.as_str()?.to_string(),
                    used_by: v.get("used_by").and_then(Value::as_str).map(String::from),
                    expired: v.get("expired").and_then(Value::as_bool).unwrap_or(false),
                    expires: v.get("expires").and_then(Value::as_str).unwrap_or_default().to_string(),
                })
            })
            .collect())
    }

    /// An invite this account used: its congregation, kind and (until deleted) bundle.
    pub async fn used_invite(&self, id: &str) -> Result<Option<UsedInvite>, DbError> {
        let sql = "SELECT record::id(congregation) AS c, kind, bundle FROM type::record('invite', $id) WHERE used_by = $auth.id;";
        let r = self.query(sql, json!({ "id": id })).await?;
        Ok(rows(&r, 0).first().and_then(|v| {
            Some(UsedInvite {
                congregation: v.get("c")?.as_str()?.to_string(),
                kind: v.get("kind")?.as_str()?.to_string(),
                bundle: v.get("bundle").and_then(Value::as_str).map(String::from),
            })
        }))
    }

    /// Deletes the bundle of an invite this account used (after making its grants).
    pub async fn clear_bundle(&self, id: &str) -> Result<(), DbError> {
        self.query("UPDATE type::record('invite', $id) SET bundle = NONE WHERE used_by = $auth.id;", json!({ "id": id })).await.map(|_| ())
    }

    /// Stores a grant for this account.
    pub async fn put_grant(&self, congregation: RecordId, domain: u8, key_version: u32, sealed: &str) -> Result<(), DbError> {
        let sql = format!(
            "LET $w = (UPSERT type::record('grant', [$c, record::id($auth.id), $d, $v]) SET congregation = type::record('congregation', $c), account = $auth.id,
                domain = $d, key_version = $v, sealed = $s RETURN VALUE id);
            {WRITTEN}"
        );
        self.query(&sql, json!({ "c": congregation.to_hex(), "d": domain, "v": key_version, "s": sealed })).await.map(|_| ())
    }

    /// This account's grants in a congregation.
    pub async fn grants(&self, congregation: RecordId) -> Result<Vec<GrantRow>, DbError> {
        let sql = "SELECT domain, key_version, sealed FROM grant WHERE congregation = type::record('congregation', $c) AND account = $auth.id;";
        let r = self.query(sql, json!({ "c": congregation.to_hex() })).await?;
        Ok(rows(&r, 0)
            .iter()
            .filter_map(|v| {
                Some(GrantRow {
                    domain: u8::try_from(v.get("domain")?.as_u64()?).ok()?,
                    key_version: u32::try_from(v.get("key_version")?.as_u64()?).ok()?,
                    sealed: v.get("sealed")?.as_str()?.to_string(),
                })
            })
            .collect())
    }

    /// Lines of a congregation's audit log, newest first (owner and full-access members): `limit`
    /// lines after skipping `start`.
    pub async fn audit(&self, congregation: RecordId, start: u32, limit: u32) -> Result<Vec<AuditEntry>, DbError> {
        let sql = "SELECT <string> at AS at, actor.username AS actor, action, what, target FROM audit
            WHERE congregation = type::record('congregation', $c) ORDER BY at DESC LIMIT $n START $s;";
        let r = self.query(sql, json!({ "c": congregation.to_hex(), "n": limit, "s": start })).await?;
        Ok(rows(&r, 0).iter().filter_map(audit_of).collect())
    }
}

fn audit_of(v: &Value) -> Option<AuditEntry> {
    let s = |k: &str| v.get(k).and_then(Value::as_str).map(String::from);
    Some(AuditEntry { at: s("at")?, actor: s("actor"), action: s("action")?, what: s("what")?, target: s("target") })
}

/// One congregation's data on a server.
pub(crate) struct Surreal {
    session: Session,
    /// The congregation id, hex.
    c: String,
}

fn rows(results: &[Value], i: usize) -> Vec<Value> {
    results.get(i).and_then(Value::as_array).cloned().unwrap_or_default()
}

fn record_of(v: &Value) -> Result<Record, DbError> {
    let hex = |k: &str| v.get(k).and_then(Value::as_str).and_then(RecordId::from_hex);
    Ok(Record {
        collection: v.get("collection").and_then(Value::as_str).ok_or_else(|| storage("a record has no collection"))?.to_string(),
        record_id: hex("record_id").ok_or_else(|| storage("a record has no id"))?,
        scope_id: hex("scope_id"),
        version: v.get("version").and_then(Value::as_u64).unwrap_or(0),
        seq: v.get("seq").and_then(Value::as_u64).unwrap_or(0),
        updated_at: v.get("updated_at").and_then(Value::as_i64).unwrap_or(0),
        deleted: v.get("deleted").and_then(Value::as_bool).unwrap_or(false),
        envelope: unb64(v.get("envelope").unwrap_or(&Value::Null))?,
    })
}

/// The statement that checks the version: `$cur` is the current version (0: none).
const CHECK: &str = "IF $cur != $expected { THROW string::concat('conflict:', <string> $cur); };";
/// A write the permissions refuse does not fail: it returns nothing (`[]`). `$w` holds what the
/// write returned, so a refused write becomes an error instead of a silent no-op.
const WRITTEN: &str = "IF array::len($w) = 0 { THROW 'denied'; };";

impl Surreal {
    pub(crate) async fn open(session: Session, congregation: &str) -> Result<Self, DbError> {
        session.check_schema().await?;
        Ok(Surreal { session, c: congregation.to_string() })
    }

    pub(crate) async fn read_header(&self) -> Result<Option<Header>, DbError> {
        let r = self.session.query("SELECT version, bytes FROM type::record('vault_header', $c);", json!({ "c": self.c })).await?;
        rows(&r, 0)
            .first()
            .map(|h| Ok(Header { version: h.get("version").and_then(Value::as_u64).unwrap_or(0), bytes: unb64(h.get("bytes").unwrap_or(&Value::Null))? }))
            .transpose()
    }

    pub(crate) async fn write_header(&self, bytes: &[u8], expected: Option<u64>) -> Result<u64, DbError> {
        let sql = format!(
            "BEGIN TRANSACTION;
            LET $id = type::record('vault_header', $c);
            LET $cur = (SELECT VALUE version FROM $id)[0] ?? 0;
            {CHECK}
            LET $w = (UPSERT $id SET congregation = type::record('congregation', $c), version = $cur + 1, bytes = $bytes RETURN VALUE version);
            {WRITTEN}
            COMMIT TRANSACTION;"
        );
        let expected = expected.unwrap_or(0);
        self.session.query(&sql, json!({ "c": self.c, "expected": expected, "bytes": b64(bytes) })).await?;
        Ok(expected + 1)
    }

    pub(crate) async fn keyring(&self) -> Result<Vec<KeyringEntry>, DbError> {
        let r = self.session.query("SELECT * FROM keyring WHERE congregation = type::record('congregation', $c);", json!({ "c": self.c })).await?;
        rows(&r, 0)
            .iter()
            .map(|e| {
                // From the server: a missing or out-of-range number is refused, not guessed or cut.
                let number = |k: &str| e.get(k).and_then(Value::as_u64).ok_or_else(|| storage(format!("a keyring entry has no valid {k}")));
                Ok(KeyringEntry {
                    domain: u8::try_from(number("domain")?).map_err(storage)?,
                    key_version: u32::try_from(number("key_version")?).map_err(storage)?,
                    public_key: unb64(e.get("public_key").unwrap_or(&Value::Null))?,
                    wrapped_secret: unb64(e.get("wrapped_secret").unwrap_or(&Value::Null))?,
                })
            })
            .collect()
    }

    pub(crate) async fn put_keyring(&self, entry: &KeyringEntry) -> Result<(), DbError> {
        let sql = format!(
            "LET $w = (UPSERT type::record('keyring', [$c, $d, $v]) SET congregation = type::record('congregation', $c), domain = $d, key_version = $v, public_key = $pk, wrapped_secret = $ws RETURN VALUE version);
            {WRITTEN}"
        );
        let vars = json!({ "c": self.c, "d": entry.domain, "v": entry.key_version, "pk": b64(&entry.public_key), "ws": b64(&entry.wrapped_secret) });
        self.session.query(&sql, vars).await.map(|_| ())
    }

    pub(crate) async fn put(&self, rec: &NewRecord<'_>, expected: Option<u64>) -> Result<u64, DbError> {
        Ok(self.write_many(&[Write::Put(*rec, expected)]).await?.first().copied().unwrap_or(0))
    }

    /// Many writes in one request and one transaction: each checks its version, takes the next
    /// change number and writes; any failure (a conflict, a refusal) undoes them all. Statement
    /// `i` uses variables suffixed with `i`, all bound (no value is pasted into the query).
    pub(crate) async fn write_many(&self, writes: &[Write<'_>]) -> Result<Vec<u64>, DbError> {
        if writes.is_empty() {
            return Ok(Vec::new());
        }
        // One change-number update for the whole batch: write `i` takes `$seq0 + i + 1`.
        let mut sql = String::from("BEGIN TRANSACTION;\nLET $seq0 = (UPDATE ONLY type::record('counter', $c) SET seq += $n RETURN VALUE seq) - $n;\n");
        let mut vars = serde_json::Map::new();
        vars.insert("c".into(), json!(self.c));
        vars.insert("n".into(), json!(writes.len()));
        let mut versions = Vec::with_capacity(writes.len());
        for (i, w) in writes.iter().enumerate() {
            let (collection, record_id, expected) = match w {
                Write::Put(rec, expected) => (rec.collection, rec.record_id, expected.unwrap_or(0)),
                Write::Delete { collection, record_id, expected } => (*collection, *record_id, *expected),
            };
            vars.insert(format!("coll{i}"), json!(collection));
            vars.insert(format!("rid{i}"), json!(record_id.to_hex()));
            vars.insert(format!("expected{i}"), json!(expected));
            versions.push(expected + 1);
            sql.push_str(&format!(
                "LET $id{i} = type::record('item', [$c, $coll{i}, $rid{i}]);
                LET $cur{i} = (SELECT VALUE version FROM $id{i})[0] ?? 0;\n"
            ));
            if matches!(w, Write::Delete { .. }) {
                sql.push_str(&format!("IF $cur{i} = 0 {{ THROW 'notfound'; }};\n"));
            }
            let next = i + 1;
            sql.push_str(&format!(
                "IF $cur{i} != $expected{i} {{ THROW string::concat('conflict:', <string> $cur{i}); }};
                LET $seq{i} = $seq0 + {next};\n"
            ));
            match w {
                Write::Put(rec, _) => {
                    // The scope is set only when there is one (`option<string>` takes NONE, not NULL).
                    let scope = match rec.scope_id {
                        Some(sc) => {
                            vars.insert(format!("scope{i}"), json!(sc.to_hex()));
                            format!(", scope_id = $scope{i}")
                        }
                        None => String::new(),
                    };
                    vars.insert(format!("env{i}"), json!(b64(rec.envelope)));
                    sql.push_str(&format!(
                        "LET $w{i} = (UPSERT $id{i} SET congregation = type::record('congregation', $c), collection = $coll{i}, record_id = $rid{i}{scope}, version = $cur{i} + 1, seq = $seq{i}, deleted = false, envelope = $env{i} RETURN VALUE version);\n"
                    ));
                }
                Write::Delete { .. } => {
                    sql.push_str(&format!("LET $w{i} = (UPDATE $id{i} SET version = $cur{i} + 1, seq = $seq{i}, deleted = true, envelope = '' RETURN VALUE version);\n"));
                }
            }
            sql.push_str(&format!("IF array::len($w{i}) = 0 {{ THROW 'denied'; }};\n"));
        }
        sql.push_str("COMMIT TRANSACTION;");
        self.session.query(&sql, Value::Object(vars)).await?;
        Ok(versions)
    }

    pub(crate) async fn get(&self, collection: &str, id: RecordId) -> Result<Option<Record>, DbError> {
        let r = self.session.query("SELECT * FROM type::record('item', [$c, $coll, $rid]);", json!({ "c": self.c, "coll": collection, "rid": id.to_hex() })).await?;
        rows(&r, 0).first().map(record_of).transpose()
    }

    pub(crate) async fn changes(&self, since_seq: u64) -> Result<Vec<Record>, DbError> {
        let sql = "SELECT * FROM item WHERE congregation = type::record('congregation', $c) AND seq > $since ORDER BY seq;";
        let r = self.session.query(sql, json!({ "c": self.c, "since": since_seq })).await?;
        rows(&r, 0).iter().map(record_of).collect()
    }

    pub(crate) async fn delete(&self, collection: &str, id: RecordId, expected: u64) -> Result<u64, DbError> {
        Ok(self.write_many(&[Write::Delete { collection, record_id: id, expected }]).await?.first().copied().unwrap_or(0))
    }

    pub(crate) async fn purge(&self, collection: &str, id: RecordId) -> Result<(), DbError> {
        self.session.query("DELETE type::record('item', [$c, $coll, $rid]);", json!({ "c": self.c, "coll": collection, "rid": id.to_hex() })).await.map(|_| ())
    }

    pub(crate) async fn last_seq(&self) -> Result<u64, DbError> {
        let r = self.session.query("SELECT VALUE seq FROM type::record('counter', $c);", json!({ "c": self.c })).await?;
        Ok(r.first().and_then(|v| v.get(0)).and_then(Value::as_u64).unwrap_or(0))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn answers_are_read_without_a_server() {
        assert_eq!(token_of(r#"{"code":200,"details":"ok","token":"abc"}"#).as_deref(), Some("abc"));
        assert_eq!(token_of(r#"{"code":200,"token":{"access":"jwt","refresh":"r"}}"#).as_deref(), Some("jwt"));
        assert_eq!(token_of(r#"{"code":401,"details":"no"}"#), None);
        assert_eq!(conflict_in("An error occurred: conflict:12"), Some(12));
        assert_eq!(conflict_in("something else"), None);

        let ok = r#"{"id":1,"result":[{"status":"OK","time":"1ms","result":[{"seq":3}]}]}"#;
        assert_eq!(parse_rpc(200, ok), Ok(vec![json!([{ "seq": 3 }])]));
        let thrown = r#"{"id":1,"result":[{"status":"ERR","result":"The query was not executed due to a failed transaction"},{"status":"ERR","result":"An error occurred: conflict:4"}]}"#;
        assert_eq!(parse_rpc(200, thrown), Err(DbError::Conflict { current: 4 }));
        let gone = r#"{"id":1,"result":[{"status":"ERR","result":"An error occurred: notfound"}]}"#;
        assert_eq!(parse_rpc(200, gone), Err(DbError::NotFound));
        let denied = r#"{"id":1,"result":[{"status":"ERR","kind":"Thrown","result":"An error occurred: denied"}]}"#;
        assert_eq!(parse_rpc(200, denied), Err(DbError::Denied));
        let audit = json!({ "at": "2026-11-26T10:00:00Z", "actor": "ann", "action": "use", "what": "invite", "target": "member" });
        assert_eq!(audit_of(&audit).map(|a| (a.actor, a.action)), Some((Some("ann".into()), "use".into())));
        let rpc_error = r#"{"id":1,"error":{"code":-32000,"message":"There was a problem: conflict:2"}}"#;
        assert_eq!(parse_rpc(200, rpc_error), Err(DbError::Conflict { current: 2 }));
        assert!(matches!(parse_rpc(401, ""), Err(DbError::Auth(_))));
        assert!(matches!(parse_rpc(500, "<html>"), Err(DbError::Storage(_))));
    }

    #[test]
    fn records_are_read_from_rows() -> Result<(), DbError> {
        let id = RecordId([7; 16]);
        let row = json!({ "collection": "people", "record_id": id.to_hex(), "version": 2, "seq": 9, "updated_at": 1_800_000_000, "deleted": false, "envelope": b64(b"\x00\xffTH") });
        let r = record_of(&row)?;
        assert_eq!((r.collection.as_str(), r.record_id, r.scope_id, r.version, r.seq, r.envelope.as_slice()), ("people", id, None, 2, 9, b"\x00\xffTH".as_slice()));
        assert_eq!(Server::new(" https://DB.example.org/ ").origin(), "https://db.example.org");
        Ok(())
    }
}
