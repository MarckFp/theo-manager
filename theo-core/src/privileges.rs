//! Privileges (M-01): what each person may take part in. Rules:
//! `docs/model/m01-people.md`, "Privileges".
//!
//! Privileges are not app permissions: they say who can be assigned to a part or a duty.
//! Permissions (M-02) say what an account may see and change in the app.

use crate::people::{Directory, Gender, Person, PublisherType};

/// Who may have a privilege. Visitors never have privileges.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Rule {
    /// Baptised brothers.
    BaptisedMale,
    /// Brothers, also Bible students and unbaptised publishers.
    Male,
    /// Baptised publishers (brothers and sisters).
    Baptised,
    /// Everyone except visitors.
    Anyone,
}

impl Rule {
    pub fn label(self) -> &'static str {
        match self {
            Rule::BaptisedMale => "Baptised brothers",
            Rule::Male => "Brothers, also Bible students",
            Rule::Baptised => "Baptised publishers",
            Rule::Anyone => "Everyone except visitors",
        }
    }

    pub fn allows(self, ty: PublisherType, gender: Gender) -> bool {
        if ty == PublisherType::Visitor {
            return false;
        }
        match self {
            Rule::BaptisedMale => ty.is_baptised() && gender == Gender::Male,
            Rule::Male => gender == Gender::Male,
            Rule::Baptised => ty.is_baptised(),
            Rule::Anyone => true,
        }
    }
}

/// Where a privilege is used; also the sections of the privileges dialog.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Section {
    BothMeetings,
    Midweek,
    Weekend,
    Duties,
    Other,
}

impl Section {
    pub const ALL: [Section; 5] = [Section::BothMeetings, Section::Midweek, Section::Weekend, Section::Duties, Section::Other];
    pub fn label(self) -> &'static str {
        match self {
            Section::BothMeetings => "Both meetings",
            Section::Midweek => "Midweek meeting",
            Section::Weekend => "Weekend meeting",
            Section::Duties => "Duties",
            Section::Other => "Ministry and care",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize)]
pub enum Privilege {
    Prayer,
    MidweekChairman,
    AuxiliaryChairman,
    Treasures,
    SpiritualGems,
    BibleReading,
    FieldMinistryDiscussion,
    StartingConversation,
    FollowingUp,
    MakingDisciples,
    Assistant,
    StudentTalk,
    LivingAsChristians,
    BibleStudyConductor,
    BibleStudyReader,
    WeekendChairman,
    PublicTalk,
    PublicTalkAway,
    WatchtowerConductor,
    WatchtowerReader,
    Attendant,
    ZoomAttendant,
    EntranceAttendant,
    Stage,
    Audio,
    Video,
    Microphones,
    Cleaning,
    Hospitality,
    Interpreter,
    FieldServiceMeeting,
    PublicWitnessing,
    ElderlyCare,
}

use Privilege as P;

/// (privilege, stored value, label, section, rule), in display order.
const TABLE: [(Privilege, &str, &str, Section, Rule); 33] = [
    (P::Prayer, "prayer", "Prayer", Section::BothMeetings, Rule::BaptisedMale),
    (P::MidweekChairman, "mw-chairman", "Chairman", Section::Midweek, Rule::BaptisedMale),
    (P::AuxiliaryChairman, "aux-chairman", "Auxiliary classroom chairman", Section::Midweek, Rule::BaptisedMale),
    (P::Treasures, "treasures", "Treasures from God's Word", Section::Midweek, Rule::BaptisedMale),
    (P::SpiritualGems, "gems", "Spiritual gems", Section::Midweek, Rule::BaptisedMale),
    (P::BibleReading, "bible-reading", "Bible reading", Section::Midweek, Rule::Male),
    (P::FieldMinistryDiscussion, "ministry-discussion", "Field ministry discussion", Section::Midweek, Rule::BaptisedMale),
    (P::StartingConversation, "starting", "Starting a conversation", Section::Midweek, Rule::Anyone),
    (P::FollowingUp, "following-up", "Following up", Section::Midweek, Rule::Anyone),
    (P::MakingDisciples, "making-disciples", "Making disciples", Section::Midweek, Rule::Anyone),
    (P::Assistant, "assistant", "Assistant", Section::Midweek, Rule::Anyone),
    (P::StudentTalk, "student-talk", "Student talk", Section::Midweek, Rule::BaptisedMale),
    (P::LivingAsChristians, "living", "Living as Christians", Section::Midweek, Rule::BaptisedMale),
    (P::BibleStudyConductor, "cbs-conductor", "Congregation Bible Study", Section::Midweek, Rule::BaptisedMale),
    (P::BibleStudyReader, "cbs-reader", "Congregation Bible Study reader", Section::Midweek, Rule::BaptisedMale),
    (P::WeekendChairman, "we-chairman", "Chairman", Section::Weekend, Rule::BaptisedMale),
    (P::PublicTalk, "public-talk", "Public talk", Section::Weekend, Rule::BaptisedMale),
    (P::PublicTalkAway, "public-talk-away", "Public talk in other congregations", Section::Weekend, Rule::BaptisedMale),
    (P::WatchtowerConductor, "wt-conductor", "Watchtower Study conductor", Section::Weekend, Rule::BaptisedMale),
    (P::WatchtowerReader, "wt-reader", "Watchtower Study reader", Section::Weekend, Rule::Baptised),
    (P::Interpreter, "interpreter", "Interpreter", Section::Weekend, Rule::Baptised),
    (P::Attendant, "attendant", "Attendant", Section::Duties, Rule::Baptised),
    (P::ZoomAttendant, "zoom", "Zoom attendant", Section::Duties, Rule::Baptised),
    (P::EntranceAttendant, "entrance", "Entrance attendant", Section::Duties, Rule::Baptised),
    (P::Stage, "stage", "Stage", Section::Duties, Rule::Baptised),
    (P::Audio, "audio", "Audio", Section::Duties, Rule::Baptised),
    (P::Video, "video", "Video", Section::Duties, Rule::Baptised),
    (P::Microphones, "microphones", "Microphones", Section::Duties, Rule::Baptised),
    (P::Cleaning, "cleaning", "Cleaning", Section::Duties, Rule::Anyone),
    (P::Hospitality, "hospitality", "Hospitality", Section::Other, Rule::Baptised),
    (P::FieldServiceMeeting, "fs-meeting", "Meeting for field service", Section::Other, Rule::Baptised),
    (P::PublicWitnessing, "public-witnessing", "Public witnessing", Section::Other, Rule::Baptised),
    (P::ElderlyCare, "elderly-care", "Elderly care", Section::Other, Rule::Anyone),
];

