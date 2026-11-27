//! M-06 Public talks: the talks (by **number only**, no titles: the outlines are not ours to
//! store), the directory of other congregations and their coordinators, visiting speakers, the
//! talks each local speaker gives, and **outgoing** talks (local speakers in other congregations).
//! The incoming schedule is the public talk of each weekend meeting (`weekend.rs`).
//! Rules: `docs/model/m06-public-talks.md`.

use std::collections::BTreeSet;

use crate::assignments::AssignmentKind;
use crate::people::{fold, Appointment, Directory};
use crate::prefs::Preferences;
use crate::privileges::Privilege;
use crate::scheduling::SentPlan;
use crate::service_meetings::SendStatus;

/// The highest talk number accepted.
pub const MAX_TALK_NUMBER: u16 = 999;

/// Who gives a public talk.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize)]
pub enum Speaker {
    /// A brother of this congregation (a person of All users).
    Local(u32),
    /// A visiting speaker of another congregation ([`VisitingSpeaker`]).
    Visiting(u32),
}

/// Settings of public talks.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct TalkSettings {
    /// A talk given in the congregation less than this many months ago gets a warning.
    pub repeat_months: u8,
    /// A local speaker with more outgoing talks in one month gets a warning.
    pub max_outgoing_per_month: u8,
    /// Talk numbers not to schedule (for example withdrawn outlines).
    pub not_in_use: BTreeSet<u16>,
}

impl Default for TalkSettings {
    fn default() -> Self {
        TalkSettings { repeat_months: 6, max_outgoing_per_month: 1, not_in_use: BTreeSet::new() }
    }
}

/// Another congregation (`other_congregation`): where visiting speakers come from and where
/// local speakers go.
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct OtherCongregation {
    pub id: u32,
    pub name: String,
    pub number: Option<String>,
    /// The Kingdom Hall address.
    pub address: Option<String>,
    /// When their weekend meeting is, as text ("Sunday 10:00").
    pub meeting_time: Option<String>,
    /// The public talk coordinator.
    pub coordinator_name: Option<String>,
    pub coordinator_phone: Option<String>,
    pub coordinator_email: Option<String>,
    pub notes: Option<String>,
}

/// A speaker of another congregation (`visiting_speaker`).
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct VisitingSpeaker {
    pub id: u32,
    pub first_name: String,
    pub last_name: String,
    pub congregation: u32,
    pub appointment: Option<Appointment>,
    pub phone: Option<String>,
    pub email: Option<String>,
    /// The talk numbers they give.
    pub talks: BTreeSet<u16>,
    pub notes: Option<String>,
}

/// A local speaker's talk in another congregation (`outgoing_talk`).
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct OutgoingTalk {
    pub id: u32,
    /// `YYYY-MM-DD`.
    pub date: String,
    /// `HH:MM`, when known.
    pub time: Option<String>,
    pub speaker: u32,
    pub congregation: u32,
    pub talk: Option<u16>,
    pub notes: Option<String>,
    pub sent: Option<SentPlan>,
}

/// The other-congregation form. Text as typed.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct OtherCongregationDraft {
    pub name: String,
    pub number: String,
    pub address: String,
    pub meeting_time: String,
    pub coordinator_name: String,
    pub coordinator_phone: String,
    pub coordinator_email: String,
    pub notes: String,
}

/// The visiting-speaker form.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct VisitingSpeakerDraft {
    pub first_name: String,
    pub last_name: String,
    pub congregation: Option<u32>,
    pub appointment: Option<Appointment>,
    pub phone: String,
    pub email: String,
    pub talks: BTreeSet<u16>,
    pub notes: String,
}

/// The outgoing-talk form.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct OutgoingDraft {
    pub date: String,
    pub time: String,
    pub speaker: Option<u32>,
    pub congregation: Option<u32>,
    pub talk: Option<u16>,
    pub notes: String,
}

