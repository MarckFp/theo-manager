//! M-04 Weekend meeting: the chairman, the public talk (speaker, talk number, hospitality for a
//! visiting speaker), the Watchtower Study conductor and reader, and the prayers, week by week.
//! The week reads the events (M-12, decision E1): no meeting in a convention or assembly week, and
//! the circuit overseer gives the talk in the week of his visit.
//! Rules: `docs/model/m04-weekend-meeting.md`.

use std::collections::BTreeMap;

use crate::assignments::AssignmentKind;
use crate::events::EventKind;
use crate::midweek::meeting_date;
use crate::people::Directory;
use crate::prefs::Preferences;
use crate::privileges::Privilege;
use crate::scheduling::{self, Candidate, Fit, SentPlan};
use crate::service_meetings::SendStatus;
use crate::talks::Speaker;

/// Where the Watchtower Study article is (a reference only; the text stays in the device's
/// EPUB, R-033).
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct WatchtowerRef {
    /// The issue, `YYYYMM`.
    pub issue: String,
    /// The study article number in the issue.
    pub article: u8,
}

/// The plan of one weekend meeting (`weekend_week`).
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct WeekendWeek {
    pub id: u32,
    /// Monday of the week, `YYYY-MM-DD`. The meeting is on the congregation's weekend day.
    pub monday: String,
    /// Opening, middle and closing song.
    pub songs: [Option<u16>; 3],
    pub chairman: Option<u32>,
    pub opening_prayer: Option<u32>,
    pub closing_prayer: Option<u32>,
    pub speaker: Option<Speaker>,
    /// The talk number (no titles are stored).
    pub talk: Option<u16>,
    /// Who gives hospitality to a visiting speaker.
    pub host: Option<u32>,
    pub watchtower: Option<WatchtowerRef>,
    pub wt_conductor: Option<u32>,
    pub wt_reader: Option<u32>,
    /// The circuit overseer's visit: he gives the public talk and a service talk at the end.
    pub co_visit: bool,
    /// No meeting this week (for example a convention).
    pub no_meeting: bool,
    pub note: Option<String>,
    pub sent: Option<SentPlan>,
}

/// A place to fill.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum WeRole {
    Chairman,
    OpeningPrayer,
    /// A local speaker (a visiting speaker is not a person of this congregation).
    Speaker,
    /// Hospitality for a visiting speaker.
    Host,
    WtConductor,
    WtReader,
    ClosingPrayer,
}

impl WeRole {
    pub const ALL: [WeRole; 7] = [WeRole::Chairman, WeRole::OpeningPrayer, WeRole::Speaker, WeRole::Host, WeRole::WtConductor, WeRole::WtReader, WeRole::ClosingPrayer];

    pub fn label(self) -> &'static str {
        match self {
            WeRole::Chairman => "Chairman",
            WeRole::OpeningPrayer => "Opening prayer",
            WeRole::Speaker => "Public talk",
            WeRole::Host => "Hospitality",
            WeRole::WtConductor => "Watchtower Study conductor",
            WeRole::WtReader => "Watchtower reader",
            WeRole::ClosingPrayer => "Closing prayer",
        }
    }

    /// A stable name of the place (sent plans keep it).
    pub fn key(self) -> &'static str {
        match self {
            WeRole::Chairman => "chairman",
            WeRole::OpeningPrayer => "opening-prayer",
            WeRole::Speaker => "speaker",
            WeRole::Host => "host",
            WeRole::WtConductor => "wt-conductor",
            WeRole::WtReader => "wt-reader",
            WeRole::ClosingPrayer => "closing-prayer",
        }
    }

    /// The privilege a person needs for the place.
    pub fn privilege(self) -> Privilege {
        match self {
            WeRole::Chairman => Privilege::WeekendChairman,
            WeRole::OpeningPrayer | WeRole::ClosingPrayer => Privilege::Prayer,
            WeRole::Speaker => Privilege::PublicTalk,
            WeRole::Host => Privilege::Hospitality,
            WeRole::WtConductor => Privilege::WatchtowerConductor,
            WeRole::WtReader => Privilege::WatchtowerReader,
        }
    }
}

