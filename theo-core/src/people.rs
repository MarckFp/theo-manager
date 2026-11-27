//! M-01 People and M-09 Field service groups: persons, families, groups, labels and the
//! congregation `Directory` that holds every module's records.
//!
//! The rules follow `docs/model/m01-people.md` and `docs/model/m09-field-service-groups.md`.

use crate::prefs::{self, Preferences};

// ---------------------------------------------------------------------------------------------
// Value types
// ---------------------------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Gender {
    Male,
    Female,
}

impl Gender {
    pub fn value(self) -> &'static str {
        match self {
            Gender::Male => "male",
            Gender::Female => "female",
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            Gender::Male => "Male",
            Gender::Female => "Female",
        }
    }
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "male" => Some(Gender::Male),
            "female" => Some(Gender::Female),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum PublisherType {
    #[default]
    Student,
    Visitor,
    UnbaptisedPublisher,
    BaptisedPublisher,
    ContinuousAuxiliaryPioneer,
    RegularPioneer,
    SpecialPioneer,
    Missionary,
}

impl PublisherType {
    pub const ALL: [PublisherType; 8] = [
        PublisherType::Student,
        PublisherType::Visitor,
        PublisherType::UnbaptisedPublisher,
        PublisherType::BaptisedPublisher,
        PublisherType::ContinuousAuxiliaryPioneer,
        PublisherType::RegularPioneer,
        PublisherType::SpecialPioneer,
        PublisherType::Missionary,
    ];

    pub fn value(self) -> &'static str {
        match self {
            PublisherType::Student => "student",
            PublisherType::Visitor => "visitor",
            PublisherType::UnbaptisedPublisher => "unbaptised",
            PublisherType::BaptisedPublisher => "baptised",
            PublisherType::ContinuousAuxiliaryPioneer => "aux-pioneer",
            PublisherType::RegularPioneer => "regular-pioneer",
            PublisherType::SpecialPioneer => "special-pioneer",
            PublisherType::Missionary => "missionary",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            PublisherType::Student => "Bible student",
            PublisherType::Visitor => "Visitor",
            PublisherType::UnbaptisedPublisher => "Unbaptised publisher",
            PublisherType::BaptisedPublisher => "Baptised publisher",
            PublisherType::ContinuousAuxiliaryPioneer => "Continuous auxiliary pioneer",
            PublisherType::RegularPioneer => "Regular pioneer",
            PublisherType::SpecialPioneer => "Special pioneer",
            PublisherType::Missionary => "Missionary",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|t| t.value() == s)
    }

    /// Every type except Bible student and visitor.
    pub fn is_publisher(self) -> bool {
        !matches!(self, PublisherType::Student | PublisherType::Visitor)
    }

    pub fn is_baptised(self) -> bool {
        self.is_publisher() && self != PublisherType::UnbaptisedPublisher
    }

    pub fn has_pioneer_number(self) -> bool {
        matches!(self, PublisherType::RegularPioneer | PublisherType::SpecialPioneer | PublisherType::Missionary)
    }

    pub fn is_pioneer(self) -> bool {
        self.has_pioneer_number() || self == PublisherType::ContinuousAuxiliaryPioneer
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Appointment {
    Elder,
    MinisterialServant,
}

impl Appointment {
    pub fn value(self) -> &'static str {
        match self {
            Appointment::Elder => "elder",
            Appointment::MinisterialServant => "ms",
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            Appointment::Elder => "Elder",
            Appointment::MinisterialServant => "Ministerial servant",
        }
    }
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "elder" => Some(Appointment::Elder),
            "ms" => Some(Appointment::MinisterialServant),
            _ => None,
        }
    }
}

/// Derived from field service reports (M-10). Never stored.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Activity {
    Active,
    Irregular,
    Inactive,
}

impl Activity {
    pub const ALL: [Activity; 3] = [Activity::Active, Activity::Irregular, Activity::Inactive];
    pub fn value(self) -> &'static str {
        match self {
            Activity::Active => "active",
            Activity::Irregular => "irregular",
            Activity::Inactive => "inactive",
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            Activity::Active => "Active",
            Activity::Irregular => "Irregular",
            Activity::Inactive => "Inactive",
        }
    }
}

/// Account state of a person (the account itself is M-02).
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum AccountLink {
    #[default]
    None,
    /// Invited, the invite not used yet. Every member who can read people sees this, so it never
    /// holds what uses the invite: the link is shown once, to the administrator who made it.
    Invited {
        /// A sample link (demo only); empty for real invites.
        #[serde(default)]
        link: String,
        /// The invite's id on the server (SHA-256 of its proof): enough to see whether it was
        /// used, or to cancel it, but not to use it.
        #[serde(default)]
        invite: String,
    },
    Linked { username: String },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum RelationKind {
    Father,
    Mother,
    Spouse,
    Son,
    Daughter,
    Brother,
    Sister,
    Other,
}

impl RelationKind {
    pub const ALL: [RelationKind; 8] = [
        RelationKind::Father,
        RelationKind::Mother,
        RelationKind::Spouse,
        RelationKind::Son,
        RelationKind::Daughter,
        RelationKind::Brother,
        RelationKind::Sister,
        RelationKind::Other,
    ];
    pub fn value(self) -> &'static str {
        match self {
            RelationKind::Father => "father",
            RelationKind::Mother => "mother",
            RelationKind::Spouse => "spouse",
            RelationKind::Son => "son",
            RelationKind::Daughter => "daughter",
            RelationKind::Brother => "brother",
            RelationKind::Sister => "sister",
            RelationKind::Other => "other",
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            RelationKind::Father => "Father",
            RelationKind::Mother => "Mother",
            RelationKind::Spouse => "Spouse",
            RelationKind::Son => "Son",
            RelationKind::Daughter => "Daughter",
            RelationKind::Brother => "Brother",
            RelationKind::Sister => "Sister",
            RelationKind::Other => "Other relative",
        }
    }
    pub fn parse(s: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|k| k.value() == s)
    }
    /// The gender `from` must have for this kind.
    fn required_gender(self) -> Option<Gender> {
        match self {
            RelationKind::Father | RelationKind::Son | RelationKind::Brother => Some(Gender::Male),
            RelationKind::Mother | RelationKind::Daughter | RelationKind::Sister => Some(Gender::Female),
            RelationKind::Spouse | RelationKind::Other => None,
        }
    }
    /// What `to` is for `from`, given the gender of `to`.
    fn inverse(self, gender_of_to: Gender) -> RelationKind {
        let male = gender_of_to == Gender::Male;
        match self {
            RelationKind::Father | RelationKind::Mother => {
                if male { RelationKind::Son } else { RelationKind::Daughter }
            }
            RelationKind::Son | RelationKind::Daughter => {
                if male { RelationKind::Father } else { RelationKind::Mother }
            }
            RelationKind::Brother | RelationKind::Sister => {
                if male { RelationKind::Brother } else { RelationKind::Sister }
            }
            k => k,
        }
    }
}

// ---------------------------------------------------------------------------------------------
// Records
// ---------------------------------------------------------------------------------------------

/// A person (M-01). `id` is internal (never shown); in `theo-core` it is the random record id.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Person {
    pub id: u32,
    pub first_name: String,
    pub last_name: String,
    pub birthday: Option<String>,
    pub gender: Gender,
    pub email: Option<String>,
    pub telephone: Option<String>,
    pub mobile_phone: Option<String>,
    pub baptism_date: Option<String>,
    pub publisher_type: PublisherType,
    pub infirm: bool,
    pub pioneer_number: Option<String>,
    pub appointment: Option<Appointment>,
    pub anointed: bool,
    pub address_line_1: Option<String>,
    pub address_line_2: Option<String>,
    pub postal_code: Option<String>,
    pub city: Option<String>,
    pub consent_date: Option<String>,
    pub notes: Option<String>,
    pub account: AccountLink,
    /// The date the person moved away (`YYYY-MM-DD`), or `None` for current members.
    pub moved_away: Option<String>,
    /// What the person may take part in (M-01, "Privileges").
    pub privileges: std::collections::BTreeSet<crate::privileges::Privilege>,
    /// People to call in an emergency (M-01, "Emergency contacts"). Not members themselves.
    pub emergency_contacts: Vec<EmergencyContact>,
    /// Labels of the congregation (ids into `Directory::labels`).
    pub labels: std::collections::BTreeSet<u32>,
    /// Earlier publisher types and the month each one started (oldest first). Empty: the person
    /// always had `publisher_type`. A month's field service report follows the type of that
    /// month ([`Person::type_in`]), so a publisher who becomes a pioneer keeps their old reports.
    #[serde(default)]
    pub type_history: Vec<TypeChange>,
}

/// A publisher type and the first month it applies from.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct TypeChange {
    pub from: crate::reports::Ym,
    pub publisher_type: PublisherType,
}

