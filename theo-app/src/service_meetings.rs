//! ⚠️ MOCK of meetings for field service (M-15): when and where the congregation meets for the
//! ministry, and who conducts. Presets (weekly slots) with availability, filling a month, an
//! automatic assignment, and sending: only sent meetings are seen by everyone.
//! Rules: `docs/model/m15-field-service-meetings.md`.

use crate::people::{parse_date, Directory, Person};
use crate::privileges::Privilege;
use crate::reports::Ym;
use std::collections::BTreeSet;

/// English weekday names, Sunday first (as `weekday` numbers: 0 = Sunday).
pub const WEEKDAYS: [&str; 7] = ["Sunday", "Monday", "Tuesday", "Wednesday", "Thursday", "Friday", "Saturday"];

/// A weekly slot: every `weekday` at `time` in `place`. Only the people in `available` (who also
/// have the privilege) can conduct it: nobody is available until a planner ticks them.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ServicePreset {
    pub id: u32,
    /// 0 = Sunday … 6 = Saturday.
    pub weekday: u8,
    /// `HH:MM`.
    pub time: String,
    pub place: String,
    pub available: BTreeSet<u32>,
    /// Off: Fill month skips it (for example for a few weeks), until it is switched on again.
    #[serde(default = "on")]
    pub enabled: bool,
}

fn on() -> bool {
    true
}

impl ServicePreset {
    pub fn weekday_name(&self) -> &'static str {
        WEEKDAYS[self.weekday as usize % 7]
    }

    /// "Every Monday at 10:00".
    pub fn label(&self) -> String {
        format!("Every {} at {}", self.weekday_name(), self.time)
    }
}

/// What a meeting looks like to everyone: the version that was last sent.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct MeetingVersion {
    /// `YYYY-MM-DD`.
    pub date: String,
    /// `HH:MM`.
    pub time: String,
    pub place: String,
    pub conductor: Option<u32>,
}

/// A meeting for field service. Planners see and change the current fields; everyone else sees
/// `sent` (nothing until it is sent).
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ServiceMeeting {
    pub id: u32,
    pub date: String,
    pub time: String,
    pub place: String,
    pub conductor: Option<u32>,
    /// The preset it was made from (for availability), if any.
    pub preset: Option<u32>,
    pub sent: Option<MeetingVersion>,
    /// Deleted after it was sent: everyone still sees it until the next send.
    pub removed: bool,
}

/// Whether everyone sees a meeting as it is now.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SendStatus {
    /// Never sent: only planners see it.
    NotSent,
    /// Everyone sees it as it is.
    Sent,
    /// Changed after it was sent: everyone still sees the sent version.
    Changed,
    /// Deleted after it was sent: everyone still sees it until the next send.
    Removed,
}

impl SendStatus {
    pub fn value(self) -> &'static str {
        match self {
            SendStatus::NotSent => "not-sent",
            SendStatus::Sent => "sent",
            SendStatus::Changed => "changed",
            SendStatus::Removed => "removed",
        }
    }
}

impl ServiceMeeting {
    pub fn current(&self) -> MeetingVersion {
        MeetingVersion { date: self.date.clone(), time: self.time.clone(), place: self.place.clone(), conductor: self.conductor }
    }

    pub fn status(&self) -> SendStatus {
        match &self.sent {
            None => SendStatus::NotSent,
            Some(_) if self.removed => SendStatus::Removed,
            Some(v) if *v == self.current() => SendStatus::Sent,
            Some(_) => SendStatus::Changed,
        }
    }
}

/// Can someone conduct a meeting on that day?
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Availability {
    Available,
    /// Has the privilege, but is not ticked as available for this preset.
    NotForThisSlot,
    /// Already conducts another meeting that day.
    BusyThatDay,
    /// Has an absence that day.
    Away,
    /// Does not have the privilege (shown in a separate list).
    NoPrivilege,
}