/// Settings of the weekend meeting.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct WeekendSettings {
    /// The chairman also says the opening prayer (new weeks leave that place out).
    pub chairman_prays: bool,
    /// The speaker also says the closing prayer.
    pub speaker_prays: bool,
}

impl Default for WeekendSettings {
    fn default() -> Self {
        WeekendSettings { chairman_prays: false, speaker_prays: true }
    }
}

/// The form of the public talk of a week.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct TalkDraft {
    pub speaker: Option<Speaker>,
    pub talk: Option<u16>,
    pub host: Option<u32>,
}

impl WeekendWeek {
    /// The places of the week with their person, in meeting order. Only the places that apply:
    /// the speaker only when local or empty (and not the circuit overseer), hospitality only for
    /// a visiting speaker, prayers only when not said by the chairman or the speaker.
    pub fn places_with(&self, settings: &WeekendSettings) -> Vec<(WeRole, Option<u32>)> {
        if self.no_meeting {
            return Vec::new();
        }
        let mut v = vec![(WeRole::Chairman, self.chairman)];
        if !settings.chairman_prays {
            v.push((WeRole::OpeningPrayer, self.opening_prayer));
        }
        if !self.co_visit {
            match self.speaker {
                Some(Speaker::Visiting(_)) => v.push((WeRole::Host, self.host)),
                Some(Speaker::Local(p)) => v.push((WeRole::Speaker, Some(p))),
                None => v.push((WeRole::Speaker, None)),
            }
        }
        v.push((WeRole::WtConductor, self.wt_conductor));
        v.push((WeRole::WtReader, self.wt_reader));
        let speaker_prays = settings.speaker_prays && !self.co_visit && matches!(self.speaker, Some(Speaker::Local(_)));
        if !speaker_prays {
            v.push((WeRole::ClosingPrayer, self.closing_prayer));
        }
        v
    }

    /// The places with the default settings.
    pub fn places(&self) -> Vec<(WeRole, Option<u32>)> {
        self.places_with(&WeekendSettings::default())
    }

    /// The person field of a place. None for the speaker, which is a [`Speaker`].
    fn slot(&mut self, role: WeRole) -> Option<&mut Option<u32>> {
        match role {
            WeRole::Chairman => Some(&mut self.chairman),
            WeRole::OpeningPrayer => Some(&mut self.opening_prayer),
            WeRole::Host => Some(&mut self.host),
            WeRole::WtConductor => Some(&mut self.wt_conductor),
            WeRole::WtReader => Some(&mut self.wt_reader),
            WeRole::ClosingPrayer => Some(&mut self.closing_prayer),
            WeRole::Speaker => None,
        }
    }

    /// (person, what they do) of the week.
    pub fn roles_with(&self, settings: &WeekendSettings) -> Vec<(u32, String)> {
        let speaker_prays = settings.speaker_prays && matches!(self.speaker, Some(Speaker::Local(_)));
        self.places_with(settings)
            .into_iter()
            .filter_map(|(r, p)| {
                let label = match r {
                    WeRole::Speaker if speaker_prays => format!("{} and closing prayer", self.talk.map(|n| format!("Public talk {n}")).unwrap_or_else(|| "Public talk".into())),
                    WeRole::Speaker => self.talk.map(|n| format!("Public talk {n}")).unwrap_or_else(|| "Public talk".into()),
                    WeRole::Chairman if settings.chairman_prays => "Chairman and opening prayer".into(),
                    _ => r.label().to_string(),
                };
                p.map(|p| (p, label))
            })
            .collect()
    }

    /// (filled, places).
    pub fn filled(&self, settings: &WeekendSettings) -> (usize, usize) {
        let p = self.places_with(settings);
        let visiting = usize::from(matches!(self.speaker, Some(Speaker::Visiting(_))) && !self.co_visit);
        (p.iter().filter(|(_, x)| x.is_some()).count() + visiting, p.len() + visiting)
    }
}

impl Directory {
    pub fn weekend_week(&self, monday: &str) -> Option<&WeekendWeek> {
        self.weekend_weeks.iter().find(|w| w.monday == monday)
    }

    fn weekend_week_mut(&mut self, monday: &str) -> Option<&mut WeekendWeek> {
        self.weekend_weeks.iter_mut().find(|w| w.monday == monday)
    }

