//! App state: accounts, congregations and the open congregation's data.
//!
//! **Local** congregations are real: they are created, saved and unlocked through
//! `theo_core::store::Store`, encrypted in their own database on this device (`local.rs`). Every
//! change to the data is saved right away, and only what changed is written. Locking drops the
//! keys and the data from memory (R-310).
//!
//! **Cloud** congregations are real too: they live on the SurrealDB server of the signed-in Cloud
//! account (`theo_db::surreal`), in the server's one shared database, kept apart by its
//! permissions. The app signs in as an ordinary account with the auth key derived from the
//! account password; it never has the server's owner credentials, and the session token stays in
//! memory. Each member keeps the congregation's name encrypted with their own account key.
//!
//! **Invites** (5.6): an administrator with full access makes an invite for a person; the link is
//! shown once. The new member joins with it ([`AppState::join`]). A member opens with the domain
//! keys of their grants, and writes only the collections their permissions allow (R-241). The
//! owner and full-access members open with the root key granted to their account (after their
//! first unlock with the congregation password), so day-to-day they need only their account
//! password (6.2).
//!
//! **Removing a member** ([`AppState::remove_member`], R-246): the server drops their membership
//! and grants at once; then this app makes new versions of the domains they held and grants them
//! to the remaining members. For a full-access member it also replaces the root key and the
//! congregation password.
//!
//! ⚠️ MOCK still: the two demo congregations live in memory only, with their password compared in
//! plain text. Screens talk only to [`AppState`], so this does not change them.

use dioxus::prelude::*;
use std::collections::HashMap;
use std::rc::Rc;
use theo_core::store::{Store, StoreError};
use theo_core::invites::{domains_for, writes_for, InviteLink, INVITE_DAYS};
use theo_crypto::invite::{Bundle, InviteToken};
use theo_crypto::{AccountKeys, DomainId, PersonalKeys};
use theo_db::surreal::{AuditEntry, InviteState, Server, Session};

/// The signed-in Cloud account: its server session and its keys (in memory only).
#[derive(Clone)]
pub struct CloudAccount {
    session: Session,
    username: String,
    keys: Rc<AccountKeys>,
    /// The personal keys (grants are made to them).
    personal: Rc<PersonalKeys>,
}

fn storage(e: impl std::fmt::Display) -> AuthError {
    AuthError::Storage(e.to_string())
}
use crate::people::Directory;
use crate::prefs::{Overrides, Preferences};
use theo_ui::{FilterSelection, Icon, Theme, WidgetLayout};

/// Where an account lives.
#[derive(Clone, Debug, PartialEq)]
pub enum AccountKind {
    /// A profile on this device (Local congregations only).
    Device,
    /// An account on a SurrealDB server.
    Cloud { server: String },
}

/// A user account.
#[derive(Clone, Debug, PartialEq)]
pub struct Account {
    pub username: String,
    pub display_name: String,
    pub kind: AccountKind,
    /// Display preferences that override the congregation defaults (R-062).
    pub overrides: Overrides,
    password: String, // MOCK ONLY (demo and Cloud accounts)
    /// Device accounts: the hex auth key (`local.rs`); the password is not kept.
    auth: Option<String>,
}

/// Where a congregation's data lives (REQUIREMENTS.md 1.2).
#[derive(Clone, Debug, PartialEq)]
pub enum StorageMode {
    Local,
    Cloud { server: String },
}

impl StorageMode {
    pub fn icon(&self) -> Icon {
        match self {
            StorageMode::Local => Icon::Local,
            StorageMode::Cloud { .. } => Icon::Cloud,
        }
    }

    pub fn label(&self) -> String {
        match self {
            StorageMode::Local => "On this device".into(),
            StorageMode::Cloud { .. } => "Cloud".into(),
        }
    }
}

/// A weekly meeting slot.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct MeetingSlot {
    pub day: String,
    pub time: String,
}

/// The minimum data of a congregation (see the data-modeling questions in the README).
#[derive(Clone, Debug, PartialEq)]
pub struct Congregation {
    pub id: String,
    pub name: String,
    pub number: String,
    pub language: String,
    pub time_zone: String,
    pub midweek: MeetingSlot,
    pub weekend: MeetingSlot,
    pub mode: StorageMode,
    pub owner: String,
    pub biometric: bool,
    /// Default display preferences for everyone in the congregation (R-062).
    pub defaults: Preferences,
    password: String, // MOCK ONLY (demo and Cloud): the real app never stores it (R-124)
    /// A Local congregation stored encrypted on this device (`local.rs`), not a mock.
    pub stored: bool,
    /// A Cloud member without full access: opens with the keys of their grants, no password.
    pub member: bool,
    /// A Cloud member's collections to write (R-241). Empty for everyone else.
    pub writes: Vec<String>,
    /// The owner or a full-access member who holds a grant of the root key: opens without the
    /// congregation password.
    pub root_grant: bool,
}

/// New congregation input from the wizard.
#[derive(Clone, Debug, PartialEq)]
pub struct CongregationDraft {
    pub name: String,
    pub number: String,
    pub language: String,
    pub time_zone: String,
    pub midweek: MeetingSlot,
    pub weekend: MeetingSlot,
    pub mode: StorageMode,
    pub password: String,
    /// The creator, added to All users, linked to their account and administrator.
    pub my_first_name: String,
    pub my_last_name: String,
    /// `"male"` or `"female"`.
    pub my_gender: String,
}

/// Errors shown to the user.
#[derive(Clone, Debug, PartialEq)]
pub enum AuthError {
    WrongCredentials,
    UsernameTaken,
    WrongPassword,
    /// The device storage failed.
    Storage(String),
}

impl std::fmt::Display for AuthError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            AuthError::WrongCredentials => "Username or password is not correct.",
            AuthError::UsernameTaken => "This username is already taken.",
            AuthError::WrongPassword => "This password does not unlock the congregation.",
            AuthError::Storage(e) => e,
        })
    }
}

/// Shown when a build has no Cloud server (`cloud.rs`).
const NO_CLOUD: &str = "This version of the app has no Cloud service.";

/// What opening a congregation asks for ([`AppState::secret_for`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Secret {
    /// Nothing: the signed-in account's keys open it.
    None,
    /// The congregation password (or its passkey).
    Congregation,
    /// The account password (or the account's passkey): a Cloud congregation after a restart.
    Account,
}

/// Every key domain (6.2).
const ALL_DOMAINS: &[DomainId] = &[DomainId::SCHEDULE, DomainId::PEOPLE, DomainId::REPORTS, DomainId::TERRITORIES];

/// Global app state, provided once in `App`.
#[derive(Clone, Copy)]
pub struct AppState {
    accounts: Signal<Vec<Account>>,
    congregations: Signal<Vec<Congregation>>,
    /// Username of the signed-in account.
    pub session: Signal<Option<String>>,
    /// Id of the unlocked congregation.
    pub unlocked: Signal<Option<String>>,
    /// Congregation to preselect on the unlock screen.
    pub preselect: Signal<Option<String>>,
    pub theme: Signal<Theme>,
    layouts: Signal<HashMap<String, Vec<WidgetLayout>>>,
    /// People, families and groups per congregation (MOCK of M-01 / M-09).
    directories: Signal<HashMap<String, Directory>>,
    /// The open stores of unlocked congregations (their keys are in memory only while open).
    stores: Signal<HashMap<String, Rc<Store>>>,
    /// The signed-in Cloud account, if any.
    cloud: Signal<Option<CloudAccount>>,
    /// The names of Cloud congregations as stored in this account's memberships, to update them
    /// on the server when a congregation is renamed.
    cloud_names: Signal<HashMap<String, String>>,
    /// Invites this device just used, by congregation: when it is opened, the person of the invite
    /// is linked to this account.
    joined: Signal<HashMap<String, String>>,
    /// True while the app reopens the congregation of *Stay unlocked* after a reload.
    pub restoring: Signal<bool>,
    /// A message for the next screen (for example why passkey unlock could not be set up).
    pub notice: Signal<Option<String>>,
    /// The open congregation's invites on the server, by id (administrators: expiry and use).
    pub invite_states: Signal<HashMap<String, InviteState>>,
    /// Search and filters of the All users list. Kept while you open a person and come back.
    pub user_query: Signal<String>,
    pub user_filters: Signal<FilterSelection>,
    /// The tab of All users: `"people"` or `"families"`.
    pub users_tab: Signal<String>,
    /// Field service reports page: tab, month (None = the reporting month), search and filters.
    pub report_tab: Signal<String>,
    pub report_month: Signal<Option<crate::reports::Ym>>,
    pub report_query: Signal<String>,
    pub report_filters: Signal<FilterSelection>,
    /// Service year of the Statistics and Goals tabs (None = the current one).
    pub report_year: Signal<Option<i32>>,
    /// Public witnessing: list, calendar, presets or statistics, and the month (None = this month).
    pub pw_view: Signal<String>,
    pub pw_month: Signal<Option<(i32, u32)>>,
    /// Meetings for field service: list, calendar or presets, and the month (None = this month).
    pub service_view: Signal<String>,
    pub service_month: Signal<Option<(i32, u32)>>,
    /// Events page: list or calendar, and the month of the calendar (None = this month).
    pub events_view: Signal<String>,
    pub events_month: Signal<Option<(i32, u32)>>,
    /// Meeting attendance page: tab and service year (None = the current one).
    pub attendance_tab: Signal<String>,
    pub attendance_year: Signal<Option<i32>>,
    /// Territories page: tab, the part of Assignments, search and filters, and list or map.
    pub terr_tab: Signal<String>,
    pub terr_part: Signal<String>,
    pub terr_query: Signal<String>,
    pub terr_filters: Signal<FilterSelection>,
    pub terr_view: Signal<String>,
    /// Midweek meeting page: tab, the week shown (its Monday) and the month.
    pub mw_tab: Signal<String>,
    pub mw_monday: Signal<Option<String>>,
    pub mw_month: Signal<Option<(i32, u32)>>,
    /// Workbooks read on this device (never uploaded, R-032), by `issue-lang`.
    workbooks: Signal<HashMap<String, crate::workbook::Workbook>>,
}

impl AppState {
    pub fn new() -> Self {
        let mut state = Self {
            accounts: Signal::new(Vec::new()),
            congregations: Signal::new(Vec::new()),
            session: Signal::new(None),
            unlocked: Signal::new(None),
            preselect: Signal::new(None),
            theme: Signal::new(Theme::System),
            layouts: Signal::new(HashMap::new()),
            directories: Signal::new(HashMap::new()),
            stores: Signal::new(HashMap::new()),
            cloud: Signal::new(None),
            cloud_names: Signal::new(HashMap::new()),
            joined: Signal::new(HashMap::new()),
            invite_states: Signal::new(HashMap::new()),
            restoring: Signal::new(false),
            notice: Signal::new(None),
            user_query: Signal::new(String::new()),
            user_filters: Signal::new(crate::people::default_filters()),
            users_tab: Signal::new("people".into()),
            report_tab: Signal::new("month".into()),
            report_month: Signal::new(None),
            report_query: Signal::new(String::new()),
            report_filters: Signal::new(FilterSelection::new()),
            report_year: Signal::new(None),
            pw_view: Signal::new("list".into()),
            pw_month: Signal::new(None),
            service_view: Signal::new("list".into()),
            service_month: Signal::new(None),
            events_view: Signal::new("list".into()),
            events_month: Signal::new(None),
            attendance_tab: Signal::new("months".into()),
            attendance_year: Signal::new(None),
            terr_tab: Signal::new("assignments".into()),
            terr_part: Signal::new("assigned".into()),
            terr_query: Signal::new(String::new()),
            terr_filters: Signal::new(FilterSelection::new()),
            terr_view: Signal::new("list".into()),
            mw_tab: Signal::new("week".into()),
            mw_monday: Signal::new(None),
            mw_month: Signal::new(None),
            workbooks: Signal::new(HashMap::new()),
        };
        state.load_device();
        state.restore_session();
        state
    }

    /// The accounts and Local congregations stored on this device.
    fn load_device(&mut self) {
        let reg = crate::local::Registry::load();
        for a in reg.accounts {
            self.accounts.write().push(Account {
                overrides: load_overrides(&a.username),
                username: a.username,
                display_name: a.display_name,
                kind: AccountKind::Device,
                password: String::new(),
                auth: Some(a.auth),
            });
        }
        for c in reg.congregations {
            // Only the name is known until it is unlocked; the rest is encrypted.
            self.congregations.write().push(Congregation {
                id: c.id,
                name: c.name,
                number: String::new(),
                language: "en".into(),
                time_zone: String::new(),
                midweek: MeetingSlot { day: "Thursday".into(), time: "19:00".into() },
                weekend: MeetingSlot { day: "Sunday".into(), time: "10:00".into() },
                mode: StorageMode::Local,
                owner: c.owner,
                biometric: false,
                defaults: Preferences::default(),
                password: String::new(),
                stored: true,
                member: false,
                writes: Vec::new(),
                root_grant: false,
            });
        }
    }

