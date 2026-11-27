//! App permissions (M-02). Rules: `docs/model/m02-users-and-access.md`.
//!
//! Permissions belong to the **link between an account and a person** (the membership), never to
//! the person alone: only people who joined the app through an invite can have them. A person
//! with only contact details, or managed by others, has no permissions.

use crate::people::{AccountLink, Directory};
use std::collections::BTreeMap;

/// Access to one page (sub-menu). Write includes read.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize)]
pub enum Level {
    #[default]
    None,
    Read,
    Write,
}

impl Level {
    pub fn value(self) -> &'static str {
        match self {
            Level::None => "none",
            Level::Read => "read",
            Level::Write => "write",
        }
    }
    pub fn parse(s: &str) -> Self {
        match s {
            "read" => Level::Read,
            "write" => Level::Write,
            _ => Level::None,
        }
    }
}

/// Pages whose access is not a page level: only administrators manage permissions.
pub const ADMIN_ONLY_PAGES: [&str; 1] = ["users/permissions"];

/// Everything one account may do in the app.
#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Access {
    /// All permissions, now and for pages added later.
    pub admin: bool,
    /// Per page (`"section/page"`, as in the navigation): read or write.
    pub pages: BTreeMap<String, Level>,
    /// Write the field service reports of their own group (without "write" on Reports).
    pub own_group_reports: bool,
    /// Read their own group's members: contact details, field service reports, family.
    pub own_group_read: bool,
    /// Read the members of every field service group (the same information).
    pub all_groups_read: bool,
}

impl Access {
    pub fn administrator() -> Self {
        Access { admin: true, ..Default::default() }
    }

    /// The access to a page. Administrators have write everywhere.
    pub fn level(&self, page: &str) -> Level {
        if self.admin {
            Level::Write
        } else if ADMIN_ONLY_PAGES.contains(&page) {
            Level::None
        } else {
            self.pages.get(page).copied().unwrap_or_default()
        }
    }

    /// How many permissions are switched on (for summaries).
    pub fn count(&self) -> usize {
        self.pages.values().filter(|l| **l != Level::None).count()
            + usize::from(self.own_group_reports)
            + usize::from(self.own_group_read)
            + usize::from(self.all_groups_read)
    }

    /// "Administrator", "No permissions" or "5 permissions".
    pub fn summary(&self) -> String {
        match (self.admin, self.count()) {
            (true, _) => "Administrator".into(),
            (false, 0) => "No permissions".into(),
            (false, 1) => "1 permission".into(),
            (false, n) => format!("{n} permissions"),
        }
    }
}

/// A preset: a ready set of permissions to start from. Applying it copies the permissions into
/// the account, which can then be changed; later changes to presets do not touch accounts.
pub struct Preset {
    pub id: &'static str,
    pub label: &'static str,
    pub description: &'static str,
    pub access: fn() -> Access,
}

fn pages(pairs: &[(&str, Level)]) -> BTreeMap<String, Level> {
    pairs.iter().map(|(p, l)| (p.to_string(), *l)).collect()
}

/// Built-in presets, in the order they are offered.
pub const PRESETS: [Preset; 7] = [
    Preset { id: "secretary", label: "Secretary", description: "People, families, privileges, all reports and groups.", access: || Access {
        pages: pages(&[("users/all", Level::Write), ("users/families", Level::Write), ("users/privileges", Level::Write), ("users/moved-away", Level::Write), ("ministry/reports", Level::Write), ("ministry/groups", Level::Write), ("meetings/events", Level::Read), ("settings", Level::Read)]),
        all_groups_read: true,
        ..Default::default()
    } },
    Preset { id: "service-overseer", label: "Service overseer", description: "Everything in Ministry, and reading all groups.", access: || Access {
        pages: pages(&[("ministry/reports", Level::Write), ("ministry/groups", Level::Write), ("ministry/territories", Level::Write), ("ministry/witnessing", Level::Write), ("ministry/service-meetings", Level::Write), ("users/all", Level::Read)]),
        all_groups_read: true,
        ..Default::default()
    } },
    Preset { id: "group-overseer", label: "Group overseer", description: "Their own group: members and reports.", access: || Access {
        pages: pages(&[("ministry/service-meetings", Level::Read), ("meetings/midweek", Level::Read), ("meetings/weekend", Level::Read)]),
        own_group_read: true,
        own_group_reports: true,
        ..Default::default()
    } },
    Preset { id: "lmm-overseer", label: "Life and Ministry overseer", description: "The midweek meeting schedule.", access: || Access {
        pages: pages(&[("meetings/midweek", Level::Write), ("users/privileges", Level::Read), ("users/all", Level::Read)]),
        ..Default::default()
    } },
    Preset { id: "talk-coordinator", label: "Public talk coordinator", description: "Public talks, outgoing talks and the weekend meeting.", access: || Access {
        pages: pages(&[("meetings/talks", Level::Write), ("meetings/outgoing", Level::Write), ("meetings/weekend", Level::Write), ("users/privileges", Level::Read)]),
        ..Default::default()
    } },
    Preset { id: "duties-coordinator", label: "Duties coordinator", description: "Audio, video and stage, attendants, microphones, cleaning and maintenance.", access: || Access {
        pages: pages(&[("departments/audio-video", Level::Write), ("departments/attendants", Level::Write), ("departments/microphones", Level::Write), ("departments/cleaning", Level::Write), ("departments/maintenance", Level::Write), ("meetings/events", Level::Read), ("users/privileges", Level::Read)]),
        ..Default::default()
    } },
    Preset { id: "publisher", label: "Publisher", description: "No extra permissions: their own assignments, reports and group names.", access: Access::default },
];

