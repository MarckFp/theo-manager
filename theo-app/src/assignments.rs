//! My assignments: the model is in `theo-core`. This module adds the colour and icon of each kind.

pub use theo_core::assignments::*;

/// How an assignment kind is shown.
pub trait AssignmentStyle {
    /// Its colour in lists (each kind its own).
    fn accent(self) -> theo_ui::Accent;
    /// Its icon.
    fn icon(self) -> theo_ui::Icon;
}

impl AssignmentStyle for AssignmentKind {
    fn accent(self) -> theo_ui::Accent {
        match self {
            AssignmentKind::FieldServiceMeeting => theo_ui::Accent::One,
            AssignmentKind::PublicWitnessing => theo_ui::Accent::Two,
            AssignmentKind::MidweekMeeting => theo_ui::Accent::Three,
            AssignmentKind::WeekendMeeting => theo_ui::Accent::Four,
            AssignmentKind::Duty => theo_ui::Accent::Five,
            AssignmentKind::OutgoingTalk => theo_ui::Accent::Six,
        }
    }

    fn icon(self) -> theo_ui::Icon {
        match self {
            AssignmentKind::FieldServiceMeeting => theo_ui::Icon::Footprints,
            AssignmentKind::PublicWitnessing => theo_ui::Icon::PublicWitnessing,
            AssignmentKind::MidweekMeeting => theo_ui::Icon::BookOpen,
            AssignmentKind::WeekendMeeting => theo_ui::Icon::Talk,
            AssignmentKind::Duty => theo_ui::Icon::Audio,
            AssignmentKind::OutgoingTalk => theo_ui::Icon::Outgoing,
        }
    }
}