    // ----- accounts -----

    pub fn sign_up(
        &mut self,
        username: &str,
        display_name: &str,
        password: &str,
        kind: AccountKind,
    ) -> Result<(), AuthError> {
        let username = username.trim().to_lowercase();
        if self.accounts.read().iter().any(|a| a.username == username) {
            return Err(AuthError::UsernameTaken);
        }
        // A device account is kept on this device, by its auth key only.
        let auth = match kind {
            AccountKind::Device => {
                let auth = crate::local::auth_hex(&username, password).ok_or_else(|| AuthError::Storage("The password could not be processed.".into()))?;
                let display = display_name.trim().to_string();
                let (u, a) = (username.clone(), auth.clone());
                crate::local::Registry::update(|r| r.accounts.push(crate::local::LocalAccount { username: u, display_name: display, auth: a }));
                Some(auth)
            }
            AccountKind::Cloud { .. } => None,
        };
        self.accounts.write().push(Account {
            username: username.clone(),
            display_name: display_name.trim().to_string(),
            overrides: load_overrides(&username),
            kind,
            password: if auth.is_some() { String::new() } else { password.to_string() },
            auth,
        });
        self.session.set(Some(username.clone()));
        self.unlocked.set(None);
        self.remember_account(Some(&username));
        Ok(())
    }

    pub fn sign_in(&mut self, username: &str, password: &str) -> Result<(), AuthError> {
        let username = username.trim().to_lowercase();
        let account = self.accounts.read().iter().find(|a| a.username == username).cloned();
        let ok = match account {
            Some(Account { auth: Some(auth), .. }) => crate::local::auth_hex(&username, password).is_some_and(|a| a == auth),
            Some(a) => a.password == password,
            None => false,
        };
        if !ok {
            return Err(AuthError::WrongCredentials);
        }
        self.session.set(Some(username.clone()));
        self.unlocked.set(None);
        self.remember_account(Some(&username));
        Ok(())
    }

    pub fn sign_out(&mut self) {
        self.forget_session();
        crate::platform::device_set(ACCOUNT_KEY, None);
        self.unlocked.set(None);
        self.session.set(None);
        self.preselect.set(None);
        // Keys and decrypted data leave memory; Cloud congregations leave the list until the next
        // sign-in.
        let stored: Vec<String> = self.congregations.peek().iter().filter(|c| c.stored).map(|c| c.id.clone()).collect();
        for id in stored {
            self.stores.write().remove(&id);
            self.directories.write().remove(&id);
        }
        self.congregations.write().retain(|c| !(c.stored && matches!(c.mode, StorageMode::Cloud { .. })));
        self.cloud.set(None);
        self.cloud_names.write().clear();
    }

    // ----- Cloud accounts -----

    /// Creates an account on the Cloud service and signs it in (slow on purpose: Argon2id).
    pub async fn sign_up_cloud(&mut self, username: &str, display_name: &str, password: &str) -> Result<(), AuthError> {
        let server = crate::cloud::server().ok_or(AuthError::Storage(NO_CLOUD.into()))?;
        self.sign_up_cloud_at(&server, username, display_name, password).await
    }

    async fn sign_up_cloud_at(&mut self, server: &Server, username: &str, display_name: &str, password: &str) -> Result<(), AuthError> {
        let (session, keys) = Self::cloud_session(server, username, password, Some(display_name)).await?;
        self.start_cloud(server, &username.trim().to_lowercase(), session, keys).await
    }

    /// Signs in to the Cloud service and lists the account's congregations.
    pub async fn sign_in_cloud(&mut self, username: &str, password: &str) -> Result<(), AuthError> {
        let server = crate::cloud::server().ok_or(AuthError::Storage(NO_CLOUD.into()))?;
        self.sign_in_cloud_at(&server, username, password).await
    }

    async fn sign_in_cloud_at(&mut self, server: &Server, username: &str, password: &str) -> Result<(), AuthError> {
        let (session, keys) = Self::cloud_session(server, username, password, None).await?;
        self.start_cloud(server, &username.trim().to_lowercase(), session, keys).await
    }

    /// Signs in to the Cloud service (or, with `new_name`, creates the account there first),
    /// without changing anything in the app yet.
    async fn cloud_session(server: &Server, username: &str, password: &str, new_name: Option<&str>) -> Result<(Session, AccountKeys), AuthError> {
        let username = username.trim().to_lowercase();
        let keys = crate::local::cloud_keys(server, &username, password).ok_or_else(|| AuthError::Storage("The password could not be processed.".into()))?;
        let auth = crate::local::to_hex(keys.auth_key());
        let session = match new_name {
            Some(_) => theo_db::surreal::sign_up(server, &username, &auth).await.map_err(storage)?,
            None => theo_db::surreal::sign_in(server, &username, &auth).await.map_err(|e| match e {
                theo_db::DbError::Auth(_) => AuthError::WrongCredentials,
                e => storage(e),
            })?,
        };
        session.check_schema().await.map_err(storage)?;
        if let Some(name) = new_name {
            // The display name is kept on the server, encrypted with the account key.
            let profile = keys.seal_label(name.trim()).map_err(storage)?;
            session.set_profile(&crate::local::to_hex(&profile)).await.map_err(storage)?;
        }
        Ok((session, keys))
    }

    /// Signs in to the Cloud service with keys kept on this device (passkey, *Stay unlocked*).
    async fn sign_in_with_keys(&mut self, username: &str, keys: AccountKeys) -> Result<(), AuthError> {
        let server = crate::cloud::server().ok_or(AuthError::Storage(NO_CLOUD.into()))?;
        let session = theo_db::surreal::sign_in(&server, username, &crate::local::to_hex(keys.auth_key())).await.map_err(|e| match e {
            theo_db::DbError::Auth(_) => AuthError::WrongCredentials,
            e => storage(e),
        })?;
        session.check_schema().await.map_err(storage)?;
        self.start_cloud(&server, username, session, keys).await
    }

    /// After sign-in: the account's name and personal keys, then its congregations. Any open
    /// congregation is closed.
    async fn start_cloud(&mut self, server: &Server, username: &str, session: Session, keys: AccountKeys) -> Result<(), AuthError> {
        let (account, display_name) = Self::cloud_account(username, session, keys).await?;
        self.use_cloud_account(server, account, display_name, true).await
    }

    /// The account's display name and personal keys (made on its first sign-in), without changing
    /// anything in the app yet.
    async fn cloud_account(username: &str, session: Session, keys: AccountKeys) -> Result<(CloudAccount, String), AuthError> {
        let data = session.account_data().await.map_err(storage)?;
        let display_name = data.profile.as_deref().and_then(crate::local::from_hex).and_then(|b| keys.open_label(&b).ok()).unwrap_or_else(|| username.to_string());
        let personal = match data.keys.as_deref().and_then(crate::local::from_hex) {
            Some(wrapped) => PersonalKeys::unwrap(&keys, username, &wrapped).map_err(storage)?,
            None => {
                let personal = PersonalKeys::generate().map_err(storage)?;
                let wrapped = personal.wrap(&keys, username).map_err(storage)?;
                session.set_personal_keys(&crate::local::to_hex(&wrapped), &crate::local::to_hex(&personal.public().to_bytes())).await.map_err(storage)?;
                personal
            }
        };
        Ok((CloudAccount { session, username: username.to_string(), keys: Rc::new(keys), personal: Rc::new(personal) }, display_name))
    }

    /// The app continues as this Cloud account. With `close`, any open congregation is closed
    /// (a normal sign-in); without, it stays open (moving it to the Cloud).
    async fn use_cloud_account(&mut self, server: &Server, account: CloudAccount, display_name: String, close: bool) -> Result<(), AuthError> {
        let username = account.username.clone();
        self.accounts.write().retain(|a| a.username != username);
        self.accounts.write().push(Account {
            overrides: load_overrides(&username),
            username: username.clone(),
            display_name,
            kind: AccountKind::Cloud { server: server.url.clone() },
            password: String::new(),
            auth: None,
        });
        self.cloud.set(Some(account));
        self.refresh_cloud_congregations().await?;
        // Signed in only now: screens that watch the session see the congregations at once.
        self.session.set(Some(username));
        if close {
            self.unlocked.set(None);
        }
        Ok(())
    }

    /// Lists the Cloud account's congregations again (after sign-in, or after joining one).
    async fn refresh_cloud_congregations(&mut self) -> Result<(), AuthError> {
        let Some(account) = self.cloud.peek().clone() else { return Ok(()) };
        let server = account.session.server.clone();
        let username = account.username.clone();
        let memberships = account.session.memberships().await.map_err(storage)?;
        let mut list = Vec::new();
        for m in memberships {
            let name = crate::local::from_hex(&m.label).and_then(|b| account.keys.open_label(&b).ok()).unwrap_or_else(|| "Congregation".into());
            self.cloud_names.write().insert(m.congregation.clone(), name.clone());
            list.push(Congregation {
                id: m.congregation,
                member: m.role == "member",
                writes: m.writes,
                root_grant: m.root_grant,
                name,
                number: String::new(),
                language: "en".into(),
                time_zone: String::new(),
                midweek: MeetingSlot { day: "Thursday".into(), time: "19:00".into() },
                weekend: MeetingSlot { day: "Sunday".into(), time: "10:00".into() },
                mode: StorageMode::Cloud { server: server.url.clone() },
                owner: username.clone(),
                biometric: false,
                defaults: Preferences::default(),
                password: String::new(),
                stored: true,
            });
        }
        self.congregations.write().retain(|c| !(c.stored && matches!(c.mode, StorageMode::Cloud { .. })));
        self.congregations.write().extend(list);
        self.remember_account(Some(&username));
        Ok(())
    }

    /// Remembers who is signed in on this device, until sign-out (6.5): the app then opens on the
    /// congregation choice, without the log-in screen. Never keys: a Cloud account keeps its
    /// display name and its congregation list, sealed with the device key; opening a Cloud
    /// congregation still asks for the fingerprint or the account password.
    fn remember_account(&self, username: Option<&str>) {
        let Some(user) = username.map(String::from).or_else(|| self.session.peek().clone()) else { return };
        let Some(account) = self.accounts.peek().iter().find(|a| a.username == user).cloned() else { return };
        match account.kind {
            AccountKind::Device => {
                let record = serde_json::json!({ "username": user, "kind": "device" });
                crate::platform::device_set(ACCOUNT_KEY, Some(&record.to_string()));
            }
            AccountKind::Cloud { .. } => {
                let list: Vec<serde_json::Value> = self
                    .congregations
                    .peek()
                    .iter()
                    .filter(|c| c.stored && c.owner == user && matches!(c.mode, StorageMode::Cloud { .. }))
                    .map(|c| serde_json::json!({ "id": c.id, "name": c.name, "member": c.member, "root_grant": c.root_grant, "writes": c.writes }))
                    .collect();
                let plain = serde_json::json!({ "display_name": account.display_name, "congregations": list }).to_string();
                let purpose = format!("account:{user}");
                dioxus::dioxus_core::spawn_forever(async move {
                    if let Some(sealed) = crate::platform::device_seal(&purpose, plain.as_bytes()).await {
                        let record = serde_json::json!({ "username": user, "kind": "cloud", "sealed": sealed });
                        crate::platform::device_set(ACCOUNT_KEY, Some(&record.to_string()));
                    }
                });
            }
        }
    }

    /// After a restart: the remembered account is signed in again, its congregations locked.
    fn restore_account(&mut self) {
        let Some(record) = crate::platform::device_get(ACCOUNT_KEY).and_then(|t| serde_json::from_str::<serde_json::Value>(&t).ok()) else { return };
        let user = record.get("username").and_then(|v| v.as_str()).unwrap_or_default().to_string();
        match record.get("kind").and_then(|v| v.as_str()) {
            Some("device") if self.accounts.peek().iter().any(|a| a.username == user && a.kind == AccountKind::Device) => {
                self.session.set(Some(user));
            }
            Some("cloud") => {
                let Some(server) = crate::cloud::server() else { return };
                let sealed = record.get("sealed").and_then(|v| v.as_str()).unwrap_or_default().to_string();
                self.restoring.set(true);
                let mut state = *self;
                dioxus::dioxus_core::spawn_forever(async move {
                    if let Some(plain) = crate::platform::device_open(&format!("account:{user}"), &sealed).await
                        && let Ok(v) = serde_json::from_slice::<serde_json::Value>(&plain)
                    {
                        state.restore_cloud_account(&server, &user, &v);
                    }
                    state.restoring.set(false);
                });
            }
            _ => crate::platform::device_set(ACCOUNT_KEY, None),
        }
    }

