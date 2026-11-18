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
            page("all", "All users", Icon::Contact, "Publishers and other members, contact details and families."),
            page("privileges", "Privileges", Icon::Privilege, "What each person may take part in: meeting parts and duties."),
            page("absences", "Absences", Icon::Absence, "Who is away when, so schedules leave them out."),
            page("permissions", "Permissions", Icon::Permission, "What each account may see and change in the app."),
            page("elderly-care", "Elderly care", Icon::Family, "Transport, visits and help for older ones."),
            page("moved-away", "Moved away", Icon::MovedAway, "People who moved away, kept for 1 year."),
        ]),
        NavEntry::new("meetings", "Meetings", Icon::Meetings).with_description("Meeting schedules, talks and attendance").with_children(vec![
            page("midweek", "Midweek meeting", Icon::BookOpen, "Life and Ministry meeting parts and students."),
            page("weekend", "Weekend meeting", Icon::Talk, "Chairman, public talk and Watchtower Study."),
            page("talks", "Public talks", Icon::PublicTalk, "Incoming speakers and the talk catalogue."),
            page("outgoing", "Outgoing talks", Icon::Outgoing, "Local speakers visiting other congregations."),
            page("attendance", "Meeting attendance", Icon::Chart, "Attendance per meeting, in person and online."),
            page("events", "Events", Icon::Calendar, "Circuit overseer visits, assemblies, Memorial."),
        ]),
        NavEntry::new("ministry", "Ministry", Icon::Ministry).with_description("Field service organisation").with_children(vec![
            page("reports", "Field service reports", Icon::Report, "Monthly reports, pioneers and missing reports."),
            page("groups", "Field service groups", Icon::Group, "Groups, overseers and assistants."),
            page("territories", "Territories", Icon::Map, "Territory cards, maps, check-out and coverage."),
            page("witnessing", "Public witnessing", Icon::PublicWitnessing, "Locations, carts and shifts."),
            page("service-meetings", "Meetings for field service", Icon::Footprints, "Who conducts, where and when."),
        ]),
        NavEntry::new("departments", "Departments", Icon::Building).with_description("Duties that keep the meetings and the Kingdom Hall running").with_children(vec![
            page("audio-video", "Audio, video and stage", Icon::Video, "Sound, video, Zoom and the stage at each meeting."),
            page("attendants", "Attendants", Icon::Attendant, "Attendants at the entrance and in the hall."),
            page("microphones", "Microphones", Icon::Audio, "Roving microphones at each meeting."),
            page("cleaning", "Cleaning", Icon::Cleaning, "Cleaning groups and rota."),
            page("maintenance", "Maintenance", Icon::Maintenance, "Kingdom Hall maintenance tasks and who does them."),
            page("custom-tasks", "Custom tasks", Icon::Task, "Any other task, like the literature desk, and who does it."),
        ]),
    ]
}

pub fn section(id: &str) -> Option<NavEntry> {
    sections().into_iter().find(|s| s.id == id)
}

pub fn page_of(section_id: &str, page_id: &str) -> Option<NavEntry> {
    section(section_id)?.children.into_iter().find(|p| p.id == page_id)
}