pub fn preset(id: &str) -> Option<&'static Preset> {
    PRESETS.iter().find(|p| p.id == id)
}

/// One line in the activity log (in *Congregation › Settings*). Kept inside the encrypted data.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct LogEntry {
    /// `YYYY-MM-DD HH:MM`, UTC.
    pub at: String,
    /// Who made the change (display name of the account).
    pub actor: String,
    pub text: String,
}

/// Every page a permission can be given for (`section/page`), with its name. The app's
/// navigation adds icons and descriptions; this is the permission model's own list (R-245).
pub const PAGES: &[(&str, &str)] = &[
    ("users/all", "All users"),
    ("users/privileges", "Privileges"),
    ("users/absences", "Absences"),
    ("users/permissions", "Permissions"),
    ("users/elderly-care", "Elderly care"),
    ("users/moved-away", "Moved away"),
    ("meetings/midweek", "Midweek meeting"),
    ("meetings/weekend", "Weekend meeting"),
    ("meetings/talks", "Public talks"),
    ("meetings/outgoing", "Outgoing talks"),
    ("meetings/attendance", "Meeting attendance"),
    ("meetings/events", "Events"),
    ("ministry/reports", "Field service reports"),
    ("ministry/groups", "Field service groups"),
    ("ministry/territories", "Territories"),
    ("ministry/witnessing", "Public witnessing"),
    ("ministry/service-meetings", "Meetings for field service"),
    ("departments/audio-video", "Audio, video and stage"),
    ("departments/attendants", "Attendants"),
    ("departments/microphones", "Microphones"),
    ("departments/cleaning", "Cleaning"),
    ("departments/maintenance", "Maintenance"),
    ("departments/custom-tasks", "Custom tasks"),
];

/// The name of a page key ("meetings/midweek" → "Midweek meeting").
pub fn page_label(key: &str) -> String {
    if key == "settings" {
        return "Congregation settings".into();
    }
    PAGES.iter().find(|(k, _)| *k == key).map(|(_, l)| l.to_string()).unwrap_or_else(|| key.to_string())
}

fn level_label(l: Level) -> &'static str {
    match l {
        Level::None => "no access",
        Level::Read => "read",
        Level::Write => "write",
    }
}

/// What changed between two sets of permissions, in words ("Audio, video and stage: read → write").
pub fn describe_changes(old: &Access, new: &Access) -> Vec<String> {
    let mut out = Vec::new();
    if old.admin != new.admin {
        out.push(if new.admin { "made administrator".to_string() } else { "no longer administrator".to_string() });
    }
    for (flag, a, b) in [
        ("read their own group", old.own_group_read, new.own_group_read),
        ("read all groups", old.all_groups_read, new.all_groups_read),
        ("write the reports of their own group", old.own_group_reports, new.own_group_reports),
    ] {
        if a != b {
            out.push(format!("{flag}: {}", if b { "on" } else { "off" }));
        }
    }
    let keys: std::collections::BTreeSet<&String> = old.pages.keys().chain(new.pages.keys()).collect();
    for k in keys {
        let (a, b) = (old.pages.get(k).copied().unwrap_or_default(), new.pages.get(k).copied().unwrap_or_default());
        if a != b {
            out.push(format!("{}: {} → {}", page_label(k), level_label(a), level_label(b)));
        }
    }
    out
}