impl Availability {
    pub fn label(self) -> &'static str {
        match self {
            Availability::Available => "Available",
            Availability::NotForThisSlot => "Not available at this time",
            Availability::BusyThatDay => "Already conducts that day",
            Availability::Away => "Away",
            Availability::NoPrivilege => "Without the privilege",
        }
    }
}

/// The meeting form.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ServiceMeetingDraft {
    pub date: String,
    pub time: String,
    pub place: String,
    pub conductor: Option<u32>,
    pub preset: Option<u32>,
}

/// What filling or assigning a month did.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct FillReport {
    pub added: usize,
    pub assigned: usize,
    /// Meetings left without a conductor because nobody was available.
    pub unassigned: usize,
    /// Days skipped because the whole congregation is away (an assembly or convention).
    pub skipped_days: Vec<String>,
}

fn day_number(iso: &str) -> Option<i64> {
    parse_date(iso).map(crate::prefs::days_from_ymd)
}

/// 0 = Sunday.
pub fn weekday_of(iso: &str) -> Option<u8> {
    day_number(iso).map(|z| (z + 4).rem_euclid(7) as u8)
}

/// Monday-based week number (for spreading assignments over weeks).
fn week_of(iso: &str) -> i64 {
    day_number(iso).map(|z| (z + 3).div_euclid(7)).unwrap_or(0)
}

fn valid_time(t: &str) -> bool {
    let Some((h, m)) = t.split_once(':') else { return false };
    h.len() == 2 && m.len() == 2 && h.parse::<u32>().is_ok_and(|h| h < 24) && m.parse::<u32>().is_ok_and(|m| m < 60)
}

fn month_of(iso: &str) -> Option<Ym> {
    parse_date(iso).map(|(y, m, _)| (y, m))
}

impl Directory {
    // ----- presets -----

    pub fn service_preset(&self, id: u32) -> Option<&ServicePreset> {
        self.service_presets.iter().find(|p| p.id == id)
    }

    /// Presets in week order (from `first_weekday`), then by time.
    pub fn service_presets_sorted(&self, first_weekday: u8) -> Vec<&ServicePreset> {
        let mut v: Vec<&ServicePreset> = self.service_presets.iter().collect();
        v.sort_by_key(|p| ((p.weekday + 7 - first_weekday) % 7, p.time.clone(), p.place.clone()));
        v
    }

    pub fn save_service_preset(
        &mut self,
        id: Option<u32>,
        weekday: u8,
        time: &str,
        place: &str,
        available: BTreeSet<u32>,
    ) -> Result<u32, Vec<(&'static str, String)>> {
        let mut e = Vec::new();
        if weekday > 6 {
            e.push(("weekday", "Choose a day.".to_string()));
        }
        if !valid_time(time) {
            e.push(("time", "Enter the time, for example 10:00.".into()));
        }
        if place.trim().is_empty() {
            e.push(("place", "Enter the place.".into()));
        }
        if e.is_empty()
            && self.service_presets.iter().any(|p| Some(p.id) != id && p.weekday == weekday && p.time == time && p.place.trim().eq_ignore_ascii_case(place.trim()))
        {
            e.push(("time", "There is already a preset for this day, time and place.".into()));
        }
        if !e.is_empty() {
            return Err(e);
        }
        let new_id = id.unwrap_or_else(|| self.service_presets.iter().map(|p| p.id).max().unwrap_or(0) + 1);
        let enabled = self.service_preset(new_id).is_none_or(|p| p.enabled);
        let p = ServicePreset { id: new_id, weekday, time: time.into(), place: place.trim().into(), available, enabled };
        match self.service_presets.iter_mut().find(|x| x.id == new_id) {
            Some(slot) => *slot = p,
            None => self.service_presets.push(p),
        }
        Ok(new_id)
    }

    /// Switches a preset on or off. Off: *Fill month* skips it; meetings already made stay.
    pub fn set_service_preset_enabled(&mut self, id: u32, enabled: bool) {
        if let Some(p) = self.service_presets.iter_mut().find(|p| p.id == id) {
            p.enabled = enabled;
        }
    }