    /// The remembered Cloud account and its congregation list, without keys.
    fn restore_cloud_account(&mut self, server: &Server, user: &str, v: &serde_json::Value) {
        let s = |x: &serde_json::Value, k: &str| x.get(k).and_then(|y| y.as_str()).unwrap_or_default().to_string();
        self.accounts.write().retain(|a| a.username != user);
        self.accounts.write().push(Account {
            username: user.to_string(),
            display_name: s(v, "display_name"),
            kind: AccountKind::Cloud { server: server.url.clone() },
            overrides: load_overrides(user),
            password: String::new(),
            auth: None,
        });
        let list = v.get("congregations").and_then(|l| l.as_array()).cloned().unwrap_or_default();
        self.congregations.write().retain(|c| !(c.stored && matches!(c.mode, StorageMode::Cloud { .. })));
        for c in list {
            self.congregations.write().push(Congregation {
                id: s(&c, "id"),
                name: s(&c, "name"),
                number: String::new(),
                language: "en".into(),
                time_zone: String::new(),
                midweek: MeetingSlot { day: "Thursday".into(), time: "19:00".into() },
                weekend: MeetingSlot { day: "Sunday".into(), time: "10:00".into() },
                mode: StorageMode::Cloud { server: server.url.clone() },
                owner: user.to_string(),
                biometric: false,
                defaults: Preferences::default(),
                password: String::new(),
                stored: true,
                member: c.get("member").and_then(|b| b.as_bool()).unwrap_or(false),
                writes: c.get("writes").and_then(|w| w.as_array()).map(|w| w.iter().filter_map(|x| x.as_str().map(String::from)).collect()).unwrap_or_default(),
                root_grant: c.get("root_grant").and_then(|b| b.as_bool()).unwrap_or(false),
            });
        }
        self.session.set(Some(user.to_string()));
    }

    // ----- invites (5.6) -----

    /// The signed-in Cloud account and the open Cloud congregation, for an administrator with
    /// full access (they hold the root key, so they can give keys away).
    fn managed_congregation(&self) -> Result<(CloudAccount, Congregation, Rc<Store>, theo_db::RecordId), String> {
        let c = self.active_congregation().filter(|c| c.stored && matches!(c.mode, StorageMode::Cloud { .. })).ok_or("Invites are for Cloud congregations.")?;
        let account = self.cloud.peek().clone().ok_or("Sign in to the server again.")?;
        let store = self.stores.peek().get(&c.id).cloned().ok_or("Unlock the congregation again.")?;
        if !store.has_root() {
            return Err("Only members with full access can invite.".into());
        }
        let rid = theo_db::RecordId::from_hex(&c.id).ok_or("This congregation has no valid id.")?;
        Ok((account, c, store, rid))
    }

    /// Makes an invite for `person` and returns its link, to send privately. `preset` is a
    /// permission preset (`access::PRESETS`), or `"admin"` for a full-access invite (they will
    /// also need the congregation password). The link is not kept: only the invite's id is.
    pub async fn create_invite(&mut self, person: u32, preset: &str) -> Result<String, String> {
        let (account, c, store, rid) = self.managed_congregation()?;
        let (kind, access, label) = if preset == "admin" {
            ("full", crate::access::Access::administrator(), "Administrator, full access".to_string())
        } else {
            let p = crate::access::preset(preset).ok_or("Choose what they may do.")?;
            ("member", (p.access)(), p.label.to_string())
        };
        let token = InviteToken::generate().map_err(|e| e.to_string())?;
        // A member invite carries the keys of the domains its permissions read; a full-access
        // invite carries none (the congregation password gives them).
        let secrets = if kind == "member" { store.domain_secrets(&domains_for(&access)) } else { Vec::new() };
        let bundle = token.seal_bundle(&c.id, &Bundle { name: c.name.clone(), secrets }).map_err(|e| e.to_string())?;
        let writes = if kind == "member" { writes_for(&access) } else { Vec::new() };
        account.session.create_invite(rid, &token.id(), kind, &crate::local::to_hex(&bundle), &writes, INVITE_DAYS).await.map_err(|e| e.to_string())?;
        let link = InviteLink::new(&token.to_text()).to_url(&crate::platform::app_address());
        let invite = token.id();
        let name = self.directory().person(person).map(|p| format!("{} {}", p.first_name, p.last_name)).unwrap_or_default();
        self.update_directory(|d| {
            d.set_account(person, crate::people::AccountLink::Invited { link: String::new(), invite });
            // Kept for when they join (permissions count only for linked accounts).
            d.access.insert(person, access);
        });
        self.log(&format!("Invited {name} ({label})."));
        Ok(link)
    }

    /// Cancels the open invite of `person`.
    pub async fn revoke_invite(&mut self, person: u32) -> Result<(), String> {
        let invite = match self.directory().person(person).map(|p| p.account.clone()) {
            Some(crate::people::AccountLink::Invited { invite, .. }) => invite,
            _ => return Ok(()),
        };
        if !invite.is_empty() {
            let (account, ..) = self.managed_congregation()?;
            account.session.revoke_invite(&invite).await.map_err(|e| e.to_string())?;
        }
        let name = self.directory().person(person).map(|p| format!("{} {}", p.first_name, p.last_name)).unwrap_or_default();
        self.update_directory(|d| {
            d.set_account(person, crate::people::AccountLink::None);
            d.access.remove(&person);
        });
        self.log(&format!("Cancelled the invite of {name}."));
        Ok(())
    }

    /// Links people whose invite was used to the account that used it (administrators with full
    /// access, when they open the congregation). Returns how many were linked.
    pub async fn sync_invites(&mut self) -> Result<usize, String> {
        let (account, _, _, rid) = self.managed_congregation()?;
        let open: Vec<(u32, String)> = self
            .directory()
            .people
            .iter()
            .filter_map(|p| match &p.account {
                crate::people::AccountLink::Invited { invite, .. } if !invite.is_empty() => Some((p.id, invite.clone())),
                _ => None,
            })
            .collect();
        let states = account.session.invites(rid).await.map_err(|e| e.to_string())?;
        self.invite_states.set(states.iter().map(|s| (s.id.clone(), s.clone())).collect());
        let used: Vec<(u32, String)> =
            open.into_iter().filter_map(|(p, id)| states.iter().find(|s| s.id == id).and_then(|s| s.used_by.clone()).map(|u| (p, u))).collect();
        let n = used.len();
        if n > 0 {
            self.update_directory(|d| {
                for (p, username) in used {
                    d.set_account(p, crate::people::AccountLink::Linked { username });
                }
            });
        }
        Ok(n)
    }

    /// Lines of the server's audit log of the open congregation, newest first (R-227).
    pub async fn audit_log(&self, start: u32, limit: u32) -> Result<Vec<AuditEntry>, String> {
        let (account, _, _, rid) = self.managed_congregation()?;
        account.session.audit(rid, start, limit).await.map_err(|e| e.to_string())
    }

    // ----- people leaving: their account and invite end with them -----

    /// Before people leave the data (deleted, or moved away): in a Cloud congregation, someone
    /// whose account is still a member would keep their access, since the server knows nothing
    /// of the people list. So that is refused until the account is removed (*Remove account*,
    /// which also replaces the keys they held). None when nothing stands in the way.
    pub fn account_still_member(&self, people: &[u32]) -> Option<String> {
        self.active_congregation().filter(|c| c.stored && matches!(c.mode, StorageMode::Cloud { .. }))?;
        let me = self.session.peek().clone().unwrap_or_default();
        let dir = self.directory();
        people.iter().filter_map(|n| dir.person(*n)).find(|p| matches!(&p.account, crate::people::AccountLink::Linked { username } if *username != me)).map(|p| {
            format!(
                "{} {} still has an account in this congregation. Remove it first (their page › Account › Remove account): that ends their access and replaces the keys they held. Then delete them or mark them moved away.",
                p.first_name, p.last_name
            )
        })
    }

    /// Cancels the open invites of these people on the server (they are deleted or moved away, so
    /// the link must stop working).
    pub async fn cancel_invites(&mut self, people: &[u32]) -> Result<(), String> {
        for p in people {
            let invited = self.directory().person(*p).is_some_and(|x| matches!(&x.account, crate::people::AccountLink::Invited { invite, .. } if !invite.is_empty()));
            if invited {
                self.revoke_invite(*p).await?;
            }
        }
        Ok(())
    }

    // ----- export, import and moving between Local and Cloud (D-51) -----

    /// May this account export, import and move the open congregation? Local: its user. Cloud:
    /// the owner and full-access members (they hold the root key). The demo: yes.
    pub fn can_manage_data(&self) -> bool {
        match self.active_congregation() {
            Some(c) if !c.stored || c.mode == StorageMode::Local => true,
            Some(_) => self.manages_cloud(),
            None => false,
        }
    }

    /// Is the signed-in account the owner of the open Cloud congregation? (Only the owner moves
    /// it to a device: that deletes it on the server for everyone.)
    pub async fn owns_cloud_congregation(&self) -> bool {
        let Ok((account, _, _, rid)) = self.managed_congregation() else { return false };
        account.session.members(rid).await.is_ok_and(|m| m.iter().any(|x| x.username == account.username && x.role == "owner"))
    }

    /// Moves the open Local congregation to the Cloud service: signs in to (or, with `new_name`,
    /// creates) the Cloud account, copies everything there with the same keys and congregation
    /// password, makes it its owner, and only then deletes the copy on this device. On a failure
    /// before the copy is complete, nothing changes and the app is back on the device account.
    pub async fn move_to_cloud(&mut self, username: &str, password: &str, new_name: Option<&str>) -> Result<(), String> {
        if !self.can_manage_data() {
            return Err("Only an administrator can move the congregation.".into());
        }
        let c = self.active_congregation().filter(|c| c.stored && c.mode == StorageMode::Local).ok_or("Only a congregation on this device can move to the Cloud.")?;
        let rid = theo_db::RecordId::from_hex(&c.id).ok_or("This congregation has no valid id.")?;
        let server = crate::cloud::server().ok_or(NO_CLOUD)?;
        let device_user = self.session.peek().clone().unwrap_or_default();
        // 1. The Cloud account, and everything on the server: the app itself does not change yet,
        //    so the congregation stays open here and nothing is lost on a failure.
        let (session, keys) = Self::cloud_session(&server, username, password, new_name).await.map_err(|e| e.to_string())?;
        let (account, display_name) = Self::cloud_account(&username.trim().to_lowercase(), session, keys).await.map_err(|e| e.to_string())?;
        let (copy, dir) = {
            let store = self.stores.peek().get(&c.id).cloned().ok_or("Unlock the congregation again.")?;
            self.copy_to_cloud(&c, &store, rid, self.directory(), &device_user, &account).await.map_err(|e| format!("Nothing was moved: {e}"))?
        };
        // 2. One switch: the Cloud copy is the open congregation, as the Cloud account.
        let gone = c.id.clone();
        crate::local::Registry::update(|r| r.congregations.retain(|x| x.id != gone));
        self.stores.write().insert(c.id.clone(), Rc::new(copy));
        self.directories.write().insert(c.id.clone(), dir);
        self.cloud_names.write().insert(c.id.clone(), c.name.clone());
        self.use_cloud_account(&server, account, display_name, false).await.map_err(|e| e.to_string())?;
        self.congregations.write().retain(|x| !(x.id == c.id && x.mode == StorageMode::Local));
        self.set_unlocked(&c.id);
        self.remember_account(None);
        self.save_session_secret();
        self.log("Moved the congregation to the Cloud.");
        // 3. The copy on this device goes last, once nothing here uses it any more.
        let config = crate::local::db_config(&c.id);
        dioxus::dioxus_core::spawn_forever(async move {
            if let Err(e) = theo_db::Db::destroy(config).await {
                dioxus::logger::tracing::warn!("deleting the copy on this device failed: {e}");
            }
        });
        Ok(())
    }

