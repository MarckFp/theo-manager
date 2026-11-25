//! ⚠️ MOCK of the midweek meeting (M-03): the plan of each week (sections, parts, songs, the
//! Bible reading), who has each part (main hall and, when on, the auxiliary classroom), the
//! candidates for a part, filling, sending with notices, and statistics.
//!
//! The record keeps the **plan**: references to the workbook (issue, language, document, part
//! number), minutes, kinds, songs and assignments. Titles and descriptions come from the workbook
//! read on each device (`src/workbook.rs`, R-033); only what a planner types is stored.
//! Rules: `docs/model/m03-midweek-meeting.md`.

use crate::people::Directory;
use crate::privileges::Privilege;
use std::collections::{BTreeMap, BTreeSet};

/// The sections of the meeting.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize)]
pub enum SectionKind {
    Treasures,
    Ministry,
    Living,
    /// Added by a planner (for example for the circuit overseer's visit).
    Other,
}

impl SectionKind {
    pub fn label(self) -> &'static str {
        match self {
            SectionKind::Treasures => "Treasures from God's Word",
            SectionKind::Ministry => "Apply yourself to the field ministry",
            SectionKind::Living => "Living as Christians",
            SectionKind::Other => "Other",
        }
    }
}

/// What a part is: who may have it, and whether it has an assistant or an auxiliary classroom.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize)]
pub enum PartKind {
    TreasuresTalk,
    SpiritualGems,
    BibleReading,
    StartingConversation,
    FollowingUp,
    MakingDisciples,
    ExplainingBeliefs,
    StudentTalk,
    /// A discussion of the field ministry by an elder.
    Discussion,
    Living,
    BibleStudy,
    Other,
}

impl PartKind {
    pub const ALL: [PartKind; 12] = [
        PartKind::TreasuresTalk,
        PartKind::SpiritualGems,
        PartKind::BibleReading,
        PartKind::StartingConversation,
        PartKind::FollowingUp,
        PartKind::MakingDisciples,
        PartKind::ExplainingBeliefs,
        PartKind::StudentTalk,
        PartKind::Discussion,
        PartKind::Living,
        PartKind::BibleStudy,
        PartKind::Other,
    ];

    pub fn value(self) -> &'static str {
        match self {
            PartKind::TreasuresTalk => "treasures",
            PartKind::SpiritualGems => "gems",
            PartKind::BibleReading => "bible-reading",
            PartKind::StartingConversation => "starting",
            PartKind::FollowingUp => "following-up",
            PartKind::MakingDisciples => "making-disciples",
            PartKind::ExplainingBeliefs => "explaining",
            PartKind::StudentTalk => "student-talk",
            PartKind::Discussion => "discussion",
            PartKind::Living => "living",
            PartKind::BibleStudy => "cbs",
            PartKind::Other => "other",
        }
    }

    pub fn parse(s: &str) -> PartKind {
        Self::ALL.into_iter().find(|k| k.value() == s).unwrap_or(PartKind::Other)
    }

    pub fn label(self) -> &'static str {
        match self {
            PartKind::TreasuresTalk => "Treasures talk",
            PartKind::SpiritualGems => "Spiritual gems",
            PartKind::BibleReading => "Bible reading",
            PartKind::StartingConversation => "Starting a conversation",
            PartKind::FollowingUp => "Following up",
            PartKind::MakingDisciples => "Making disciples",
            PartKind::ExplainingBeliefs => "Explaining your beliefs",
            PartKind::StudentTalk => "Student talk",
            PartKind::Discussion => "Field ministry discussion",
            PartKind::Living => "Living as Christians part",
            PartKind::BibleStudy => "Congregation Bible Study",
            PartKind::Other => "Other part",
        }
    }

    /// The privilege the person needs (M-01, privileges). None: any publisher.
    pub fn privilege(self) -> Option<Privilege> {
        Some(match self {
            PartKind::TreasuresTalk => Privilege::Treasures,
            PartKind::SpiritualGems => Privilege::SpiritualGems,
            PartKind::BibleReading => Privilege::BibleReading,
            PartKind::StartingConversation => Privilege::StartingConversation,
            PartKind::FollowingUp => Privilege::FollowingUp,
            PartKind::MakingDisciples | PartKind::ExplainingBeliefs => Privilege::MakingDisciples,
            PartKind::StudentTalk => Privilege::StudentTalk,
            PartKind::Discussion => Privilege::FieldMinistryDiscussion,
            PartKind::Living => Privilege::LivingAsChristians,
            PartKind::BibleStudy => Privilege::BibleStudyConductor,
            PartKind::Other => return None,
        })
    }

    /// Student demonstrations have an assistant; the Congregation Bible Study has a reader.
    pub fn second(self) -> Option<&'static str> {
        match self {
            PartKind::StartingConversation | PartKind::FollowingUp | PartKind::MakingDisciples | PartKind::ExplainingBeliefs => Some("Assistant"),
            PartKind::BibleStudy => Some("Reader"),
            _ => None,
        }
    }

    /// The chairman gives 1 minute of counsel after it (the Bible reading and student parts).
    pub fn has_counsel(self) -> bool {
        self.in_aux_classroom()
    }

    /// Student parts are given in the auxiliary classroom too.
    pub fn in_aux_classroom(self) -> bool {
        matches!(
            self,
            PartKind::BibleReading | PartKind::StartingConversation | PartKind::FollowingUp | PartKind::MakingDisciples | PartKind::ExplainingBeliefs | PartKind::StudentTalk
        )
    }
}

