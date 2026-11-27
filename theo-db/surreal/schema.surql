-- Theo Manager: SurrealDB schema, version 3 (SurrealDB 4.x).
--
-- ONE database holds every congregation of a server. Congregations are kept apart by the
-- permissions below: a signed-in account sees and changes only the congregations it is a member
-- of. Everything a congregation stores is ciphertext (REQUIREMENTS.md 6.4); in plaintext the
-- server sees only ids, version and change numbers, collection names, roles and times (5.1).
--
-- Apply it ONCE, as the database owner, with your own tools. The app never has these credentials:
--
--   curl -X POST https://db.example.org/sql -u '<owner>:<password>' -H 'Accept: application/json' \
--       -H 'Surreal-NS: theo' -H 'Surreal-DB: theo' --data-binary @theo-db/surreal/schema.surql
--
-- (Not `surreal import`: since SurrealDB 4 it is for data dumps and switches events off. Not
-- `surreal sql < file` either: it splits at line ends and breaks the blocks. docs/self-hosting.md)
-- Every statement uses OVERWRITE, so applying a newer version of this file upgrades in place.
-- The app reads meta:schema and refuses a server whose version it does not know.
--
-- Browsers: the web app calls this server from its own address, so the server must allow that
-- origin (CORS). `surreal start` allows every origin unless `--allow-origin` is set; a reverse
-- proxy in front of it must pass the CORS headers through (docs/self-hosting.md).

DEFINE TABLE OVERWRITE meta SCHEMAFULL PERMISSIONS FOR select FULL, FOR create, update, delete NONE;
DEFINE FIELD OVERWRITE version ON meta TYPE int;
UPSERT meta:schema SET version = 3;

-- ----- Roles -----
-- owner: created the congregation. full: full access (knows the congregation password, manages
-- members and invites). member: joined with a member invite, holds only its roles' key domains.

-- The caller's role in congregation $c, or NONE. Used by the permissions below, which read with
-- the schema's authority (so this lookup is not limited by the caller's own permissions).
DEFINE FUNCTION OVERWRITE fn::role($c: any) {
    RETURN (SELECT VALUE role FROM membership WHERE account = $auth.id AND congregation = $c LIMIT 1)[0];
};