/// The type history after a save: a new type starts in `since` (`YYYY-MM`, default the current
/// month), and the old one is kept for the months before; with the same type, `since` moves the
/// start of the current one.
fn type_history(existing: Option<&Person>, ty: PublisherType, since: &str) -> Vec<TypeChange> {
    let Some(old) = existing else { return Vec::new() };
    let (y, m, _) = crate::prefs::today();
    let since = parse_month(since).unwrap_or((y, m));
    let mut history = old.type_history.clone();
    if old.publisher_type != ty {
        if history.is_empty() {
            history.push(TypeChange { from: SINCE_ALWAYS, publisher_type: old.publisher_type });
        }
        history.retain(|c| c.from < since);
        history.push(TypeChange { from: since, publisher_type: ty });
    } else if let Some(last) = history.last().copied().filter(|c| c.from != SINCE_ALWAYS && c.from != since) {
        history.pop();
        history.retain(|c| c.from < since);
        history.push(TypeChange { from: since, ..last });
    }
    // Two months in a row with the same type are one stretch; a history of one type says nothing.
    history.dedup_by(|later, earlier| later.publisher_type == earlier.publisher_type);
    if history.len() == 1 {
        history.clear();
    }
    history
}

/// `YYYY-MM` to a month.
pub fn parse_month(s: &str) -> Option<crate::reports::Ym> {
    let (y, m) = s.trim().split_once('-')?;
    let (y, m) = (y.parse::<i32>().ok()?, m.parse::<u32>().ok()?);
    (1900..=2200).contains(&y).then_some(())?;
    (1..=12).contains(&m).then_some((y, m))
}

/// "Since always": the `from` of the type a person had before their first recorded change.
pub const SINCE_ALWAYS: crate::reports::Ym = (0, 1);

impl Person {
    /// The publisher type in month `ym` (for field service reports and goals).
    pub fn type_in(&self, ym: crate::reports::Ym) -> PublisherType {
        self.type_history.iter().rev().find(|c| c.from <= ym).map(|c| c.publisher_type).unwrap_or(self.publisher_type)
    }

    /// The month the current type started, if it changed at some point.
    pub fn type_since(&self) -> Option<crate::reports::Ym> {
        self.type_history.last().map(|c| c.from).filter(|f| *f != SINCE_ALWAYS)
    }
}

/// Someone to call in an emergency. Usually not a member of the congregation.
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct EmergencyContact {
    pub first_name: String,
    pub last_name: String,
    pub telephone: Option<String>,
    pub mobile_phone: Option<String>,
    pub address_line_1: Option<String>,
    pub address_line_2: Option<String>,
    pub postal_code: Option<String>,
    pub city: Option<String>,
    pub notes: Option<String>,
}

/// The colour of a label (the same colours as the other tags in the app).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum LabelColor {
    #[default]
    Gray,
    Blue,
    Green,
    Yellow,
    Red,
}

impl LabelColor {
    pub const ALL: [LabelColor; 5] = [LabelColor::Gray, LabelColor::Blue, LabelColor::Green, LabelColor::Yellow, LabelColor::Red];
    pub fn value(self) -> &'static str {
        match self {
            LabelColor::Gray => "gray",
            LabelColor::Blue => "blue",
            LabelColor::Green => "green",
            LabelColor::Yellow => "yellow",
            LabelColor::Red => "red",
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            LabelColor::Gray => "Gray",
            LabelColor::Blue => "Blue",
            LabelColor::Green => "Green",
            LabelColor::Yellow => "Yellow",
            LabelColor::Red => "Red",
        }
    }
    pub fn parse(s: &str) -> Self {
        Self::ALL.into_iter().find(|c| c.value() == s).unwrap_or_default()
    }
}

/// A label the congregation defines and puts on people (as many as needed).
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Label {
    pub id: u32,
    pub name: String,
    pub color: LabelColor,
}

impl Person {
    pub fn display_name(&self, p: &Preferences) -> String {
        p.format_name(&self.first_name, &self.last_name)
    }

    /// Only current baptised brothers can be a group overseer or assistant (M-09).
    pub fn may_lead(&self) -> bool {
        self.publisher_type.is_baptised() && self.gender == Gender::Male && self.moved_away.is_none()
    }
}

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Relationship {
    /// `from` is the `kind` of `to`.
    pub from: u32,
    pub to: u32,
    pub kind: RelationKind,
}

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Family {
    pub id: u32,
    pub head: u32,
    pub members: Vec<u32>,
    pub relationships: Vec<Relationship>,
}

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Group {
    pub id: u32,
    pub name: String,
    pub overseer: Option<u32>,
    pub assistant: Option<u32>,
    /// Plain members. The overseer and assistant are members too, without being listed here.
    pub members: Vec<u32>,
}

impl Group {
    pub fn includes(&self, n: u32) -> bool {
        self.overseer == Some(n) || self.assistant == Some(n) || self.members.contains(&n)
    }
}

/// The role of a person in a group.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GroupRole {
    Overseer,
    Assistant,
    Member,
}

/// One relative of a person, seen from that person.
#[derive(Clone, Debug, PartialEq)]
pub struct Relative {
    pub id: u32,
    /// What the relative is for the person ("Father", "Spouse"…). `None`: same family, but no
    /// direct relationship between the two.
    pub kind: Option<RelationKind>,
    pub is_head: bool,
}

// ---------------------------------------------------------------------------------------------
// Directory: all people data of one congregation
// ---------------------------------------------------------------------------------------------

/// `serde(default)`: a backup made before a field existed still restores (the field starts empty).
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct Directory {
    pub people: Vec<Person>,
    pub families: Vec<Family>,
    pub groups: Vec<Group>,
    /// Field service reports (M-10, `src/reports.rs`).
    pub reports: Vec<crate::reports::Report>,
    /// Months with a 15-hour auxiliary pioneer goal (M-10, "Goals").
    pub aux_short_months: std::collections::BTreeSet<crate::reports::Ym>,
    /// Months a baptised publisher is auxiliary pioneer, planned ahead or set later (M-10).
    pub aux_plans: std::collections::BTreeSet<(u32, crate::reports::Ym)>,
    /// App permissions per person with a linked account (M-02, `src/access.rs`).
    pub access: std::collections::BTreeMap<u32, crate::access::Access>,
    /// The person who created the congregation: always an administrator.
    pub owner: Option<u32>,
    /// Activity log (shown in *Congregation › Settings*): permission changes for now.
    pub log: Vec<crate::access::LogEntry>,
    /// Labels of the congregation, in display order.
    pub labels: Vec<Label>,
    /// Absences: who is away when (M-01, "Absences").
    pub absences: Vec<crate::absences::Absence>,
    /// Custom tasks of the Departments section.
    pub custom_tasks: Vec<crate::tasks::CustomTask>,
    /// Meeting attendance per month (M-11, `src/attendance.rs`).
    pub attendance: Vec<crate::attendance::MonthAttendance>,
    /// Events (M-12, `src/events.rs`).
    pub events: Vec<crate::events::Event>,
    /// Meetings for field service: weekly presets and the meetings (M-15, `src/service_meetings.rs`).
    pub service_presets: Vec<crate::service_meetings::ServicePreset>,
    pub service_meetings: Vec<crate::service_meetings::ServiceMeeting>,
    /// Notices of "My assignments": new, changed or cancelled assignments per person.
    pub notices: Vec<crate::assignments::Notice>,
    /// Public witnessing: settings (shift supervisors), weekly presets and shifts (M-08).
    pub witnessing: crate::public_witnessing::WitnessingSettings,
    pub shift_presets: Vec<crate::public_witnessing::ShiftPreset>,
    pub shifts: Vec<crate::public_witnessing::Shift>,
    /// Territories (M-07, `src/territories.rs`): the territories, the registry of who had them,
    /// the page settings, and the categories and labels of territories.
    pub territories: Vec<crate::territories::Territory>,
    pub territory_assignments: Vec<crate::territories::TerritoryAssignment>,
    pub territory_settings: crate::territories::TerritorySettings,
    pub territory_categories: Vec<Label>,
    pub territory_labels: Vec<Label>,
    /// Labels of territory assignments (for example "Memorial campaign").
    pub assignment_labels: Vec<Label>,
    /// *My ministry*: each publisher's daily log (`src/ministry.rs`).
    pub ministry_log: Vec<crate::ministry::MinistryDay>,
    /// Months whose left-over minutes were carried over to the next month.
    pub ministry_carried: std::collections::BTreeSet<(u32, crate::reports::Ym)>,
    /// Personal goals: hours per month, per person (*My ministry*).
    pub personal_goals: std::collections::BTreeMap<u32, u32>,
    /// The midweek meeting (M-03, `src/midweek.rs`): the plan of each week, and its settings.
    pub midweek_weeks: Vec<crate::midweek::MidweekWeek>,
    pub midweek_settings: crate::midweek::MidweekSettings,
    /// The weekend meeting (M-04, `weekend.rs`): the plan of each week, and its settings.
    pub weekend_weeks: Vec<crate::weekend::WeekendWeek>,
    pub weekend_settings: crate::weekend::WeekendSettings,
    /// Public talks (M-06, `talks.rs`): settings, other congregations, visiting speakers, the
    /// talks of each local speaker, and outgoing talks.
    pub talk_settings: crate::talks::TalkSettings,
    pub other_congregations: Vec<crate::talks::OtherCongregation>,
    pub visiting_speakers: Vec<crate::talks::VisitingSpeaker>,
    pub speaker_talks: std::collections::BTreeMap<u32, std::collections::BTreeSet<u16>>,
    pub outgoing_talks: Vec<crate::talks::OutgoingTalk>,
    /// Duties (M-05, `duties.rs`): duty types, the duties of each meeting, the cleaning rota and
    /// maintenance tasks.
    pub duty_types: Vec<crate::duties::DutyType>,
    pub duty_meetings: Vec<crate::duties::DutyMeeting>,
    pub cleaning: crate::duties::CleaningSettings,
    pub maintenance_tasks: Vec<crate::duties::MaintenanceTask>,
}