/// Where a week comes from in the workbook.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct WeekSource {
    pub issue: String,
    pub lang: String,
    pub doc: String,
}

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct MwPart {
    pub id: u32,
    pub kind: PartKind,
    /// Its number in the workbook (for the title and description on this device).
    pub source: Option<u8>,
    /// Typed by a planner (otherwise the workbook's).
    pub title: Option<String>,
    pub description: Option<String>,
    pub minutes: u8,
    /// Changed by a planner: a refresh of the workbook keeps it as it is.
    pub edited: bool,
    pub main: Option<u32>,
    pub assistant: Option<u32>,
    /// The auxiliary classroom (student parts, when it is on that week).
    pub aux: Option<u32>,
    pub aux_assistant: Option<u32>,
}

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct MwSection {
    pub id: u32,
    pub kind: SectionKind,
    /// Typed by a planner (sections they added).
    pub title: Option<String>,
    pub parts: Vec<MwPart>,
}

/// What everyone sees after sending: who does what, on which day.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct MwSent {
    pub date: String,
    pub no_meeting: bool,
    /// (person, what they do).
    pub roles: Vec<(u32, String)>,
    /// (place, person), to show the week as it was sent (see [`role_key`]).
    #[serde(default)]
    pub places: Vec<(String, u32)>,
}

/// A stable name of a place ("chairman", "main:12").
pub fn role_key(role: MwRole) -> String {
    match role {
        MwRole::Chairman => "chairman".into(),
        MwRole::AuxCounselor => "aux-counselor".into(),
        MwRole::OpeningPrayer => "opening-prayer".into(),
        MwRole::ClosingPrayer => "closing-prayer".into(),
        MwRole::Main(id) => format!("main:{id}"),
        MwRole::Assistant(id) => format!("assistant:{id}"),
        MwRole::AuxMain(id) => format!("aux:{id}"),
        MwRole::AuxAssistant(id) => format!("aux-assistant:{id}"),
    }
}

/// The plan of one week.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct MidweekWeek {
    pub id: u32,
    /// Monday of the week, `YYYY-MM-DD`.
    pub monday: String,
    pub source: Option<WeekSource>,
    /// The weekly Bible reading ("JEREMIAH 51-52").
    pub reading: String,
    /// Opening, middle and closing song.
    pub songs: [Option<u16>; 3],
    pub sections: Vec<MwSection>,
    pub chairman: Option<u32>,
    /// Counselor of the auxiliary classroom.
    pub aux_counselor: Option<u32>,
    pub opening_prayer: Option<u32>,
    pub closing_prayer: Option<u32>,
    /// The auxiliary classroom is used this week.
    pub aux_on: bool,
    /// No meeting this week (for example a convention).
    pub no_meeting: bool,
    pub sent: Option<MwSent>,
}

/// Settings of the midweek meeting.
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct MidweekSettings {
    /// The congregation has an auxiliary classroom (new weeks use it).
    pub aux_classroom: bool,
}

// Minutes of what is not a numbered part (the usual length; the workbook gives only the parts).
/// Opening song and prayer.
pub const OPENING_SONG: u32 = 4;
pub const OPENING_PRAYER: u32 = 1;
pub const OPENING_COMMENTS: u32 = 1;
pub const MIDDLE_SONG: u32 = 4;
pub const CONCLUDING_COMMENTS: u32 = 3;
pub const CLOSING_SONG: u32 = 4;
pub const CLOSING_PRAYER: u32 = 1;
/// The chairman's counsel after the Bible reading and each student part.
pub const COUNSEL: u32 = 1;

/// When each moment of a meeting starts, in minutes after midnight.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Timeline {
    pub opening_song: u32,
    pub opening_prayer: u32,
    pub opening_comments: u32,
    /// (part, start).
    pub parts: Vec<(u32, u32)>,
    pub middle_song: Option<u32>,
    pub concluding: u32,
    pub closing_song: u32,
    pub closing_prayer: u32,
    pub end: u32,
}

impl Timeline {
    pub fn part(&self, id: u32) -> Option<u32> {
        self.parts.iter().find(|(p, _)| *p == id).map(|(_, t)| *t)
    }
}

/// "19:00" → 1140.
pub fn minutes_of(hhmm: &str) -> Option<u32> {
    let (h, m) = hhmm.trim().split_once(':')?;
    Some(h.parse::<u32>().ok()? * 60 + m.get(..2).unwrap_or(m).parse::<u32>().ok()?)
}

/// 1140 → "19:00".
pub fn hhmm(minutes: u32) -> String {
    format!("{:02}:{:02}", minutes / 60 % 24, minutes % 60)
}

/// A place to fill in a week.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MwRole {
    Chairman,
    AuxCounselor,
    OpeningPrayer,
    ClosingPrayer,
    Main(u32),
    Assistant(u32),
    AuxMain(u32),
    AuxAssistant(u32),
}

/// Whether a person can take a place.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MwFit {
    Available,
    Away,
}

/// One person who can take a place, and what helps to choose.
#[derive(Clone, Debug, PartialEq)]
pub struct MwCandidate {
    pub person: u32,
    pub fit: MwFit,
    /// What else they do that week.
    pub busy: Vec<String>,
    /// Monday of the last week they had this kind of assignment (before this week).
    pub last_this: Option<String>,
    /// Monday of the last week they had any assignment of the meeting (before this week).
    pub last_any: Option<String>,
}

/// The part form.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct PartDraft {
    pub kind: Option<PartKind>,
    pub title: String,
    pub description: String,
    pub minutes: String,
}

/// The day of the meeting: Monday plus the weekday of the midweek meeting (0 = Monday).
pub fn meeting_date(monday: &str, weekday_from_monday: i64) -> String {
    crate::events::add_days(monday, weekday_from_monday).unwrap_or_else(|| monday.to_string())
}