    /// The server part of a move to the Cloud: the congregation, a copy of everything with the
    /// same keys, and the owner's root grant. Returns the copy's store and the directory as
    /// stored there (your person linked to the Cloud account). Undone on a failure.
    async fn copy_to_cloud(&self, c: &Congregation, store: &Store, rid: theo_db::RecordId, mut dir: Directory, device_user: &str, account: &CloudAccount) -> Result<(Store, Directory), String> {
        // You, in the Cloud: the person of your device account becomes your Cloud account's.
        for p in dir.people.iter_mut() {
            if matches!(&p.account, crate::people::AccountLink::Linked { username } if username == device_user) {
                p.account = crate::people::AccountLink::Linked { username: account.username.clone() };
            }
        }
        let label = account.keys.seal_label(&c.name).map_err(|e| e.to_string())?;
        account.session.create_congregation(rid, &crate::local::to_hex(&label)).await.map_err(|e| e.to_string())?;
        let copied = async {
            let target = theo_db::Db::open(theo_db::DbConfig::Surreal { session: account.session.clone(), congregation: c.id.clone() }).await.map_err(|e| e.to_string())?;
            let copy = store.copy_to(target, details_value(c), &dir).await.map_err(|e| e.to_string())?;
            Self::grant_root_to_self(account, &c.id, &copy).await.map_err(|e| e.to_string())?;
            Ok::<Store, String>(copy)
        }
        .await;
        match copied {
            Ok(copy) => Ok((copy, dir)),
            Err(e) => {
                // Half a copy on the server helps nobody: it goes again.
                let _ = account.session.delete_congregation(rid).await;
                Err(e)
            }
        }
    }

    /// Moves the open Cloud congregation to this device (its owner only). Everybody else loses
    /// access: the server copy, the memberships, invites and grants are deleted. The data is
    /// copied first, with the same keys and congregation password, for a device account (an
    /// existing one with its password, or a new one with `new_name`); the app then continues on
    /// that account, and the congregation opens with the congregation password on this device.
    pub async fn move_to_device(&mut self, username: &str, password: &str, new_name: Option<&str>) -> Result<(), String> {
        let (account, c, store, rid) = self.managed_congregation()?;
        if !self.owns_cloud_congregation().await {
            return Err("Only the owner can move the congregation to a device.".into());
        }
        let device_user = username.trim().to_lowercase();
        let auth = crate::local::auth_hex(&device_user, password).ok_or("The password could not be processed.")?;
        let existing = self.accounts.peek().iter().find(|a| a.username == device_user && a.kind == AccountKind::Device).cloned();
        match (&existing, new_name) {
            (Some(a), _) if a.auth.as_deref() != Some(auth.as_str()) => return Err("Username or password is not correct.".into()),
            (Some(_), _) => {}
            (None, Some(_)) if self.accounts.peek().iter().any(|a| a.username == device_user) => return Err("This username is already taken.".into()),
            (None, Some(_)) => {}
            (None, None) => return Err("There is no account with this username on this device.".into()),
        }
        // One user on a device: only you stay linked; the others' accounts and invites end.
        let mut dir = self.directory();
        for p in dir.people.iter_mut() {
            p.account = match &p.account {
                crate::people::AccountLink::Linked { username } if *username == account.username => crate::people::AccountLink::Linked { username: device_user.clone() },
                _ => crate::people::AccountLink::None,
            };
        }
        let owner = dir.owner;
        dir.access.retain(|p, _| Some(*p) == owner);
        // A leftover of this congregation on the device that the device does not list (an earlier
        // move, or a move cut short) is only an old copy: it goes, so the copy starts empty.
        if !crate::local::Registry::load().congregations.iter().any(|x| x.id == c.id) {
            let _ = theo_db::Db::destroy(crate::local::db_config(&c.id)).await;
        }
        let target = theo_db::Db::open(crate::local::db_config(&c.id)).await.map_err(|e| e.to_string())?;
        let copy = store.copy_to(target, details_value(&c), &dir).await.map_err(|e| format!("Nothing was moved: {e}"))?;
        // The copy is complete: the account and the congregation on this device.
        if existing.is_none() {
            let display = new_name.unwrap_or_default().trim().to_string();
            let (u, a, d) = (device_user.clone(), auth.clone(), display.clone());
            crate::local::Registry::update(|r| r.accounts.push(crate::local::LocalAccount { username: u, display_name: d, auth: a }));
            self.accounts.write().push(Account {
                overrides: load_overrides(&device_user),
                username: device_user.clone(),
                display_name: display,
                kind: AccountKind::Device,
                password: String::new(),
                auth: Some(auth),
            });
        }
        let entry = crate::local::LocalCongregation { id: c.id.clone(), name: c.name.clone(), owner: device_user.clone() };
        crate::local::Registry::update(|r| r.congregations.push(entry));
        // Then the Cloud copy goes, for everyone.
        let deleted = account.session.delete_congregation(rid).await;
        let moved = Congregation { mode: StorageMode::Local, owner: device_user.clone(), member: false, root_grant: false, writes: Vec::new(), ..c.clone() };
        self.cloud.set(None);
        self.congregations.write().retain(|x| !(x.stored && matches!(x.mode, StorageMode::Cloud { .. })) && x.id != c.id);
        self.congregations.write().push(moved);
        self.session.set(Some(device_user.clone()));
        self.stores.write().insert(c.id.clone(), Rc::new(copy));
        self.directories.write().insert(c.id.clone(), dir);
        self.set_unlocked(&c.id);
        self.remember_account(Some(&device_user));
        self.save_session_secret();
        self.log("Moved the congregation from the Cloud to this device.");
        deleted.map_err(|e| format!("Moved to this device, but the Cloud copy could not be deleted ({e}). Sign in to the Cloud later and delete it."))
    }

    // ----- members: permissions, keys and removal (R-241, R-246, R-248) -----

    /// After an administrator changes someone's permissions: what the server lets them write,
    /// and keys for the domains they now read. Domains they no longer read get new key versions
    /// (they keep what they could already read). Making someone an administrator gives them full
    /// access (owner only); they then open with their account password.
    pub async fn sync_member(&mut self, person: u32) -> Result<(), String> {
        let (account, c, store, rid) = self.managed_congregation()?;
        let dir = self.directory();
        let Some(crate::people::AccountLink::Linked { username }) = dir.person(person).map(|p| p.account.clone()) else { return Ok(()) };
        if username == account.username {
            return Ok(());
        }
        let access = dir.access(person);
        let members = account.session.members(rid).await.map_err(|e| e.to_string())?;
        let Some(member) = members.iter().find(|m| m.username == username) else { return Err(format!("@{username} is not a member on the server.")) };
        let public = member.public.as_deref().and_then(crate::local::from_hex).and_then(|b| theo_crypto::PublicIdentity::from_bytes(&b).ok());
        let public = public.ok_or_else(|| format!("@{username} has no keys yet: they must sign in once."))?;
        if access.admin {
            if member.role == "member" {
                account.session.set_member_role(rid, &username, "full").await.map_err(|_| "Only the owner can give full access.".to_string())?;
            }
            if member.role != "owner" {
                let sealed = store
                    .with_root(|root| theo_crypto::grant::grant_root(&public.kem, &c.id, &username, root))
                    .ok_or("The root key is not open.")?
                    .map_err(|e| e.to_string())?;
                account.session.put_grant_for(rid, &username, theo_crypto::grant::ROOT_DOMAIN.0, 0, &crate::local::to_hex(&sealed)).await.map_err(|e| e.to_string())?;
            }
            return Ok(());
        }
        if member.role == "full" {
            // They hold the root key and know the congregation password: taking that back means
            // new keys and a new password, which is what removing the account does.
            return Err("They still have full access on the server. To take it away, remove their account (you choose a new congregation password), then invite them again.".into());
        }
        account.session.set_member_writes(rid, &username, &writes_for(&access)).await.map_err(|e| e.to_string())?;
        let wanted = domains_for(&access);
        let held: Vec<(u8, u32)> = account.session.member_grants(rid).await.map_err(|e| e.to_string())?.into_iter().filter(|(u, _, _)| *u == username).map(|(_, d, v)| (d, v)).collect();
        for (d, v, secret) in store.domain_secrets(&wanted) {
            if !held.contains(&(d.0, v)) {
                let g = theo_crypto::grant::grant(&public.kem, &c.id, &username, d, v, &secret).map_err(|e| e.to_string())?;
                account.session.put_grant_for(rid, &username, d.0, v, &crate::local::to_hex(&g)).await.map_err(|e| e.to_string())?;
            }
        }
        let lost: Vec<DomainId> = ALL_DOMAINS.iter().copied().filter(|d| !wanted.contains(d) && held.iter().any(|(x, _)| *x == d.0)).collect();
        if !lost.is_empty() {
            self.rekey(&c.id, &account, &store, rid, &lost, Some(&username)).await?;
        }
        Ok(())
    }

    /// New versions of `domains`, granted to every member who held them, except `without`.
    async fn rekey(&mut self, id: &str, account: &CloudAccount, store: &Store, rid: theo_db::RecordId, domains: &[DomainId], without: Option<&str>) -> Result<(), String> {
        let grants = account.session.member_grants(rid).await.map_err(|e| e.to_string())?;
        let members = account.session.members(rid).await.map_err(|e| e.to_string())?;
        let fresh = store.rotate_domains(domains).await.map_err(|e| e.to_string())?;
        for m in members.iter().filter(|m| m.role == "member" && Some(m.username.as_str()) != without) {
            let Some(public) = m.public.as_deref().and_then(crate::local::from_hex).and_then(|b| theo_crypto::PublicIdentity::from_bytes(&b).ok()) else { continue };
            for (d, v, secret) in &fresh {
                if grants.iter().any(|(u, x, _)| *u == m.username && *x == d.0) {
                    let g = theo_crypto::grant::grant(&public.kem, id, &m.username, *d, *v, secret).map_err(|e| e.to_string())?;
                    account.session.put_grant_for(rid, &m.username, d.0, *v, &crate::local::to_hex(&g)).await.map_err(|e| e.to_string())?;
                }
            }
        }
        Ok(())
    }

    /// The role of a linked person's account on the server (`"owner"`, `"full"`, `"member"`).
    pub async fn member_role(&self, person: u32) -> Option<String> {
        let (account, _, _, rid) = self.managed_congregation().ok()?;
        let Some(crate::people::AccountLink::Linked { username }) = self.directory().person(person).map(|p| p.account.clone()) else { return None };
        account.session.members(rid).await.ok()?.into_iter().find(|m| m.username == username).map(|m| m.role)
    }

    /// Removes a person's account from the congregation (R-246). The server denies them at once;
    /// then the domains they held get new key versions for the others. A full-access member also
    /// knew the congregation password and the root key: `new_password` replaces both, and the
    /// other full-access members get the new root key.
    pub async fn remove_member(&mut self, person: u32, new_password: Option<&str>) -> Result<(), String> {
        let (account, c, store, rid) = self.managed_congregation()?;
        let dir = self.directory();
        let Some(crate::people::AccountLink::Linked { username }) = dir.person(person).map(|p| p.account.clone()) else { return Err("This person has no account.".into()) };
        if username == account.username || dir.is_owner(person) {
            return Err("The owner, and you yourself, cannot be removed here.".into());
        }
        let members = account.session.members(rid).await.map_err(|e| e.to_string())?;
        let full = members.iter().any(|m| m.username == username && m.role != "member");
        if full && new_password.is_none_or(str::is_empty) {
            return Err("They had full access: choose a new congregation password.".into());
        }
        let held: Vec<DomainId> = if full {
            ALL_DOMAINS.to_vec()
        } else {
            let grants = account.session.member_grants(rid).await.map_err(|e| e.to_string())?;
            ALL_DOMAINS.iter().copied().filter(|d| grants.iter().any(|(u, x, _)| *u == username && *x == d.0)).collect()
        };
        account.session.remove_member(rid, &username).await.map_err(|e| e.to_string())?;
        self.rekey(&c.id, &account, &store, rid, &held, Some(&username)).await?;
        if let (true, Some(pw)) = (full, new_password) {
            store.rotate_root(pw, theo_crypto::KdfParams::DEFAULT).await.map_err(|e| e.to_string())?;
            // The new root key for the remaining owner and full-access members (this account too).
            for m in members.iter().filter(|m| m.role != "member" && m.username != username) {
                let Some(public) = m.public.as_deref().and_then(crate::local::from_hex).and_then(|b| theo_crypto::PublicIdentity::from_bytes(&b).ok()) else { continue };
                let sealed = store.with_root(|root| theo_crypto::grant::grant_root(&public.kem, &c.id, &m.username, root)).ok_or("The root key is not open.")?.map_err(|e| e.to_string())?;
                account.session.put_grant_for(rid, &m.username, theo_crypto::grant::ROOT_DOMAIN.0, 0, &crate::local::to_hex(&sealed)).await.map_err(|e| e.to_string())?;
            }
        }
        let name = dir.person(person).map(|p| format!("{} {}", p.first_name, p.last_name)).unwrap_or_default();
        self.update_directory(|d| {
            d.set_account(person, crate::people::AccountLink::None);
            d.access.remove(&person);
        });
        self.log(&format!("Removed the account of {name} (@{username}); their keys were replaced{}.", if full { ", with a new congregation password" } else { "" }));
        Ok(())
    }