fn opt(s: &str) -> Option<String> {
    Some(s.trim().to_string()).filter(|s| !s.is_empty())
}

/// Reads talk numbers typed as "1, 5, 12-14". Wrong items are an error that names them.
pub fn parse_talks(s: &str) -> Result<BTreeSet<u16>, String> {
    let mut out = BTreeSet::new();
    let mut wrong = Vec::new();
    for item in s.split([',', ';', ' ']).map(str::trim).filter(|x| !x.is_empty()) {
        let range = match item.split_once('-') {
            Some((a, b)) => a.trim().parse::<u16>().ok().zip(b.trim().parse::<u16>().ok()),
            None => item.parse::<u16>().ok().map(|n| (n, n)),
        };
        match range {
            Some((a, b)) if (1..=MAX_TALK_NUMBER).contains(&a) && (a..=MAX_TALK_NUMBER).contains(&b) => out.extend(a..=b),
            _ => wrong.push(item.to_string()),
        }
    }
    if wrong.is_empty() { Ok(out) } else { Err(format!("These are not talk numbers: {}.", wrong.join(", "))) }
}

/// Talk numbers as text, with runs joined ("1-3, 7").
pub fn talks_text(talks: &BTreeSet<u16>) -> String {
    let mut parts: Vec<String> = Vec::new();
    let mut it = talks.iter().copied().peekable();
    while let Some(a) = it.next() {
        let mut b = a;
        while it.peek() == Some(&(b + 1)) {
            b += 1;
            it.next();
        }
        parts.push(if a == b { a.to_string() } else { format!("{a}-{b}") });
    }
    parts.join(", ")
}

impl Directory {
    // ----- other congregations -----

    pub fn other_congregation(&self, id: u32) -> Option<&OtherCongregation> {
        self.other_congregations.iter().find(|c| c.id == id)
    }

    /// Adds (`id` None) or changes another congregation. Names are unique.
    pub fn save_other_congregation(&mut self, id: Option<u32>, d: &OtherCongregationDraft) -> Result<u32, String> {
        let name = d.name.trim();
        if name.is_empty() {
            return Err("Enter the name of the congregation.".into());
        }
        if self.other_congregations.iter().any(|c| Some(c.id) != id && fold(&c.name) == fold(name)) {
            return Err("There is already a congregation with this name.".into());
        }
        if let Some(e) = opt(&d.coordinator_email).filter(|e| !e.contains('@')) {
            return Err(format!("\"{e}\" is not an e-mail address."));
        }
        let new_id = id.unwrap_or_else(|| self.other_congregations.iter().map(|c| c.id).max().unwrap_or(0) + 1);
        let c = OtherCongregation {
            id: new_id,
            name: name.into(),
            number: opt(&d.number),
            address: opt(&d.address),
            meeting_time: opt(&d.meeting_time),
            coordinator_name: opt(&d.coordinator_name),
            coordinator_phone: opt(&d.coordinator_phone),
            coordinator_email: opt(&d.coordinator_email),
            notes: opt(&d.notes),
        };
        match self.other_congregations.iter_mut().find(|x| x.id == new_id) {
            Some(slot) => *slot = c,
            None => self.other_congregations.push(c),
        }
        Ok(new_id)
    }

    /// Deletes another congregation. Refused while speakers or outgoing talks use it.
    pub fn delete_other_congregation(&mut self, id: u32) -> Result<(), String> {
        if self.visiting_speakers.iter().any(|s| s.congregation == id) {
            return Err("Move or delete the speakers of this congregation first.".into());
        }
        if self.outgoing_talks.iter().any(|t| t.congregation == id) {
            return Err("This congregation has outgoing talks. Delete them first.".into());
        }
        self.other_congregations.retain(|c| c.id != id);
        Ok(())
    }

    // ----- speakers -----