/// Days after Monday of a weekday name ("Thursday" → 3).
pub fn weekday_offset(day: &str) -> i64 {
    ["Monday", "Tuesday", "Wednesday", "Thursday", "Friday", "Saturday", "Sunday"].iter().position(|d| d.eq_ignore_ascii_case(day)).unwrap_or(3) as i64
}

/// Monday of the week of a date.
pub fn monday_of(date: &str) -> String {
    let Some(d) = crate::people::parse_date(date) else { return date.to_string() };
    let z = crate::prefs::days_from_ymd(d);
    let wd = (z + 3).rem_euclid(7);
    let (y, m, d) = crate::prefs::ymd_from_days(z - wd);
    format!("{y:04}-{m:02}-{d:02}")
}

impl MidweekWeek {
    pub fn parts(&self) -> impl Iterator<Item = &MwPart> {
        self.sections.iter().flat_map(|s| s.parts.iter())
    }

    fn part_mut(&mut self, id: u32) -> Option<&mut MwPart> {
        self.sections.iter_mut().flat_map(|s| s.parts.iter_mut()).find(|p| p.id == id)
    }

    pub fn part(&self, id: u32) -> Option<&MwPart> {
        self.parts().find(|p| p.id == id)
    }

    /// When everything starts, from the meeting's start (minutes after midnight). The middle song
    /// opens the first *Living as Christians* section; 1 minute of counsel follows the Bible
    /// reading and each student part.
    pub fn timeline(&self, start: u32) -> Timeline {
        let mut t = Timeline { opening_song: start, opening_prayer: start + OPENING_SONG, ..Default::default() };
        t.opening_comments = t.opening_prayer + OPENING_PRAYER;
        let mut now = t.opening_comments + OPENING_COMMENTS;
        for s in &self.sections {
            if s.kind == SectionKind::Living && t.middle_song.is_none() {
                t.middle_song = Some(now);
                now += MIDDLE_SONG;
            }
            for p in &s.parts {
                t.parts.push((p.id, now));
                now += p.minutes as u32 + if p.kind.has_counsel() { COUNSEL } else { 0 };
            }
        }
        t.concluding = now;
        t.closing_song = now + CONCLUDING_COMMENTS;
        t.closing_prayer = t.closing_song + CLOSING_SONG;
        t.end = t.closing_prayer + CLOSING_PRAYER;
        t
    }

    /// Every place of the week with its person (filled or not), in meeting order.
    pub fn places(&self) -> Vec<(MwRole, Option<u32>)> {
        let mut v = vec![(MwRole::Chairman, self.chairman), (MwRole::OpeningPrayer, self.opening_prayer)];
        if self.aux_on {
            v.push((MwRole::AuxCounselor, self.aux_counselor));
        }
        for p in self.parts() {
            v.push((MwRole::Main(p.id), p.main));
            if p.kind.second().is_some() {
                v.push((MwRole::Assistant(p.id), p.assistant));
            }
            if self.aux_on && p.kind.in_aux_classroom() {
                v.push((MwRole::AuxMain(p.id), p.aux));
                if p.kind.second().is_some() {
                    v.push((MwRole::AuxAssistant(p.id), p.aux_assistant));
                }
            }
        }
        v.push((MwRole::ClosingPrayer, self.closing_prayer));
        v
    }

    pub fn person_at(&self, role: MwRole) -> Option<u32> {
        match role {
            MwRole::Chairman => self.chairman,
            MwRole::AuxCounselor => self.aux_counselor,
            MwRole::OpeningPrayer => self.opening_prayer,
            MwRole::ClosingPrayer => self.closing_prayer,
            MwRole::Main(id) => self.part(id)?.main,
            MwRole::Assistant(id) => self.part(id)?.assistant,
            MwRole::AuxMain(id) => self.part(id)?.aux,
            MwRole::AuxAssistant(id) => self.part(id)?.aux_assistant,
        }
    }

    /// How a role reads ("Bible reading · auxiliary classroom").
    pub fn role_label(&self, role: MwRole) -> String {
        let kind = |id: u32| self.part(id).map(|p| p.kind.label()).unwrap_or("Part");
        let second = |id: u32| self.part(id).and_then(|p| p.kind.second()).unwrap_or("Assistant");
        match role {
            MwRole::Chairman => "Chairman".into(),
            MwRole::AuxCounselor => "Auxiliary classroom counselor".into(),
            MwRole::OpeningPrayer => "Opening prayer".into(),
            MwRole::ClosingPrayer => "Closing prayer".into(),
            MwRole::Main(id) => kind(id).into(),
            MwRole::Assistant(id) => format!("{} · {}", kind(id), second(id).to_lowercase()),
            MwRole::AuxMain(id) => format!("{} · auxiliary classroom", kind(id)),
            MwRole::AuxAssistant(id) => format!("{} · {} · auxiliary classroom", kind(id), second(id).to_lowercase()),
        }
    }

    /// (person, role) of everyone assigned.
    pub fn roles(&self) -> Vec<(u32, String)> {
        if self.no_meeting {
            return Vec::new();
        }
        self.places().into_iter().filter_map(|(r, p)| p.map(|p| (p, self.role_label(r)))).collect()
    }

    /// (filled, places).
    pub fn filled(&self) -> (usize, usize) {
        let places = self.places();
        (places.iter().filter(|(_, p)| p.is_some()).count(), places.len())
    }

    pub fn status(&self, date: &str) -> crate::service_meetings::SendStatus {
        use crate::service_meetings::SendStatus;
        match &self.sent {
            None => SendStatus::NotSent,
            Some(s) if s.roles == self.roles() && s.no_meeting == self.no_meeting && s.date == date => SendStatus::Sent,
            Some(_) => SendStatus::Changed,
        }
    }
}