    /// The events of the congregation that touch the weekend meeting of a week: a convention or
    /// assembly on the meeting day, or the circuit overseer's visit that week.
    pub fn weekend_events(&self, monday: &str, weekday: i64) -> (bool, bool) {
        let date = meeting_date(monday, weekday);
        let sunday = crate::events::add_days(monday, 6).unwrap_or_default();
        let public = self.events.iter().filter(|e| e.people.is_none());
        let mut no_meeting = false;
        let mut co = false;
        for e in public {
            match e.kind {
                EventKind::RegionalConvention | EventKind::CircuitAssembly if e.from <= date && date <= e.to => no_meeting = true,
                EventKind::CircuitOverseerVisit if e.from.as_str() <= sunday.as_str() && monday <= e.to.as_str() => co = true,
                _ => {}
            }
        }
        (no_meeting, co)
    }

    /// Adds the plan of a week (empty), following the events of that week. Returns its id.
    pub fn add_weekend_week(&mut self, monday: &str, weekday: i64) -> u32 {
        if let Some(w) = self.weekend_week(monday) {
            return w.id;
        }
        let (no_meeting, co_visit) = self.weekend_events(monday, weekday);
        let id = self.weekend_weeks.iter().map(|w| w.id).max().unwrap_or(0) + 1;
        self.weekend_weeks.push(WeekendWeek { id, monday: monday.into(), no_meeting, co_visit, ..Default::default() });
        self.weekend_weeks.sort_by(|a, b| a.monday.cmp(&b.monday));
        id
    }

    pub fn delete_weekend_week(&mut self, monday: &str) {
        self.weekend_weeks.retain(|w| w.monday != monday);
    }

    pub fn set_weekend_flags(&mut self, monday: &str, no_meeting: bool, co_visit: bool) {
        if let Some(w) = self.weekend_week_mut(monday) {
            w.no_meeting = no_meeting;
            w.co_visit = co_visit;
            if co_visit {
                w.speaker = None;
                w.talk = None;
                w.host = None;
            }
        }
    }

    pub fn set_weekend_header(&mut self, monday: &str, songs: [Option<u16>; 3], watchtower: Option<WatchtowerRef>, note: &str) {
        if let Some(w) = self.weekend_week_mut(monday) {
            w.songs = songs;
            w.watchtower = watchtower;
            w.note = Some(note.trim().to_string()).filter(|n| !n.is_empty());
        }
    }

    /// Sets the public talk of a week. A local speaker needs the public talk privilege; a visiting
    /// speaker must exist. Hospitality is only for a visiting speaker.
    pub fn set_weekend_talk(&mut self, monday: &str, d: &TalkDraft) -> Result<(), String> {
        match d.speaker {
            Some(Speaker::Local(p)) if !self.local_speakers().contains(&p) => return Err("This brother does not give public talks.".into()),
            Some(Speaker::Visiting(v)) if self.visiting_speaker(v).is_none() => return Err("Choose a visiting speaker from the list.".into()),
            _ => {}
        }
        if let Some(n) = d.talk.filter(|n| !(1..=crate::talks::MAX_TALK_NUMBER).contains(n)) {
            return Err(format!("{n} is not a talk number."));
        }
        let host = d.host.filter(|_| matches!(d.speaker, Some(Speaker::Visiting(_))));
        let w = self.weekend_week_mut(monday).ok_or("Add the week first.")?;
        if w.co_visit {
            return Err("The circuit overseer gives the talk this week.".into());
        }
        w.speaker = d.speaker;
        w.talk = d.talk;
        w.host = host;
        Ok(())
    }

    /// Fills or empties a place. For the speaker, use [`Directory::set_weekend_talk`].
    pub fn assign_weekend(&mut self, monday: &str, role: WeRole, person: Option<u32>) {
        if role == WeRole::Speaker {
            let talk = self.weekend_week(monday).and_then(|w| w.talk);
            let _ = self.set_weekend_talk(monday, &TalkDraft { speaker: person.map(Speaker::Local), talk, host: None });
            return;
        }
        if let Some(slot) = self.weekend_week_mut(monday).and_then(|w| w.slot(role)) {
            *slot = person;
        }
    }