    /// Deletes a preset. Meetings made from it stay (without the link).
    pub fn delete_service_preset(&mut self, id: u32) {
        self.service_presets.retain(|p| p.id != id);
        for m in self.service_meetings.iter_mut().filter(|m| m.preset == Some(id)) {
            m.preset = None;
        }
    }

    // ----- who can conduct -----

    /// Current people with the privilege "Meeting for field service", by name.
    pub fn conductors(&self) -> Vec<&Person> {
        let mut v: Vec<&Person> =
            self.people.iter().filter(|p| p.moved_away.is_none() && p.privileges.contains(&Privilege::FieldServiceMeeting)).collect();
        v.sort_by_cached_key(|p| crate::people::fold(&format!("{} {}", p.first_name, p.last_name)));
        v
    }

    /// Whether `person` can conduct a meeting on `date` (from `preset`), not counting `meeting`
    /// itself when checking the other meetings of that day.
    pub fn availability(&self, person: u32, date: &str, preset: Option<u32>, meeting: Option<u32>) -> Availability {
        let Some(p) = self.person(person) else { return Availability::NoPrivilege };
        if self.is_away(person, date) {
            return Availability::Away;
        }
        if !p.privileges.contains(&Privilege::FieldServiceMeeting) {
            return Availability::NoPrivilege;
        }
        if preset.and_then(|id| self.service_preset(id)).is_some_and(|s| !s.available.contains(&person)) {
            return Availability::NotForThisSlot;
        }
        if self.service_meetings.iter().any(|m| Some(m.id) != meeting && !m.removed && m.date == date && m.conductor == Some(person)) {
            return Availability::BusyThatDay;
        }
        Availability::Available
    }

    // ----- meetings -----

    pub fn service_meeting(&self, id: u32) -> Option<&ServiceMeeting> {
        self.service_meetings.iter().find(|m| m.id == id)
    }

    /// The meetings of a month as planners see them (removed ones too), by date and time.
    pub fn service_meetings_in(&self, ym: Ym) -> Vec<&ServiceMeeting> {
        let mut v: Vec<&ServiceMeeting> = self.service_meetings.iter().filter(|m| month_of(&m.date) == Some(ym)).collect();
        v.sort_by(|a, b| (&a.date, &a.time, &a.place).cmp(&(&b.date, &b.time, &b.place)));
        v
    }

    /// The meetings of a month as everyone sees them: the sent versions. (id, version)
    pub fn sent_service_meetings_in(&self, ym: Ym) -> Vec<(u32, MeetingVersion)> {
        let mut v: Vec<(u32, MeetingVersion)> =
            self.service_meetings.iter().filter_map(|m| m.sent.clone().map(|s| (m.id, s))).filter(|(_, s)| month_of(&s.date) == Some(ym)).collect();
        v.sort_by(|a, b| (&a.1.date, &a.1.time, &a.1.place).cmp(&(&b.1.date, &b.1.time, &b.1.place)));
        v
    }

    pub fn save_service_meeting(&mut self, id: Option<u32>, d: &ServiceMeetingDraft) -> Result<u32, Vec<(&'static str, String)>> {
        let mut e = Vec::new();
        if day_number(&d.date).is_none() || d.date.len() != 10 {
            e.push(("date", "Enter the date.".to_string()));
        }
        if !valid_time(&d.time) {
            e.push(("time", "Enter the time, for example 10:00.".into()));
        }
        if d.place.trim().is_empty() {
            e.push(("place", "Enter the place.".into()));
        }
        if d.conductor.is_some_and(|c| self.person(c).is_none()) {
            e.push(("conductor", "Choose someone from the list.".into()));
        }
        if !e.is_empty() {
            return Err(e);
        }
        let new_id = id.unwrap_or_else(|| self.service_meetings.iter().map(|m| m.id).max().unwrap_or(0) + 1);
        let preset = d.preset.filter(|p| self.service_preset(*p).is_some());
        match self.service_meetings.iter_mut().find(|m| m.id == new_id) {
            Some(m) => {
                m.date = d.date.clone();
                m.time = d.time.clone();
                m.place = d.place.trim().into();
                m.conductor = d.conductor;
                m.preset = preset;
                m.removed = false;
            }
            None => self.service_meetings.push(ServiceMeeting {
                id: new_id,
                date: d.date.clone(),
                time: d.time.clone(),
                place: d.place.trim().into(),
                conductor: d.conductor,
                preset,
                sent: None,
                removed: false,
            }),
        }
        Ok(new_id)
    }