/// The privilege needed for a role.
fn role_privilege(week: &MidweekWeek, role: MwRole) -> Option<Privilege> {
    match role {
        MwRole::Chairman => Some(Privilege::MidweekChairman),
        MwRole::AuxCounselor => Some(Privilege::AuxiliaryChairman),
        MwRole::OpeningPrayer | MwRole::ClosingPrayer => Some(Privilege::Prayer),
        MwRole::Main(id) | MwRole::AuxMain(id) => week.part(id).and_then(|p| p.kind.privilege()),
        MwRole::Assistant(id) | MwRole::AuxAssistant(id) => Some(if week.part(id).is_some_and(|p| p.kind == PartKind::BibleStudy) { Privilege::BibleStudyReader } else { Privilege::Assistant }),
    }
}

impl Directory {
    pub fn midweek_week(&self, monday: &str) -> Option<&MidweekWeek> {
        self.midweek_weeks.iter().find(|w| w.monday == monday)
    }

    fn midweek_week_mut(&mut self, monday: &str) -> Option<&mut MidweekWeek> {
        self.midweek_weeks.iter_mut().find(|w| w.monday == monday)
    }

    fn next_mw_id(&self) -> u32 {
        let parts = self.midweek_weeks.iter().flat_map(|w| w.sections.iter().flat_map(|s| std::iter::once(s.id).chain(s.parts.iter().map(|p| p.id))));
        parts.chain(self.midweek_weeks.iter().map(|w| w.id)).max().unwrap_or(0) + 1
    }

    /// Creates or refreshes the plan of a week from the workbook. Assignments, edited parts and
    /// added sections stay; parts the planner did not change follow the workbook.
    pub fn import_midweek_week(&mut self, content: &crate::workbook::WeekContent, source: Option<WeekSource>) {
        let aux = self.midweek_settings.aux_classroom;
        if self.midweek_week(&content.monday).is_none() {
            let id = self.next_mw_id();
            self.midweek_weeks.push(MidweekWeek {
                id,
                monday: content.monday.clone(),
                source: None,
                reading: String::new(),
                songs: [None; 3],
                sections: Vec::new(),
                chairman: None,
                aux_counselor: None,
                opening_prayer: None,
                closing_prayer: None,
                aux_on: aux,
                no_meeting: false,
                sent: None,
            });
        }
        let source_known = source.is_some();
        let mut next = self.next_mw_id();
        let Some(w) = self.midweek_week_mut(&content.monday) else { return };
        w.source = source;
        w.reading = content.reading.clone();
        w.songs = content.songs;
        for kind in [SectionKind::Treasures, SectionKind::Ministry, SectionKind::Living] {
            if !w.sections.iter().any(|s| s.kind == kind) {
                let at = w.sections.iter().position(|s| s.kind > kind).unwrap_or(w.sections.len());
                w.sections.insert(at, MwSection { id: next, kind, title: None, parts: Vec::new() });
                next += 1;
            }
        }
        // Parts of the workbook that are gone (and were not edited) are removed.
        let numbers: BTreeSet<u8> = content.parts.iter().map(|p| p.number).collect();
        for s in w.sections.iter_mut() {
            s.parts.retain(|p| p.edited || p.source.is_none_or(|n| numbers.contains(&n)));
        }
        for c in &content.parts {
            let existing = w.sections.iter_mut().flat_map(|s| s.parts.iter_mut()).find(|p| p.source == Some(c.number));
            match existing {
                Some(p) if p.edited => {}
                Some(p) => {
                    // Not changed by a planner: the text comes from the workbook on the device.
                    p.kind = c.kind;
                    p.minutes = c.minutes;
                    if source_known {
                        p.title = None;
                        p.description = None;
                    }
                    if p.kind.second().is_none() {
                        p.assistant = None;
                        p.aux_assistant = None;
                    }
                    if !p.kind.in_aux_classroom() {
                        p.aux = None;
                    }
                }
                None => {
                    let at = w.sections.iter().position(|s| s.kind == c.section).unwrap_or(0);
                    if let Some(s) = w.sections.get_mut(at) {
                        let at = s.parts.iter().position(|p| p.source.is_some_and(|n| n > c.number)).unwrap_or(s.parts.len());
                        s.parts.insert(
                            at,
                            MwPart { id: next, kind: c.kind, source: Some(c.number), title: None, description: None, minutes: c.minutes, edited: false, main: None, assistant: None, aux: None, aux_assistant: None },
                        );
                        next += 1;
                    }
                }
            }
        }
    }

    /// Adds an empty week (planned by hand, without the workbook).
    pub fn add_midweek_week(&mut self, monday: &str) {
        if self.midweek_week(monday).is_some() {
            return;
        }
        let id = self.next_mw_id();
        let aux = self.midweek_settings.aux_classroom;
        let sections = [SectionKind::Treasures, SectionKind::Ministry, SectionKind::Living]
            .into_iter()
            .enumerate()
            .map(|(i, kind)| MwSection { id: id + 1 + i as u32, kind, title: None, parts: Vec::new() })
            .collect();
        self.midweek_weeks.push(MidweekWeek {
            id,
            monday: monday.into(),
            source: None,
            reading: String::new(),
            songs: [None; 3],
            sections,
            chairman: None,
            aux_counselor: None,
            opening_prayer: None,
            closing_prayer: None,
            aux_on: aux,
            no_meeting: false,
            sent: None,
        });
    }

    // ----- editing -----