    pub fn visiting_speaker(&self, id: u32) -> Option<&VisitingSpeaker> {
        self.visiting_speakers.iter().find(|s| s.id == id)
    }

    /// Adds or changes a visiting speaker.
    pub fn save_visiting_speaker(&mut self, id: Option<u32>, d: &VisitingSpeakerDraft) -> Result<u32, String> {
        let (first, last) = (d.first_name.trim(), d.last_name.trim());
        if first.is_empty() || last.is_empty() {
            return Err("Enter the first and last name of the speaker.".into());
        }
        let congregation = d.congregation.filter(|c| self.other_congregation(*c).is_some()).ok_or("Choose the speaker's congregation.")?;
        if let Some(n) = d.talks.iter().find(|n| !(1..=MAX_TALK_NUMBER).contains(*n)) {
            return Err(format!("{n} is not a talk number."));
        }
        let new_id = id.unwrap_or_else(|| self.visiting_speakers.iter().map(|s| s.id).max().unwrap_or(0) + 1);
        let s = VisitingSpeaker {
            id: new_id,
            first_name: first.into(),
            last_name: last.into(),
            congregation,
            appointment: d.appointment,
            phone: opt(&d.phone),
            email: opt(&d.email),
            talks: d.talks.clone(),
            notes: opt(&d.notes),
        };
        match self.visiting_speakers.iter_mut().find(|x| x.id == new_id) {
            Some(slot) => *slot = s,
            None => self.visiting_speakers.push(s),
        }
        Ok(new_id)
    }

    /// Deletes a visiting speaker. Weekends that are not sent yet lose them as speaker; sent
    /// weekends keep them (what was sent stays as it was).
    pub fn delete_visiting_speaker(&mut self, id: u32) {
        self.visiting_speakers.retain(|s| s.id != id);
        for w in self.weekend_weeks.iter_mut().filter(|w| w.speaker == Some(Speaker::Visiting(id)) && w.sent.is_none()) {
            w.speaker = None;
            w.host = None;
        }
    }

    /// Local speakers: brothers with the public talk privilege who did not move away.
    pub fn local_speakers(&self) -> Vec<u32> {
        self.people.iter().filter(|p| p.moved_away.is_none() && p.privileges.contains(&Privilege::PublicTalk)).map(|p| p.id).collect()
    }

    /// Sets the talks a local speaker gives.
    pub fn set_speaker_talks(&mut self, person: u32, talks: BTreeSet<u16>) -> Result<(), String> {
        if !self.local_speakers().contains(&person) {
            return Err("Only brothers with the public talk privilege give public talks.".into());
        }
        if let Some(n) = talks.iter().find(|n| !(1..=MAX_TALK_NUMBER).contains(*n)) {
            return Err(format!("{n} is not a talk number."));
        }
        if talks.is_empty() {
            self.speaker_talks.remove(&person);
        } else {
            self.speaker_talks.insert(person, talks);
        }
        Ok(())
    }

    /// The talks a speaker gives.
    pub fn talks_of(&self, speaker: Speaker) -> BTreeSet<u16> {
        match speaker {
            Speaker::Local(p) => self.speaker_talks.get(&p).cloned().unwrap_or_default(),
            Speaker::Visiting(v) => self.visiting_speaker(v).map(|s| s.talks.clone()).unwrap_or_default(),
        }
    }

    /// "Peter Novak" or "Ann Example (Lakeside)".
    pub fn speaker_name(&self, speaker: Speaker, prefs: &Preferences) -> String {
        match speaker {
            Speaker::Local(p) => self.person(p).map(|p| p.display_name(prefs)).unwrap_or_else(|| "Unknown person".into()),
            Speaker::Visiting(v) => match self.visiting_speaker(v) {
                Some(s) => {
                    let cong = self.other_congregation(s.congregation).map(|c| format!(" ({})", c.name)).unwrap_or_default();
                    format!("{} {}{cong}", s.first_name, s.last_name)
                }
                None => "Unknown speaker".into(),
            },
        }
    }