impl Directory {
    pub fn person(&self, n: u32) -> Option<&Person> {
        self.people.iter().find(|p| p.id == n)
    }

    fn name(&self, n: u32, prefs: &Preferences) -> String {
        self.person(n).map(|p| p.display_name(prefs)).unwrap_or_else(|| "Unknown person".into())
    }

    fn next_id(&self) -> u32 {
        self.people.iter().map(|p| p.id).max().unwrap_or(0) + 1
    }

    // ----- groups -----

    pub fn groups_of(&self, n: u32) -> Vec<(&Group, GroupRole)> {
        self.groups
            .iter()
            .filter_map(|g| {
                if g.overseer == Some(n) {
                    Some((g, GroupRole::Overseer))
                } else if g.assistant == Some(n) {
                    Some((g, GroupRole::Assistant))
                } else if g.members.contains(&n) {
                    Some((g, GroupRole::Member))
                } else {
                    None
                }
            })
            .collect()
    }

    /// The group where the person is a plain member, else the first group they lead (M-09).
    pub fn home_group(&self, n: u32) -> Option<&Group> {
        let gs = self.groups_of(n);
        gs.iter().find(|(_, r)| *r == GroupRole::Member).or_else(|| gs.first()).map(|(g, _)| *g)
    }

    /// Plain membership (what the person form edits).
    pub fn member_group(&self, n: u32) -> Option<u32> {
        self.groups.iter().find(|g| g.members.contains(&n)).map(|g| g.id)
    }

    fn leave_groups(&mut self, n: u32) {
        for g in &mut self.groups {
            g.members.retain(|m| *m != n);
            if g.overseer == Some(n) {
                g.overseer = None;
            }
            if g.assistant == Some(n) {
                g.assistant = None;
            }
        }
    }

    fn set_member_group(&mut self, n: u32, group: Option<u32>) {
        for g in &mut self.groups {
            g.members.retain(|m| *m != n);
        }
        if let Some(g) = group.and_then(|id| self.groups.iter_mut().find(|g| g.id == id))
            && !g.includes(n)
        {
            g.members.push(n);
        }
    }

    // ----- families -----

    pub fn family_of(&self, n: u32) -> Option<&Family> {
        self.families.iter().find(|f| f.members.contains(&n))
    }

    pub fn relatives(&self, n: u32) -> Vec<Relative> {
        let Some(f) = self.family_of(n) else { return Vec::new() };
        let mut out: Vec<Relative> = f
            .relationships
            .iter()
            .filter_map(|r| {
                if r.from == n {
                    let g = self.person(r.to)?.gender;
                    Some(Relative { id: r.to, kind: Some(r.kind.inverse(g)), is_head: f.head == r.to })
                } else if r.to == n {
                    Some(Relative { id: r.from, kind: Some(r.kind), is_head: f.head == r.from })
                } else {
                    None
                }
            })
            .collect();
        // Members without a direct relationship to this person still belong to the family.
        for m in &f.members {
            if *m != n && !out.iter().any(|r| r.id == *m) {
                out.push(Relative { id: *m, kind: None, is_head: f.head == *m });
            }
        }
        out
    }

    /// What `other` is for `person` ("Father", "Spouse"…), if they have a direct relationship.
    pub fn relation_label(&self, person: u32, other: u32) -> Option<String> {
        self.relatives(person).into_iter().find(|r| r.id == other).and_then(|r| r.kind).map(|k| k.label().to_string())
    }

    fn spouse_of(&self, n: u32) -> Option<u32> {
        self.relatives(n).into_iter().find(|r| r.kind == Some(RelationKind::Spouse)).map(|r| r.id)
    }

    /// Who becomes head if `leaving` (the head) leaves: the spouse, else the oldest member.
    pub fn proposed_new_head(&self, leaving: u32) -> Option<u32> {
        let f = self.family_of(leaving)?;
        if f.head != leaving || f.members.len() <= 2 {
            return None;
        }
        self.spouse_of(leaving).or_else(|| {
            f.members
                .iter()
                .filter(|m| **m != leaving)
                .min_by_key(|m| self.person(**m).and_then(|p| p.birthday.clone()).unwrap_or_else(|| "9999".into()))
                .copied()
        })
    }

    /// "`relative` is the `kind` of `n`". Creates or extends a family (m01-people.md, "Joining").
    pub fn add_relationship(&mut self, n: u32, relative: u32, kind: RelationKind, prefs: &Preferences) -> Result<(), String> {
        if n == relative {
            return Err("Choose another person.".into());
        }
        let (Some(_), Some(r)) = (self.person(n), self.person(relative)) else { return Err("Unknown person.".into()) };
        if let Some(g) = kind.required_gender()
            && r.gender != g
        {
            return Err(format!("{} cannot be the {} (check the gender).", r.display_name(prefs), kind.label().to_lowercase()));
        }
        if self.family_of(n).is_some_and(|f| f.relationships.iter().any(|x| (x.from == n && x.to == relative) || (x.from == relative && x.to == n))) {
            return Err("These two persons already have a relationship.".into());
        }
        if kind == RelationKind::Spouse && (self.spouse_of(n).is_some() || self.spouse_of(relative).is_some()) {
            return Err("A person can have only one spouse.".into());
        }
        let fa = self.family_of(n).map(|f| f.id);
        let fb = self.family_of(relative).map(|f| f.id);
        let rel = Relationship { from: relative, to: n, kind };
        match (fa, fb) {
            (Some(a), Some(b)) if a != b => {
                let head = self.families.iter().find(|f| f.id == b).map(|f| f.head).unwrap_or(relative);
                return Err(format!(
                    "{} is already in the family of {}. Remove them from that family first.",
                    self.name(relative, prefs),
                    self.name(head, prefs)
                ));
            }
            (Some(id), _) | (None, Some(id)) => {
                if let Some(f) = self.families.iter_mut().find(|f| f.id == id) {
                    for m in [n, relative] {
                        if !f.members.contains(&m) {
                            f.members.push(m);
                        }
                    }
                    f.relationships.push(rel);
                }
            }
            (None, None) => {
                let head = match kind {
                    RelationKind::Father | RelationKind::Mother => relative,
                    RelationKind::Spouse => {
                        if self.person(relative).is_some_and(|x| x.gender == Gender::Male) { relative } else { n }
                    }
                    _ => n,
                };
                let id = self.families.iter().map(|f| f.id).max().unwrap_or(0) + 1;
                self.families.push(Family { id, head, members: vec![head, if head == n { relative } else { n }], relationships: vec![rel] });
            }
        }
        Ok(())
    }

    /// Removes a person from their family. Returns the new head, if the head changed.
    pub fn remove_from_family(&mut self, n: u32) -> Option<u32> {
        let new_head = self.proposed_new_head(n);
        let id = self.family_of(n)?.id;
        let f = self.families.iter_mut().find(|f| f.id == id)?;
        f.members.retain(|m| *m != n);
        f.relationships.retain(|r| r.from != n && r.to != n);
        if f.head == n {
            f.head = new_head.or_else(|| f.members.first().copied()).unwrap_or(n);
        }
        if f.members.len() < 2 {
            self.families.retain(|f| f.id != id);
            return None;
        }
        new_head
    }

    pub fn make_head(&mut self, n: u32) {
        if let Some(f) = self.families.iter_mut().find(|f| f.members.contains(&n)) {
            f.head = n;
        }
    }

    // ----- save and delete -----