    /// When each person last had each weekend place, before `monday`: (any, per role).
    fn weekend_history(&self, monday: &str) -> BTreeMap<u32, (String, BTreeMap<WeRole, String>)> {
        let mut out: BTreeMap<u32, (String, BTreeMap<WeRole, String>)> = BTreeMap::new();
        for w in self.weekend_weeks.iter().filter(|w| w.monday.as_str() < monday) {
            for (role, person) in w.places_with(&self.weekend_settings) {
                let Some(p) = person else { continue };
                let e = out.entry(p).or_default();
                if e.0 < w.monday {
                    e.0 = w.monday.clone();
                }
                let last = e.1.entry(role).or_default();
                if *last < w.monday {
                    *last = w.monday.clone();
                }
            }
        }
        out
    }

    /// What a person does that week outside this place: the weekend meeting, duties at the
    /// weekend meeting, and an outgoing talk on the meeting day.
    fn weekend_busy(&self, w: &WeekendWeek, role: WeRole, person: u32, date: &str) -> Vec<String> {
        let mut busy: Vec<String> = w.places_with(&self.weekend_settings).into_iter().filter(|(r, p)| *p == Some(person) && *r != role).map(|(r, _)| r.label().to_string()).collect();
        busy.extend(self.duty_roles_of(&w.monday, crate::duties::Meeting::Weekend, person));
        if self.outgoing_talks.iter().any(|t| t.speaker == person && t.date == date) {
            busy.push("Outgoing talk".into());
        }
        busy
    }

    /// Who can take a place, sorted (see [`scheduling::sort`]). A reader must be a brother other
    /// than the conductor.
    pub fn weekend_candidates(&self, monday: &str, role: WeRole, weekday: i64) -> Vec<Candidate> {
        let Some(w) = self.weekend_week(monday) else { return Vec::new() };
        let date = meeting_date(monday, weekday);
        let history = self.weekend_history(monday);
        let privilege = role.privilege();
        let mut v: Vec<Candidate> = self
            .people
            .iter()
            .filter(|p| p.moved_away.is_none() && p.privileges.contains(&privilege))
            .map(|p| {
                let h = history.get(&p.id);
                Candidate {
                    person: p.id,
                    fit: if self.is_away(p.id, &date) { Fit::Away } else { Fit::Available },
                    busy: self.weekend_busy(w, role, p.id, &date),
                    last_this: h.and_then(|h| h.1.get(&role).cloned()),
                    last_any: h.map(|h| h.0.clone()),
                }
            })
            .collect();
        scheduling::sort(self, &mut v);
        v
    }

    /// Fills the empty places (not the speaker: talks are arranged with the speakers) with the
    /// first free candidate. Returns (filled, still empty).
    pub fn autofill_weekend(&mut self, mondays: &[String], weekday: i64) -> (usize, usize) {
        let (mut filled, mut left) = (0, 0);
        let mut order = mondays.to_vec();
        order.sort();
        for monday in order {
            let Some(w) = self.weekend_week(&monday) else { continue };
            let empty: Vec<WeRole> = w.places_with(&self.weekend_settings).into_iter().filter(|(r, p)| p.is_none() && *r != WeRole::Speaker).map(|(r, _)| r).collect();
            for role in empty {
                match self.weekend_candidates(&monday, role, weekday).into_iter().find(Candidate::is_free) {
                    Some(c) => {
                        self.assign_weekend(&monday, role, Some(c.person));
                        filled += 1;
                    }
                    None => left += 1,
                }
            }
        }
        (filled, left)
    }

    pub fn weekend_status(&self, w: &WeekendWeek, weekday: i64) -> SendStatus {
        scheduling::status(w.sent.as_ref(), &w.roles_with(&self.weekend_settings), w.no_meeting, &meeting_date(&w.monday, weekday))
    }

    /// Weeks with changes that are not sent.
    pub fn unsent_weekend(&self, mondays: &[String], weekday: i64) -> Vec<String> {
        self.weekend_weeks.iter().filter(|w| mondays.contains(&w.monday) && self.weekend_status(w, weekday) != SendStatus::Sent).map(|w| w.monday.clone()).collect()
    }