    /// Deletes a meeting. A sent meeting stays visible to everyone until the next send.
    pub fn delete_service_meeting(&mut self, id: u32) {
        if let Some(m) = self.service_meetings.iter_mut().find(|m| m.id == id)
            && m.sent.is_some()
        {
            m.removed = true;
            return;
        }
        self.service_meetings.retain(|m| m.id != id);
    }

    /// Undoes a delete that was not sent yet.
    pub fn keep_service_meeting(&mut self, id: u32) {
        if let Some(m) = self.service_meetings.iter_mut().find(|m| m.id == id) {
            m.removed = false;
        }
    }

    /// Is the whole congregation away that day (an assembly, a convention)?
    pub fn congregation_away(&self, date: &str) -> bool {
        self.absences.iter().any(|a| a.person.is_none() && a.from.as_str() <= date && date <= a.to.as_str())
    }

    /// Adds a meeting for every preset on every matching day of the month that does not have one
    /// yet (same day, time and place), except days when the whole congregation is away. With
    /// `assign`, then gives every meeting of the month without a conductor one (see
    /// [`Directory::assign_service_meetings`]).
    pub fn fill_service_month(&mut self, ym: Ym, assign: bool) -> FillReport {
        let mut report = FillReport::default();
        let first = crate::prefs::days_from_ymd((ym.0, ym.1, 1));
        let next = crate::reports::shift(ym, 1);
        let last = crate::prefs::days_from_ymd((next.0, next.1, 1)) - 1;
        let presets: Vec<ServicePreset> = self.service_presets.iter().filter(|p| p.enabled).cloned().collect();
        for z in first..=last {
            let (y, m, d) = crate::prefs::ymd_from_days(z);
            let date = format!("{y:04}-{m:02}-{d:02}");
            let weekday = (z + 4).rem_euclid(7) as u8;
            let todays: Vec<&ServicePreset> = presets.iter().filter(|p| p.weekday == weekday).collect();
            if todays.is_empty() {
                continue;
            }
            if self.congregation_away(&date) {
                report.skipped_days.push(date);
                continue;
            }
            for p in todays {
                let exists = self
                    .service_meetings
                    .iter()
                    .any(|x| !x.removed && x.date == date && x.time == p.time && x.place.eq_ignore_ascii_case(&p.place));
                if exists {
                    continue;
                }
                let draft = ServiceMeetingDraft { date: date.clone(), time: p.time.clone(), place: p.place.clone(), conductor: None, preset: Some(p.id) };
                if self.save_service_meeting(None, &draft).is_ok() {
                    report.added += 1;
                }
            }
        }
        if assign {
            let (assigned, unassigned) = self.assign_service_meetings(ym);
            report.assigned = assigned;
            report.unassigned = unassigned;
        }
        report
    }