    /// Is the open congregation a Cloud one whose server this account may manage (invites, audit
    /// log)? True for the owner and full-access members.
    pub fn manages_cloud(&self) -> bool {
        self.managed_congregation().is_ok()
    }

    /// Joins a congregation with an invite link. Signs in (or, with `new_name`, creates the
    /// account) on the link's server first, unless this account is already signed in there. A
    /// member invite's keys become this account's grants. Returns the congregation's id.
    pub async fn join(&mut self, link: &InviteLink, username: &str, password: &str, new_name: Option<&str>) -> Result<String, AuthError> {
        let token = InviteToken::from_text(&link.token).map_err(|_| AuthError::Storage("This invite link is not complete. Copy the whole link.".into()))?;
        let server = crate::cloud::server().ok_or(AuthError::Storage(NO_CLOUD.into()))?;
        if link.server.as_deref().is_some_and(|s| Server::new(s).origin() != server.origin()) {
            return Err(AuthError::Storage("This invite is for another Theo Manager server. Open it with that app.".into()));
        }
        let signed_in = self.cloud.peek().as_ref().is_some_and(|a| a.session.server == server && self.session.peek().as_deref() == Some(a.username.as_str()));
        if !signed_in {
            match new_name {
                Some(name) => self.sign_up_cloud_at(&server, username, name, password).await?,
                None => self.sign_in_cloud_at(&server, username, password).await?,
            }
        }
        let account = self.cloud.peek().clone().ok_or(AuthError::WrongCredentials)?;
        theo_db::surreal::join(&server, &account.username, &crate::local::to_hex(account.keys.auth_key()), &token.proof()).await.map_err(storage)?;
        let used = account.session.used_invite(&token.id()).await.map_err(storage)?.ok_or_else(|| AuthError::Storage("The invite was used, but its details could not be read.".into()))?;
        let rid = theo_db::RecordId::from_hex(&used.congregation).ok_or_else(|| AuthError::Storage("The invite names no valid congregation.".into()))?;
        if let Some(sealed) = used.bundle.as_deref().and_then(crate::local::from_hex) {
            let bundle = token.open_bundle(&used.congregation, &sealed).map_err(storage)?;
            let label = account.keys.seal_label(&bundle.name).map_err(storage)?;
            account.session.set_label(rid, &crate::local::to_hex(&label)).await.map_err(storage)?;
            // The keys of the invite become grants to this account's own public key.
            let me = account.personal.public();
            for (domain, version, secret) in &bundle.secrets {
                let g = theo_crypto::grant::grant(&me.kem, &used.congregation, &account.username, *domain, *version, secret).map_err(storage)?;
                account.session.put_grant(rid, domain.0, *version, &crate::local::to_hex(&g)).await.map_err(storage)?;
            }
            account.session.clear_bundle(&token.id()).await.map_err(storage)?;
        }
        self.joined.write().insert(used.congregation.clone(), token.id());
        self.refresh_cloud_congregations().await?;
        self.preselect.set(Some(used.congregation.clone()));
        Ok(used.congregation)
    }

    /// Does opening this congregation ask for a password (or the passkey)? See [`AppState::secret_for`].
    pub fn needs_password(&self, id: &str) -> bool {
        self.secret_for(id) != Secret::None
    }

    /// What opening a congregation asks for. Local: its congregation password. Cloud: nothing
    /// while the account's keys are open; after a restart or a lock, the account password (the
    /// account's keys open the grants), and the congregation password only for a full-access
    /// member's very first unlock.
    pub fn secret_for(&self, id: &str) -> Secret {
        let Some(c) = self.congregations.read().iter().find(|c| c.id == id).cloned() else { return Secret::Congregation };
        match c.mode {
            StorageMode::Cloud { .. } if c.stored && self.cloud.read().is_none() => Secret::Account,
            StorageMode::Cloud { .. } if c.stored && (c.member || c.root_grant) => Secret::None,
            _ => Secret::Congregation,
        }
    }

    /// Grants the root key to this account (owner and full-access members), so it opens with the
    /// account password from now on.
    async fn grant_root_to_self(account: &CloudAccount, id: &str, store: &Store) -> Result<(), AuthError> {
        let rid = theo_db::RecordId::from_hex(id).ok_or(AuthError::WrongPassword)?;
        let me = account.personal.public();
        let sealed = store
            .with_root(|root| theo_crypto::grant::grant_root(&me.kem, id, &account.username, root))
            .ok_or_else(|| AuthError::Storage("The root key is not open.".into()))?
            .map_err(storage)?;
        account.session.put_grant(rid, theo_crypto::grant::ROOT_DOMAIN.0, 0, &crate::local::to_hex(&sealed)).await.map_err(storage)
    }

    /// The root key from this account's root grant, if it has one.
    async fn granted_root(account: &CloudAccount, id: &str) -> Result<Option<theo_crypto::RootKey>, AuthError> {
        let rid = theo_db::RecordId::from_hex(id).ok_or(AuthError::WrongPassword)?;
        let grants = account.session.grants(rid).await.map_err(storage)?;
        let Some(g) = grants.iter().find(|g| g.domain == theo_crypto::grant::ROOT_DOMAIN.0) else { return Ok(None) };
        let sealed = crate::local::from_hex(&g.sealed).ok_or_else(|| AuthError::Storage("Your key grant is damaged.".into()))?;
        theo_crypto::grant::open_root_grant(&account.personal.kem(), id, &account.username, &sealed).map(Some).map_err(storage)
    }

    /// The domain keys of a member's grants in a congregation.
    async fn granted_keys(account: &CloudAccount, id: &str) -> Result<Vec<(DomainId, u32, theo_crypto::DomainSecret)>, AuthError> {
        let rid = theo_db::RecordId::from_hex(id).ok_or(AuthError::WrongPassword)?;
        let kem = account.personal.kem();
        let grants = account.session.grants(rid).await.map_err(storage)?;
        let keys: Vec<_> = grants
            .iter()
            .filter(|g| g.domain != theo_crypto::grant::ROOT_DOMAIN.0)
            .filter_map(|g| {
                let sealed = crate::local::from_hex(&g.sealed)?;
                let secret = theo_crypto::grant::open_grant(&kem, id, &account.username, DomainId(g.domain), g.key_version, &sealed).ok()?;
                Some((DomainId(g.domain), g.key_version, secret))
            })
            .collect();
        if keys.is_empty() {
            return Err(AuthError::Storage("You have no keys for this congregation yet. Ask an administrator.".into()));
        }
        Ok(keys)
    }

    /// Is the signed-in account an account of this build's Cloud service?
    pub fn signed_in_to_cloud(&self) -> bool {
        crate::cloud::server().is_some_and(|s| self.is_signed_in_to(&s))
    }

    /// Is the signed-in account a Cloud account on `server`?
    pub fn is_signed_in_to(&self, server: &Server) -> bool {
        self.cloud.read().as_ref().is_some_and(|a| a.session.server == *server && self.session.read().as_deref() == Some(a.username.as_str()))
    }

    /// The server of the signed-in Cloud account (Cloud congregations are created there).
    pub fn cloud_server(&self) -> Option<String> {
        self.cloud.read().as_ref().map(|c| c.session.server.url.clone())
    }

    /// Where a stored congregation's data is: this device, or the Cloud account's server.
    fn db_config(&self, c: &Congregation) -> Option<theo_db::DbConfig> {
        match c.mode {
            StorageMode::Local => Some(crate::local::db_config(&c.id)),
            StorageMode::Cloud { .. } => {
                self.cloud.peek().as_ref().map(|a| theo_db::DbConfig::Surreal { session: a.session.clone(), congregation: c.id.clone() })
            }
        }
    }

    pub fn account(&self) -> Option<Account> {
        let user = self.session.read().clone()?;
        self.accounts.read().iter().find(|a| a.username == user).cloned()
    }

    // ----- congregations -----

    /// Congregations of the signed-in account.
    pub fn my_congregations(&self) -> Vec<Congregation> {
        let Some(user) = self.session.read().clone() else { return Vec::new() };
        self.congregations.read().iter().filter(|c| c.owner == user).cloned().collect()
    }

    /// The person of the open congregation who is signed in (their account is linked to them).
    pub fn me(&self) -> Option<u32> {
        let username = self.session.read().clone()?;
        self.directory()
            .people
            .iter()
            .find(|p| matches!(&p.account, crate::people::AccountLink::Linked { username: u } if *u == username))
            .map(|p| p.id)
    }

    /// Administrator of the open congregation: always on a Local congregation (one user).
    pub fn i_am_admin(&self) -> bool {
        match self.active_congregation().map(|c| c.mode) {
            Some(StorageMode::Local) => true,
            Some(_) => self.me().is_some_and(|p| self.directory().access(p).admin),
            None => false,
        }
    }

    /// May see `page` (`"section/page"`): administrators, and people with read or write on it.
    pub fn can_read(&self, page: &str) -> bool {
        self.i_am_admin() || self.me().is_some_and(|p| self.directory().access(p).level(page) != crate::access::Level::None)
    }

    /// May change `page` (`"section/page"`): administrators, and people with write on it.
    pub fn can_write(&self, page: &str) -> bool {
        self.i_am_admin() || self.me().is_some_and(|p| self.directory().access(p).level(page) == crate::access::Level::Write)
    }

    pub fn active_congregation(&self) -> Option<Congregation> {
        let id = self.unlocked.read().clone()?;
        self.congregations.read().iter().find(|c| c.id == id).cloned()
    }

    /// Creates a congregation and opens it. A Local one is stored encrypted on this device
    /// (slow on purpose: Argon2id of the password).
    pub async fn create_congregation(&mut self, new: CongregationDraft) -> Result<String, String> {
        let local = new.mode == StorageMode::Local;
        let record_id = theo_db::RecordId::random().map_err(|e| e.to_string())?;
        let id = record_id.to_hex();
        let owner = self.session.read().clone().unwrap_or_default();
        let congregation = Congregation {
            id: id.clone(),
            name: new.name.trim().to_string(),
            number: new.number.trim().to_string(),
            language: new.language,
            time_zone: new.time_zone,
            midweek: new.midweek,
            weekend: new.weekend,
            mode: new.mode,
            owner: owner.clone(),
            biometric: false,
            defaults: Preferences::default(),
            password: String::new(),
            stored: true,
            member: false,
            writes: Vec::new(),
            root_grant: !local,
        };
        // The creator is a person of the congregation, linked to their account and its owner.
        let mut dir = Directory { assignment_labels: crate::territories::default_assignment_labels(), ..Directory::default() };
        dir.ensure_duty_types();
        let draft = crate::people::PersonDraft {
            first_name: new.my_first_name.trim().into(),
            last_name: new.my_last_name.trim().into(),
            gender: new.my_gender.clone(),
            publisher_type: crate::people::PublisherType::BaptisedPublisher.value().into(),
            ..Default::default()
        };
        if let Ok(me) = dir.save(None, &draft) {
            if let Some(p) = dir.people.iter_mut().find(|p| p.id == me) {
                p.account = crate::people::AccountLink::Linked { username: owner.clone() };
            }
            dir.owner = Some(me);
        }
        if !local {
            // On the server first: the congregation, this account's membership (with the name
            // encrypted for this account only) and its change counter.
            let account = self.cloud.peek().clone().ok_or("Sign in with a Cloud account first.")?;
            let label = account.keys.seal_label(&congregation.name).map_err(|e| e.to_string())?;
            account.session.create_congregation(record_id, &crate::local::to_hex(&label)).await.map_err(|e| e.to_string())?;
            self.cloud_names.write().insert(id.clone(), congregation.name.clone());
        }
        let config = self.db_config(&congregation).ok_or("Sign in with a Cloud account first.")?;
        let db = theo_db::Db::open(config).await.map_err(|e| e.to_string())?;
        let store = Store::create(db, &id, &new.password, theo_crypto::KdfParams::DEFAULT, &details_value(&congregation), &dir).await.map_err(|e| e.to_string())?;
        if !local {
            // The owner opens it from now on with their account password (root grant, 6.2).
            let account = self.cloud.peek().clone().ok_or("Sign in with a Cloud account first.")?;
            Self::grant_root_to_self(&account, &id, &store).await.map_err(|e| e.to_string())?;
        }
        if local {
            let entry = crate::local::LocalCongregation { id: id.clone(), name: congregation.name.clone(), owner };
            crate::local::Registry::update(|r| r.congregations.push(entry));
        }
        self.stores.write().insert(id.clone(), Rc::new(store));
        self.congregations.write().push(congregation);
        self.directories.write().insert(id.clone(), dir);
        self.set_unlocked(&id);
        if !local {
            // The remembered Cloud list includes it from now on.
            self.remember_account(None);
        }
        Ok(id)
    }

