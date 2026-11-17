//! The app's navigation model: 5 main sections (sidebar and dock) and their pages.
//!
//! This is a first proposal of the sub-menus; each page gets its data model in phase 3.
//! Ids are used in URLs: `/app/<section>` (hub) and `/app/<section>/<page>`.

use theo_ui::{Icon, NavEntry};

fn page(id: &str, label: &str, icon: Icon, description: &str) -> NavEntry {
    NavEntry::new(id, label, icon).with_description(description)
}

pub fn sections() -> Vec<NavEntry> {
    vec![
        NavEntry::new("home", "Home", Icon::Home),
        NavEntry::new("users", "Users", Icon::Users).with_description("People, accounts and what they may do").with_children(vec![
            page("all", "All users", Icon::Contact, "Publishers and other members, contact details and households."),
            page("groups", "Field service groups", Icon::Group, "Groups, overseers and assistants."),
            page("reports", "Field service reports", Icon::Report, "Monthly reports, pioneers and missing reports."),
            page("privileges", "Privileges", Icon::Privilege, "Elders, ministerial servants, pioneers and qualifications."),
            page("permissions", "Permissions", Icon::Permission, "Roles and what each role may see and change."),
            page("moved-away", "Moved away", Icon::MovedAway, "People who moved away, kept for 1 year."),
        ]),
        NavEntry::new("meetings", "Meetings", Icon::Meetings).with_description("Schedules for meetings and duties").with_children(vec![
            page("midweek", "Midweek meeting", Icon::BookOpen, "Life and Ministry meeting parts and students."),
            page("weekend", "Weekend meeting", Icon::Talk, "Chairman, public talk and Watchtower Study."),
            page("talks", "Public talks", Icon::PublicTalk, "Incoming speakers and the talk catalogue."),
            page("outgoing", "Outgoing talks", Icon::Outgoing, "Local speakers visiting other congregations."),
            page("duties", "Duties", Icon::Audio, "Attendants, audio, video, stage, microphones and Zoom."),
            page("cleaning", "Cleaning", Icon::Cleaning, "Cleaning groups and rota."),
            page("attendance", "Attendance", Icon::Chart, "Attendance per meeting, in person and online."),
        ]),
        NavEntry::new("ministry", "Ministry", Icon::Ministry).with_description("Field service organisation").with_children(vec![
            page("territories", "Territories", Icon::Map, "Territory cards, maps, check-out and coverage."),
            page("witnessing", "Public witnessing", Icon::PublicWitnessing, "Locations, carts and shifts."),
            page("service-meetings", "Meetings for field service", Icon::Footprints, "Who conducts, where and when."),
        ]),
        NavEntry::new("congregation", "Congregation", Icon::Congregation).with_description("Settings and shared information").with_children(vec![
            page("settings", "Settings", Icon::Settings, "Details, meeting times and default display settings."),
            page("calendar", "Calendar and events", Icon::Calendar, "Circuit overseer visits, assemblies, Memorial."),
            page("congregations", "Other congregations", Icon::Building, "Nearby congregations and their coordinators."),
            page("data", "Import and export", Icon::Download, "Move data in and out, backups."),
            page("audit", "Activity log", Icon::History, "Who changed what, and when."),
        ]),
    ]
}

pub fn section(id: &str) -> Option<NavEntry> {
    sections().into_iter().find(|s| s.id == id)
}

pub fn page_of(section_id: &str, page_id: &str) -> Option<NavEntry> {
    section(section_id)?.children.into_iter().find(|p| p.id == page_id)
}