    // ----- talk history -----

    /// When a talk was given in this congregation: (Monday of the week, speaker), newest first.
    pub fn talk_history(&self, talk: u16) -> Vec<(String, Option<Speaker>)> {
        let mut v: Vec<(String, Option<Speaker>)> =
            self.weekend_weeks.iter().filter(|w| !w.no_meeting && w.talk == Some(talk)).map(|w| (w.monday.clone(), w.speaker)).collect();
        v.sort_by(|a, b| b.0.cmp(&a.0));
        v
    }

    /// Why a talk may not be a good choice for the week of `monday`: not in the speaker's list,
    /// not in use, or given here less than `repeat_months` ago (before or after).
    pub fn talk_warnings(&self, monday: &str, speaker: Option<Speaker>, talk: u16) -> Vec<String> {
        let mut out = Vec::new();
        if self.talk_settings.not_in_use.contains(&talk) {
            out.push(format!("Talk {talk} is marked as not in use."));
        }
        if let Some(s) = speaker {
            let list = self.talks_of(s);
            if !list.is_empty() && !list.contains(&talk) {
                out.push(format!("Talk {talk} is not in the speaker's list."));
            }
        }
        let window = i64::from(self.talk_settings.repeat_months) * 30;
        if let Some((when, _)) = self
            .talk_history(talk)
            .into_iter()
            .filter(|(m, _)| m != monday)
            .find(|(m, _)| crate::events::days_between(m, monday).is_some_and(|d| d.abs() < window))
        {
            out.push(format!("Talk {talk} is also given in the week of {when}."));
        }
        out
    }

    // ----- outgoing talks -----

    pub fn outgoing_talk(&self, id: u32) -> Option<&OutgoingTalk> {
        self.outgoing_talks.iter().find(|t| t.id == id)
    }

    /// Adds or changes an outgoing talk. The speaker needs the "public talk in other
    /// congregations" privilege.
    pub fn save_outgoing_talk(&mut self, id: Option<u32>, d: &OutgoingDraft) -> Result<u32, String> {
        if crate::people::parse_date(d.date.trim()).is_none() {
            return Err("Enter the date of the talk.".into());
        }
        let speaker = d.speaker.ok_or("Choose the speaker.")?;
        if !self.person(speaker).is_some_and(|p| p.moved_away.is_none() && p.privileges.contains(&Privilege::PublicTalkAway)) {
            return Err("This brother does not give public talks in other congregations.".into());
        }
        let congregation = d.congregation.filter(|c| self.other_congregation(*c).is_some()).ok_or("Choose the congregation.")?;
        if let Some(n) = d.talk.filter(|n| !(1..=MAX_TALK_NUMBER).contains(n)) {
            return Err(format!("{n} is not a talk number."));
        }
        let time = opt(&d.time);
        if time.as_deref().is_some_and(|t| crate::midweek::minutes_of(t).is_none()) {
            return Err("Enter the time as HH:MM.".into());
        }
        let new_id = id.unwrap_or_else(|| self.outgoing_talks.iter().map(|t| t.id).max().unwrap_or(0) + 1);
        let sent = self.outgoing_talk(new_id).and_then(|t| t.sent.clone());
        let t = OutgoingTalk { id: new_id, date: d.date.trim().into(), time, speaker, congregation, talk: d.talk, notes: opt(&d.notes), sent };
        match self.outgoing_talks.iter_mut().find(|x| x.id == new_id) {
            Some(slot) => *slot = t,
            None => self.outgoing_talks.push(t),
        }
        Ok(new_id)
    }