    /// Leaves the open congregation: it is removed from this account's list on this device.
    pub fn leave_congregation(&mut self) {
        let Some(id) = self.unlocked.peek().clone() else { return };
        let me = self.me();
        if self.active_congregation().is_some_and(|c| matches!(c.mode, StorageMode::Cloud { .. })) && me.is_some_and(|p| self.directory().is_owner(p)) {
            // The owner stays: the congregation would have nobody to run it.
            return;
        }
        if self.active_congregation().is_some_and(|c| c.stored && c.mode == StorageMode::Local) {
            // A Local congregation has one user: leaving would delete it. That is not offered.
            return;
        }
        let user = self.session.peek().clone().unwrap_or_default();
        let left = self.congregations.peek().iter().find(|c| c.id == id && c.stored).cloned();
        self.congregations.write().retain(|c| !(c.id == id && c.owner == user));
        self.directories.write().remove(&id);
        if let Some(c) = left {
            self.stores.write().remove(&id);
            match c.mode {
                StorageMode::Local => {
                    // The congregation's database is deleted from this device.
                    let gone = id.clone();
                    crate::local::Registry::update(|r| r.congregations.retain(|c| c.id != gone));
                    let config = crate::local::db_config(&id);
                    dioxus::dioxus_core::spawn_forever(async move {
                        let _ = theo_db::Db::destroy(config).await;
                    });
                }
                StorageMode::Cloud { .. } => {
                    self.remember_account(None);
                    // This account's membership ends; the data stays for the other members.
                    let session = self.cloud.peek().as_ref().map(|a| a.session.clone());
                    if let (Some(session), Some(rid)) = (session, theo_db::RecordId::from_hex(&id)) {
                        dioxus::dioxus_core::spawn_forever(async move {
                            let _ = session.leave(rid).await;
                        });
                    }
                }
            }
        }
        self.layouts.write().remove(&id);
        self.unlocked.set(None);
        self.forget_session();
    }

    /// Opens a congregation with its password. A Local one is read and decrypted from this device
    /// (slow on purpose: Argon2id).
    pub async fn unlock(&mut self, id: &str, password: &str, remember_biometric: bool) -> Result<(), AuthError> {
        self.unlock_with(id, password, None, remember_biometric).await
    }

    /// [`AppState::unlock`], or with the root key itself (passkey, *Stay unlocked*). With
    /// `remember_passkey`, a passkey is set up for the next time before the congregation opens.
    async fn unlock_with(&mut self, id: &str, password: &str, root_key: Option<theo_crypto::RootKey>, remember_passkey: bool) -> Result<(), AuthError> {
        let Some(mut c) = self.congregations.peek().iter().find(|c| c.id == id).cloned() else { return Err(AuthError::WrongPassword) };
        let mut password = password;
        if c.stored && matches!(c.mode, StorageMode::Cloud { .. }) && self.cloud.peek().is_none() {
            // A remembered Cloud account after a restart or a lock: the password is the account's.
            let username = self.session.peek().clone().unwrap_or_default();
            self.sign_in_cloud(&username, password).await?;
            self.session.set(Some(username));
            c = self.congregations.peek().iter().find(|x| x.id == id).cloned().ok_or_else(|| AuthError::Storage("You are no longer a member of this congregation.".into()))?;
            if !(c.member || c.root_grant) {
                return Err(AuthError::Storage("Signed in. This first time, type the congregation password.".into()));
            }
            password = "";
        }
        let remember_biometric = remember_passkey;
        if c.stored {
            let config = self.db_config(&c).ok_or_else(|| AuthError::Storage("Sign in to the server again.".into()))?;
            let db = theo_db::Db::open(config).await.map_err(storage)?;
            let account = self.cloud.peek().clone().filter(|_| matches!(c.mode, StorageMode::Cloud { .. }));
            let root = match (root_key, &account, c.root_grant) {
                (Some(r), _, _) => Some(r),
                (None, Some(a), true) => Self::granted_root(a, id).await?,
                _ => None,
            };
            let opened = match (&account, root) {
                (Some(a), _) if c.member => {
                    let keys = Self::granted_keys(a, id).await?;
                    Store::open_with_keys(db, id, keys).await
                }
                (_, Some(root)) => Store::open_with_root(db, id, root).await,
                _ => Store::open(db, id, password).await,
            };
            let (store, details, dir) = opened.map_err(|e| match e {
                StoreError::Crypto(theo_crypto::Error::WrongPasswordOrDamaged) => AuthError::WrongPassword,
                e => storage(e),
            })?;
            if let Ok(d) = serde_json::from_value::<crate::backup::Details>(details)
                && let Some(x) = self.congregations.write().iter_mut().find(|x| x.id == id)
            {
                apply_details(x, d);
            }
            if c.member {
                // What the server lets this member write; other changes stay on this device.
                store.set_writable(Some(c.writes.iter().cloned().collect()));
            }
            let store = Rc::new(store);
            if let Some(a) = account.filter(|_| !c.member && !c.root_grant) {
                // A full-access member's first unlock with the password: from now on the account
                // password is enough (root grant).
                if Self::grant_root_to_self(&a, id, &store).await.is_ok()
                    && let Some(x) = self.congregations.write().iter_mut().find(|x| x.id == id)
                {
                    x.root_grant = true;
                }
            }
            if remember_passkey {
                let result = if matches!(c.mode, StorageMode::Cloud { .. }) { self.enable_account_passkey().await } else { self.enable_passkey_for(id, &store).await };
                let message = match result {
                    Ok(()) => "Passkey unlock is on for this device.".to_string(),
                    Err(e) => format!("Passkey unlock is not on: {e}"),
                };
                self.notice.set(Some(message));
            }
            self.stores.write().insert(id.to_string(), store);
            self.directories.write().insert(id.to_string(), dir);
            self.set_unlocked(id);
            self.after_cloud_unlock(id);
            self.save_session_secret();
            return Ok(());
        }
        if c.password != password {
            return Err(AuthError::WrongPassword);
        }
        if remember_biometric {
            self.set_biometric(id, true);
        }
        self.set_unlocked(id);
        Ok(())
    }

    /// After a Cloud congregation opens: a person whose invite this device just used is linked to
    /// this account, and an administrator's app links the people whose invites were used.
    fn after_cloud_unlock(&mut self, id: &str) {
        let username = self.session.peek().clone().unwrap_or_default();
        let invite = self.joined.write().remove(id);
        if let Some(invite) = invite {
            let person = self.directory().people.iter().find(|p| matches!(&p.account, crate::people::AccountLink::Invited { invite: i, .. } if *i == invite)).map(|p| p.id);
            if let Some(p) = person {
                self.update_directory(|d| d.set_account(p, crate::people::AccountLink::Linked { username }));
            }
        }
        if self.manages_cloud() {
            let mut state = *self;
            dioxus::dioxus_core::spawn_forever(async move {
                if let Err(e) = state.sync_invites().await {
                    dioxus::logger::tracing::warn!("checking the invites failed: {e}");
                }
            });
        }
    }

    /// Is this congregation stored on this device (not a mock)?
    pub fn is_stored(&self, id: &str) -> bool {
        self.congregations.read().iter().any(|c| c.id == id && c.stored)
    }

    /// Is this the password of the congregation? (Checks only; does not unlock.)
    pub fn password_matches(&self, id: &str, password: &str) -> bool {
        self.congregations.read().iter().any(|c| c.id == id && !c.stored && c.password == password)
    }

    // ----- passkey unlock (6.5) -----

    /// Can this congregation unlock with a passkey on this device? Stored Local congregations in
    /// the browser (the passkey's PRF output seals the root key); the demo simulates it. Cloud
    /// congregations open with the account already.
    pub fn passkey_available(&self, id: &str) -> bool {
        self.congregations.read().iter().any(|c| c.id == id && if c.stored { crate::platform::PASSKEY_UNLOCK } else { crate::platform::biometric_label().is_some() })
    }

    /// Is passkey unlock on for this congregation on this device? Cloud: the account's passkey
    /// (it opens the account's keys, and so all its Cloud congregations).
    pub fn has_passkey(&self, id: &str) -> bool {
        let Some(c) = self.congregations.read().iter().find(|c| c.id == id).cloned() else { return false };
        match (&c.mode, c.stored) {
            (StorageMode::Cloud { .. }, true) => self.session.read().as_deref().is_some_and(|u| passkey_record(&account_passkey_key(u)).is_some()),
            (_, true) => passkey_record(&passkey_key(id)).is_some(),
            _ => c.biometric || crate::platform::stored_biometric(id).is_some(),
        }
    }

    /// The PRF output of this account's passkey on this device, for a new `salt`: the browser asks
    /// once. The account's first passkey is made here; later ones reuse it, so the device lists
    /// one passkey per account, not one per congregation.
    async fn passkey_secret(&self) -> Result<(String, String, zeroize::Zeroizing<Vec<u8>>), String> {
        let account = self.account().ok_or("Sign in first.")?;
        let credential_key = format!("theo-manager.passkey.credential.{}", account.username);
        if let Some(credential) = crate::platform::device_get(&credential_key) {
            let salt = theo_crypto::device::new_key().map(|k| crate::local::to_hex(k.as_ref())).map_err(|e| e.to_string())?;
            match crate::platform::passkey_prf(&credential, &salt).await {
                Ok(prf) => return Ok((credential, salt, prf)),
                // Deleted from the device, or from another browser profile: make a new one.
                Err(e) if !e.starts_with("Cancelled") => crate::platform::device_set(&credential_key, None),
                Err(e) => return Err(e),
            }
        }
        let passkey = crate::platform::passkey_register(&account.username, &account.display_name).await?;
        crate::platform::device_set(&credential_key, Some(&passkey.credential));
        Ok((passkey.credential, passkey.salt, passkey.prf))
    }

    /// Seals `secret` with the passkey (asks once) and keeps it on this device under `key`.
    async fn seal_with_passkey(&self, key: &str, secret: &[u8]) -> Result<(), String> {
        let (credential, salt, prf) = self.passkey_secret().await?;
        let k = theo_crypto::device::key_from_prf(&prf).map_err(|e| e.to_string())?;
        let sealed = theo_crypto::device::seal(&k, key, secret).map_err(|e| e.to_string())?;
        let record = serde_json::json!({ "credential": credential, "salt": salt, "sealed": crate::local::to_hex(&sealed) });
        crate::platform::device_set(key, Some(&record.to_string()));
        Ok(())
    }

    /// Opens what [`AppState::seal_with_passkey`] kept under `key` (the browser asks once).
    async fn open_with_passkey(key: &str) -> Result<zeroize::Zeroizing<Vec<u8>>, AuthError> {
        let record = passkey_record(key).ok_or_else(|| AuthError::Storage("Passkey unlock is not on here.".into()))?;
        let field = |k: &str| record.get(k).and_then(|v| v.as_str()).unwrap_or_default().to_string();
        let prf = crate::platform::passkey_prf(&field("credential"), &field("salt")).await.map_err(AuthError::Storage)?;
        let k = theo_crypto::device::key_from_prf(&prf).map_err(storage)?;
        let sealed = crate::local::from_hex(&field("sealed")).ok_or(AuthError::WrongPassword)?;
        theo_crypto::device::open(&k, key, &sealed).map_err(|_| AuthError::Storage("This passkey does not open it. Use the password.".into()))
    }

    /// Cloud: seals the account's keys with its passkey, so one fingerprint opens the account and
    /// its Cloud congregations after a restart.
    async fn enable_account_passkey(&self) -> Result<(), String> {
        let account = self.cloud.peek().clone().ok_or("Sign in to the Cloud first.")?;
        self.seal_with_passkey(&account_passkey_key(&account.username), &account.keys.to_bytes()).await
    }

    /// Turns passkey unlock on for the open congregation (the browser asks for the fingerprint,
    /// face or PIN).
    pub async fn enable_passkey(&mut self, id: &str) -> Result<(), String> {
        let stored = self.is_stored(id);
        if !stored {
            let account = self.account();
            let credential = crate::platform::biometric_register(&account.as_ref().map(|a| a.username.clone()).unwrap_or_default(), &account.map(|a| a.display_name).unwrap_or_default()).await?;
            crate::platform::store_biometric(id, Some(&credential));
            self.set_biometric(id, true);
            return Ok(());
        }
        if self.congregations.peek().iter().any(|c| c.id == id && matches!(c.mode, StorageMode::Cloud { .. })) {
            return self.enable_account_passkey().await;
        }
        let store = self.stores.peek().get(id).cloned().ok_or("Unlock the congregation first.")?;
        self.enable_passkey_for(id, &store).await
    }