    /// Validates and saves a person. Returns the person's id.
    pub fn save(&mut self, id: Option<u32>, d: &PersonDraft) -> Result<u32, Vec<(&'static str, String)>> {
        let errors = d.validate();
        if !errors.is_empty() {
            return Err(errors);
        }
        let d = d.normalized();
        let n = id.unwrap_or_else(|| self.next_id());
        let ty = PublisherType::parse(&d.publisher_type).unwrap_or_default();
        let existing = self.person(n).cloned();
        let type_history = type_history(existing.as_ref(), ty, &d.type_since);
        let person = Person {
            id: n,
            first_name: d.first_name.trim().into(),
            last_name: d.last_name.trim().into(),
            birthday: opt(&d.birthday),
            gender: Gender::parse(&d.gender).unwrap_or(Gender::Male),
            email: opt(&d.email),
            telephone: opt(&d.telephone),
            mobile_phone: opt(&d.mobile_phone),
            baptism_date: opt(&d.baptism_date),
            publisher_type: ty,
            // Infirm only applies to those with an hour goal of 600: regular and special pioneers
            // and missionaries (it frees them from it).
            infirm: d.infirm && ty.has_pioneer_number(),
            pioneer_number: opt(&d.pioneer_number),
            appointment: Appointment::parse(&d.appointment),
            anointed: d.anointed,
            address_line_1: opt(&d.address_line_1),
            address_line_2: opt(&d.address_line_2),
            postal_code: opt(&d.postal_code),
            city: opt(&d.city),
            consent_date: opt(&d.consent_date),
            notes: opt(&d.notes),
            account: existing.as_ref().map(|p| p.account.clone()).unwrap_or_default(),
            moved_away: existing.as_ref().and_then(|p| p.moved_away.clone()),
            privileges: existing.as_ref().map(|p| p.privileges.clone()).unwrap_or_default(),
            emergency_contacts: existing.as_ref().map(|p| p.emergency_contacts.clone()).unwrap_or_default(),
            labels: existing.as_ref().map(|p| p.labels.clone()).unwrap_or_default(),
            type_history,
        };
        let moved = person.moved_away.is_some();
        match self.people.iter_mut().find(|p| p.id == n) {
            Some(p) => *p = person,
            None => self.people.push(person),
        }
        if ty.is_publisher() && !moved {
            self.set_member_group(n, d.group.parse().ok());
        } else {
            self.leave_groups(n);
        }
        // Privileges the new type or gender does not allow are removed.
        self.prune_privileges(n);
        // A leader who is no longer a baptised brother stops leading, and stays a plain member.
        if self.person(n).is_some_and(|p| !p.may_lead()) {
            let keep_in = self.groups.iter().find(|g| g.overseer == Some(n) || g.assistant == Some(n)).map(|g| g.id);
            for g in &mut self.groups {
                if g.overseer == Some(n) {
                    g.overseer = None;
                }
                if g.assistant == Some(n) {
                    g.assistant = None;
                }
            }
            if ty.is_publisher() && !moved && self.member_group(n).is_none() {
                self.set_member_group(n, keep_in);
            }
        }
        Ok(n)
    }

    fn leaving_family_consequences(&self, n: u32, prefs: &Preferences, out: &mut Vec<String>) {
        if let Some(h) = self.proposed_new_head(n) {
            out.push(format!("{} becomes the head of the family.", self.name(h, prefs)));
        } else if self.family_of(n).is_some_and(|f| f.members.len() <= 2) {
            out.push("Their family has only one other member, so the family is removed.".into());
        }
    }

    fn group_consequences(&self, n: u32, prefs: &Preferences, out: &mut Vec<String>) {
        for (g, role) in self.groups_of(n) {
            match role {
                GroupRole::Overseer => out.push(format!("{} has no group overseer anymore.", g.name)),
                GroupRole::Assistant => out.push(format!("{} has no group assistant anymore.", g.name)),
                GroupRole::Member => out.push(format!("{} leaves the group {}.", self.name(n, prefs), g.name)),
            }
        }
    }

    /// What else changes when this person is deleted (shown before deleting).
    pub fn delete_consequences(&self, n: u32, prefs: &Preferences) -> Vec<String> {
        let mut out = Vec::new();
        self.leaving_family_consequences(n, prefs, &mut out);
        for (g, role) in self.groups_of(n) {
            match role {
                GroupRole::Overseer => out.push(format!("{} has no group overseer anymore.", g.name)),
                GroupRole::Assistant => out.push(format!("{} has no group assistant anymore.", g.name)),
                GroupRole::Member => {}
            }
        }
        out
    }

    // ----- moving away (m01-people.md, "Moving away") -----

    /// Current family members who could move away together with `n`.
    pub fn family_to_move(&self, n: u32) -> Vec<u32> {
        self.family_of(n)
            .map(|f| {
                f.members.iter().copied().filter(|m| *m != n && self.person(*m).is_some_and(|p| p.moved_away.is_none())).collect()
            })
            .unwrap_or_default()
    }

    /// What else changes when `n` (and maybe their family) moves away.
    pub fn move_away_consequences(&self, n: u32, with_family: bool, prefs: &Preferences) -> Vec<String> {
        let mut out = Vec::new();
        let mut who = vec![n];
        if with_family {
            who.extend(self.family_to_move(n));
        } else {
            self.leaving_family_consequences(n, prefs, &mut out);
        }
        for m in &who {
            self.group_consequences(*m, prefs, &mut out);
        }
        if who.iter().any(|m| self.person(*m).is_some_and(|p| p.account != AccountLink::None)) {
            out.push("Their app access to this congregation ends, and open invitations are cancelled.".into());
        }
        out
    }

    /// Marks `n` (and maybe the rest of the family) as moved away on `date`. A family that moves
    /// together stays a family; a person who moves alone leaves their family.
    pub fn move_away(&mut self, n: u32, date: &str, with_family: bool) {
        let mut who = vec![n];
        if with_family {
            who.extend(self.family_to_move(n));
        } else {
            self.remove_from_family(n);
        }
        for m in who {
            self.leave_groups(m);
            if let Some(p) = self.people.iter_mut().find(|p| p.id == m) {
                p.moved_away = Some(date.to_string());
                p.account = AccountLink::None;
            }
            self.drop_access(m);
        }
    }

    /// Makes a moved-away person current again (no group; the family stays as it is).
    pub fn move_back(&mut self, n: u32) {
        if let Some(p) = self.people.iter_mut().find(|p| p.id == n) {
            p.moved_away = None;
        }
    }

    /// Removes for good everyone who moved away at least `RETENTION_YEARS` ago. Returns how
    /// many were removed.
    pub fn purge_expired(&mut self, today: &str) -> usize {
        let expired: Vec<u32> = self
            .people
            .iter()
            .filter(|p| p.moved_away.as_deref().is_some_and(|d| removal_date(d).as_str() <= today))
            .map(|p| p.id)
            .collect();
        for n in &expired {
            self.delete(*n);
        }
        expired.len()
    }

    pub fn delete(&mut self, n: u32) {
        self.remove_from_family(n);
        self.leave_groups(n);
        self.drop_access(n);
        self.people.retain(|p| p.id != n);
    }

    pub fn set_account(&mut self, n: u32, account: AccountLink) {
        if let Some(p) = self.people.iter_mut().find(|p| p.id == n) {
            p.account = account;
        }
    }
}

// ---------------------------------------------------------------------------------------------
// Field service groups (docs/model/m09-field-service-groups.md)
// ---------------------------------------------------------------------------------------------

/// The group form: name, and the overseer and assistant as person ids (empty = none).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct GroupDraft {
    pub name: String,
    pub overseer: String,
    pub assistant: String,
}

impl GroupDraft {
    pub fn from_group(g: &Group) -> Self {
        let s = |o: Option<u32>| o.map(|n| n.to_string()).unwrap_or_default();
        Self { name: g.name.clone(), overseer: s(g.overseer), assistant: s(g.assistant) }
    }
    fn overseer_id(&self) -> Option<u32> {
        self.overseer.parse().ok()
    }
    fn assistant_id(&self) -> Option<u32> {
        self.assistant.parse().ok()
    }
}

/// Activity of a group's publishers (derived from reports).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct GroupActivity {
    pub active: usize,
    pub irregular: usize,
    pub inactive: usize,
}

impl Directory {
    pub fn group(&self, id: u32) -> Option<&Group> {
        self.groups.iter().find(|g| g.id == id)
    }

    /// Groups sorted by name.
    pub fn groups_sorted(&self) -> Vec<&Group> {
        let mut out: Vec<&Group> = self.groups.iter().collect();
        out.sort_by_cached_key(|g| fold(&g.name));
        out
    }

    /// Current publishers: the only people who can be in or lead a group.
    pub fn current_publishers(&self) -> Vec<&Person> {
        self.people.iter().filter(|p| p.publisher_type.is_publisher() && p.moved_away.is_none()).collect()
    }