    /// Deletes an outgoing talk. If it was sent, the speaker gets a "cancelled" notice.
    pub fn delete_outgoing_talk(&mut self, id: u32) {
        if let Some(t) = self.outgoing_talk(id).cloned().filter(|t| t.sent.is_some()) {
            let title = self.outgoing_title(&t);
            self.send_plan(AssignmentKind::OutgoingTalk, id, t.sent.as_ref(), &t.date, true, Vec::new(), Vec::new(), &title);
        }
        self.outgoing_talks.retain(|t| t.id != id);
    }

    /// Why an outgoing talk may be a problem: the speaker is away, has a part in the weekend
    /// meeting here that week, or more outgoing talks that month than the setting allows.
    pub fn outgoing_warnings(&self, t: &OutgoingTalk) -> Vec<String> {
        let mut out = Vec::new();
        if self.is_away(t.speaker, &t.date) {
            out.push("The speaker has an absence that day.".into());
        }
        let monday = crate::midweek::monday_of(&t.date);
        if self.weekend_week(&monday).is_some_and(|w| w.places_with(&self.weekend_settings).iter().any(|(_, p)| *p == Some(t.speaker))) {
            out.push("The speaker has a part in the weekend meeting here that week.".into());
        }
        let month = &t.date[..t.date.len().min(7)];
        let same_month = self.outgoing_talks.iter().filter(|x| x.speaker == t.speaker && x.date.starts_with(month)).count();
        if same_month > usize::from(self.talk_settings.max_outgoing_per_month) {
            out.push(format!("The speaker has {same_month} outgoing talks this month."));
        }
        if let Some(n) = t.talk {
            let list = self.talks_of(Speaker::Local(t.speaker));
            if !list.is_empty() && !list.contains(&n) {
                out.push(format!("Talk {n} is not in the speaker's list."));
            }
        }
        out
    }

    fn outgoing_title(&self, t: &OutgoingTalk) -> String {
        let cong = self.other_congregation(t.congregation).map(|c| c.name.clone()).unwrap_or_default();
        let talk = t.talk.map(|n| format!(", talk {n}")).unwrap_or_default();
        let time = t.time.as_ref().map(|x| format!(" at {x}")).unwrap_or_default();
        format!("Public talk in {cong}{time}{talk}")
    }

    fn outgoing_roles(&self, t: &OutgoingTalk) -> Vec<(u32, String)> {
        vec![(t.speaker, self.outgoing_title(t))]
    }

    pub fn outgoing_status(&self, t: &OutgoingTalk) -> SendStatus {
        crate::scheduling::status(t.sent.as_ref(), &self.outgoing_roles(t), false, &t.date)
    }

