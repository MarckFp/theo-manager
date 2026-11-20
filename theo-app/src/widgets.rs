//! The home widgets: which exist, their first size, and who sees them. **Personal** widgets
//! (*For you*) are about the signed-in person and everyone has them. **Congregation** widgets
//! need read access to the page they summarise (M-02).

use theo_ui::{Icon, WidgetLayout};

pub struct WidgetDef {
    pub id: &'static str,
    pub title: &'static str,
    pub icon: Icon,
    pub cols: u8,
    pub rows: u8,
    /// About me (shown first, to everyone).
    pub personal: bool,
    /// The page whose read access a congregation widget needs.
    pub page: Option<&'static str>,
}

const fn def(id: &'static str, title: &'static str, icon: Icon, cols: u8, rows: u8, page: Option<&'static str>) -> WidgetDef {
    WidgetDef { id, title, icon, cols, rows, personal: page.is_none(), page }
}

/// Every widget, in the default order.
pub const ALL: [WidgetDef; 13] = [
    // For you
    def("assignments", "My assignments", Icon::Assignment, 1, 2, None),
    def("my-reports", "My field service reports", Icon::Footprints, 1, 1, None),
    def("my-goal", "My goal", Icon::Sparkles, 1, 1, None),
    def("my-territories", "My territories", Icon::Map, 1, 1, None),
    def("next-meeting", "Next meeting", Icon::Meetings, 2, 1, None),
    def("service-meetings", "Meetings for field service", Icon::Group, 1, 1, None),
    def("events", "Upcoming events", Icon::Calendar, 1, 1, None),
    // Congregation
    def("publishers", "Publishers", Icon::Users, 1, 1, Some("users/all")),
    def("reports", "Field service reports", Icon::Report, 1, 1, Some("ministry/reports")),
    def("territories", "Territories", Icon::Map, 1, 1, Some("ministry/territories")),
    def("away", "Away this week", Icon::Absence, 1, 1, Some("users/absences")),
    def("attendance", "Meeting attendance", Icon::Chart, 2, 1, Some("meetings/attendance")),
    def("duties", "Duties this week", Icon::Audio, 2, 1, Some("departments/audio-video")),
];

pub fn find(id: &str) -> Option<&'static WidgetDef> {
    ALL.iter().find(|d| d.id == id)
}

pub fn layout_of(d: &WidgetDef) -> WidgetLayout {
    WidgetLayout::new(d.id, d.title, d.icon, d.cols, d.rows)
}

/// The layout a new user starts with: every widget.
pub fn default_layout() -> Vec<WidgetLayout> {
    ALL.iter().map(layout_of).collect()
}

/// May this user see the widget? `can_read` answers for a page.
pub fn allowed(id: &str, can_read: impl Fn(&str) -> bool) -> bool {
    find(id).is_some_and(|d| d.page.is_none_or(can_read))
}

pub fn is_personal(id: &str) -> bool {
    find(id).is_some_and(|d| d.personal)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn personal_widgets_come_first_and_need_no_permission() {
        let first_admin = ALL.iter().position(|d| !d.personal).unwrap_or(ALL.len());
        assert!(ALL[first_admin..].iter().all(|d| !d.personal));
        assert!(allowed("my-reports", |_| false));
        assert!(!allowed("publishers", |_| false));
        assert!(allowed("publishers", |p| p == "users/all"));
        assert!(!allowed("unknown", |_| true));
    }
}