    /// Current publishers who are in no group (also not as overseer or assistant).
    pub fn unassigned_publishers(&self) -> Vec<&Person> {
        self.current_publishers().into_iter().filter(|p| self.groups_of(p.id).is_empty()).collect()
    }

    /// Everyone in the group: overseer, assistant, then the members sorted by name.
    pub fn group_members(&self, id: u32, prefs: &Preferences) -> Vec<(u32, GroupRole)> {
        let Some(g) = self.group(id) else { return Vec::new() };
        let mut out: Vec<(u32, GroupRole)> = Vec::new();
        if let Some(o) = g.overseer {
            out.push((o, GroupRole::Overseer));
        }
        if let Some(a) = g.assistant {
            out.push((a, GroupRole::Assistant));
        }
        let mut members: Vec<u32> = g.members.iter().copied().filter(|m| Some(*m) != g.overseer && Some(*m) != g.assistant).collect();
        members.sort_by_cached_key(|m| fold(&self.name(*m, prefs)));
        out.extend(members.into_iter().map(|m| (m, GroupRole::Member)));
        out
    }

    pub fn group_activity(&self, id: u32) -> GroupActivity {
        let mut a = GroupActivity::default();
        for (n, _) in self.group_members(id, &Preferences::default()) {
            match self.person(n).and_then(|p| self.activity(p)) {
                Some(Activity::Active) => a.active += 1,
                Some(Activity::Irregular) => a.irregular += 1,
                Some(Activity::Inactive) => a.inactive += 1,
                None => {}
            }
        }
        a
    }

    pub fn validate_group(&self, id: Option<u32>, d: &GroupDraft) -> Vec<(&'static str, String)> {
        let mut e = Vec::new();
        let name = d.name.trim();
        if name.is_empty() {
            e.push(("name", "Enter a name for the group.".to_string()));
        } else if self.groups.iter().any(|g| Some(g.id) != id && fold(&g.name) == fold(name)) {
            e.push(("name", "Another group already has this name.".into()));
        }
        if d.overseer_id().is_some() && d.overseer_id() == d.assistant_id() {
            e.push(("assistant", "The assistant must be another person than the overseer.".into()));
        }
        let ok = |n: Option<u32>| n.is_none_or(|n| self.person(n).is_some_and(Person::may_lead));
        if !ok(d.overseer_id()) {
            e.push(("overseer", "Only a baptised brother can be the overseer.".into()));
        }
        if !ok(d.assistant_id()) {
            e.push(("assistant", "Only a baptised brother can be the assistant.".into()));
        }
        e
    }

    /// Things to check, shown before and after saving. They never block saving (m09, "Rules").
    pub fn group_warnings(&self, id: Option<u32>, d: &GroupDraft, prefs: &Preferences) -> Vec<String> {
        let mut out = Vec::new();
        let other_groups = |n: u32| -> Vec<(String, GroupRole)> {
            self.groups_of(n).into_iter().filter(|(g, _)| Some(g.id) != id).map(|(g, r)| (g.name.clone(), r)).collect()
        };
        for (who, none) in [(d.overseer_id(), "This group has no overseer yet."), (d.assistant_id(), "This group has no assistant yet.")] {
            match who.and_then(|n| self.person(n)) {
                None => out.push(none.to_string()),
                Some(p) if p.appointment.is_none() => out.push(format!(
                    "{} is not an elder or a ministerial servant. They are recommended first.",
                    p.display_name(prefs)
                )),
                _ => {}
            }
        }
        for n in [d.overseer_id(), d.assistant_id()].into_iter().flatten() {
            for (g, role) in other_groups(n) {
                let who = self.name(n, prefs);
                out.push(match role {
                    GroupRole::Member => format!("{who} is a member of {g}, and stays a member there."),
                    _ => format!("{who} also leads {g}."),
                });
            }
        }
        out
    }

    /// Saves a group. The overseer and assistant are always members; a replaced overseer or
    /// assistant stays in the group, unless they have another home group (m09).
    pub fn save_group(&mut self, id: Option<u32>, d: &GroupDraft) -> Result<u32, Vec<(&'static str, String)>> {
        let errors = self.validate_group(id, d);
        if !errors.is_empty() {
            return Err(errors);
        }
        let gid = id.unwrap_or_else(|| self.groups.iter().map(|g| g.id).max().unwrap_or(0) + 1);
        let (overseer, assistant) = (d.overseer_id(), d.assistant_id());
        let old: Vec<u32> = self.group(gid).map(|g| [g.overseer, g.assistant].into_iter().flatten().collect()).unwrap_or_default();
        let keep: Vec<u32> = old
            .into_iter()
            .filter(|n| Some(*n) != overseer && Some(*n) != assistant)
            .filter(|n| !self.groups.iter().any(|g| g.id != gid && g.members.contains(n)))
            .collect();
        match self.groups.iter_mut().find(|g| g.id == gid) {
            Some(g) => {
                g.name = d.name.trim().to_string();
                g.overseer = overseer;
                g.assistant = assistant;
            }
            None => self.groups.push(Group { id: gid, name: d.name.trim().to_string(), overseer, assistant, members: Vec::new() }),
        }
        if let Some(g) = self.groups.iter_mut().find(|g| g.id == gid) {
            for n in keep {
                if !g.members.contains(&n) {
                    g.members.push(n);
                }
            }
            g.members.retain(|m| Some(*m) != overseer && Some(*m) != assistant);
        }
        Ok(gid)
    }

    /// Adds people as plain members. Someone in another group moves here (one home group).
    /// Only publishers without a group can be added. To move someone, remove them from their
    /// group first (decided 2026-11-17).
    pub fn add_members(&mut self, id: u32, people: &[u32]) {
        for n in people {
            if self.person(*n).is_some_and(|p| p.publisher_type.is_publisher() && p.moved_away.is_none()) && self.groups_of(*n).is_empty() {
                self.set_member_group(*n, Some(id));
            }
        }
    }

    /// Who can be overseer or assistant, in the recommended order: elders, ministerial servants,
    /// then other baptised brothers. Each list is sorted by name.
    pub fn leader_candidates(&self, prefs: &Preferences) -> Vec<(&'static str, Vec<&Person>)> {
        let mut sections: Vec<(&'static str, Vec<&Person>)> =
            vec![("Elders", Vec::new()), ("Ministerial servants", Vec::new()), ("Other baptised brothers", Vec::new())];
        for p in self.people.iter().filter(|p| p.may_lead()) {
            let i = match p.appointment {
                Some(Appointment::Elder) => 0,
                Some(Appointment::MinisterialServant) => 1,
                None => 2,
            };
            sections[i].1.push(p);
        }
        for (_, list) in &mut sections {
            list.sort_by_cached_key(|p| fold(&p.display_name(prefs)));
        }
        sections.retain(|(_, l)| !l.is_empty());
        sections
    }

    pub fn remove_member(&mut self, id: u32, n: u32) {
        if let Some(g) = self.groups.iter_mut().find(|g| g.id == id) {
            g.members.retain(|m| *m != n);
        }
    }

    pub fn delete_group(&mut self, id: u32) {
        self.groups.retain(|g| g.id != id);
    }
}

/// Who is in a field service group, by appointment and type, and their average age.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct GroupComposition {
    pub members: usize,
    pub elders: usize,
    pub ministerial_servants: usize,
    /// Regular and special pioneers, and missionaries.
    pub pioneers: usize,
    pub continuous_auxiliary: usize,
    /// Average age of the members with a birthday.
    pub average_age: Option<f64>,
    pub with_birthday: usize,
}

impl Directory {
    pub fn group_composition(&self, id: u32) -> GroupComposition {
        let mut c = GroupComposition::default();
        let mut ages = Vec::new();
        for (n, _) in self.group_members(id, &Preferences::default()) {
            let Some(p) = self.person(n) else { continue };
            c.members += 1;
            match p.appointment {
                Some(Appointment::Elder) => c.elders += 1,
                Some(Appointment::MinisterialServant) => c.ministerial_servants += 1,
                None => {}
            }
            if p.publisher_type.has_pioneer_number() {
                c.pioneers += 1;
            } else if p.publisher_type == PublisherType::ContinuousAuxiliaryPioneer {
                c.continuous_auxiliary += 1;
            }
            if let Some(age) = p.birthday.as_deref().and_then(parse_date).and_then(prefs::years_since) {
                ages.push(age);
            }
        }
        c.with_birthday = ages.len();
        c.average_age = (!ages.is_empty()).then(|| ages.iter().sum::<f64>() / ages.len() as f64);
        c
    }

    // ----- families (list and page) -----

    pub fn family(&self, id: u32) -> Option<&Family> {
        self.families.iter().find(|f| f.id == id)
    }

    /// All members of the family moved away (they moved together).
    pub fn family_moved_away(&self, f: &Family) -> bool {
        f.members.iter().all(|m| self.person(*m).is_some_and(|p| p.moved_away.is_some()))
    }