    /// Sends outgoing talks: the speaker sees them in *My assignments* and gets a notice. A
    /// changed speaker gets "cancelled", the new one "new". Returns how many were sent.
    pub fn send_outgoing_talks(&mut self, ids: &[u32]) -> usize {
        let todo: Vec<OutgoingTalk> = self.outgoing_talks.iter().filter(|t| ids.contains(&t.id) && self.outgoing_status(t) != SendStatus::Sent).cloned().collect();
        for t in &todo {
            let roles = self.outgoing_roles(t);
            let sent = self.send_plan(AssignmentKind::OutgoingTalk, t.id, t.sent.as_ref(), &t.date, false, roles, vec![("speaker".into(), t.speaker)], "Outgoing talk");
            if let Some(x) = self.outgoing_talks.iter_mut().find(|x| x.id == t.id) {
                x.sent = Some(sent);
            }
        }
        todo.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::assignments::NoticeKind;

    fn setup() -> (Directory, u32) {
        let mut d = crate::demo::demo_directory(20);
        d.other_congregations.clear();
        d.visiting_speakers.clear();
        d.outgoing_talks.clear();
        d.speaker_talks.clear();
        d.notices.clear();
        let lake = d.save_other_congregation(None, &OtherCongregationDraft { name: "Lakeside".into(), coordinator_email: "talks@example.org".into(), ..Default::default() }).unwrap_or(0);
        (d, lake)
    }

    #[test]
    fn talk_numbers_are_read_and_written_with_ranges() {
        assert_eq!(parse_talks("3, 1-2 ,7").map(|t| talks_text(&t)), Ok("1-3, 7".into()));
        assert!(parse_talks("1, x, 2000").is_err_and(|e| e.contains("x") && e.contains("2000")));
        assert_eq!(parse_talks(""), Ok(BTreeSet::new()));
    }

    #[test]
    fn congregations_are_unique_and_kept_while_used() {
        let (mut d, lake) = setup();
        assert!(d.save_other_congregation(None, &OtherCongregationDraft { name: " lakeside ".into(), ..Default::default() }).is_err());
        assert!(d.save_other_congregation(None, &OtherCongregationDraft { name: "Hill".into(), coordinator_email: "nope".into(), ..Default::default() }).is_err());
        let s = d.save_visiting_speaker(None, &VisitingSpeakerDraft { first_name: "Sam".into(), last_name: "Visitor".into(), congregation: Some(lake), talks: [12, 40].into(), ..Default::default() });
        assert!(s.is_ok());
        assert!(d.delete_other_congregation(lake).is_err());
        assert!(d.save_visiting_speaker(None, &VisitingSpeakerDraft { first_name: "No".into(), last_name: "Home".into(), congregation: Some(999), ..Default::default() }).is_err());
    }

    #[test]
    fn local_speaker_talks_need_the_privilege() {
        let (mut d, _) = setup();
        let speaker = d.local_speakers().first().copied().unwrap_or(0);
        assert!(d.set_speaker_talks(speaker, [5, 6].into()).is_ok());
        assert_eq!(d.talks_of(Speaker::Local(speaker)), [5, 6].into());
        let sister = d.people.iter().find(|p| p.gender == crate::people::Gender::Female).map(|p| p.id).unwrap_or(0);
        assert!(d.set_speaker_talks(sister, [5].into()).is_err());
        assert!(d.set_speaker_talks(speaker, [0].into()).is_err());
    }

    #[test]
    fn outgoing_talks_are_checked_and_sent_to_the_speaker() {
        let (mut d, lake) = setup();
        let speaker = d.people.iter().find(|p| p.privileges.contains(&Privilege::PublicTalkAway)).map(|p| p.id).unwrap_or(0);
        let draft = OutgoingDraft { date: "2026-12-06".into(), time: "10:00".into(), speaker: Some(speaker), congregation: Some(lake), talk: Some(12), notes: String::new() };
        let id = d.save_outgoing_talk(None, &draft).unwrap_or(0);
        assert!(d.save_outgoing_talk(None, &OutgoingDraft { time: "ten".into(), ..draft.clone() }).is_err());
        assert!(d.save_outgoing_talk(None, &OutgoingDraft { congregation: None, ..draft.clone() }).is_err());
        assert_eq!(d.outgoing_talk(id).map(|t| d.outgoing_status(t)), Some(SendStatus::NotSent));
        assert_eq!(d.send_outgoing_talks(&[id]), 1);
        assert_eq!(d.my_notices(speaker).first().map(|n| (n.kind, n.what)), Some((NoticeKind::New, AssignmentKind::OutgoingTalk)));
        assert!(d.all_my_assignments(speaker).iter().any(|a| a.kind == AssignmentKind::OutgoingTalk && a.date == "2026-12-06"));
        // A second talk the same month is over the limit.
        let second = d.save_outgoing_talk(None, &OutgoingDraft { date: "2026-12-20".into(), ..draft.clone() }).unwrap_or(0);
        assert!(d.outgoing_talk(second).is_some_and(|t| d.outgoing_warnings(t).iter().any(|w| w.contains("this month"))));
        d.mark_notices_seen(speaker);
        d.delete_outgoing_talk(id);
        assert_eq!(d.my_notices(speaker).first().map(|n| n.kind), Some(NoticeKind::Cancelled));
    }
}