    /// Saves a part (new when `part` is None, in `section`).
    pub fn save_midweek_part(&mut self, monday: &str, section: u32, part: Option<u32>, d: &PartDraft) -> Result<u32, String> {
        let minutes: u8 = d.minutes.trim().parse().map_err(|_| "Enter the minutes, for example 5.".to_string())?;
        if minutes == 0 || minutes > 120 {
            return Err("A part has 1 to 120 minutes.".into());
        }
        let id = self.next_mw_id();
        let w = self.midweek_week_mut(monday).ok_or("This week has no plan yet.")?;
        let text = |s: &str| (!s.trim().is_empty()).then(|| s.trim().to_string());
        match part.and_then(|p| w.part_mut(p)) {
            Some(p) => {
                p.kind = d.kind.unwrap_or(p.kind);
                p.title = text(&d.title);
                p.description = text(&d.description);
                p.minutes = minutes;
                p.edited = true;
                Ok(p.id)
            }
            None => {
                if d.title.trim().is_empty() {
                    return Err("Give the part a title.".into());
                }
                let s = w.sections.iter_mut().find(|s| s.id == section).ok_or("This section does not exist any more.")?;
                s.parts.push(MwPart {
                    id,
                    kind: d.kind.unwrap_or(PartKind::Other),
                    source: None,
                    title: text(&d.title),
                    description: text(&d.description),
                    minutes,
                    edited: true,
                    main: None,
                    assistant: None,
                    aux: None,
                    aux_assistant: None,
                });
                Ok(id)
            }
        }
    }

    pub fn delete_midweek_part(&mut self, monday: &str, part: u32) {
        if let Some(w) = self.midweek_week_mut(monday) {
            for s in w.sections.iter_mut() {
                s.parts.retain(|p| p.id != part);
            }
        }
    }

    pub fn add_midweek_section(&mut self, monday: &str, title: &str) -> Result<u32, String> {
        if title.trim().is_empty() {
            return Err("Give the section a title.".into());
        }
        let id = self.next_mw_id();
        let w = self.midweek_week_mut(monday).ok_or("This week has no plan yet.")?;
        w.sections.push(MwSection { id, kind: SectionKind::Other, title: Some(title.trim().into()), parts: Vec::new() });
        Ok(id)
    }

    pub fn delete_midweek_section(&mut self, monday: &str, section: u32) {
        if let Some(w) = self.midweek_week_mut(monday) {
            w.sections.retain(|s| s.id != section);
        }
    }

    /// Moves a section one place up (`-1`) or down (`1`).
    pub fn move_midweek_section(&mut self, monday: &str, section: u32, delta: i32) {
        let Some(w) = self.midweek_week_mut(monday) else { return };
        let Some(i) = w.sections.iter().position(|s| s.id == section) else { return };
        let j = i as i64 + delta as i64;
        if j >= 0 && (j as usize) < w.sections.len() {
            w.sections.swap(i, j as usize);
        }
    }

    /// Moves a part one place up or down. At the edge of its section it goes to the end of the
    /// section before, or the start of the section after.
    pub fn move_midweek_part(&mut self, monday: &str, part: u32, delta: i32) {
        let Some(w) = self.midweek_week_mut(monday) else { return };
        let Some((si, pi)) = w.sections.iter().enumerate().find_map(|(si, s)| s.parts.iter().position(|p| p.id == part).map(|pi| (si, pi))) else { return };
        let len = w.sections[si].parts.len();
        if delta < 0 && pi > 0 {
            w.sections[si].parts.swap(pi, pi - 1);
        } else if delta > 0 && pi + 1 < len {
            w.sections[si].parts.swap(pi, pi + 1);
        } else if delta < 0 && si > 0 {
            let p = w.sections[si].parts.remove(pi);
            w.sections[si - 1].parts.push(p);
        } else if delta > 0 && si + 1 < w.sections.len() {
            let p = w.sections[si].parts.remove(pi);
            w.sections[si + 1].parts.insert(0, p);
        }
    }

    pub fn set_midweek_header(&mut self, monday: &str, reading: &str, songs: [Option<u16>; 3]) {
        if let Some(w) = self.midweek_week_mut(monday) {
            w.reading = reading.trim().to_string();
            w.songs = songs;
        }
    }

    pub fn set_midweek_aux(&mut self, monday: &str, on: bool) {
        if let Some(w) = self.midweek_week_mut(monday) {
            w.aux_on = on;
        }
    }

    pub fn set_midweek_no_meeting(&mut self, monday: &str, no: bool) {
        if let Some(w) = self.midweek_week_mut(monday) {
            w.no_meeting = no;
        }
    }

    pub fn delete_midweek_week(&mut self, monday: &str) {
        self.midweek_weeks.retain(|w| w.monday != monday);
    }

    // ----- assigning -----

    pub fn assign_midweek(&mut self, monday: &str, role: MwRole, person: Option<u32>) {
        let Some(w) = self.midweek_week_mut(monday) else { return };
        let slot: Option<&mut Option<u32>> = match role {
            MwRole::Chairman => Some(&mut w.chairman),
            MwRole::AuxCounselor => Some(&mut w.aux_counselor),
            MwRole::OpeningPrayer => Some(&mut w.opening_prayer),
            MwRole::ClosingPrayer => Some(&mut w.closing_prayer),
            MwRole::Main(id) => w.part_mut(id).map(|p| &mut p.main),
            MwRole::Assistant(id) => w.part_mut(id).map(|p| &mut p.assistant),
            MwRole::AuxMain(id) => w.part_mut(id).map(|p| &mut p.aux),
            MwRole::AuxAssistant(id) => w.part_mut(id).map(|p| &mut p.aux_assistant),
        };
        if let Some(s) = slot {
            *s = person;
        }
    }