    /// Current people who are in no family (they can join or start one).
    /// Who can become a relative of `n` (*Add relative*): never someone in another family, so
    /// nobody ends up in two families. With a family: people without one, and its own members (to
    /// say how two of them are related). Without one: anyone (they join that person's family).
    pub fn relative_candidates(&self, n: u32) -> Vec<&Person> {
        let mine = self.family_of(n).map(|f| f.id);
        self.people
            .iter()
            .filter(|p| p.id != n && p.moved_away.is_none())
            .filter(|p| match (mine, self.family_of(p.id).map(|f| f.id)) {
                (Some(a), Some(b)) => a == b,
                _ => true,
            })
            .collect()
    }

    pub fn without_family(&self) -> Vec<&Person> {
        self.people.iter().filter(|p| p.moved_away.is_none() && self.family_of(p.id).is_none()).collect()
    }

    /// Removes the family. Its members stay, without a family.
    pub fn delete_family(&mut self, id: u32) {
        self.families.retain(|f| f.id != id);
    }
}

// ---------------------------------------------------------------------------------------------
// Emergency contacts and labels
// ---------------------------------------------------------------------------------------------

impl EmergencyContact {
    pub fn name(&self) -> String {
        format!("{} {}", self.first_name, self.last_name).trim().to_string()
    }
}

impl Directory {
    /// Adds (`index` = None) or replaces an emergency contact. First and last name are required.
    pub fn save_emergency_contact(&mut self, person: u32, index: Option<usize>, c: EmergencyContact) -> Result<(), String> {
        if c.first_name.trim().is_empty() || c.last_name.trim().is_empty() {
            return Err("Enter the first and last name.".into());
        }
        let trim = |o: Option<String>| o.map(|s| s.trim().to_string()).filter(|s| !s.is_empty());
        let c = EmergencyContact {
            first_name: c.first_name.trim().into(),
            last_name: c.last_name.trim().into(),
            telephone: trim(c.telephone),
            mobile_phone: trim(c.mobile_phone),
            address_line_1: trim(c.address_line_1),
            address_line_2: trim(c.address_line_2),
            postal_code: trim(c.postal_code),
            city: trim(c.city),
            notes: trim(c.notes),
        };
        let p = self.people.iter_mut().find(|p| p.id == person).ok_or("Unknown person.")?;
        match index.and_then(|i| p.emergency_contacts.get_mut(i)) {
            Some(slot) => *slot = c,
            None => p.emergency_contacts.push(c),
        }
        Ok(())
    }

    pub fn remove_emergency_contact(&mut self, person: u32, index: usize) {
        if let Some(p) = self.people.iter_mut().find(|p| p.id == person)
            && index < p.emergency_contacts.len()
        {
            p.emergency_contacts.remove(index);
        }
    }

    pub fn label(&self, id: u32) -> Option<&Label> {
        self.labels.iter().find(|l| l.id == id)
    }

    /// Adds (`id` = None) or renames and recolours a label. Names are unique.
    pub fn save_label(&mut self, id: Option<u32>, name: &str, color: LabelColor) -> Result<u32, String> {
        let name = name.trim();
        if name.is_empty() {
            return Err("Enter a name for the label.".into());
        }
        if self.labels.iter().any(|l| Some(l.id) != id && fold(&l.name) == fold(name)) {
            return Err("There is already a label with this name.".into());
        }
        match id.and_then(|i| self.labels.iter_mut().find(|l| l.id == i)) {
            Some(l) => {
                l.name = name.into();
                l.color = color;
                Ok(l.id)
            }
            None => {
                let new = self.labels.iter().map(|l| l.id).max().unwrap_or(0) + 1;
                self.labels.push(Label { id: new, name: name.into(), color });
                Ok(new)
            }
        }
    }

    /// Deletes a label and takes it off everyone.
    pub fn delete_label(&mut self, id: u32) {
        self.labels.retain(|l| l.id != id);
        for p in &mut self.people {
            p.labels.remove(&id);
        }
    }

    pub fn set_label(&mut self, person: u32, label: u32, on: bool) {
        if self.label(label).is_none() {
            return;
        }
        if let Some(p) = self.people.iter_mut().find(|p| p.id == person) {
            if on {
                p.labels.insert(label);
            } else {
                p.labels.remove(&label);
            }
        }
    }
}

/// People who moved away are kept this long, then removed for good.
pub const RETENTION_YEARS: i32 = 1;

/// The date a person who moved away on `moved` is removed (`YYYY-MM-DD`).
pub fn removal_date(moved: &str) -> String {
    match moved.get(..4).and_then(|y| y.parse::<i32>().ok()) {
        Some(y) => format!("{:04}{}", y + RETENTION_YEARS, &moved[4..]),
        None => moved.to_string(),
    }
}

/// Parses `YYYY-MM-DD` for the date formatter.
pub fn parse_date(s: &str) -> Option<(i32, u32, u32)> {
    let mut it = s.splitn(3, '-');
    Some((it.next()?.parse().ok()?, it.next()?.parse().ok()?, it.next()?.parse().ok()?))
}

fn opt(s: &str) -> Option<String> {
    let t = s.trim();
    (!t.is_empty()).then(|| t.to_string())
}

// ---------------------------------------------------------------------------------------------
// Form draft
// ---------------------------------------------------------------------------------------------

/// The person form. Text fields as typed; empty means "no value".
#[derive(Clone, Debug, PartialEq)]
pub struct PersonDraft {
    pub first_name: String,
    pub last_name: String,
    pub birthday: String,
    pub gender: String,
    pub email: String,
    pub telephone: String,
    pub mobile_phone: String,
    pub baptism_date: String,
    pub publisher_type: String,
    pub infirm: bool,
    pub pioneer_number: String,
    pub appointment: String,
    pub anointed: bool,
    pub address_line_1: String,
    pub address_line_2: String,
    pub postal_code: String,
    pub city: String,
    pub consent_date: String,
    pub notes: String,
    /// Plain group membership: a group id, or empty.
    pub group: String,
    /// The first month of the current publisher type (`YYYY-MM`). Empty: the current month when
    /// the type changes now, or no change.
    pub type_since: String,
}

impl Default for PersonDraft {
    fn default() -> Self {
        Self {
            first_name: String::new(),
            last_name: String::new(),
            birthday: String::new(),
            gender: String::new(),
            email: String::new(),
            telephone: String::new(),
            mobile_phone: String::new(),
            baptism_date: String::new(),
            // New people are most often baptised publishers.
            publisher_type: PublisherType::BaptisedPublisher.value().into(),
            infirm: false,
            pioneer_number: String::new(),
            appointment: String::new(),
            anointed: false,
            address_line_1: String::new(),
            address_line_2: String::new(),
            postal_code: String::new(),
            city: String::new(),
            consent_date: String::new(),
            notes: String::new(),
            group: String::new(),
            type_since: String::new(),
        }
    }
}

const NOTES_MAX: usize = 2000;

/// Today as `YYYY-MM-DD`.
pub fn today_iso() -> String {
    let (y, m, d) = prefs::today();
    format!("{y:04}-{m:02}-{d:02}")
}

fn valid_date(s: &str) -> bool {
    let b = s.as_bytes();
    b.len() == 10 && b[4] == b'-' && b[7] == b'-' && s.chars().enumerate().all(|(i, c)| i == 4 || i == 7 || c.is_ascii_digit())
}

impl PersonDraft {
    pub fn from_person(p: &Person, dir: &Directory) -> Self {
        let s = |o: &Option<String>| o.clone().unwrap_or_default();
        Self {
            first_name: p.first_name.clone(),
            last_name: p.last_name.clone(),
            birthday: s(&p.birthday),
            gender: p.gender.value().into(),
            email: s(&p.email),
            telephone: s(&p.telephone),
            mobile_phone: s(&p.mobile_phone),
            baptism_date: s(&p.baptism_date),
            publisher_type: p.publisher_type.value().into(),
            infirm: p.infirm,
            pioneer_number: s(&p.pioneer_number),
            appointment: p.appointment.map(|a| a.value().to_string()).unwrap_or_default(),
            anointed: p.anointed,
            address_line_1: s(&p.address_line_1),
            address_line_2: s(&p.address_line_2),
            postal_code: s(&p.postal_code),
            city: s(&p.city),
            consent_date: s(&p.consent_date),
            notes: s(&p.notes),
            // Their group: plain membership, else the group they lead (an overseer or assistant
            // is in that group too, without being listed as a plain member).
            group: dir.member_group(p.id).or_else(|| dir.home_group(p.id).map(|g| g.id)).map(|g| g.to_string()).unwrap_or_default(),
            type_since: p.type_since().map(|(y, m)| format!("{y}-{m:02}")).unwrap_or_default(),
        }
    }