impl Privilege {
    pub fn all() -> impl Iterator<Item = Privilege> {
        TABLE.iter().map(|t| t.0)
    }
    fn row(self) -> &'static (Privilege, &'static str, &'static str, Section, Rule) {
        TABLE.iter().find(|t| t.0 == self).unwrap_or(&TABLE[0])
    }
    pub fn value(self) -> &'static str {
        self.row().1
    }
    pub fn label(self) -> &'static str {
        self.row().2
    }
    pub fn section(self) -> Section {
        self.row().3
    }
    pub fn rule(self) -> Rule {
        self.row().4
    }
    /// The label with its meeting where the label alone is ambiguous ("Chairman").
    pub fn full_label(self) -> String {
        match self {
            P::MidweekChairman | P::WeekendChairman => format!("{} ({})", self.label(), self.section().label().to_lowercase()),
            _ => self.label().to_string(),
        }
    }
    pub fn parse(s: &str) -> Option<Self> {
        TABLE.iter().find(|t| t.1 == s).map(|t| t.0)
    }
    pub fn allowed_for(self, ty: PublisherType, gender: Gender) -> bool {
        self.rule().allows(ty, gender)
    }
}

impl Directory {
    /// Current people who may have the privilege.
    pub fn eligible_for(&self, privilege: Privilege) -> Vec<&Person> {
        self.people.iter().filter(|p| p.moved_away.is_none() && privilege.allowed_for(p.publisher_type, p.gender)).collect()
    }

    /// Gives or takes a privilege. Ignored when the person may not have it.
    pub fn set_privilege(&mut self, person: u32, privilege: Privilege, on: bool) {
        if let Some(p) = self.people.iter_mut().find(|p| p.id == person) {
            if on && privilege.allowed_for(p.publisher_type, p.gender) {
                p.privileges.insert(privilege);
            } else {
                p.privileges.remove(&privilege);
            }
        }
    }

    /// Privileges a person would lose with this type and gender (shown before saving).
    pub fn privileges_lost(&self, person: u32, ty: PublisherType, gender: Gender) -> Vec<Privilege> {
        self.person(person).map(|p| p.privileges.iter().copied().filter(|x| !x.allowed_for(ty, gender)).collect()).unwrap_or_default()
    }

    /// Removes privileges the person no longer qualifies for (after a change of type or gender).
    pub(crate) fn prune_privileges(&mut self, person: u32) {
        if let Some(p) = self.people.iter_mut().find(|p| p.id == person) {
            let (ty, g) = (p.publisher_type, p.gender);
            p.privileges.retain(|x| x.allowed_for(ty, g));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rules_follow_the_list() {
        use PublisherType as T;
        let (m, f) = (Gender::Male, Gender::Female);
        // Baptised brothers only.
        assert!(P::Prayer.allowed_for(T::BaptisedPublisher, m));
        assert!(!P::Prayer.allowed_for(T::BaptisedPublisher, f));
        assert!(!P::Prayer.allowed_for(T::UnbaptisedPublisher, m));
        // Bible reading: every brother, also Bible students, but not visitors.
        assert!(P::BibleReading.allowed_for(T::Student, m));
        assert!(!P::BibleReading.allowed_for(T::Student, f));
        assert!(!P::BibleReading.allowed_for(T::Visitor, m));
        // Student parts: everyone except visitors.
        assert!(P::StartingConversation.allowed_for(T::Student, f));
        assert!(!P::StartingConversation.allowed_for(T::Visitor, f));
        // Duties: baptised publishers, brothers and sisters.
        assert!(P::Audio.allowed_for(T::RegularPioneer, f));
        assert!(!P::Audio.allowed_for(T::UnbaptisedPublisher, m));
        assert_eq!(Privilege::all().count(), 33);
    }

    #[test]
    fn privileges_are_removed_when_the_person_no_longer_qualifies() {
        let mut d = crate::demo::demo_directory(40);
        d.set_privilege(1, P::Prayer, true);
        assert!(d.person(1).is_some_and(|p| p.privileges.contains(&P::Prayer)));
        assert!(d.privileges_lost(1, PublisherType::UnbaptisedPublisher, Gender::Male).contains(&P::Prayer));
        // A sister cannot be given a brothers' privilege.
        d.set_privilege(2, P::Prayer, true);
        assert!(d.person(2).is_some_and(|p| !p.privileges.contains(&P::Prayer)));
    }
}