    /// When each person last had a meeting assignment, before `monday`: (any, per privilege).
    fn midweek_history(&self, monday: &str) -> BTreeMap<u32, (String, BTreeMap<Privilege, String>)> {
        let mut out: BTreeMap<u32, (String, BTreeMap<Privilege, String>)> = BTreeMap::new();
        for w in self.midweek_weeks.iter().filter(|w| w.monday.as_str() < monday && !w.no_meeting) {
            for (role, person) in w.places() {
                let (Some(p), Some(privilege)) = (person, role_privilege(w, role)) else { continue };
                let e = out.entry(p).or_insert_with(|| (String::new(), BTreeMap::new()));
                if e.0 < w.monday {
                    e.0 = w.monday.clone();
                }
                let last = e.1.entry(privilege).or_default();
                if *last < w.monday {
                    *last = w.monday.clone();
                }
            }
        }
        out
    }

    /// Who can take a place: people with the privilege (and, for an assistant, the same gender
    /// as the student or a family member). Sorted: available first, then not busy that week,
    /// then the longest without **this kind** of assignment (never first), then the longest
    /// without **any** assignment, then by name.
    pub fn midweek_candidates(&self, monday: &str, role: MwRole, weekday: i64) -> Vec<MwCandidate> {
        let Some(w) = self.midweek_week(monday) else { return Vec::new() };
        let privilege = role_privilege(w, role);
        let date = meeting_date(monday, weekday);
        let history = self.midweek_history(monday);
        let student = match role {
            MwRole::Assistant(id) => w.part(id).filter(|p| p.kind != PartKind::BibleStudy).and_then(|p| p.main),
            MwRole::AuxAssistant(id) => w.part(id).and_then(|p| p.aux),
            _ => None,
        };
        let student = student.and_then(|s| self.person(s));
        let mut v: Vec<MwCandidate> = self
            .people
            .iter()
            .filter(|p| p.moved_away.is_none())
            .filter(|p| match privilege {
                Some(x) => p.privileges.contains(&x),
                None => p.publisher_type.is_publisher(),
            })
            .filter(|p| match student {
                Some(s) => s.id != p.id && (s.gender == p.gender || self.family_of(s.id).is_some_and(|f| f.members.contains(&p.id))),
                None => true,
            })
            .map(|p| {
                let busy: Vec<String> = w.places().into_iter().filter(|(r, x)| *x == Some(p.id) && *r != role).map(|(r, _)| w.role_label(r)).collect();
                let h = history.get(&p.id);
                MwCandidate {
                    person: p.id,
                    fit: if self.is_away(p.id, &date) { MwFit::Away } else { MwFit::Available },
                    busy,
                    last_this: privilege.and_then(|x| h.and_then(|h| h.1.get(&x).cloned())),
                    last_any: h.map(|h| h.0.clone()),
                }
            })
            .collect();
        let name = |id: u32| self.person(id).map(|p| (p.last_name.to_lowercase(), p.first_name.to_lowercase())).unwrap_or_default();
        v.sort_by(|a, b| {
            (a.fit != MwFit::Available, !a.busy.is_empty(), a.last_this.clone().unwrap_or_default(), a.last_any.clone().unwrap_or_default(), name(a.person)).cmp(&(
                b.fit != MwFit::Available,
                !b.busy.is_empty(),
                b.last_this.clone().unwrap_or_default(),
                b.last_any.clone().unwrap_or_default(),
                name(b.person),
            ))
        });
        v
    }

    /// Fills the empty places of the weeks with the first available candidate who has nothing
    /// else that week. Returns (filled, still empty).
    pub fn autofill_midweek(&mut self, mondays: &[String], weekday: i64) -> (usize, usize) {
        let (mut filled, mut left) = (0, 0);
        let mut order: Vec<String> = mondays.to_vec();
        order.sort();
        for monday in order {
            let Some(w) = self.midweek_week(&monday) else { continue };
            if w.no_meeting {
                continue;
            }
            // Students first, so their assistants can match them.
            let mut places: Vec<MwRole> = w.places().into_iter().filter(|(_, p)| p.is_none()).map(|(r, _)| r).collect();
            places.sort_by_key(|r| matches!(r, MwRole::Assistant(_) | MwRole::AuxAssistant(_)));
            for role in places {
                let pick = self.midweek_candidates(&monday, role, weekday).into_iter().find(|c| c.fit == MwFit::Available && c.busy.is_empty());
                match pick {
                    Some(c) => {
                        self.assign_midweek(&monday, role, Some(c.person));
                        filled += 1;
                    }
                    None => left += 1,
                }
            }
        }
        (filled, left)
    }

    /// Empties every place of the weeks.
    pub fn clear_midweek(&mut self, mondays: &[String]) {
        for w in self.midweek_weeks.iter_mut().filter(|w| mondays.contains(&w.monday)) {
            for (role, _) in w.places() {
                match role {
                    MwRole::Chairman => w.chairman = None,
                    MwRole::AuxCounselor => w.aux_counselor = None,
                    MwRole::OpeningPrayer => w.opening_prayer = None,
                    MwRole::ClosingPrayer => w.closing_prayer = None,
                    _ => {}
                }
            }
            for p in w.sections.iter_mut().flat_map(|s| s.parts.iter_mut()) {
                p.main = None;
                p.assistant = None;
                p.aux = None;
                p.aux_assistant = None;
            }
        }
    }

    // ----- sending -----

    /// Weeks with changes that are not sent.
    pub fn unsent_midweek(&self, mondays: &[String], weekday: i64) -> Vec<String> {
        self.midweek_weeks
            .iter()
            .filter(|w| mondays.contains(&w.monday) && w.status(&meeting_date(&w.monday, weekday)) != crate::service_meetings::SendStatus::Sent)
            .map(|w| w.monday.clone())
            .collect()
    }