    /// Gives every meeting of the month without a conductor one, keeping the conductors already
    /// chosen. Only people with the privilege, without an absence, available for the preset and
    /// not conducting another meeting that day. Among them: the fewest meetings that week, then
    /// that month, then the longest since their last one. Returns (assigned, left without).
    pub fn assign_service_meetings(&mut self, ym: Ym) -> (usize, usize) {
        let targets: Vec<u32> = self.service_meetings_in(ym).iter().filter(|m| !m.removed && m.conductor.is_none()).map(|m| m.id).collect();
        let conductors: Vec<u32> = self.conductors().iter().map(|p| p.id).collect();
        let (mut assigned, mut left) = (0, 0);
        for id in targets {
            let Some(m) = self.service_meeting(id).cloned() else { continue };
            let week = week_of(&m.date);
            let today = day_number(&m.date).unwrap_or(0);
            let month: Vec<&ServiceMeeting> = self.service_meetings_in(ym).into_iter().filter(|x| !x.removed).collect();
            let best = conductors
                .iter()
                .copied()
                .filter(|c| self.availability(*c, &m.date, m.preset, Some(m.id)) == Availability::Available)
                .min_by_key(|c| {
                    let mine: Vec<&&ServiceMeeting> = month.iter().filter(|x| x.conductor == Some(*c)).collect();
                    let same_week = mine.iter().filter(|x| week_of(&x.date) == week).count();
                    // Days to their nearest other meeting (more is better).
                    let gap = self
                        .service_meetings
                        .iter()
                        .filter(|x| !x.removed && x.id != m.id && x.conductor == Some(*c))
                        .filter_map(|x| day_number(&x.date))
                        .map(|d| (d - today).abs())
                        .min()
                        .unwrap_or(i64::MAX / 2);
                    let name = self.person(*c).map(|p| crate::people::fold(&format!("{} {}", p.first_name, p.last_name))).unwrap_or_default();
                    (same_week, mine.len(), std::cmp::Reverse(gap), name)
                });
            match best {
                Some(c) => {
                    if let Some(x) = self.service_meetings.iter_mut().find(|x| x.id == id) {
                        x.conductor = Some(c);
                    }
                    assigned += 1;
                }
                None => left += 1,
            }
        }
        (assigned, left)
    }

    /// Deletes every meeting of the month (sent ones stay visible until the next send).
    pub fn clear_service_month(&mut self, ym: Ym) {
        let ids: Vec<u32> = self.service_meetings_in(ym).iter().map(|m| m.id).collect();
        for id in ids {
            self.delete_service_meeting(id);
        }
    }

    /// Changes of the month that everyone does not see yet.
    pub fn unsent_service_changes(&self, ym: Ym) -> Vec<(u32, SendStatus)> {
        self.service_meetings_in(ym).iter().map(|m| (m.id, m.status())).filter(|(_, s)| *s != SendStatus::Sent).collect()
    }

