//! ⚠️ MOCK helpers and sample data. Replaced by `theo-crypto` (passphrases, zxcvbn) and real
//! module data in later phases. Nothing here is cryptographically secure.

use theo_ui::{ChartPoint, Icon, ListEntry, WidgetLayout};

pub const DEMO_USER: &str = "demo";
pub const DEMO_PASSWORD: &str = "demo-password";
pub const DEMO_CONGREGATION_PASSWORD: &str = "demo";

pub use theo_core::demo::{demo_directory, token};

pub fn default_layout() -> Vec<WidgetLayout> {
    crate::widgets::default_layout()
}

pub fn attendance() -> Vec<ChartPoint> {
    [("Jun", 92.0), ("Jul", 88.0), ("Aug", 85.0), ("Sep", 96.0), ("Oct", 101.0), ("Nov", 104.0)]
        .into_iter()
        .map(|(l, v)| ChartPoint::new(l, v))
        .collect()
}

pub fn publishers_trend() -> Vec<f64> {
    vec![78.0, 79.0, 79.0, 81.0, 80.0, 82.0, 84.0, 84.0]
}

pub fn my_assignments() -> Vec<ListEntry> {
    vec![
        entry(Icon::Meetings, "Bible reading", "Midweek meeting", "Thu 20"),
        entry(Icon::Audio, "Audio and video", "Weekend meeting", "Sun 23"),
        entry(Icon::Users, "Attendant", "Midweek meeting", "Thu 4 Dec"),
        entry(Icon::PublicTalk, "Prayer", "Weekend meeting", "Sun 7 Dec"),
    ]
}

pub fn duties() -> Vec<ListEntry> {
    vec![
        entry(Icon::Audio, "Audio · Video", "J. Smith · M. Brown", "Thu"),
        entry(Icon::Users, "Attendants", "A. Lee · D. Clark", "Thu"),
        entry(Icon::Globe, "Zoom host", "P. Walker", "Sun"),
    ]
}

fn entry(icon: Icon, title: &str, subtitle: &str, meta: &str) -> ListEntry {
    ListEntry { icon, title: title.into(), subtitle: subtitle.into(), meta: meta.into() }
}