    /// Sends the weeks: everyone sees them as they are now, and only the people whose
    /// assignments are new, changed or gone get a notice.
    pub fn send_midweek(&mut self, mondays: &[String], weekday: i64) -> usize {
        use crate::assignments::{AssignmentKind, NoticeKind};
        let todo = self.unsent_midweek(mondays, weekday);
        for monday in &todo {
            let Some(w) = self.midweek_week(monday).cloned() else { continue };
            let date = meeting_date(monday, weekday);
            let now = w.roles();
            let before = w.sent.clone().map(|s| (s.date.clone(), s.roles.clone())).unwrap_or_default();
            let people: BTreeSet<u32> = now.iter().chain(before.1.iter()).map(|(p, _)| *p).collect();
            for p in people {
                let mine = |list: &[(u32, String)]| list.iter().filter(|(x, _)| *x == p).map(|(_, r)| r.clone()).collect::<Vec<_>>();
                let (was, is) = (mine(&before.1), mine(&now));
                let kind = match (was.is_empty(), is.is_empty()) {
                    (true, false) => Some(NoticeKind::New),
                    (false, true) => Some(NoticeKind::Cancelled),
                    (false, false) if was != is || before.0 != date => Some(NoticeKind::Changed),
                    _ => None,
                };
                if let Some(k) = kind {
                    let (d, roles) = if k == NoticeKind::Cancelled { (before.0.clone(), was) } else { (date.clone(), is) };
                    self.notify(p, k, AssignmentKind::MidweekMeeting, w.id, &d, None, format!("Midweek meeting · {}", roles.join(", ")));
                }
            }
            if let Some(x) = self.midweek_week_mut(monday) {
                let places = x.places().into_iter().filter_map(|(r, p)| p.map(|p| (role_key(r), p))).collect();
                x.sent = Some(MwSent { date, no_meeting: x.no_meeting, roles: now, places });
            }
        }
        todo.len()
    }

    // ----- statistics -----

    /// (person, assignments) in the weeks from `from` to `to` (Mondays), most first; people
    /// with a midweek privilege and none are included with 0.
    pub fn midweek_per_person(&self, from: &str, to: &str) -> Vec<(u32, usize)> {
        let mut count: BTreeMap<u32, usize> = BTreeMap::new();
        for p in self.people.iter().filter(|p| p.moved_away.is_none() && p.privileges.iter().any(|x| x.section() == crate::privileges::Section::Midweek)) {
            count.insert(p.id, 0);
        }
        for w in self.midweek_weeks.iter().filter(|w| w.monday.as_str() >= from && w.monday.as_str() <= to) {
            for (p, _) in w.roles() {
                *count.entry(p).or_default() += 1;
            }
        }
        let mut v: Vec<(u32, usize)> = count.into_iter().collect();
        v.sort_by_key(|x| std::cmp::Reverse(x.1));
        v
    }

    /// (kind, parts) given in the weeks, most first.
    pub fn midweek_per_kind(&self, from: &str, to: &str) -> Vec<(PartKind, usize)> {
        let mut count: BTreeMap<PartKind, usize> = BTreeMap::new();
        for w in self.midweek_weeks.iter().filter(|w| !w.no_meeting && w.monday.as_str() >= from && w.monday.as_str() <= to) {
            for p in w.parts() {
                *count.entry(p.kind).or_default() += 1;
            }
        }
        let mut v: Vec<(PartKind, usize)> = count.into_iter().collect();
        v.sort_by_key(|x| std::cmp::Reverse(x.1));
        v
    }