    /// Sends the weeks; only the people whose assignments changed get a notice.
    pub fn send_weekend(&mut self, mondays: &[String], weekday: i64) -> usize {
        let todo = self.unsent_weekend(mondays, weekday);
        for monday in &todo {
            let Some(w) = self.weekend_week(monday).cloned() else { continue };
            let roles = w.roles_with(&self.weekend_settings);
            let places = w.places_with(&self.weekend_settings).into_iter().filter_map(|(r, p)| p.map(|p| (r.key().to_string(), p))).collect();
            let date = meeting_date(monday, weekday);
            let sent = self.send_plan(AssignmentKind::WeekendMeeting, w.id, w.sent.as_ref(), &date, w.no_meeting, roles, places, "Weekend meeting");
            if let Some(x) = self.weekend_week_mut(monday) {
                x.sent = Some(sent);
            }
        }
        todo.len()
    }

    /// One line about the public talk of a week ("Talk 12 · Sam Visitor (Lakeside)").
    pub fn weekend_talk_line(&self, w: &WeekendWeek, prefs: &Preferences) -> String {
        if w.no_meeting {
            return "No meeting".into();
        }
        if w.co_visit {
            return "Circuit overseer".into();
        }
        let talk = w.talk.map(|n| format!("Talk {n}")).unwrap_or_else(|| "Talk not chosen".into());
        match w.speaker {
            Some(s) => format!("{talk} · {}", self.speaker_name(s, prefs)),
            None => format!("{talk} · no speaker yet"),
        }
    }

