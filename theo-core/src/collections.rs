//! Collections and key domains (`REQUIREMENTS.md` 5.x and 6.2): every collection a congregation
//! stores belongs to exactly **one** key domain, and its records are encrypted with that domain's
//! keys (`theo_crypto::envelope`). The mapping is fixed here; `theo-db` only sees collection
//! names and envelopes.
//!
//! A collection is a field of [`crate::people::Directory`] with the same name. A test checks that
//! every field has a domain, so a new module cannot forget it.

use theo_crypto::DomainId;

/// One collection.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Collection {
    /// The collection name (the `Directory` field, and the name `theo-db` stores).
    pub name: &'static str,
    pub domain: DomainId,
}

const fn c(name: &'static str, domain: DomainId) -> Collection {
    Collection { name, domain }
}

const SCHEDULE: DomainId = DomainId::SCHEDULE;
const PEOPLE: DomainId = DomainId::PEOPLE;
const REPORTS: DomainId = DomainId::REPORTS;
const TERRITORIES: DomainId = DomainId::TERRITORIES;

/// Every collection and its key domain.
pub const COLLECTIONS: &[Collection] = &[
    // The congregation's details.
    c(DETAILS, SCHEDULE),
    // People (M-01, M-02, M-09): contact details, families, groups, privileges, permissions.
    c("people", PEOPLE),
    c("families", PEOPLE),
    c("groups", PEOPLE),
    c("labels", PEOPLE),
    c("absences", PEOPLE),
    c("access", PEOPLE),
    c("owner", PEOPLE),
    c("log", PEOPLE),
    c("custom_tasks", PEOPLE),
    // Reports (M-10, M-11, My ministry).
    c("reports", REPORTS),
    c("aux_short_months", REPORTS),
    c("aux_plans", REPORTS),
    c("attendance", REPORTS),
    c("ministry_log", REPORTS),
    c("ministry_carried", REPORTS),
    c("personal_goals", REPORTS),
    // Schedule: meetings, duties, public talks, public witnessing, calendar, notices.
    c("events", SCHEDULE),
    c("service_presets", SCHEDULE),
    c("service_meetings", SCHEDULE),
    c("notices", SCHEDULE),
    c("witnessing", SCHEDULE),
    c("shift_presets", SCHEDULE),
    c("shifts", SCHEDULE),
    c("midweek_weeks", SCHEDULE),
    c("midweek_settings", SCHEDULE),
    c("weekend_weeks", SCHEDULE),
    c("weekend_settings", SCHEDULE),
    c("talk_settings", SCHEDULE),
    c("other_congregations", SCHEDULE),
    c("visiting_speakers", SCHEDULE),
    c("speaker_talks", SCHEDULE),
    c("outgoing_talks", SCHEDULE),
    c("duty_types", SCHEDULE),
    c("duty_meetings", SCHEDULE),
    c("cleaning", SCHEDULE),
    c("maintenance_tasks", SCHEDULE),
    // Territories (M-07).
    c("territories", TERRITORIES),
    c("territory_assignments", TERRITORIES),
    c("territory_settings", TERRITORIES),
    c("territory_categories", TERRITORIES),
    c("territory_labels", TERRITORIES),
    c("assignment_labels", TERRITORIES),
];

/// The congregation's own details (name, meeting times, display defaults): not a `Directory`
/// field, stored next to it.
pub const DETAILS: &str = "congregation";

/// The key domain of a collection.
pub fn domain_of(collection: &str) -> Option<DomainId> {
    COLLECTIONS.iter().find(|c| c.name == collection).map(|c| c.domain)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_directory_field_has_exactly_one_domain() {
        let json = serde_json::to_value(crate::people::Directory::default()).unwrap_or_default();
        let fields: std::collections::BTreeSet<String> = json.as_object().map(|o| o.keys().cloned().collect()).unwrap_or_default();
        let named: std::collections::BTreeSet<String> = COLLECTIONS.iter().filter(|c| c.name != DETAILS).map(|c| c.name.to_string()).collect();
        assert_eq!(named.len() + 1, COLLECTIONS.len(), "a collection is listed twice");
        assert_eq!(fields, named, "Directory fields and collections differ");
        assert_eq!(domain_of("weekend_weeks"), Some(DomainId::SCHEDULE));
        assert_eq!(domain_of("nope"), None);
    }
}