    /// Makes a passkey and seals the root key with its PRF output, kept on this device only.
    async fn enable_passkey_for(&self, id: &str, store: &Store) -> Result<(), String> {
        let root = store.with_root(|r| r.to_bytes()).ok_or("Only a congregation opened with its password can use a passkey.")?;
        self.seal_with_passkey(&passkey_key(id), root.as_ref()).await
    }

    /// Turns passkey unlock off on this device (the sealed key is deleted). Cloud: for the account.
    pub fn disable_passkey(&mut self, id: &str) {
        let cloud = self.congregations.peek().iter().any(|c| c.id == id && c.stored && matches!(c.mode, StorageMode::Cloud { .. }));
        if cloud {
            if let Some(u) = self.session.peek().clone() {
                crate::platform::device_set(&account_passkey_key(&u), None);
            }
        } else {
            crate::platform::device_set(&passkey_key(id), None);
        }
        crate::platform::store_biometric(id, None);
        self.set_biometric(id, false);
    }

    /// Unlocks with the passkey: its PRF output opens the sealed root key.
    pub async fn unlock_with_passkey(&mut self, id: &str) -> Result<(), AuthError> {
        if !self.is_stored(id) {
            let credential = crate::platform::stored_biometric(id).unwrap_or_else(|| crate::platform::SIMULATED.into());
            return if crate::platform::biometric_verify(&credential).await && self.unlock_with_biometrics(id) {
                Ok(())
            } else {
                Err(AuthError::Storage("Biometric unlock did not work. Use the password.".into()))
            };
        }
        let cloud = self.congregations.peek().iter().any(|c| c.id == id && matches!(c.mode, StorageMode::Cloud { .. }));
        if cloud {
            // The account's keys, then the congregation through its grants: one fingerprint.
            if self.cloud.peek().is_none() {
                let username = self.session.peek().clone().unwrap_or_default();
                let bytes = Self::open_with_passkey(&account_passkey_key(&username)).await?;
                let keys = AccountKeys::from_bytes(&bytes).map_err(storage)?;
                self.sign_in_with_keys(&username, keys).await?;
            }
            return self.unlock_with(id, "", None, false).await;
        }
        let bytes = Self::open_with_passkey(&passkey_key(id)).await?;
        let root = theo_crypto::RootKey::from_bytes(&bytes).map_err(|_| AuthError::Storage("This passkey does not open the congregation. Use the password.".into()))?;
        self.unlock_with(id, "", Some(root), false).await
    }

    /// Opens a congregation after the platform's biometric check said it is the user
    /// (`platform::biometric_verify`). Demo only: its data is not encrypted.
    fn unlock_with_biometrics(&mut self, id: &str) -> bool {
        // Stored congregations need the password until the key is kept in secure hardware (R-312).
        let known = self.congregations.read().iter().any(|c| c.id == id && !c.stored);
        if known {
            self.set_biometric(id, true);
            self.set_unlocked(id);
        }
        known
    }

    /// Opens a congregation. Lists start without filters (group ids differ per congregation).
    fn set_unlocked(&mut self, id: &str) {
        if self.unlocked.peek().as_deref() != Some(id) {
            self.user_query.set(String::new());
            self.user_filters.set(crate::people::default_filters());
            self.users_tab.set("people".into());
            self.report_month.set(None);
            self.report_query.set(String::new());
            self.report_filters.set(FilterSelection::new());
            self.attendance_year.set(None);
        }
        self.unlocked.set(Some(id.to_string()));
        // Retention (m01-people.md): people who moved away more than a year ago are removed for
        // good when a device with access opens the congregation.
        let today = crate::people::today_iso();
        self.update_directory(|d| d.purge_expired(&today));
        self.remember_session();
    }

    pub fn set_biometric(&mut self, id: &str, on: bool) {
        if let Some(c) = self.congregations.write().iter_mut().find(|c| c.id == id) {
            c.biometric = on;
        }
    }

    pub fn lock(&mut self) {
        self.forget_session();
        let current = self.unlocked.read().clone();
        // A stored congregation forgets its keys and its data (R-310); the demo stays in memory.
        if let Some(id) = current.as_ref().filter(|id| self.is_stored(id)) {
            self.stores.write().remove(id);
            self.directories.write().remove(id);
        }
        // The Cloud account's keys go too: they open every Cloud congregation. The account stays
        // signed in (remembered); the next unlock asks for the fingerprint or its password.
        self.cloud.set(None);
        self.preselect.set(current);
        self.unlocked.set(None);
    }

    // ----- display preferences (R-062) -----

    /// Effective preferences: the user's overrides on top of the active congregation's defaults.
    pub fn preferences(&self) -> Preferences {
        let base = self.active_congregation().map(|c| c.defaults).unwrap_or_default();
        match self.account() {
            Some(a) => base.with(&a.overrides),
            None => base,
        }
    }

    pub fn update_overrides(&mut self, f: impl FnOnce(&mut Overrides)) {
        let Some(user) = self.session.read().clone() else { return };
        if let Some(a) = self.accounts.write().iter_mut().find(|a| a.username == user) {
            f(&mut a.overrides);
            // Kept on this device for this account, so they survive a restart.
            if let Ok(json) = serde_json::to_string(&a.overrides) {
                crate::platform::device_set(&overrides_key(&user), Some(&json));
            }
        }
    }

    /// Changes the open congregation's details (name, number, language, time zone, meetings) and
    /// saves them with its data. A Cloud congregation's new name is also stored, encrypted, in
    /// this account's membership (`persist`).
    pub fn update_details(&mut self, f: impl FnOnce(&mut Congregation)) {
        let Some(id) = self.unlocked.read().clone() else { return };
        let before = self.active_congregation();
        if let Some(c) = self.congregations.write().iter_mut().find(|c| c.id == id) {
            f(c);
        }
        let after = self.active_congregation();
        if before != after {
            self.persist();
            self.log("Changed the congregation details.");
        }
    }

    pub fn update_congregation_defaults(&mut self, f: impl FnOnce(&mut Preferences)) {
        let Some(id) = self.unlocked.read().clone() else { return };
        if let Some(c) = self.congregations.write().iter_mut().find(|c| c.id == id) {
            f(&mut c.defaults);
        }
        self.persist();
    }

    // ----- people (MOCK of M-01 / M-09) -----

    /// People, families and groups of the unlocked congregation.
    pub fn directory(&self) -> Directory {
        let id = self.unlocked.read().clone().unwrap_or_default();
        self.directories.read().get(&id).cloned().unwrap_or_default()
    }

    pub fn update_directory<R>(&mut self, f: impl FnOnce(&mut Directory) -> R) -> R {
        let id = self.unlocked.read().clone().unwrap_or_default();
        let out = f(self.directories.write().entry(id).or_default());
        self.persist();
        out
    }

    /// Saves the open congregation when it is stored on this device: its details and data, only
    /// what changed, in the background. Saves never overlap (`Store::save`).
    pub fn persist(&self) {
        let Some(id) = self.unlocked.peek().clone() else { return };
        let Some(store) = self.stores.peek().get(&id).cloned() else { return };
        let Some(c) = self.congregations.peek().iter().find(|c| c.id == id).cloned() else { return };
        let Some(dir) = self.directories.peek().get(&id).cloned() else { return };
        match c.mode {
            StorageMode::Local => crate::local::Registry::update(|r| {
                if let Some(x) = r.congregations.iter_mut().find(|x| x.id == id) {
                    x.name = c.name.clone();
                }
            }),
            StorageMode::Cloud { .. } => {
                let renamed = self.cloud_names.peek().get(&id) != Some(&c.name);
                let account = self.cloud.peek().clone();
                if let (true, Some(account), Some(rid)) = (renamed, account, theo_db::RecordId::from_hex(&id)) {
                    // Signals are `Copy`: a copy writes the same value.
                    let mut names = self.cloud_names;
                    names.write().insert(id.clone(), c.name.clone());
                    self.remember_account(None);
                    if let Ok(label) = account.keys.seal_label(&c.name) {
                        dioxus::dioxus_core::spawn_forever(async move {
                            let _ = account.session.set_label(rid, &crate::local::to_hex(&label)).await;
                        });
                    }
                }
            }
        }
        let details = details_value(&c);
        // Not `spawn`: that task would end with the page that made the change (for example a
        // form that closes after saving), possibly half-way through the write.
        dioxus::dioxus_core::spawn_forever(async move {
            if let Err(e) = store.save(details, &dir).await {
                dioxus::logger::tracing::error!("saving the congregation failed: {e}");
            }
        });
    }

    // ----- backup (docs/design/local-sharing.md, "Backup") -----

    /// Is this the open congregation's password? A stored one unwraps its root key again.
    pub async fn congregation_password_ok(&self, password: &str) -> bool {
        let Some(c) = self.active_congregation() else { return false };
        if !c.stored {
            return c.password == password;
        }
        let store = self.stores.peek().get(&c.id).cloned();
        match store {
            Some(s) => s.password_ok(password).await,
            None => false,
        }
    }

    pub fn backup(&self) -> Option<crate::backup::Backup> {
        let c = self.active_congregation()?;
        let details = crate::backup::Details {
            name: c.name,
            number: c.number,
            language: c.language,
            time_zone: c.time_zone,
            midweek: c.midweek,
            weekend: c.weekend,
            defaults: c.defaults,
        };
        Some(crate::backup::Backup::new(details, self.directory()))
    }

    /// Replaces the details and all data of the open congregation with the backup. The storage
    /// mode, the password and biometric unlock stay as they are.
    pub fn restore(&mut self, b: crate::backup::Backup) {
        let Some(id) = self.unlocked.read().clone() else { return };
        let mut b = b;
        // The file may come from another account or storage (an export of a Local congregation
        // imported in the Cloud, for example): its creator becomes you, so you keep your access.
        let me = self.session.peek().clone().unwrap_or_default();
        let linked = b.directory.people.iter().any(|p| matches!(&p.account, crate::people::AccountLink::Linked { username } if *username == me));
        if !linked && let Some(owner) = b.directory.owner {
            b.directory.set_account(owner, crate::people::AccountLink::Linked { username: me });
        }
        if let Some(c) = self.congregations.write().iter_mut().find(|c| c.id == id) {
            c.name = b.details.name;
            c.number = b.details.number;
            c.language = b.details.language;
            c.time_zone = b.details.time_zone;
            c.midweek = b.details.midweek;
            c.weekend = b.details.weekend;
            c.defaults = b.details.defaults;
        }
        self.directories.write().insert(id, b.directory);
        self.persist();
        self.user_query.set(String::new());
        self.user_filters.set(crate::people::default_filters());
        self.report_month.set(None);
        self.report_query.set(String::new());
        self.report_filters.set(FilterSelection::new());
        self.log(&format!("Restored a backup made on {} UTC.", b.created));
    }

    /// Adds a line to the activity log, as the signed-in user.
    pub fn log(&mut self, text: &str) {
        let actor = self.account().map(|a| a.display_name).unwrap_or_default();
        self.update_directory(|d| d.log.push(crate::access::LogEntry { at: crate::prefs::now_stamp(), actor, text: text.to_string() }));
    }

    // ----- dashboard layout (per congregation) -----

    /// The home widgets: changed in this session, else remembered on this device, else the default.
    pub fn layout(&self) -> Vec<WidgetLayout> {
        let id = self.unlocked.read().clone().unwrap_or_default();
        if let Some(l) = self.layouts.read().get(&id) {
            return l.clone();
        }
        load_layout(&id).unwrap_or_else(crate::mock::default_layout)
    }

    pub fn set_layout(&mut self, layout: Vec<WidgetLayout>) {
        let id = self.unlocked.read().clone().unwrap_or_default();
        save_layout(&id, Some(&layout));
        self.layouts.write().insert(id, layout);
    }

    pub fn reset_layout(&mut self) {
        let id = self.unlocked.read().clone().unwrap_or_default();
        save_layout(&id, None);
        self.layouts.write().remove(&id);
    }

    // ----- demo -----