    pub fn ty(&self) -> PublisherType {
        PublisherType::parse(&self.publisher_type).unwrap_or_default()
    }

    /// Appointments are only for baptised brothers.
    pub fn may_be_appointed(&self) -> bool {
        self.ty().is_baptised() && self.gender == Gender::Male.value()
    }

    /// Field errors: (field, message). Empty = valid.
    pub fn validate(&self) -> Vec<(&'static str, String)> {
        let mut e = Vec::new();
        let today = today_iso();
        if !self.type_since.trim().is_empty() {
            let (y, m, _) = crate::prefs::today();
            match parse_month(&self.type_since) {
                None => e.push(("type_since", "Choose the month the type started.".into())),
                Some(since) if since > (y, m) => e.push(("type_since", "The type cannot start in a future month.".into())),
                Some(_) => {}
            }
        }
        if self.first_name.trim().is_empty() {
            e.push(("first_name", "Enter the first name.".into()));
        }
        if self.last_name.trim().is_empty() {
            e.push(("last_name", "Enter the last name.".into()));
        }
        if Gender::parse(&self.gender).is_none() {
            e.push(("gender", "Choose the gender.".into()));
        }
        let email = self.email.trim();
        if !email.is_empty() && !(email.contains('@') && email.rsplit('@').next().is_some_and(|d| d.contains('.'))) {
            e.push(("email", "Enter an e-mail address like name@example.org.".into()));
        }
        for (field, value, label) in [("birthday", &self.birthday, "birthday"), ("baptism_date", &self.baptism_date, "baptism date"), ("consent_date", &self.consent_date, "consent date")] {
            if value.is_empty() {
                continue;
            }
            if !valid_date(value) {
                e.push((field, format!("Enter the {label} as a date.")));
            } else if *value > today {
                e.push((field, format!("The {label} cannot be in the future.")));
            }
        }
        if self.ty().is_baptised() && !self.birthday.is_empty() && !self.baptism_date.is_empty() && self.baptism_date <= self.birthday {
            e.push(("baptism_date", "The baptism date must be after the birthday.".into()));
        }
        if self.notes.chars().count() > NOTES_MAX {
            e.push(("notes", format!("Notes can have at most {NOTES_MAX} characters.")));
        }
        e
    }

    /// Values that saving will clear, because the type or gender no longer allows them.
    pub fn cleanup_notes(&self, leads: &[String]) -> Vec<String> {
        let ty = self.ty();
        let mut out = Vec::new();
        if !ty.has_pioneer_number() && !self.pioneer_number.trim().is_empty() {
            out.push("The pioneer number will be removed.".to_string());
        }
        if !ty.is_baptised() && !self.baptism_date.is_empty() {
            out.push("The baptism date will be removed.".into());
        }
        if !self.appointment.is_empty() && !self.may_be_appointed() {
            out.push("The appointment will be removed (only for baptised brothers).".into());
        }
        if self.infirm && !ty.has_pioneer_number() {
            out.push("\"Infirm\" will be turned off (only for regular and special pioneers and missionaries).".into());
        }
        if self.anointed && !ty.is_baptised() {
            out.push("\"Anointed\" will be turned off (only for baptised persons).".into());
        }
        if !ty.is_publisher() && !self.group.is_empty() {
            out.push("They will leave their field service group (only for publishers).".into());
        }
        if !(ty.is_baptised() && self.gender == Gender::Male.value()) {
            for l in leads {
                out.push(format!("They will stop being {l} (only for baptised brothers)."));
            }
        }
        out
    }

    /// The draft with the cleanups of [`Self::cleanup_notes`] applied.
    fn normalized(&self) -> Self {
        let mut d = self.clone();
        let ty = d.ty();
        if !ty.has_pioneer_number() {
            d.pioneer_number.clear();
        }
        if !ty.is_baptised() {
            d.baptism_date.clear();
            d.anointed = false;
        }
        if !d.may_be_appointed() {
            d.appointment.clear();
        }
        if !ty.is_publisher() {
            d.group.clear();
        }
        d
    }
}

// ---------------------------------------------------------------------------------------------
// Search and filters (m01-people.md, "The All users screen")
// ---------------------------------------------------------------------------------------------