    /// (person, assignments) in the weeks from `from` to `to` (Mondays), most first; people with a
    /// weekend privilege and none are included with 0.
    pub fn weekend_per_person(&self, from: &str, to: &str) -> Vec<(u32, usize)> {
        let mut count: BTreeMap<u32, usize> = BTreeMap::new();
        for p in self.people.iter().filter(|p| p.moved_away.is_none() && WeRole::ALL.iter().any(|r| p.privileges.contains(&r.privilege()))) {
            count.insert(p.id, 0);
        }
        for w in self.weekend_weeks.iter().filter(|w| w.monday.as_str() >= from && w.monday.as_str() <= to) {
            for (p, _) in w.roles_with(&self.weekend_settings) {
                *count.entry(p).or_default() += 1;
            }
        }
        let mut v: Vec<(u32, usize)> = count.into_iter().collect();
        v.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
        v
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::assignments::NoticeKind;
    use crate::talks::{OtherCongregationDraft, VisitingSpeakerDraft};

    const SUNDAY: i64 = 6;

    fn setup() -> Directory {
        let mut d = crate::demo::demo_directory(30);
        d.events.clear();
        d.weekend_weeks.clear();
        d.other_congregations.clear();
        d.visiting_speakers.clear();
        d.outgoing_talks.clear();
        d.duty_meetings.clear();
        d.notices.clear();
        d
    }

    fn with(d: &Directory, privilege: Privilege) -> Vec<u32> {
        d.people.iter().filter(|p| p.moved_away.is_none() && p.privileges.contains(&privilege)).map(|p| p.id).collect()
    }

    #[test]
    fn places_follow_the_speaker_and_the_settings() {
        let mut d = setup();
        d.add_weekend_week("2026-12-07", SUNDAY);
        let local = with(&d, Privilege::PublicTalk)[0];
        assert!(d.set_weekend_talk("2026-12-07", &TalkDraft { speaker: Some(Speaker::Local(local)), talk: Some(12), host: Some(3) }).is_ok());
        let w = d.weekend_week("2026-12-07").cloned().unwrap_or_default();
        // Local speaker: no hospitality, and he says the closing prayer.
        let roles: Vec<WeRole> = w.places().iter().map(|(r, _)| *r).collect();
        assert_eq!(roles, [WeRole::Chairman, WeRole::OpeningPrayer, WeRole::Speaker, WeRole::WtConductor, WeRole::WtReader]);
        assert_eq!(w.host, None);
        assert!(w.roles_with(&d.weekend_settings).contains(&(local, "Public talk 12 and closing prayer".into())));
        // A visiting speaker brings hospitality and a closing prayer place.
        let lake = d.save_other_congregation(None, &OtherCongregationDraft { name: "Lakeside".into(), ..Default::default() }).unwrap_or(0);
        let v = d.save_visiting_speaker(None, &VisitingSpeakerDraft { first_name: "Sam".into(), last_name: "Visitor".into(), congregation: Some(lake), ..Default::default() }).unwrap_or(0);
        assert!(d.set_weekend_talk("2026-12-07", &TalkDraft { speaker: Some(Speaker::Visiting(v)), talk: Some(40), host: None }).is_ok());
        let roles: Vec<WeRole> = d.weekend_week("2026-12-07").map(|w| w.places().iter().map(|(r, _)| *r).collect()).unwrap_or_default();
        assert!(roles.contains(&WeRole::Host) && roles.contains(&WeRole::ClosingPrayer) && !roles.contains(&WeRole::Speaker));
        // Someone without the privilege cannot speak.
        let sister = d.people.iter().find(|p| p.gender == crate::people::Gender::Female).map(|p| p.id).unwrap_or(0);
        assert!(d.set_weekend_talk("2026-12-07", &TalkDraft { speaker: Some(Speaker::Local(sister)), talk: None, host: None }).is_err());
    }

    #[test]
    fn events_decide_the_week() {
        let mut d = setup();
        d.events.push(crate::events::Event { id: 1, kind: EventKind::CircuitAssembly, title: "Assembly".into(), description: None, from: "2026-12-13".into(), to: "2026-12-13".into(), people: None, absence: false });
        d.events.push(crate::events::Event { id: 2, kind: EventKind::CircuitOverseerVisit, title: "Visit".into(), description: None, from: "2026-12-15".into(), to: "2026-12-20".into(), people: None, absence: false });
        d.add_weekend_week("2026-12-07", SUNDAY);
        d.add_weekend_week("2026-12-14", SUNDAY);
        assert!(d.weekend_week("2026-12-07").is_some_and(|w| w.no_meeting && w.places().is_empty()));
        assert!(d.weekend_week("2026-12-14").is_some_and(|w| w.co_visit && !w.places().iter().any(|(r, _)| *r == WeRole::Speaker)));
        assert!(d.set_weekend_talk("2026-12-14", &TalkDraft { speaker: None, talk: Some(1), host: None }).is_err());
    }

    #[test]
    fn autofill_sends_and_notifies() {
        let mut d = setup();
        let weeks: Vec<String> = ["2026-12-07", "2026-12-14"].iter().map(|s| s.to_string()).collect();
        for w in &weeks {
            d.add_weekend_week(w, SUNDAY);
        }
        let (filled, _) = d.autofill_weekend(&weeks, SUNDAY);
        assert!(filled >= 4);
        // Nobody has two places in one week, and the speaker place is left for the coordinator.
        for w in &weeks {
            let wk = d.weekend_week(w).cloned().unwrap_or_default();
            let people: Vec<u32> = wk.places().iter().filter_map(|(_, p)| *p).collect();
            let unique: std::collections::BTreeSet<u32> = people.iter().copied().collect();
            assert_eq!(people.len(), unique.len());
            assert!(wk.speaker.is_none());
        }
        // The second week does not repeat the first chairman when others can do it.
        if with(&d, Privilege::WeekendChairman).len() > 1 {
            assert_ne!(d.weekend_week(&weeks[0]).and_then(|w| w.chairman), d.weekend_week(&weeks[1]).and_then(|w| w.chairman));
        }
        assert_eq!(d.send_weekend(&weeks, SUNDAY), 2);
        let chairman = d.weekend_week(&weeks[0]).and_then(|w| w.chairman).unwrap_or(0);
        assert!(d.my_notices(chairman).iter().any(|n| n.kind == NoticeKind::New && n.what == AssignmentKind::WeekendMeeting));
        assert!(d.unsent_weekend(&weeks, SUNDAY).is_empty());
        assert!(d.all_my_assignments(chairman).iter().any(|a| a.kind == AssignmentKind::WeekendMeeting && a.date == "2026-12-13"));
        // The talk history sees talks of planned weeks.
        let local = with(&d, Privilege::PublicTalk)[0];
        let _ = d.set_weekend_talk(&weeks[0], &TalkDraft { speaker: Some(Speaker::Local(local)), talk: Some(12), host: None });
        assert_eq!(d.talk_history(12).len(), 1);
        assert!(!d.talk_warnings(&weeks[1], None, 12).is_empty());
        assert!(d.talk_warnings("2028-01-03", None, 12).is_empty());
    }
}