-- May the caller write records of collection $coll in congregation $c? The owner and full-access
-- members write everything; a member writes the collections in their membership's `writes`, which
-- only managers set (from the member's permissions, R-241). So a member cannot overwrite records
-- of other collections, even the ones they cannot decrypt.
DEFINE FUNCTION OVERWRITE fn::can_write($c: any, $coll: string) {
    LET $m = (SELECT role, writes FROM membership WHERE account = $auth.id AND congregation = $c LIMIT 1)[0];
    RETURN $m != NONE AND ($m.role INSIDE ["owner", "full"] OR $coll INSIDE ($m.writes ?? []));
};

-- ----- Accounts (record access: sign up and sign in with the derived auth key, R-222) -----

-- Co-members can read an account's username and public keys (to make grants). The wrapped
-- personal keys and the encrypted display name are readable by the account only.
DEFINE TABLE OVERWRITE account SCHEMAFULL
    PERMISSIONS
        FOR select WHERE id = $auth.id
            OR id INSIDE (SELECT VALUE account FROM membership WHERE congregation INSIDE (SELECT VALUE congregation FROM membership WHERE account = $auth.id)),
        FOR update WHERE id = $auth.id,
        FOR create, delete NONE;
DEFINE FIELD OVERWRITE username ON account TYPE string READONLY
    ASSERT string::len($value) >= 3 AND string::len($value) <= 64;
-- argon2(auth_key): the server never sees the account password, and cannot derive the
-- account's encryption key from the auth key (HKDF is one-way, 6.2).
DEFINE FIELD OVERWRITE auth_hash ON account TYPE string PERMISSIONS FOR select, update NONE;
DEFINE FIELD OVERWRITE created ON account TYPE datetime VALUE $before OR time::now();
-- The personal keys (X-Wing, ML-DSA-65, Ed25519) wrapped by the account key, and their public part.
DEFINE FIELD OVERWRITE keys ON account TYPE option<string> PERMISSIONS FOR select WHERE id = $auth.id;
DEFINE FIELD OVERWRITE public ON account TYPE option<string>;
-- The display name, encrypted with the account key.
DEFINE FIELD OVERWRITE profile ON account TYPE option<string> PERMISSIONS FOR select WHERE id = $auth.id;
DEFINE INDEX OVERWRITE account_username ON account FIELDS username UNIQUE;

DEFINE ACCESS OVERWRITE account ON DATABASE TYPE RECORD
    SIGNUP ( CREATE account SET username = $user, auth_hash = crypto::argon2::generate($pass) )
    SIGNIN ( SELECT * FROM account WHERE username = $user AND crypto::argon2::compare(auth_hash, $pass) )
    DURATION FOR TOKEN 1h, FOR SESSION 12h;

-- Joining with an invite (5.6): the account signs in with its auth key and the invite's proof.
-- The server finds the invite by SHA-256(proof), checks it is open and not expired, makes the
-- membership with the invite's role and spends the invite, all in one step. This clause runs with
-- the access method's own rights, so the permissions below do not stop it. The app then signs in
-- again with `account` access; the short token of this access is not used for anything else.
DEFINE ACCESS OVERWRITE join ON DATABASE TYPE RECORD
    SIGNIN {
        LET $acct = (SELECT * FROM account WHERE username = $user AND crypto::argon2::compare(auth_hash, $pass))[0];
        IF $acct = NONE { THROW "signin"; };
        LET $inv = (SELECT * FROM type::record('invite', crypto::sha256($proof)))[0];
        IF $inv = NONE OR $inv.used_by != NONE OR $inv.expires < time::now() { THROW "invite:unusable"; };
        IF (SELECT VALUE id FROM membership WHERE account = $acct.id AND congregation = $inv.congregation)[0] != NONE { THROW "invite:member"; };
        CREATE membership SET account = $acct.id, congregation = $inv.congregation, role = $inv.kind, label = '', writes = $inv.writes ?? [];
        UPDATE $inv.id SET used_by = $acct.id, used_at = time::now();
        RETURN SELECT * FROM account WHERE id = $acct.id;
    }
    DURATION FOR TOKEN 5m, FOR SESSION 5m;

-- ----- Congregations and memberships -----

-- The id is the congregation id (32 hex digits). Its name is NOT here: each member keeps it
-- encrypted with their own account key in their membership (`label`).
DEFINE TABLE OVERWRITE congregation SCHEMAFULL
    PERMISSIONS
        FOR select WHERE owner = $auth.id OR fn::role(id) != NONE,
        FOR create WHERE $access = "account",
        FOR update NONE,
        FOR delete WHERE owner = $auth.id;
DEFINE FIELD OVERWRITE owner ON congregation TYPE record<account> VALUE $auth.id READONLY;
DEFINE FIELD OVERWRITE created ON congregation TYPE datetime VALUE $before OR time::now();

-- Members see their own membership; full-access members and the owner see all of the
-- congregation's (to manage them). Only the owner changes roles. Managers remove members; only
-- the owner removes the owner's membership (when they delete the whole congregation).
DEFINE TABLE OVERWRITE membership SCHEMAFULL
    PERMISSIONS
        FOR select WHERE account = $auth.id OR fn::role(congregation) INSIDE ["owner", "full"],
        FOR create WHERE account = $auth.id AND role = "owner" AND congregation.owner = $auth.id,
        FOR update WHERE account = $auth.id OR fn::role(congregation) INSIDE ["owner", "full"],
        FOR delete WHERE congregation.owner = $auth.id
            OR (role != "owner" AND (account = $auth.id OR fn::role(congregation) INSIDE ["owner", "full"]));
DEFINE FIELD OVERWRITE account ON membership TYPE record<account> READONLY;
DEFINE FIELD OVERWRITE congregation ON membership TYPE record<congregation> READONLY;
DEFINE FIELD OVERWRITE role ON membership TYPE string ASSERT $value INSIDE ["owner", "full", "member"]
    PERMISSIONS FOR update WHERE fn::role(congregation) = "owner";
-- The congregation's name, encrypted with this member's account key (hex). Only they change it.
DEFINE FIELD OVERWRITE label ON membership TYPE string PERMISSIONS FOR update WHERE account = $auth.id;
-- The collections a member may write (R-241). Managers set it; the member cannot change it.
-- Rows from before version 3 get an empty list first. ⚠️ SurrealDB 4.0 keeps only the type and
-- default of a DEFINE FIELD with MIGRATE (its permissions are dropped), so the field is defined
-- again in full right after.
DEFINE FIELD OVERWRITE writes ON membership TYPE array<string> DEFAULT [] MIGRATE EXISTING ROWS AS row USING [];
DEFINE FIELD OVERWRITE writes ON membership TYPE array<string> DEFAULT []
    PERMISSIONS FOR update WHERE fn::role(congregation) INSIDE ["owner", "full"];
DEFINE INDEX OVERWRITE membership_unique ON membership FIELDS account, congregation UNIQUE;

-- ----- Invites (5.6) -----
-- The id is SHA-256 of the invite's proof: the server never stores the link's token (R-253).
-- `bundle` (member invites) is sealed with a key only the link gives; the new member deletes it
-- once they have made their grants. Single use: `used_by` is set only by the `join` access.
DEFINE TABLE OVERWRITE invite SCHEMAFULL
    PERMISSIONS
        FOR select WHERE used_by = $auth.id OR fn::role(congregation) INSIDE ["owner", "full"],
        FOR create WHERE fn::role(congregation) INSIDE ["owner", "full"],
        FOR update WHERE used_by = $auth.id OR fn::role(congregation) INSIDE ["owner", "full"],
        FOR delete WHERE fn::role(congregation) INSIDE ["owner", "full"];
DEFINE FIELD OVERWRITE congregation ON invite TYPE record<congregation> READONLY;
DEFINE FIELD OVERWRITE kind ON invite TYPE string READONLY ASSERT $value INSIDE ["full", "member"];
DEFINE FIELD OVERWRITE bundle ON invite TYPE option<string>;
-- The collections the new member may write (copied to their membership when they join).
DEFINE FIELD OVERWRITE writes ON invite TYPE array<string> DEFAULT [] MIGRATE EXISTING ROWS AS row USING [];
DEFINE FIELD OVERWRITE writes ON invite TYPE array<string> DEFAULT [] READONLY;
DEFINE FIELD OVERWRITE expires ON invite TYPE datetime READONLY;
DEFINE FIELD OVERWRITE created_by ON invite TYPE option<record<account>> VALUE $before OR $auth.id;
DEFINE FIELD OVERWRITE used_by ON invite TYPE option<record<account>> PERMISSIONS FOR update NONE;
DEFINE FIELD OVERWRITE used_at ON invite TYPE option<datetime> PERMISSIONS FOR update NONE;

-- ----- Grants (R-321) -----
-- A domain secret X-Wing-encapsulated to one member's public key: only that member opens it.
-- Domain 0 is the root key itself, for the owner and full-access members: with it they open the
-- congregation with their account password only. Members store their own grants (from an invite
-- bundle); full-access members store anyone's.
DEFINE TABLE OVERWRITE grant SCHEMAFULL
    PERMISSIONS
        FOR select WHERE account = $auth.id OR fn::role(congregation) INSIDE ["owner", "full"],
        FOR create, update WHERE (account = $auth.id AND fn::role(congregation) != NONE) OR fn::role(congregation) INSIDE ["owner", "full"],
        FOR delete WHERE account = $auth.id OR fn::role(congregation) INSIDE ["owner", "full"];
DEFINE FIELD OVERWRITE congregation ON grant TYPE record<congregation> READONLY;
DEFINE FIELD OVERWRITE account ON grant TYPE record<account> READONLY;
DEFINE FIELD OVERWRITE domain ON grant TYPE int;
DEFINE FIELD OVERWRITE key_version ON grant TYPE int;
DEFINE FIELD OVERWRITE sealed ON grant TYPE string;

-- ----- A congregation's encrypted data (5.1) -----
-- Ids embed the congregation, so rows cannot collide. The vault header (the root key under the
-- congregation password) is readable by full-access members only (R-256).

DEFINE TABLE OVERWRITE vault_header SCHEMAFULL
    PERMISSIONS
        FOR select, update WHERE fn::role(congregation) INSIDE ["owner", "full"],
        FOR create, delete WHERE congregation.owner = $auth.id;
DEFINE FIELD OVERWRITE congregation ON vault_header TYPE record<congregation> READONLY;
DEFINE FIELD OVERWRITE version ON vault_header TYPE int;
DEFINE FIELD OVERWRITE bytes ON vault_header TYPE string;

-- The domain keys wrapped by the root key. Members read the public keys (sealed submissions).
DEFINE TABLE OVERWRITE keyring SCHEMAFULL
    PERMISSIONS
        FOR select WHERE fn::role(congregation) != NONE,
        FOR create, update, delete WHERE fn::role(congregation) INSIDE ["owner", "full"];
DEFINE FIELD OVERWRITE congregation ON keyring TYPE record<congregation> READONLY;
DEFINE FIELD OVERWRITE domain ON keyring TYPE int;
DEFINE FIELD OVERWRITE key_version ON keyring TYPE int;
DEFINE FIELD OVERWRITE public_key ON keyring TYPE string;
DEFINE FIELD OVERWRITE wrapped_secret ON keyring TYPE string;

-- The change number of a congregation (every write takes the next one).
DEFINE TABLE OVERWRITE counter SCHEMAFULL
    PERMISSIONS
        FOR select, update WHERE fn::role(congregation) != NONE,
        FOR create, delete WHERE congregation.owner = $auth.id;
DEFINE FIELD OVERWRITE congregation ON counter TYPE record<congregation> READONLY;
DEFINE FIELD OVERWRITE seq ON counter TYPE int;

-- The records. `author` and `updated_at` are set by the server, so clients cannot fake them (R-243).
-- Every member reads every record but decrypts only the domains it holds keys for (R-247); it
-- writes only the collections of its membership (fn::can_write, R-241).
DEFINE TABLE OVERWRITE item SCHEMAFULL
    PERMISSIONS
        FOR select WHERE fn::role(congregation) != NONE,
        FOR create, update WHERE fn::can_write(congregation, collection),
        FOR delete WHERE fn::role(congregation) INSIDE ["owner", "full"];
DEFINE FIELD OVERWRITE congregation ON item TYPE record<congregation> READONLY;
DEFINE FIELD OVERWRITE collection ON item TYPE string READONLY;
DEFINE FIELD OVERWRITE record_id ON item TYPE string READONLY;
DEFINE FIELD OVERWRITE scope_id ON item TYPE option<string>;
DEFINE FIELD OVERWRITE version ON item TYPE int;
DEFINE FIELD OVERWRITE seq ON item TYPE int;
DEFINE FIELD OVERWRITE deleted ON item TYPE bool;
DEFINE FIELD OVERWRITE envelope ON item TYPE string;
DEFINE FIELD OVERWRITE updated_at ON item TYPE int VALUE math::floor(time::unix(time::now()) / 60) * 60;
DEFINE FIELD OVERWRITE author ON item TYPE option<record<account>> VALUE $auth.id;
DEFINE INDEX OVERWRITE item_seq ON item FIELDS congregation, seq;

-- ----- Audit log (R-227) -----
-- Metadata only: who changed what, and when. Written by the events below, which run without
-- permission checks, so no client can write, change or delete a line (only the owner, when the
-- whole congregation is deleted). Full-access members and the owner read it. Lines older than a
-- year are removed as the congregation's records change (on every 50th change).
DEFINE TABLE OVERWRITE audit SCHEMAFULL
    PERMISSIONS
        FOR select WHERE fn::role(congregation) INSIDE ["owner", "full"],
        FOR create, update NONE,
        FOR delete WHERE congregation.owner = $auth.id;
DEFINE FIELD OVERWRITE congregation ON audit TYPE record<congregation>;
DEFINE FIELD OVERWRITE actor ON audit TYPE option<record<account>>;
-- create, update, delete
DEFINE FIELD OVERWRITE action ON audit TYPE string;
-- record, member, invite, password, key, grant
DEFINE FIELD OVERWRITE what ON audit TYPE string;
-- For a record: "collection:record id". For a member, invite use or grant: the account's username.
DEFINE FIELD OVERWRITE target ON audit TYPE option<string>;
DEFINE FIELD OVERWRITE at ON audit TYPE datetime;
DEFINE INDEX OVERWRITE audit_at ON audit FIELDS congregation, at;

DEFINE EVENT OVERWRITE audit_item ON item THEN {
    LET $action = IF $event = "UPDATE" AND $after.deleted = true AND $before.deleted = false { "delete" } ELSE { string::lowercase($event) };
    CREATE audit SET congregation = $value.congregation, actor = $auth.id, action = $action, what = "record",
        target = string::concat($value.collection, ":", $value.record_id), at = time::now();
    -- Retention: on every 50th change only (cheaper than on each write, and still keeps it short).
    IF $value.seq % 50 = 0 { DELETE audit WHERE congregation = $value.congregation AND at < time::now() - 365d; };
};
DEFINE EVENT OVERWRITE audit_membership ON membership THEN {
    CREATE audit SET congregation = $value.congregation, actor = $auth.id, action = string::lowercase($event), what = "member",
        target = string::concat($value.account.username, " (", $value.role, ")"), at = time::now();
};
DEFINE EVENT OVERWRITE audit_invite ON invite THEN {
    LET $action = IF $event = "UPDATE" AND $after.used_by != NONE AND $before.used_by = NONE { "use" } ELSE { string::lowercase($event) };
    CREATE audit SET congregation = $value.congregation, actor = $auth.id ?? $value.used_by, action = $action, what = "invite",
        target = $value.kind, at = time::now();
};
DEFINE EVENT OVERWRITE audit_vault ON vault_header THEN {
    CREATE audit SET congregation = $value.congregation, actor = $auth.id, action = string::lowercase($event), what = "password", at = time::now();
};
DEFINE EVENT OVERWRITE audit_keyring ON keyring THEN {
    CREATE audit SET congregation = $value.congregation, actor = $auth.id, action = string::lowercase($event), what = "key",
        target = string::concat(<string> $value.domain, ".", <string> $value.key_version), at = time::now();
};
DEFINE EVENT OVERWRITE audit_grant ON grant THEN {
    CREATE audit SET congregation = $value.congregation, actor = $auth.id, action = string::lowercase($event), what = "grant",
        target = string::concat($value.account.username, " ", <string> $value.domain, ".", <string> $value.key_version), at = time::now();
};