    /// Sends the month: everyone sees the meetings as they are now; removed ones disappear.
    /// A meeting moved to another month after it was sent is sent with the month it is in now.
    /// Only the conductors concerned get a notice (new, changed or cancelled).
    pub fn send_service_month(&mut self, ym: Ym) -> usize {
        use crate::assignments::NoticeKind;
        let ids: Vec<u32> = self.unsent_service_changes(ym).iter().map(|(id, _)| *id).collect();
        for id in &ids {
            let Some(m) = self.service_meeting(*id).cloned() else { continue };
            let now = m.current();
            let before = m.sent.clone();
            match (&before, m.removed) {
                (Some(b), true) => {
                    if let Some(c) = b.conductor {
                        self.notify(c, NoticeKind::Cancelled, m.id, b);
                    }
                }
                (Some(b), false) if b.conductor == now.conductor => {
                    if let Some(c) = now.conductor {
                        self.notify(c, NoticeKind::Changed, m.id, &now);
                    }
                }
                (Some(b), false) => {
                    if let Some(c) = b.conductor {
                        self.notify(c, NoticeKind::Cancelled, m.id, b);
                    }
                    if let Some(c) = now.conductor {
                        self.notify(c, NoticeKind::New, m.id, &now);
                    }
                }
                (None, _) => {
                    if let Some(c) = now.conductor {
                        self.notify(c, NoticeKind::New, m.id, &now);
                    }
                }
            }
            let removed = m.removed;
            if removed {
                self.service_meetings.retain(|m| m.id != *id);
            } else if let Some(m) = self.service_meetings.iter_mut().find(|m| m.id == *id) {
                m.sent = Some(m.current());
            }
        }
        ids.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A directory with 3 conductors (ids 2, 3, 4) and two presets.
    fn setup() -> Directory {
        let mut d = crate::mock::demo_directory(10);
        d.service_presets.clear();
        d.service_meetings.clear();
        d.absences.clear();
        for p in d.people.iter_mut() {
            p.privileges.remove(&Privilege::FieldServiceMeeting);
            if [2, 3, 4].contains(&p.id) {
                p.privileges.insert(Privilege::FieldServiceMeeting);
            }
        }
        // Monday 10:00 and Saturday 9:30 (November 2026: 5 Mondays, 4 Saturdays), all three available.
        let all: BTreeSet<u32> = [2, 3, 4].into_iter().collect();
        assert!(d.save_service_preset(None, 1, "10:00", "Kingdom Hall", all.clone()).is_ok());
        assert!(d.save_service_preset(None, 6, "09:30", "Kingdom Hall", all).is_ok());
        d
    }

    #[test]
    fn filling_adds_the_preset_days_once() {
        let mut d = setup();
        let r = d.fill_service_month((2026, 11), false);
        assert_eq!(r.added, 9);
        assert!(d.service_meetings_in((2026, 11)).iter().all(|m| m.conductor.is_none()));
        // Filling again adds nothing.
        assert_eq!(d.fill_service_month((2026, 11), false).added, 0);
        assert_eq!(weekday_of("2026-11-02"), Some(1));
    }

    #[test]
    fn assignment_follows_the_rules() {
        let mut d = setup();
        // 3 is away the first week; 4 cannot do Saturdays.
        d.save_absence(None, Some(3), "2026-11-01", "2026-11-08", "").ok();
        let saturday = d.service_presets.iter().find(|p| p.weekday == 6).map(|p| p.id);
        if let Some(s) = saturday
            && let Some(mut p) = d.service_preset(s).cloned()
        {
            p.available.remove(&4);
            d.save_service_preset(Some(s), p.weekday, &p.time, &p.place, p.available).ok();
        }
        let r = d.fill_service_month((2026, 11), true);
        assert_eq!((r.added, r.assigned, r.unassigned), (9, 9, 0));
        let month = d.service_meetings_in((2026, 11));
        for m in &month {
            let c = m.conductor.unwrap_or(0);
            assert!([2, 3, 4].contains(&c));
            assert!(!(c == 3 && m.date.as_str() <= "2026-11-08"), "away: {m:?}");
            assert!(!(c == 4 && m.preset == saturday), "not for this slot: {m:?}");
        }
        // Spread: nobody has more than 4 of the 9.
        for c in [2, 3, 4] {
            assert!(month.iter().filter(|m| m.conductor == Some(c)).count() <= 4);
        }
        // Nobody available: the meeting stays empty and is counted.
        d.clear_service_month((2026, 11));
        for p in d.people.iter_mut() {
            p.privileges.remove(&Privilege::FieldServiceMeeting);
        }
        let r = d.fill_service_month((2026, 11), true);
        assert_eq!((r.assigned, r.unassigned), (0, 9));
    }

    #[test]
    fn sending_shows_meetings_to_everyone() {
        let mut d = setup();
        d.fill_service_month((2026, 11), true);
        assert!(d.sent_service_meetings_in((2026, 11)).is_empty());
        assert_eq!(d.send_service_month((2026, 11)), 9);
        assert_eq!(d.sent_service_meetings_in((2026, 11)).len(), 9);
        assert!(d.unsent_service_changes((2026, 11)).is_empty());
        // A change after sending: everyone keeps the sent version until the next send.
        let first = d.service_meetings_in((2026, 11))[0].clone();
        let mut draft = ServiceMeetingDraft { date: first.date.clone(), time: "11:00".into(), place: first.place.clone(), conductor: first.conductor, preset: first.preset };
        assert!(d.save_service_meeting(Some(first.id), &draft).is_ok());
        assert_eq!(d.service_meeting(first.id).map(|m| m.status()), Some(SendStatus::Changed));
        assert_eq!(d.sent_service_meetings_in((2026, 11))[0].1.time, "10:00");
        // A new one is not sent; a deleted sent one stays until the send.
        draft.date = "2026-11-04".into();
        let new = d.save_service_meeting(None, &draft).unwrap_or(0);
        assert_eq!(d.service_meeting(new).map(|m| m.status()), Some(SendStatus::NotSent));
        d.delete_service_meeting(first.id);
        assert_eq!(d.service_meeting(first.id).map(|m| m.status()), Some(SendStatus::Removed));
        assert_eq!(d.unsent_service_changes((2026, 11)).len(), 2);
        d.send_service_month((2026, 11));
        assert!(d.service_meeting(first.id).is_none());
        assert_eq!(d.sent_service_meetings_in((2026, 11)).len(), 9);
    }

    #[test]
    fn nobody_is_available_until_ticked() {
        let mut d = setup();
        let id = d.save_service_preset(None, 3, "18:30", "Kingdom Hall", BTreeSet::new()).unwrap_or(0);
        assert_eq!(d.availability(2, "2026-11-04", Some(id), None), Availability::NotForThisSlot);
        let r = d.fill_service_month((2026, 11), true);
        // The 4 Wednesdays stay empty; the other 9 meetings get a conductor.
        assert_eq!((r.assigned, r.unassigned), (9, 4));
    }

    #[test]
    fn only_the_conductors_concerned_get_notices() {
        use crate::assignments::NoticeKind;
        let mut d = setup();
        d.notices.clear();
        d.fill_service_month((2026, 11), true);
        d.send_service_month((2026, 11));
        let total: usize = [2, 3, 4].iter().map(|c| d.my_notices(*c).len()).sum();
        assert_eq!(total, 9);
        assert!(d.my_notices(5).is_empty());
        assert_eq!(d.my_assignments(2, "2026-11-01").len(), d.my_notices(2).len());
        for c in [2, 3, 4] {
            d.mark_notices_seen(c);
        }
        // Give a sent meeting to someone else: the old conductor hears it is cancelled, the new
        // one that it is new; nobody else hears anything.
        let m = d.service_meetings_in((2026, 11))[0].clone();
        let (old, new) = (m.conductor.unwrap_or(0), if m.conductor == Some(2) { 3 } else { 2 });
        let draft = ServiceMeetingDraft { date: m.date.clone(), time: m.time.clone(), place: m.place.clone(), conductor: Some(new), preset: m.preset };
        assert!(d.save_service_meeting(Some(m.id), &draft).is_ok());
        d.send_service_month((2026, 11));
        assert_eq!(d.my_notices(old).iter().map(|n| n.kind).collect::<Vec<_>>(), vec![NoticeKind::Cancelled]);
        assert_eq!(d.my_notices(new).iter().map(|n| n.kind).collect::<Vec<_>>(), vec![NoticeKind::New]);
        let third = [2, 3, 4].into_iter().find(|c| *c != old && *c != new).unwrap_or(4);
        assert!(d.my_notices(third).is_empty());
    }

    #[test]
    fn switched_off_presets_are_not_filled() {
        let mut d = setup();
        let saturday = d.service_presets.iter().find(|p| p.weekday == 6).map(|p| p.id).unwrap_or(0);
        d.set_service_preset_enabled(saturday, false);
        assert_eq!(d.fill_service_month((2026, 11), false).added, 5);
        // Saving it keeps it off; switching it on fills the rest.
        let p = d.service_preset(saturday).cloned();
        if let Some(p) = p {
            d.save_service_preset(Some(saturday), p.weekday, &p.time, &p.place, p.available).ok();
        }
        assert!(d.service_preset(saturday).is_some_and(|p| !p.enabled));
        d.set_service_preset_enabled(saturday, true);
        assert_eq!(d.fill_service_month((2026, 11), false).added, 4);
    }

    #[test]
    fn days_when_the_congregation_is_away_are_skipped() {
        let mut d = setup();
        d.save_absence(None, None, "2026-11-07", "2026-11-09", "Assembly").ok();
        let r = d.fill_service_month((2026, 11), false);
        assert_eq!(r.added, 7);
        assert_eq!(r.skipped_days, vec!["2026-11-07".to_string(), "2026-11-09".to_string()]);
    }
}