impl Directory {
    /// Can this person have permissions: they joined the app (their account is linked).
    pub fn has_account(&self, person: u32) -> bool {
        self.person(person).is_some_and(|p| matches!(p.account, AccountLink::Linked { .. }) && p.moved_away.is_none())
    }

    /// The people who joined the app.
    pub fn accounts(&self) -> Vec<&crate::people::Person> {
        self.people.iter().filter(|p| self.has_account(p.id)).collect()
    }

    /// The person who created the congregation: always an administrator.
    pub fn is_owner(&self, person: u32) -> bool {
        self.owner == Some(person)
    }

    pub fn access(&self, person: u32) -> Access {
        if self.is_owner(person) {
            return Access::administrator();
        }
        if !self.has_account(person) {
            return Access::default();
        }
        self.access.get(&person).cloned().unwrap_or_default()
    }

    /// Saves the permissions of an account and records the change in the activity log. Refused
    /// for people without an account; the owner always stays an administrator.
    pub fn set_access(&mut self, person: u32, mut access: Access, actor: &str) -> Result<(), String> {
        if !self.has_account(person) {
            return Err("Only people who joined the app can have permissions.".into());
        }
        if self.is_owner(person) {
            access.admin = true;
        }
        access.pages.retain(|page, level| *level != Level::None && !ADMIN_ONLY_PAGES.contains(&page.as_str()));
        let changes = describe_changes(&self.access(person), &access);
        if !changes.is_empty() {
            let name = self.person(person).map(|p| format!("{} {}", p.first_name, p.last_name)).unwrap_or_default();
            self.log.push(LogEntry { at: crate::prefs::now_stamp(), actor: actor.to_string(), text: format!("Permissions of {name}: {}.", changes.join("; ")) });
        }
        self.access.insert(person, access);
        Ok(())
    }

    /// Ends the permissions when the account link ends (moved away, deleted, access removed).
    pub(crate) fn drop_access(&mut self, person: u32) {
        self.access.remove(&person);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_linked_accounts_get_permissions_and_the_owner_stays_admin() {
        let mut d = crate::demo::demo_directory(14);
        // Person 3 (Tom) has no account.
        assert!(d.set_access(3, Access::administrator(), "Test").is_err());
        assert_eq!(d.access(3), Access::default());
        // The owner cannot lose admin.
        let owner = d.owner.unwrap_or(1);
        assert!(d.set_access(owner, Access::default(), "Test").is_ok());
        assert!(d.access(owner).admin);
        // Write includes read; the permissions page is for administrators only.
        let mut a = Access::default();
        a.pages.insert("departments/audio-video".into(), Level::Write);
        a.pages.insert("users/permissions".into(), Level::Write);
        assert_eq!(a.level("departments/audio-video"), Level::Write);
        assert_eq!(a.level("users/permissions"), Level::None);
    }

    #[test]
    fn presets_fill_permissions_and_changes_are_logged() {
        let mut d = crate::demo::demo_directory(14);
        // Andrew (13) has an account and no permissions.
        let secretary = preset("secretary").map(|p| (p.access)()).unwrap_or_default();
        assert!(d.set_access(13, secretary, "Demo User").is_ok());
        assert_eq!(d.access(13).level("ministry/reports"), Level::Write);
        let entry = d.log.last().cloned().unwrap_or(LogEntry { at: String::new(), actor: String::new(), text: String::new() });
        assert_eq!(entry.actor, "Demo User");
        assert!(entry.text.contains("Andrew Lee") && entry.text.contains("Field service reports: no access → write"));
        // Saving the same permissions again records nothing.
        let n = d.log.len();
        let same = d.access(13);
        assert!(d.set_access(13, same, "Demo User").is_ok());
        assert_eq!(d.log.len(), n);
    }

    #[test]
    fn moving_away_ends_the_permissions() {
        let mut d = crate::demo::demo_directory(14);
        let linked = d.accounts().iter().map(|p| p.id).find(|n| !d.is_owner(*n)).unwrap_or(0);
        let a = Access { own_group_read: true, ..Default::default() };
        assert!(d.set_access(linked, a, "Test").is_ok());
        d.move_away(linked, "2026-11-01", false);
        assert_eq!(d.access(linked), Access::default());
    }
}