    /// Creates the demo account with two congregations and signs in.
    pub fn start_demo(&mut self) {
        if !self.accounts.read().iter().any(|a| a.username == crate::mock::DEMO_USER) {
            self.accounts.write().push(Account {
                username: crate::mock::DEMO_USER.into(),
                display_name: "John Smith".into(),
                kind: AccountKind::Device,
                overrides: Overrides::default(),
                password: crate::mock::DEMO_PASSWORD.into(),
                auth: None,
            });
            for (id, name, mode) in [
                ("demo-1", "Riverside", StorageMode::Local),
                ("demo-2", "Hillcrest", StorageMode::Cloud { server: "https://db.example.org".into() }),
            ] {
                self.congregations.write().push(Congregation {
                    id: id.into(),
                    name: name.into(),
                    number: String::new(),
                    language: "en".into(),
                    time_zone: "Europe/London".into(),
                    midweek: MeetingSlot { day: "Thursday".into(), time: "19:00".into() },
                    weekend: MeetingSlot { day: "Sunday".into(), time: "10:00".into() },
                    mode,
                    owner: crate::mock::DEMO_USER.into(),
                    biometric: false,
                    defaults: Preferences::default(),
                    password: crate::mock::DEMO_CONGREGATION_PASSWORD.into(),
                    stored: false,
                    member: false,
                    writes: Vec::new(),
                    root_grant: false,
                });
                let size = if id == "demo-1" { 36 } else { 14 };
                let mut dir = crate::mock::demo_directory(size);
                Self::apply_seen(id, &mut dir);
                self.directories.write().insert(id.into(), dir);
            }
        }
        self.session.set(Some(crate::mock::DEMO_USER.into()));
        self.unlocked.set(None);
    }
}

/// Which congregation is open, by whom and until when (`user`, `congregation`, `until` seconds),
/// one per line. Only while *Stay unlocked* is more than 0 minutes.
const SESSION_KEY: &str = "theo-manager.session";
/// An account's own display preferences on this device (*Me › Preferences*, R-062).
fn overrides_key(username: &str) -> String {
    format!("theo-manager.prefs.{username}")
}

fn load_overrides(username: &str) -> Overrides {
    crate::platform::device_get(&overrides_key(username)).and_then(|j| serde_json::from_str(&j).ok()).unwrap_or_default()
}

/// The account signed in on this device (`remember_account`).
const ACCOUNT_KEY: &str = "theo-manager.account";

/// A Cloud account's passkey record: its keys sealed by the passkey.
fn account_passkey_key(username: &str) -> String {
    format!("theo-manager.passkey.account.{username}")
}

/// What reopens the congregation of `SESSION_KEY`, sealed with the device key (`device_seal`).
const SECRET_KEY: &str = "theo-manager.session.secret";

fn session_purpose(user: &str, congregation: &str) -> String {
    format!("stay:{user}:{congregation}")
}

/// A congregation's passkey on this device: credential id, PRF salt and the sealed root key.
fn passkey_key(congregation: &str) -> String {
    format!("theo-manager.passkey.{congregation}")
}

/// A passkey record (`credential`, `salt`, `sealed`) kept under `key`.
fn passkey_record(key: &str) -> Option<serde_json::Value> {
    crate::platform::device_get(key).and_then(|t| serde_json::from_str(&t).ok())
}

/// *Stay unlocked*, in minutes (0: lock when the app is closed or reloaded).
const STAY_KEY: &str = "theo-manager.stay-unlocked";

/// The choices of *Stay unlocked*, in minutes.
pub const STAY_UNLOCKED: [(u32, &str); 5] = [
    (0, "Lock when the app is closed or reloaded"),
    (5, "5 minutes"),
    (15, "15 minutes"),
    (60, "1 hour"),
    (480, "8 hours"),
];

/// *Stay unlocked* of this device, in minutes.
pub fn stay_unlocked_minutes() -> u32 {
    crate::platform::device_get(STAY_KEY).and_then(|v| v.parse().ok()).unwrap_or(0)
}

fn layout_key(congregation: &str) -> String {
    format!("theo-manager.layout.{congregation}")
}

/// Remembers the home widgets of a congregation on this device: `id cols rows` per line, in
/// order (or forgets them). Only the arrangement, never congregation data.
fn save_layout(congregation: &str, layout: Option<&[WidgetLayout]>) {
    let text = layout.map(|l| l.iter().map(|w| format!("{} {} {}", w.id, w.cols, w.rows)).collect::<Vec<_>>().join("
"));
    crate::platform::device_set(&layout_key(congregation), text.as_deref());
}

/// The remembered home widgets, without widgets that no longer exist.
fn load_layout(congregation: &str) -> Option<Vec<WidgetLayout>> {
    let text = crate::platform::device_get(&layout_key(congregation))?;
    let list: Vec<WidgetLayout> = text
        .lines()
        .filter_map(|line| {
            let mut parts = line.split_whitespace();
            let d = crate::widgets::find(parts.next()?)?;
            let cols = parts.next().and_then(|c| c.parse().ok()).unwrap_or(d.cols);
            let rows = parts.next().and_then(|r| r.parse().ok()).unwrap_or(d.rows);
            Some(WidgetLayout::new(d.id, d.title, d.icon, cols, rows))
        })
        .collect();
    (!list.is_empty()).then_some(list)
}

fn seen_key(congregation: &str) -> String {
    format!("theo-manager.seen.{congregation}")
}

fn notice_key(n: &crate::assignments::Notice) -> String {
    format!("{}|{:?}|{}|{:?}|{}", n.person, n.what, n.record, n.kind, n.date)
}

impl AppState {
    /// A workbook on this device: read in this session, else kept in this browser.
    pub fn workbook(&self, issue: &str, lang: &str) -> Option<crate::workbook::Workbook> {
        let key = crate::workbook::key(issue, lang);
        if let Some(w) = self.workbooks.read().get(&key) {
            return Some(w.clone());
        }
        crate::platform::device_get(&format!("theo-manager.workbook.{key}")).and_then(|j| serde_json::from_str(&j).ok())
    }

    /// Keeps a workbook on this device (and in this browser for the next time).
    pub fn store_workbook(&mut self, wb: crate::workbook::Workbook) {
        if let Ok(json) = serde_json::to_string(&wb) {
            crate::platform::device_set(&format!("theo-manager.workbook.{}", wb.key()), Some(&json));
        }
        self.workbooks.write().insert(wb.key(), wb);
    }

    /// Sets *Stay unlocked* for this device.
    pub fn set_stay_unlocked(&self, minutes: u32) {
        crate::platform::device_set(STAY_KEY, Some(&minutes.to_string()));
        if minutes == 0 {
            self.forget_session();
        }
        self.remember_session();
        self.save_session_secret();
    }

    /// Remembers the open congregation until *Stay unlocked* from now (called on unlock and on
    /// every page change), or forgets it when the setting is 0.
    pub fn remember_session(&self) {
        let minutes = stay_unlocked_minutes();
        let (Some(user), Some(id)) = (self.session.peek().clone(), self.unlocked.peek().clone()) else {
            crate::platform::device_set(SESSION_KEY, None);
            return;
        };
        if minutes == 0 {
            crate::platform::device_set(SESSION_KEY, None);
            return;
        }
        let until = crate::platform::now_secs() + minutes as u64 * 60;
        crate::platform::device_set(SESSION_KEY, Some(&format!("{user}\n{id}\n{until}")));
    }

    fn forget_session(&self) {
        crate::platform::device_set(SESSION_KEY, None);
        crate::platform::device_set(SECRET_KEY, None);
    }

    /// For *Stay unlocked*: keeps what reopens the open congregation sealed with this device's
    /// key (6.5): a Local congregation's root key, or the Cloud account's keys (the app signs in
    /// again with them; the session token itself is never kept). Only while the setting is on.
    fn save_session_secret(&self) {
        if stay_unlocked_minutes() == 0 {
            return;
        }
        let (Some(user), Some(c)) = (self.session.peek().clone(), self.active_congregation()) else { return };
        if !c.stored {
            return;
        }
        let mut material = zeroize::Zeroizing::new(Vec::new());
        match c.mode {
            StorageMode::Local => {
                let Some(root) = self.stores.peek().get(&c.id).and_then(|s| s.with_root(|r| r.to_bytes())) else { return };
                material.push(1);
                material.extend_from_slice(root.as_ref());
            }
            StorageMode::Cloud { .. } => {
                let Some(account) = self.cloud.peek().clone() else { return };
                material.push(2);
                material.extend_from_slice(&account.keys.to_bytes());
            }
        }
        let purpose = session_purpose(&user, &c.id);
        dioxus::dioxus_core::spawn_forever(async move {
            let sealed = crate::platform::device_seal(&purpose, &material).await;
            crate::platform::device_set(SECRET_KEY, sealed.as_deref());
        });
    }

    /// After a reload: opens the remembered congregation again if *Stay unlocked* has not run out.
    fn restore_session(&mut self) {
        let Some(record) = crate::platform::device_get(SESSION_KEY) else { return self.restore_account() };
        let mut lines = record.lines();
        let (Some(user), Some(id), Some(until)) = (lines.next(), lines.next(), lines.next().and_then(|u| u.parse::<u64>().ok())) else {
            self.forget_session();
            return self.restore_account();
        };
        if until < crate::platform::now_secs() {
            self.forget_session();
            return self.restore_account();
        }
        let (user, id) = (user.to_string(), id.to_string());
        if user == crate::mock::DEMO_USER {
            self.start_demo();
            if self.congregations.peek().iter().any(|c| c.id == id) {
                self.set_unlocked(&id);
            }
            return;
        }
        self.restoring.set(true);
        let mut state = *self;
        dioxus::dioxus_core::spawn_forever(async move {
            if let Err(e) = state.restore_stored(&user, &id).await {
                dioxus::logger::tracing::warn!("reopening the congregation failed: {e}");
                // Still signed in: the remembered account, with its congregations locked.
                if state.session.peek().is_none() {
                    state.restore_account();
                }
            }
            state.restoring.set(false);
        });
    }

    /// Opens a stored congregation again with the secret of [`AppState::save_session_secret`].
    async fn restore_stored(&mut self, user: &str, id: &str) -> Result<(), String> {
        let sealed = crate::platform::device_get(SECRET_KEY).ok_or("nothing kept")?;
        let Some(plain) = crate::platform::device_open(&session_purpose(user, id), &sealed).await else {
            self.forget_session();
            return Err("the kept key does not open on this device".into());
        };
        match (plain.first(), plain.len()) {
            (Some(1), 33) => {
                if !self.accounts.peek().iter().any(|a| a.username == user && a.kind == AccountKind::Device) {
                    self.forget_session();
                    return Err("the account is no longer on this device".into());
                }
                let root = theo_crypto::RootKey::from_bytes(&plain[1..]).map_err(|e| e.to_string())?;
                self.session.set(Some(user.to_string()));
                self.unlock_with(id, "", Some(root), false).await.map_err(|e| e.to_string())
            }
            (Some(2), 65) => {
                let keys = AccountKeys::from_bytes(&plain[1..]).map_err(|e| e.to_string())?;
                self.sign_in_with_keys(user, keys).await.map_err(|e| e.to_string())?;
                self.unlock(id, "", false).await.map_err(|e| e.to_string())
            }
            _ => {
                self.forget_session();
                Err("the kept key is damaged".into())
            }
        }
    }

    /// "Got it": the notices of `person` are seen, also after the app is reloaded on this device.
    pub fn mark_notices_seen(&mut self, person: u32) {
        let Some(id) = self.unlocked.peek().clone() else { return };
        let keys: Vec<String> = self.directory().my_notices(person).into_iter().map(notice_key).collect();
        self.update_directory(|d| d.mark_notices_seen(person));
        let mut seen: Vec<String> = crate::platform::device_get(&seen_key(&id)).map(|v| v.lines().map(String::from).collect()).unwrap_or_default();
        seen.extend(keys);
        let start = seen.len().saturating_sub(500);
        crate::platform::device_set(&seen_key(&id), Some(&seen[start..].join("\n")));
    }

    /// Marks the notices this device has already seen (after a reload).
    fn apply_seen(id: &str, dir: &mut Directory) {
        let Some(seen) = crate::platform::device_get(&seen_key(id)) else { return };
        let seen: std::collections::HashSet<&str> = seen.lines().collect();
        for n in dir.notices.iter_mut() {
            if seen.contains(notice_key(n).as_str()) {
                n.seen = true;
            }
        }
    }
}

pub fn use_app_state() -> AppState {
    use_context::<AppState>()
}

/// The congregation's details as stored (`theo_core::collections::DETAILS`).
fn details_value(c: &Congregation) -> serde_json::Value {
    let d = crate::backup::Details {
        name: c.name.clone(),
        number: c.number.clone(),
        language: c.language.clone(),
        time_zone: c.time_zone.clone(),
        midweek: c.midweek.clone(),
        weekend: c.weekend.clone(),
        defaults: c.defaults.clone(),
    };
    serde_json::to_value(d).unwrap_or_default()
}

fn apply_details(c: &mut Congregation, d: crate::backup::Details) {
    c.name = d.name;
    c.number = d.number;
    c.language = d.language;
    c.time_zone = d.time_zone;
    c.midweek = d.midweek;
    c.weekend = d.weekend;
    c.defaults = d.defaults;
}