/// Lower case without accents, for search.
pub fn fold(s: &str) -> String {
    s.chars()
        .flat_map(char::to_lowercase)
        .map(|c| match c {
            'á' | 'à' | 'â' | 'ä' | 'ã' | 'å' | 'ą' => 'a',
            'é' | 'è' | 'ê' | 'ë' | 'ę' | 'ě' => 'e',
            'í' | 'ì' | 'î' | 'ï' => 'i',
            'ó' | 'ò' | 'ô' | 'ö' | 'õ' | 'ø' | 'ő' => 'o',
            'ú' | 'ù' | 'û' | 'ü' | 'ů' | 'ű' => 'u',
            'ç' | 'ć' | 'č' => 'c',
            'ñ' | 'ń' | 'ň' => 'n',
            'ś' | 'š' => 's',
            'ź' | 'ż' | 'ž' => 'z',
            'ł' => 'l',
            'ř' => 'r',
            'ý' | 'ÿ' => 'y',
            c => c,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_type_change_keeps_the_old_type_for_earlier_months() {
        let mut d = crate::demo::demo_directory(12);
        let Some(p) = d.people.iter().find(|p| p.publisher_type == PublisherType::BaptisedPublisher && p.type_history.is_empty()).cloned() else { panic!("a baptised publisher") };
        let mut draft = PersonDraft::from_person(&p, &d);
        draft.publisher_type = PublisherType::ContinuousAuxiliaryPioneer.value().into();
        draft.type_since = "2026-10".into();
        assert!(d.save(Some(p.id), &draft).is_ok());
        let now = d.person(p.id).cloned().unwrap_or(p.clone());
        assert_eq!(now.type_in((2026, 9)), PublisherType::BaptisedPublisher);
        assert_eq!(now.type_in((2026, 10)), PublisherType::ContinuousAuxiliaryPioneer);
        assert_eq!(now.type_in((2020, 1)), PublisherType::BaptisedPublisher);
        assert_eq!(now.type_since(), Some((2026, 10)));
        // Moving the start, with the same type.
        let mut draft = PersonDraft::from_person(&now, &d);
        draft.type_since = "2026-08".into();
        assert!(d.save(Some(p.id), &draft).is_ok());
        let now = d.person(p.id).cloned().unwrap_or(p.clone());
        assert_eq!(now.type_in((2026, 8)), PublisherType::ContinuousAuxiliaryPioneer);
        assert_eq!(now.type_in((2026, 7)), PublisherType::BaptisedPublisher);
        // Back to the first type: the history says nothing any more.
        let mut draft = PersonDraft::from_person(&now, &d);
        draft.publisher_type = PublisherType::BaptisedPublisher.value().into();
        draft.type_since = "2026-08".into();
        assert!(d.save(Some(p.id), &draft).is_ok());
        assert!(d.person(p.id).is_some_and(|x| x.type_history.is_empty()));
    }

    #[test]
    fn infirm_is_only_for_pioneers_with_a_yearly_goal_and_new_people_are_publishers() {
        assert_eq!(PersonDraft::default().publisher_type, PublisherType::BaptisedPublisher.value());
        let mut d = crate::demo::demo_directory(12);
        let mut draft = PersonDraft { first_name: "Ina".into(), last_name: "Firm".into(), gender: "female".into(), infirm: true, ..Default::default() };
        let id = d.save(None, &draft).unwrap_or(0);
        assert!(d.person(id).is_some_and(|p| !p.infirm));
        draft.publisher_type = PublisherType::RegularPioneer.value().into();
        assert!(d.save(Some(id), &draft).is_ok());
        assert!(d.person(id).is_some_and(|p| p.infirm));
    }

    #[test]
    fn relatives_offered_never_belong_to_another_family() {
        let d = crate::demo::demo_directory(36);
        let Some(f) = d.families.first() else { panic!("the demo has families") };
        let member = f.head;
        let offered = d.relative_candidates(member);
        // Nobody from another family; people without one, and this family's own members.
        assert!(offered.iter().all(|p| d.family_of(p.id).is_none_or(|x| x.id == f.id)));
        assert!(offered.iter().any(|p| d.family_of(p.id).is_none()));
        assert!(!offered.iter().any(|p| p.id == member));
        // Someone without a family may join any family.
        if let Some(loner) = d.without_family().first().map(|p| p.id) {
            assert!(d.relative_candidates(loner).iter().any(|p| d.family_of(p.id).is_some()));
        }
    }

    #[test]
    fn a_group_leader_shows_their_group_and_saving_it_keeps_them_leading() {
        let mut d = crate::demo::demo_directory(36);
        let Some((gid, leader)) = d.groups.iter().find_map(|g| g.overseer.filter(|o| d.member_group(*o).is_none()).map(|o| (g.id, o))) else {
            panic!("the demo has an overseer who is not a plain member");
        };
        let draft = d.person(leader).map(|p| PersonDraft::from_person(p, &d)).unwrap_or_default();
        assert_eq!(draft.group, gid.to_string());
        // Saving the form unchanged keeps them leading that group, without a second membership.
        assert!(d.save(Some(leader), &draft).is_ok());
        assert!(d.group(gid).is_some_and(|g| g.overseer == Some(leader) && !g.members.contains(&leader)));
        assert_eq!(d.groups_of(leader).len(), 1);
    }

    fn dir() -> Directory {
        crate::demo::demo_directory(40)
    }

    #[test]
    fn activity_ignores_months_before_first_report() {
        use crate::reports::{reporting_month, shift, ReportDraft};
        let mut d = Directory::default();
        let draft = PersonDraft { first_name: "Ann".into(), last_name: "Test".into(), gender: "female".into(), publisher_type: PublisherType::UnbaptisedPublisher.value().into(), ..Default::default() };
        let n = d.save(None, &draft).unwrap_or(0);
        let p = d.person(n).cloned().unwrap_or_else(|| d.people[0].clone());
        assert_eq!(d.activity(&p), None, "no report yet");
        let yes = ReportDraft { took_part: "yes".into(), studies: "0".into(), ..Default::default() };
        let no = ReportDraft { took_part: "no".into(), ..yes.clone() };
        // A new publisher reporting the last 3 months is active.
        for i in 0..3 {
            assert!(d.save_report(n, shift(reporting_month(), -i), &yes).is_ok());
        }
        assert_eq!(d.activity(&p), Some(Activity::Active));
        // A month with "did not take part" makes them irregular.
        assert!(d.save_report(n, reporting_month(), &no).is_ok());
        assert_eq!(d.activity(&p), Some(Activity::Irregular));
        // Six months without taking part: inactive.
        for i in 0..6 {
            assert!(d.save_report(n, shift(reporting_month(), -i), &no).is_ok());
        }
        assert_eq!(d.activity(&p), Some(Activity::Inactive));
    }

    #[test]
    fn relationships_are_shown_from_both_sides() {
        let mut d = Directory::default();
        let prefs = Preferences::default();
        for (first, gender) in [("Ann", "female"), ("Ben", "male")] {
            let draft = PersonDraft { first_name: first.into(), last_name: "Test".into(), gender: gender.into(), ..Default::default() };
            d.save(None, &draft).ok();
        }
        // Ann (1) is the Mother of Ben (2): on Ben's page, add relative Ann as Mother.
        assert!(d.add_relationship(2, 1, RelationKind::Mother, &prefs).is_ok());
        assert_eq!(d.relatives(2)[0].kind, Some(RelationKind::Mother));
        assert_eq!(d.relatives(1)[0].kind, Some(RelationKind::Son));
        assert_eq!(d.family_of(1).map(|f| f.head), Some(1));
        // Wrong gender is refused.
        assert!(d.add_relationship(1, 2, RelationKind::Sister, &prefs).is_err());
        // Leaving a two-person family removes the family.
        d.remove_from_family(2);
        assert!(d.families.is_empty());
    }

    #[test]
    fn saving_clears_fields_the_type_does_not_allow() {
        let mut d = Directory::default();
        let draft = PersonDraft {
            first_name: "Cleo".into(),
            last_name: "Test".into(),
            gender: "female".into(),
            publisher_type: PublisherType::BaptisedPublisher.value().into(),
            pioneer_number: "123".into(),
            appointment: "elder".into(),
            ..Default::default()
        };
        assert_eq!(draft.cleanup_notes(&[]).len(), 2);
        let n = d.save(None, &draft).unwrap_or(0);
        let p = d.person(n).cloned();
        assert!(p.as_ref().is_some_and(|p| p.pioneer_number.is_none() && p.appointment.is_none()));
    }

    #[test]
    fn moving_away_keeps_families_together_or_splits_them() {
        let mut d = dir();
        // John (1) moves away alone: Mary becomes head, North loses its overseer.
        d.move_away(1, "2026-11-01", false);
        assert_eq!(d.family_of(2).map(|f| f.head), Some(2));
        assert!(d.groups[0].overseer.is_none());
        assert!(d.person(1).is_some_and(|p| p.moved_away.is_some() && d.activity(p).is_none()));
        // The Garcias (7, 8, 9) move together: the family stays.
        d.move_away(7, "2026-11-01", true);
        assert!(d.family_of(7).is_some_and(|f| f.members.len() == 3));
        assert!([7, 8, 9].iter().all(|n| d.person(*n).is_some_and(|p| p.moved_away.is_some())));
    }

    #[test]
    fn people_who_moved_away_are_removed_after_one_year() {
        let mut d = dir();
        assert_eq!(removal_date("2025-11-17"), "2026-11-17");
        d.move_away(17, "2025-11-17", false);
        d.move_away(21, "2025-11-18", false);
        // 17 (exactly one year) and Henry Doyle (36, demo data) go; 21 stays one more day.
        assert_eq!(d.purge_expired("2026-11-17"), 2);
        assert!(d.person(17).is_none() && d.person(36).is_none());
        assert!(d.person(21).is_some());
    }

    #[test]
    fn group_leaders_are_members_and_replaced_leaders_stay() {
        let mut d = dir();
        let prefs = Preferences::default();
        // North: overseer John (1), assistant Michael (5). Make Ethan (21) the assistant.
        let mut draft = GroupDraft::from_group(&d.groups[0]);
        draft.assistant = "21".into();
        assert!(d.save_group(Some(1), &draft).is_ok());
        let g = d.group(1).cloned().unwrap_or_else(|| d.groups[0].clone());
        assert_eq!(g.assistant, Some(21));
        assert!(!g.members.contains(&21), "leaders are not listed twice");
        assert!(g.members.contains(&5), "the old assistant stays a member");
        assert!(d.group_warnings(Some(1), &draft, &prefs).iter().any(|w| w.contains("not an elder or a ministerial servant")));
        // Same person twice, or an empty or duplicate name, is refused.
        draft.overseer = "21".into();
        assert!(d.save_group(Some(1), &draft).is_err());
        let dup = GroupDraft { name: " south ".into(), ..Default::default() };
        assert!(d.save_group(None, &dup).is_err());
        // Members of another group cannot be added; people without a group can.
        d.add_members(1, &[8, 27]);
        assert_eq!(d.member_group(8), Some(2));
        assert_eq!(d.member_group(27), Some(1));
        // Only baptised brothers can lead: a sister is refused.
        draft.overseer = "2".into();
        draft.assistant = "21".into();
        assert!(d.save_group(Some(1), &draft).is_err());
        // Order: elders, then ministerial servants, then other baptised brothers.
        let heads: Vec<&str> = d.leader_candidates(&prefs).iter().map(|(h, _)| *h).collect();
        assert_eq!(heads, vec!["Elders", "Ministerial servants", "Other baptised brothers"]);
    }

    #[test]
    fn a_leader_who_no_longer_qualifies_stops_leading() {
        let mut d = dir();
        // John (1) leads North. Saving him as unbaptised removes the leadership, not the group.
        let mut draft = PersonDraft::from_person(&d.people[0], &d);
        draft.publisher_type = PublisherType::UnbaptisedPublisher.value().into();
        assert!(draft.cleanup_notes(&["overseer of North".into()]).iter().any(|n| n.contains("overseer of North")));
        assert!(d.save(Some(1), &draft).is_ok());
        assert_eq!(d.groups[0].overseer, None);
        assert_eq!(d.member_group(1), Some(1));
    }

    #[test]
    fn group_composition_counts_appointments_pioneers_and_age() {
        let d = dir();
        // North (1): John elder (overseer), Michael MS (assistant), George elder; Mary and Elena
        // regular pioneers; Sarah continuous auxiliary pioneer.
        let c = d.group_composition(1);
        assert_eq!((c.elders, c.ministerial_servants, c.pioneers, c.continuous_auxiliary), (2, 1, 2, 1));
        assert_eq!(c.with_birthday, c.members);
        assert!(c.average_age.is_some_and(|a| a > 18.0 && a < 90.0));
    }

    #[test]
    fn deleting_a_family_keeps_its_members() {
        let mut d = dir();
        let before = d.people.len();
        let id = d.family_of(1).map(|f| f.id).unwrap_or(0);
        d.delete_family(id);
        assert!(d.family_of(1).is_none() && d.family_of(2).is_none());
        assert_eq!(d.people.len(), before);
        assert!(d.without_family().iter().any(|p| p.id == 1));
    }

    #[test]
    fn required_fields_are_checked() {
        let e = PersonDraft::default().validate();
        let fields: Vec<&str> = e.iter().map(|(f, _)| *f).collect();
        assert_eq!(fields, vec!["first_name", "last_name", "gender"]);
    }
}
