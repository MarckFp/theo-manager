//! App state for the scaffold.
//!
//! ⚠️ MOCK: everything lives in memory and is lost on reload. Passwords are compared in plain
//! text. This module is a stand-in for `theo-core` (session, keyring, unlock) and `theo-db`
//! (storage), which replace it in phase 1. Screens talk only to [`AppState`], so swapping the
//! implementation does not change them.

use dioxus::prelude::*;
use std::collections::HashMap;
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
    password: String, // MOCK ONLY
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
    password: String, // MOCK ONLY: the real app never stores it (R-124)
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
}

impl std::fmt::Display for AuthError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            AuthError::WrongCredentials => "Username or password is not correct.",
            AuthError::UsernameTaken => "This username is already taken.",
            AuthError::WrongPassword => "This password does not unlock the congregation.",
        })
    }
}

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
    next_id: Signal<u64>,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            accounts: Signal::new(Vec::new()),
            congregations: Signal::new(Vec::new()),
            session: Signal::new(None),
            unlocked: Signal::new(None),
            preselect: Signal::new(None),
            theme: Signal::new(Theme::System),
            layouts: Signal::new(HashMap::new()),
            directories: Signal::new(HashMap::new()),
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
            next_id: Signal::new(1),
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
        self.accounts.write().push(Account {
            username: username.clone(),
            display_name: display_name.trim().to_string(),
            kind,
            overrides: Overrides::default(),
            password: password.to_string(),
        });
        self.session.set(Some(username));
        self.unlocked.set(None);
        Ok(())
    }

    pub fn sign_in(&mut self, username: &str, password: &str) -> Result<(), AuthError> {
        let username = username.trim().to_lowercase();
        let ok = self.accounts.read().iter().any(|a| a.username == username && a.password == password);
        if !ok {
            return Err(AuthError::WrongCredentials);
        }
        self.session.set(Some(username));
        self.unlocked.set(None);
        Ok(())
    }

    pub fn sign_out(&mut self) {
        self.unlocked.set(None);
        self.session.set(None);
        self.preselect.set(None);
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

    /// May change `page` (`"section/page"`): administrators, and people with write on it.
    pub fn can_write(&self, page: &str) -> bool {
        self.i_am_admin() || self.me().is_some_and(|p| self.directory().access(p).level(page) == crate::access::Level::Write)
    }

    pub fn active_congregation(&self) -> Option<Congregation> {
        let id = self.unlocked.read().clone()?;
        self.congregations.read().iter().find(|c| c.id == id).cloned()
    }

    pub fn create_congregation(&mut self, new: CongregationDraft) -> String {
        let n = *self.next_id.peek();
        self.next_id.set(n + 1);
        let id = format!("c{n}");
        let owner = self.session.read().clone().unwrap_or_default();
        self.congregations.write().push(Congregation {
            id: id.clone(),
            name: new.name.trim().to_string(),
            number: new.number.trim().to_string(),
            language: new.language,
            time_zone: new.time_zone,
            midweek: new.midweek,
            weekend: new.weekend,
            mode: new.mode,
            owner,
            biometric: false,
            defaults: Preferences::default(),
            password: new.password,
        });
        // The creator is a person of the congregation, linked to their account and its owner.
        let mut dir = Directory::default();
        let draft = crate::people::PersonDraft {
            first_name: new.my_first_name.trim().into(),
            last_name: new.my_last_name.trim().into(),
            gender: new.my_gender.clone(),
            publisher_type: crate::people::PublisherType::BaptisedPublisher.value().into(),
            ..Default::default()
        };
        if let Ok(me) = dir.save(None, &draft) {
            if let Some(p) = dir.people.iter_mut().find(|p| p.id == me) {
                p.account = crate::people::AccountLink::Linked { username: self.session.read().clone().unwrap_or_default() };
            }
            dir.owner = Some(me);
        }
        self.directories.write().insert(id.clone(), dir);
        self.set_unlocked(&id);
        id
    }

    /// Leaves the open congregation: it is removed from this account's list on this device.
    pub fn leave_congregation(&mut self) {
        let Some(id) = self.unlocked.peek().clone() else { return };
        let user = self.session.peek().clone().unwrap_or_default();
        self.congregations.write().retain(|c| !(c.id == id && c.owner == user));
        self.directories.write().remove(&id);
        self.layouts.write().remove(&id);
        self.unlocked.set(None);
    }

    pub fn unlock(&mut self, id: &str, password: &str, remember_biometric: bool) -> Result<(), AuthError> {
        let mut list = self.congregations.write();
        let Some(c) = list.iter_mut().find(|c| c.id == id) else { return Err(AuthError::WrongPassword) };
        if c.password != password {
            return Err(AuthError::WrongPassword);
        }
        if remember_biometric {
            c.biometric = true;
        }
        drop(list);
        self.set_unlocked(id);
        Ok(())
    }

    /// Is this the password of the congregation? (Checks only; does not unlock.)
    pub fn password_matches(&self, id: &str, password: &str) -> bool {
        self.congregations.read().iter().any(|c| c.id == id && c.password == password)
    }

    /// Opens a congregation after the platform's biometric check said it is the user
    /// (`platform::biometric_verify`). MOCK: no key is unlocked yet.
    pub fn unlock_with_biometrics(&mut self, id: &str) -> bool {
        let known = self.congregations.read().iter().any(|c| c.id == id);
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
    }

    pub fn set_biometric(&mut self, id: &str, on: bool) {
        if let Some(c) = self.congregations.write().iter_mut().find(|c| c.id == id) {
            c.biometric = on;
        }
    }

    pub fn lock(&mut self) {
        let current = self.unlocked.read().clone();
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
        }
    }

    pub fn update_congregation_defaults(&mut self, f: impl FnOnce(&mut Preferences)) {
        let Some(id) = self.unlocked.read().clone() else { return };
        if let Some(c) = self.congregations.write().iter_mut().find(|c| c.id == id) {
            f(&mut c.defaults);
        }
    }

    // ----- people (MOCK of M-01 / M-09) -----

    /// People, families and groups of the unlocked congregation.
    pub fn directory(&self) -> Directory {
        let id = self.unlocked.read().clone().unwrap_or_default();
        self.directories.read().get(&id).cloned().unwrap_or_default()
    }

    pub fn update_directory<R>(&mut self, f: impl FnOnce(&mut Directory) -> R) -> R {
        let id = self.unlocked.read().clone().unwrap_or_default();
        f(self.directories.write().entry(id).or_default())
    }

    // ----- backup (docs/design/local-sharing.md, "Backup") -----

    /// MOCK: the real check unwraps the root key with the password (theo-crypto).
    pub fn congregation_password_ok(&self, password: &str) -> bool {
        self.active_congregation().is_some_and(|c| c.password == password)
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

    pub fn layout(&self) -> Vec<WidgetLayout> {
        let id = self.unlocked.read().clone().unwrap_or_default();
        self.layouts.read().get(&id).cloned().unwrap_or_else(crate::mock::default_layout)
    }

    pub fn set_layout(&mut self, layout: Vec<WidgetLayout>) {
        let id = self.unlocked.read().clone().unwrap_or_default();
        self.layouts.write().insert(id, layout);
    }

    pub fn reset_layout(&mut self) {
        let id = self.unlocked.read().clone().unwrap_or_default();
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
                });
                let size = if id == "demo-1" { 36 } else { 14 };
                self.directories.write().insert(id.into(), crate::mock::demo_directory(size));
            }
        }
        self.session.set(Some(crate::mock::DEMO_USER.into()));
        self.unlocked.set(None);
    }
}

pub fn use_app_state() -> AppState {
    use_context::<AppState>()
}