    /// The Monday of the last week each person had any assignment (before `before`).
    pub fn midweek_last_any(&self, before: &str) -> BTreeMap<u32, String> {
        self.midweek_history(before).into_iter().map(|(p, (any, _))| (p, any)).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workbook::{PartContent, WeekContent};

    fn content(monday: &str) -> WeekContent {
        let part = |number, section, kind, minutes| PartContent { number, section, kind, title: format!("Part {number}"), description: String::new(), minutes };
        WeekContent {
            doc: "d1".into(),
            monday: monday.into(),
            label: "WEEK".into(),
            reading: "BOOK 1".into(),
            songs: [Some(1), Some(2), Some(3)],
            parts: vec![
                part(1, SectionKind::Treasures, PartKind::TreasuresTalk, 10),
                part(2, SectionKind::Treasures, PartKind::SpiritualGems, 10),
                part(3, SectionKind::Treasures, PartKind::BibleReading, 4),
                part(4, SectionKind::Ministry, PartKind::StartingConversation, 3),
                part(5, SectionKind::Living, PartKind::Living, 15),
                part(6, SectionKind::Living, PartKind::BibleStudy, 30),
            ],
        }
    }

    fn setup() -> Directory {
        let mut d = crate::mock::demo_directory(36);
        d.midweek_weeks.clear();
        d.notices.clear();
        d.absences.clear();
        let src = WeekSource { issue: "202611".into(), lang: "E".into(), doc: "d1".into() };
        d.import_midweek_week(&content("2026-11-02"), Some(src.clone()));
        d.import_midweek_week(&content("2026-11-09"), Some(src));
        d
    }

    #[test]
    fn import_and_refresh_keep_assignments_and_edits() {
        let mut d = setup();
        let w = d.midweek_week("2026-11-02").cloned().unwrap_or_else(|| panic!("week"));
        assert_eq!(w.sections.len(), 3);
        assert_eq!(w.parts().count(), 6);
        let reading = w.parts().find(|p| p.kind == PartKind::BibleReading).map(|p| p.id).unwrap_or(0);
        d.assign_midweek("2026-11-02", MwRole::Main(reading), Some(3));
        let talk = w.parts().find(|p| p.kind == PartKind::TreasuresTalk).map(|p| p.id).unwrap_or(0);
        let _ = d.save_midweek_part("2026-11-02", 0, Some(talk), &PartDraft { kind: None, title: "My title".into(), description: String::new(), minutes: "12".into() });
        // The refresh brings 4 minutes for the reading and 10 for the talk; the edit stays.
        let mut c = content("2026-11-02");
        c.parts[2].minutes = 5;
        d.import_midweek_week(&c, Some(WeekSource { issue: "202611".into(), lang: "E".into(), doc: "d1".into() }));
        let w = d.midweek_week("2026-11-02").cloned().unwrap_or_else(|| panic!("week"));
        assert_eq!(w.part(reading).map(|p| (p.main, p.minutes)), Some((Some(3), 5)));
        assert_eq!(w.part(talk).map(|p| (p.minutes, p.title.clone())), Some((12, Some("My title".into()))));
    }

    #[test]
    fn candidates_have_the_privilege_and_least_recent_first() {
        let mut d = setup();
        let reading = |d: &Directory, m: &str| d.midweek_week(m).and_then(|w| w.parts().find(|p| p.kind == PartKind::BibleReading).map(|p| p.id)).unwrap_or(0);
        let r1 = reading(&d, "2026-11-02");
        let first = d.midweek_candidates("2026-11-02", MwRole::Main(r1), 3);
        assert!(!first.is_empty());
        assert!(first.iter().all(|c| d.person(c.person).is_some_and(|p| p.privileges.contains(&Privilege::BibleReading))));
        let who = first[0].person;
        d.assign_midweek("2026-11-02", MwRole::Main(r1), Some(who));
        // The next week, the one who just read is last among those who never read before.
        let r2 = reading(&d, "2026-11-09");
        let next = d.midweek_candidates("2026-11-09", MwRole::Main(r2), 3);
        assert_ne!(next[0].person, who);
        assert_eq!(next.iter().find(|c| c.person == who).and_then(|c| c.last_this.clone()).as_deref(), Some("2026-11-02"));
    }

    #[test]
    fn autofill_avoids_two_places_and_send_notifies() {
        let mut d = setup();
        let weeks = vec!["2026-11-02".to_string()];
        let (filled, _) = d.autofill_midweek(&weeks, 3);
        assert!(filled > 5);
        let w = d.midweek_week("2026-11-02").cloned().unwrap_or_else(|| panic!("week"));
        let people: Vec<u32> = w.roles().iter().map(|(p, _)| *p).collect();
        let unique: BTreeSet<u32> = people.iter().copied().collect();
        assert_eq!(people.len(), unique.len(), "nobody has two places");
        assert_eq!(d.send_midweek(&weeks, 3), 1);
        assert_eq!(d.notices.len(), unique.len(), "{:?} {:?}", d.notices.iter().map(|n| (n.person, n.kind)).collect::<Vec<_>>(), unique);
        assert!(d.unsent_midweek(&weeks, 3).is_empty());
        // Once seen, taking someone out sends a cancellation to them only.
        for n in d.notices.iter_mut() {
            n.seen = true;
        }
        let (p, _) = w.roles()[0].clone();
        d.assign_midweek("2026-11-02", MwRole::Chairman, None);
        let before = d.notices.len();
        d.send_midweek(&weeks, 3);
        assert_eq!(d.notices.len(), before + 1);
        assert!(d.notices.iter().any(|n| n.person == p && n.kind == crate::assignments::NoticeKind::Cancelled));
    }

    #[test]
    fn parts_and_sections_move() {
        let mut d = setup();
        let m = "2026-11-02";
        let ids = |d: &Directory| d.midweek_week(m).map(|w| w.parts().map(|p| p.kind).collect::<Vec<_>>()).unwrap_or_default();
        let reading = d.midweek_week(m).and_then(|w| w.parts().find(|p| p.kind == PartKind::BibleReading).map(|p| p.id)).unwrap_or(0);
        d.move_midweek_part(m, reading, -1);
        assert_eq!(ids(&d)[1], PartKind::BibleReading);
        // Down twice: past the end of Treasures into the start of the ministry section.
        d.move_midweek_part(m, reading, 1);
        d.move_midweek_part(m, reading, 1);
        let w = d.midweek_week(m).cloned().unwrap_or_else(|| panic!("week"));
        assert_eq!(w.sections[1].parts[0].kind, PartKind::BibleReading);
        let living = w.sections[2].id;
        d.move_midweek_section(m, living, -1);
        assert_eq!(d.midweek_week(m).map(|w| w.sections[1].kind), Some(SectionKind::Living));
    }

    #[test]
    fn timeline_follows_the_parts() {
        let d = setup();
        let w = d.midweek_week("2026-11-02").cloned().unwrap_or_else(|| panic!("week"));
        let t = w.timeline(minutes_of("19:00").unwrap_or(0));
        assert_eq!(hhmm(t.opening_comments), "19:05");
        // Treasures 10 + 10 + 4 (+1 counsel), then the ministry part 3 (+1).
        let first = w.parts().next().map(|p| p.id).unwrap_or(0);
        assert_eq!(t.part(first).map(hhmm).as_deref(), Some("19:06"));
        assert_eq!(t.middle_song.map(hhmm).as_deref(), Some("19:35"));
        // Middle song 4, Living 15 + 30, concluding 3, song 4, prayer 1.
        assert_eq!(hhmm(t.end), "20:32");
    }

    #[test]
    fn dates() {
        assert_eq!(monday_of("2026-11-27"), "2026-11-23");
        assert_eq!(meeting_date("2026-11-23", weekday_offset("Friday")), "2026-11-27");
    }
}
