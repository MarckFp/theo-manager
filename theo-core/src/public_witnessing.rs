//! Public witnessing (M-08): shifts at a place and time with the people who take part
//! (shift supervisors, men, women or anyone), weekly presets with availability, filling and
//! assigning a month, sending, and statistics. Same flow as meetings for field service (M-15).
//! Rules: `docs/model/m16-public-witnessing.md`.

use crate::assignments::{AssignmentKind, NoticeKind};
use crate::people::{parse_date, Directory, Gender, Person};
use crate::privileges::Privilege;
use crate::reports::Ym;
use crate::service_meetings::SendStatus;
use std::collections::BTreeSet;

pub use crate::service_meetings::WEEKDAYS;

/// The kind of place in a shift.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize)]
pub enum Role {
    /// Responsible for the people of the shift (when supervisors are switched on).
    Supervisor,
    Man,
    Woman,
    /// Any gender.
    Anyone,
}

impl Role {
    pub const ALL: [Role; 4] = [Role::Supervisor, Role::Man, Role::Woman, Role::Anyone];

    pub fn label(self) -> &'static str {
        match self {
            Role::Supervisor => "Shift supervisor",
            Role::Man => "Brother",
            Role::Woman => "Sister",
            Role::Anyone => "Publisher",
        }
    }

    /// Short plural, for the number inputs of a shift.
    pub fn short_plural(self) -> &'static str {
        match self {
            Role::Supervisor => "Supervisors",
            Role::Man => "Brothers",
            Role::Woman => "Sisters",
            Role::Anyone => "Anyone",
        }
    }

    pub fn plural(self) -> &'static str {
        match self {
            Role::Supervisor => "Shift supervisors",
            Role::Man => "Brothers",
            Role::Woman => "Sisters",
            Role::Anyone => "Brothers or sisters",
        }
    }
}

/// How many people of each kind a shift needs.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Needs {
    pub supervisors: u8,
    pub men: u8,
    pub women: u8,
    pub anyone: u8,
}

impl Needs {
    pub fn get(&self, r: Role) -> u8 {
        match r {
            Role::Supervisor => self.supervisors,
            Role::Man => self.men,
            Role::Woman => self.women,
            Role::Anyone => self.anyone,
        }
    }

    pub fn set(&mut self, r: Role, n: u8) {
        match r {
            Role::Supervisor => self.supervisors = n,
            Role::Man => self.men = n,
            Role::Woman => self.women = n,
            Role::Anyone => self.anyone = n,
        }
    }

    pub fn total(&self) -> u8 {
        self.supervisors + self.men + self.women + self.anyone
    }

    /// "1 supervisor, 1 brother, 1 sister".
    pub fn describe(&self) -> String {
        let parts: Vec<String> = [
            (self.supervisors, "supervisor", "supervisors"),
            (self.men, "brother", "brothers"),
            (self.women, "sister", "sisters"),
            (self.anyone, "brother or sister", "brothers or sisters"),
        ]
        .into_iter()
        .filter(|(n, _, _)| *n > 0)
        .map(|(n, one, many)| format!("{n} {}", if n == 1 { one } else { many }))
        .collect();
        if parts.is_empty() { "Nobody".into() } else { parts.join(", ") }
    }

    /// The places of a shift, supervisors first.
    pub fn slots(&self) -> Vec<Slot> {
        Role::ALL.iter().flat_map(|r| (0..self.get(*r)).map(|_| Slot { role: *r, person: None })).collect()
    }
}

/// One place in a shift.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Slot {
    pub role: Role,
    pub person: Option<u32>,
}

/// Settings of the page: shift supervisors on or off, and who can be one.
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct WitnessingSettings {
    pub supervisors_on: bool,
    pub supervisors: BTreeSet<u32>,
}

/// A weekly shift.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ShiftPreset {
    pub id: u32,
    /// 0 = Sunday … 6 = Saturday.
    pub weekday: u8,
    /// `HH:MM`.
    pub start: String,
    pub end: String,
    pub place: String,
    pub needs: Needs,
    /// Who can take part (with the privilege). Nobody until ticked.
    pub available: BTreeSet<u32>,
    pub enabled: bool,
}

impl ShiftPreset {
    pub fn weekday_name(&self) -> &'static str {
        WEEKDAYS[self.weekday as usize % 7]
    }

    /// "Every Monday 09:00–11:00".
    pub fn label(&self) -> String {
        format!("Every {} {}–{}", self.weekday_name(), self.start, self.end)
    }
}

/// What a shift looks like to everyone: the version last sent.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ShiftVersion {
    pub date: String,
    pub start: String,
    pub end: String,
    pub place: String,
    pub slots: Vec<Slot>,
}

impl ShiftVersion {
    pub fn people(&self) -> Vec<u32> {
        self.slots.iter().filter_map(|s| s.person).collect()
    }
}

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Shift {
    pub id: u32,
    pub date: String,
    pub start: String,
    pub end: String,
    pub place: String,
    pub preset: Option<u32>,
    pub slots: Vec<Slot>,
    pub sent: Option<ShiftVersion>,
    pub removed: bool,
}

impl Shift {
    pub fn current(&self) -> ShiftVersion {
        ShiftVersion { date: self.date.clone(), start: self.start.clone(), end: self.end.clone(), place: self.place.clone(), slots: self.slots.clone() }
    }

    pub fn status(&self) -> SendStatus {
        match &self.sent {
            None => SendStatus::NotSent,
            Some(_) if self.removed => SendStatus::Removed,
            Some(v) if *v == self.current() => SendStatus::Sent,
            Some(_) => SendStatus::Changed,
        }
    }

    /// Has someone in a supervisor place.
    pub fn has_supervisor(&self) -> bool {
        self.slots.iter().any(|s| s.role == Role::Supervisor && s.person.is_some())
    }

    /// (filled, places).
    pub fn filled(&self) -> (usize, usize) {
        (self.slots.iter().filter(|s| s.person.is_some()).count(), self.slots.len())
    }
}

/// Can someone take a place in a shift?
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Fit {
    Available,
    /// Has the privilege but is not ticked for this preset.
    NotForThisSlot,
    /// Already in another shift that day (or in this one).
    BusyThatDay,
    Away,
    /// The place is for a supervisor and they are not one.
    NotSupervisor,
    /// The place is for a brother (or a sister).
    WrongGender,
    NoPrivilege,
}

impl Fit {
    pub fn label(self) -> &'static str {
        match self {
            Fit::Available => "Available",
            Fit::NotForThisSlot => "Not available at this time",
            Fit::BusyThatDay => "Already has a shift that day",
            Fit::Away => "Away",
            Fit::NotSupervisor => "Not a shift supervisor",
            Fit::WrongGender => "Other gender",
            Fit::NoPrivilege => "Without the privilege",
        }
    }
}

/// The shift form.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ShiftDraft {
    pub date: String,
    pub start: String,
    pub end: String,
    pub place: String,
    pub preset: Option<u32>,
    pub slots: Vec<Slot>,
}

/// What filling or assigning did.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ShiftFill {
    pub added: usize,
    /// Places given a person.
    pub assigned: usize,
    /// Places left empty because nobody fitted.
    pub empty: usize,
    pub skipped_days: Vec<String>,
}

/// One row of the statistics per person.
#[derive(Clone, Debug, PartialEq)]
pub struct PersonShifts {
    pub person: u32,
    pub shifts: usize,
    pub as_supervisor: usize,
}

fn day_number(iso: &str) -> Option<i64> {
    parse_date(iso).map(crate::prefs::days_from_ymd)
}

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

fn validate_times(start: &str, end: &str, place: &str) -> Vec<(&'static str, String)> {
    let mut e = Vec::new();
    if !valid_time(start) {
        e.push(("start", "Enter the start, for example 09:00.".to_string()));
    }
    if !valid_time(end) {
        e.push(("end", "Enter the end, for example 11:00.".into()));
    } else if valid_time(start) && end <= start {
        e.push(("end", "The end must be after the start.".into()));
    }
    if place.trim().is_empty() {
        e.push(("place", "Enter the place.".into()));
    }
    e
}

/// Resizes the places of a shift to `needs`, keeping the people already chosen where they fit.
pub fn resize_slots(slots: &[Slot], needs: Needs) -> Vec<Slot> {
    Role::ALL
        .iter()
        .flat_map(|r| {
            let mut mine: Vec<Slot> = slots.iter().copied().filter(|s| s.role == *r).collect();
            mine.sort_by_key(|s| s.person.is_none());
            mine.resize(needs.get(*r) as usize, Slot { role: *r, person: None });
            mine
        })
        .collect()
}

impl Directory {
    // ----- settings and presets -----

    /// People with the privilege "Public witnessing", by name.
    pub fn witnesses(&self) -> Vec<&Person> {
        let mut v: Vec<&Person> =
            self.people.iter().filter(|p| p.moved_away.is_none() && p.privileges.contains(&Privilege::PublicWitnessing)).collect();
        v.sort_by_cached_key(|p| crate::people::fold(&format!("{} {}", p.first_name, p.last_name)));
        v
    }

    pub fn shift_preset(&self, id: u32) -> Option<&ShiftPreset> {
        self.shift_presets.iter().find(|p| p.id == id)
    }

    pub fn shift_presets_sorted(&self, first_weekday: u8) -> Vec<&ShiftPreset> {
        let mut v: Vec<&ShiftPreset> = self.shift_presets.iter().collect();
        v.sort_by_key(|p| ((p.weekday + 7 - first_weekday) % 7, p.start.clone(), p.place.clone()));
        v
    }

    #[allow(clippy::too_many_arguments)]
    pub fn save_shift_preset(
        &mut self,
        id: Option<u32>,
        weekday: u8,
        start: &str,
        end: &str,
        place: &str,
        needs: Needs,
        available: BTreeSet<u32>,
        enabled: bool,
    ) -> Result<u32, Vec<(&'static str, String)>> {
        let mut e = validate_times(start, end, place);
        if weekday > 6 {
            e.push(("weekday", "Choose a day.".into()));
        }
        let needs = if self.witnessing.supervisors_on { needs } else { Needs { supervisors: 0, ..needs } };
        if needs.total() == 0 {
            e.push(("needs", "A shift needs at least one person.".into()));
        }
        if !e.is_empty() {
            return Err(e);
        }
        let new_id = id.unwrap_or_else(|| self.shift_presets.iter().map(|p| p.id).max().unwrap_or(0) + 1);
        let p = ShiftPreset { id: new_id, weekday, start: start.into(), end: end.into(), place: place.trim().into(), needs, available, enabled };
        match self.shift_presets.iter_mut().find(|x| x.id == new_id) {
            Some(slot) => *slot = p,
            None => self.shift_presets.push(p),
        }
        Ok(new_id)
    }

    pub fn set_shift_preset_enabled(&mut self, id: u32, enabled: bool) {
        if let Some(p) = self.shift_presets.iter_mut().find(|p| p.id == id) {
            p.enabled = enabled;
        }
    }

    pub fn delete_shift_preset(&mut self, id: u32) {
        self.shift_presets.retain(|p| p.id != id);
        for s in self.shifts.iter_mut().filter(|s| s.preset == Some(id)) {
            s.preset = None;
        }
    }

    /// Switches shift supervisors on or off. Off: the places for supervisors are removed from the
    /// presets (shifts already made keep theirs until they are changed).
    pub fn set_supervisors_on(&mut self, on: bool) {
        self.witnessing.supervisors_on = on;
        if !on {
            for p in self.shift_presets.iter_mut() {
                p.needs.supervisors = 0;
            }
        }
    }

    pub fn set_supervisors(&mut self, people: BTreeSet<u32>) {
        self.witnessing.supervisors = people;
    }

    // ----- who fits -----

    /// Whether `person` can take a place of `role` in a shift on `date` (from `preset`), not
    /// counting the shift `shift` itself.
    pub fn shift_fit(&self, person: u32, role: Role, date: &str, preset: Option<u32>, shift: Option<u32>) -> Fit {
        let Some(p) = self.person(person) else { return Fit::NoPrivilege };
        if self.is_away(person, date) {
            return Fit::Away;
        }
        match role {
            Role::Man if p.gender != Gender::Male => return Fit::WrongGender,
            Role::Woman if p.gender != Gender::Female => return Fit::WrongGender,
            Role::Supervisor if !self.witnessing.supervisors.contains(&person) => return Fit::NotSupervisor,
            _ => {}
        }
        if !p.privileges.contains(&Privilege::PublicWitnessing) {
            return Fit::NoPrivilege;
        }
        if preset.and_then(|id| self.shift_preset(id)).is_some_and(|s| !s.available.contains(&person)) {
            return Fit::NotForThisSlot;
        }
        if self.shifts.iter().any(|s| Some(s.id) != shift && !s.removed && s.date == date && s.slots.iter().any(|x| x.person == Some(person))) {
            return Fit::BusyThatDay;
        }
        Fit::Available
    }

    // ----- shifts -----

    pub fn shift(&self, id: u32) -> Option<&Shift> {
        self.shifts.iter().find(|s| s.id == id)
    }

    pub fn shifts_in(&self, ym: Ym) -> Vec<&Shift> {
        let mut v: Vec<&Shift> = self.shifts.iter().filter(|s| month_of(&s.date) == Some(ym)).collect();
        v.sort_by(|a, b| (&a.date, &a.start, &a.place).cmp(&(&b.date, &b.start, &b.place)));
        v
    }

    pub fn sent_shifts_in(&self, ym: Ym) -> Vec<(u32, ShiftVersion)> {
        let mut v: Vec<(u32, ShiftVersion)> =
            self.shifts.iter().filter_map(|s| s.sent.clone().map(|v| (s.id, v))).filter(|(_, v)| month_of(&v.date) == Some(ym)).collect();
        v.sort_by(|a, b| (&a.1.date, &a.1.start, &a.1.place).cmp(&(&b.1.date, &b.1.start, &b.1.place)));
        v
    }

    pub fn save_shift(&mut self, id: Option<u32>, d: &ShiftDraft) -> Result<u32, Vec<(&'static str, String)>> {
        let mut e = validate_times(&d.start, &d.end, &d.place);
        if day_number(&d.date).is_none() || d.date.len() != 10 {
            e.insert(0, ("date", "Enter the date.".into()));
        }
        if d.slots.is_empty() {
            e.push(("slots", "Add at least one person.".into()));
        }
        let mut seen = BTreeSet::new();
        if d.slots.iter().filter_map(|s| s.person).any(|p| !seen.insert(p)) {
            e.push(("slots", "Someone is in two places of this shift.".into()));
        }
        if !e.is_empty() {
            return Err(e);
        }
        let new_id = id.unwrap_or_else(|| self.shifts.iter().map(|s| s.id).max().unwrap_or(0) + 1);
        let preset = d.preset.filter(|p| self.shift_preset(*p).is_some());
        match self.shifts.iter_mut().find(|s| s.id == new_id) {
            Some(s) => {
                s.date = d.date.clone();
                s.start = d.start.clone();
                s.end = d.end.clone();
                s.place = d.place.trim().into();
                s.preset = preset;
                s.slots = d.slots.clone();
                s.removed = false;
            }
            None => self.shifts.push(Shift {
                id: new_id,
                date: d.date.clone(),
                start: d.start.clone(),
                end: d.end.clone(),
                place: d.place.trim().into(),
                preset,
                slots: d.slots.clone(),
                sent: None,
                removed: false,
            }),
        }
        Ok(new_id)
    }

    pub fn delete_shift(&mut self, id: u32) {
        if let Some(s) = self.shifts.iter_mut().find(|s| s.id == id)
            && s.sent.is_some()
        {
            s.removed = true;
            return;
        }
        self.shifts.retain(|s| s.id != id);
    }

    pub fn keep_shift(&mut self, id: u32) {
        if let Some(s) = self.shifts.iter_mut().find(|s| s.id == id) {
            s.removed = false;
        }
    }

    /// Adds a shift for every switched-on preset on every matching day of the month without one
    /// (same day, start and place), except days when the whole congregation is away. With
    /// `assign`, then fills the empty places of the month.
    pub fn fill_shift_month(&mut self, ym: Ym, assign: bool) -> ShiftFill {
        let mut r = ShiftFill::default();
        let first = crate::prefs::days_from_ymd((ym.0, ym.1, 1));
        let next = crate::reports::shift(ym, 1);
        let last = crate::prefs::days_from_ymd((next.0, next.1, 1)) - 1;
        let presets: Vec<ShiftPreset> = self.shift_presets.iter().filter(|p| p.enabled).cloned().collect();
        for z in first..=last {
            let (y, m, d) = crate::prefs::ymd_from_days(z);
            let date = format!("{y:04}-{m:02}-{d:02}");
            let weekday = (z + 4).rem_euclid(7) as u8;
            let todays: Vec<&ShiftPreset> = presets.iter().filter(|p| p.weekday == weekday).collect();
            if todays.is_empty() {
                continue;
            }
            if self.congregation_away(&date) {
                r.skipped_days.push(date);
                continue;
            }
            for p in todays {
                let exists = self.shifts.iter().any(|s| !s.removed && s.date == date && s.start == p.start && s.place.eq_ignore_ascii_case(&p.place));
                if exists {
                    continue;
                }
                let draft = ShiftDraft { date: date.clone(), start: p.start.clone(), end: p.end.clone(), place: p.place.clone(), preset: Some(p.id), slots: p.needs.slots() };
                if self.save_shift(None, &draft).is_ok() {
                    r.added += 1;
                }
            }
        }
        if assign {
            let (a, e) = self.assign_shifts(ym);
            r.assigned = a;
            r.empty = e;
        }
        r
    }

    /// Fills every empty place of the month's shifts, keeping the people already chosen:
    /// supervisors first, then brothers, sisters and anyone. Only people who fit (privilege, no
    /// absence, available for the preset, no other shift that day, the right gender, a
    /// supervisor for supervisor places). Among them: the fewest shifts that week, then that
    /// month, then the longest since or until their nearest other shift, then by name.
    /// Returns (places filled, places left empty).
    pub fn assign_shifts(&mut self, ym: Ym) -> (usize, usize) {
        let ids: Vec<u32> = self.shifts_in(ym).iter().filter(|s| !s.removed).map(|s| s.id).collect();
        let witnesses: Vec<u32> = self.witnesses().iter().map(|p| p.id).collect();
        let (mut filled, mut left) = (0, 0);
        for id in ids {
            let Some(s) = self.shift(id).cloned() else { continue };
            let mut order: Vec<usize> = (0..s.slots.len()).collect();
            order.sort_by_key(|i| s.slots[*i].role);
            for i in order {
                let Some(shift) = self.shift(id).cloned() else { continue };
                let slot = shift.slots[i];
                if slot.person.is_some() {
                    continue;
                }
                let week = week_of(&shift.date);
                let today = day_number(&shift.date).unwrap_or(0);
                let month: Vec<&Shift> = self.shifts_in(ym).into_iter().filter(|x| !x.removed).collect();
                let best = witnesses
                    .iter()
                    .copied()
                    .filter(|c| !shift.slots.iter().any(|x| x.person == Some(*c)))
                    .filter(|c| self.shift_fit(*c, slot.role, &shift.date, shift.preset, Some(id)) == Fit::Available)
                    .min_by_key(|c| {
                        let mine: Vec<&&Shift> = month.iter().filter(|x| x.slots.iter().any(|y| y.person == Some(*c))).collect();
                        let same_week = mine.iter().filter(|x| week_of(&x.date) == week).count();
                        let gap = self
                            .shifts
                            .iter()
                            .filter(|x| !x.removed && x.id != id && x.slots.iter().any(|y| y.person == Some(*c)))
                            .filter_map(|x| day_number(&x.date))
                            .map(|d| (d - today).abs())
                            .min()
                            .unwrap_or(i64::MAX / 2);
                        let name = self.person(*c).map(|p| crate::people::fold(&format!("{} {}", p.first_name, p.last_name))).unwrap_or_default();
                        (same_week, mine.len(), std::cmp::Reverse(gap), name)
                    });
                match best {
                    Some(c) => {
                        if let Some(x) = self.shifts.iter_mut().find(|x| x.id == id) {
                            x.slots[i].person = Some(c);
                        }
                        filled += 1;
                    }
                    None => left += 1,
                }
            }
        }
        (filled, left)
    }

    pub fn clear_shift_month(&mut self, ym: Ym) {
        let ids: Vec<u32> = self.shifts_in(ym).iter().map(|s| s.id).collect();
        for id in ids {
            self.delete_shift(id);
        }
    }

    pub fn unsent_shift_changes(&self, ym: Ym) -> Vec<(u32, SendStatus)> {
        self.shifts_in(ym).iter().map(|s| (s.id, s.status())).filter(|(_, s)| *s != SendStatus::Sent).collect()
    }

    /// Sends the month. Only the people concerned get a notice: added to a shift (new), removed
    /// from it (cancelled), or still in it when its time or place changed (changed).
    pub fn send_shift_month(&mut self, ym: Ym) -> usize {
        let ids: Vec<u32> = self.unsent_shift_changes(ym).iter().map(|(id, _)| *id).collect();
        for id in &ids {
            let Some(s) = self.shift(*id).cloned() else { continue };
            let now = s.current();
            let before = s.sent.clone();
            let after: BTreeSet<u32> = if s.removed { BTreeSet::new() } else { now.people().into_iter().collect() };
            let was: BTreeSet<u32> = before.as_ref().map(|b| b.people().into_iter().collect()).unwrap_or_default();
            let moved = before.as_ref().is_some_and(|b| (&b.date, &b.start, &b.end, &b.place) != (&now.date, &now.start, &now.end, &now.place));
            let detail = |v: &ShiftVersion| format!("{}–{} · {}", v.start, v.end, v.place);
            for p in after.difference(&was) {
                self.notify(*p, NoticeKind::New, AssignmentKind::PublicWitnessing, s.id, &now.date, None, detail(&now));
            }
            if let Some(b) = &before {
                for p in was.difference(&after) {
                    self.notify(*p, NoticeKind::Cancelled, AssignmentKind::PublicWitnessing, s.id, &b.date, None, detail(b));
                }
                if moved {
                    for p in after.intersection(&was) {
                        self.notify(*p, NoticeKind::Changed, AssignmentKind::PublicWitnessing, s.id, &now.date, None, detail(&now));
                    }
                }
            }
            if s.removed {
                self.shifts.retain(|x| x.id != *id);
            } else if let Some(x) = self.shifts.iter_mut().find(|x| x.id == *id) {
                x.sent = Some(x.current());
            }
        }
        ids.len()
    }

    // ----- statistics -----

    /// Shifts per person with the privilege between `from` and `to` (dates, both included),
    /// counting shifts that were sent. Most first, then by name; people with none are included.
    pub fn shifts_per_person(&self, from: &str, to: &str) -> Vec<PersonShifts> {
        let sent: Vec<&ShiftVersion> = self.shifts.iter().filter_map(|s| s.sent.as_ref()).filter(|v| v.date.as_str() >= from && v.date.as_str() <= to).collect();
        let mut rows: Vec<PersonShifts> = self
            .witnesses()
            .iter()
            .map(|p| PersonShifts {
                person: p.id,
                shifts: sent.iter().filter(|v| v.people().contains(&p.id)).count(),
                as_supervisor: sent.iter().filter(|v| v.slots.iter().any(|s| s.role == Role::Supervisor && s.person == Some(p.id))).count(),
            })
            .collect();
        rows.sort_by_key(|r| std::cmp::Reverse(r.shifts));
        rows
    }

    /// Sent shifts between two dates, and their places (filled, total).
    pub fn shift_totals(&self, from: &str, to: &str) -> (usize, usize, usize) {
        let sent: Vec<&ShiftVersion> = self.shifts.iter().filter_map(|s| s.sent.as_ref()).filter(|v| v.date.as_str() >= from && v.date.as_str() <= to).collect();
        let places: usize = sent.iter().map(|v| v.slots.len()).sum();
        let filled: usize = sent.iter().map(|v| v.people().len()).sum();
        (sent.len(), filled, places)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Witnesses: men 2, 3 and women 4, 5 (by setting genders); 2 can supervise.
    fn setup() -> Directory {
        let mut d = crate::demo::demo_directory(10);
        d.shift_presets.clear();
        d.shifts.clear();
        d.absences.clear();
        d.notices.clear();
        for p in d.people.iter_mut() {
            p.privileges.remove(&Privilege::PublicWitnessing);
            match p.id {
                2 | 3 => {
                    p.gender = Gender::Male;
                    p.privileges.insert(Privilege::PublicWitnessing);
                }
                4 | 5 => {
                    p.gender = Gender::Female;
                    p.privileges.insert(Privilege::PublicWitnessing);
                }
                _ => {}
            }
        }
        d.set_supervisors_on(true);
        d.set_supervisors([2].into_iter().collect());
        let all: BTreeSet<u32> = [2, 3, 4, 5].into_iter().collect();
        // Mondays (5 in November 2026): 1 supervisor, 1 sister, 1 anyone.
        let needs = Needs { supervisors: 1, men: 0, women: 1, anyone: 1 };
        assert!(d.save_shift_preset(None, 1, "09:00", "11:00", "Market square", needs, all, true).is_ok());
        d
    }

    #[test]
    fn needs_make_places() {
        let n = Needs { supervisors: 1, men: 1, women: 1, anyone: 0 };
        assert_eq!(n.describe(), "1 supervisor, 1 brother, 1 sister");
        assert_eq!(n.slots().iter().map(|s| s.role).collect::<Vec<_>>(), vec![Role::Supervisor, Role::Man, Role::Woman]);
        let slots = vec![Slot { role: Role::Man, person: None }, Slot { role: Role::Man, person: Some(7) }];
        let resized = resize_slots(&slots, Needs { men: 1, women: 1, ..Default::default() });
        assert_eq!(resized, vec![Slot { role: Role::Man, person: Some(7) }, Slot { role: Role::Woman, person: None }]);
    }

    #[test]
    fn assignment_respects_roles_and_rules() {
        let mut d = setup();
        d.save_absence(None, Some(4), "2026-11-02", "2026-11-02", "").ok();
        let r = d.fill_shift_month((2026, 11), true);
        assert_eq!(r.added, 5);
        for s in d.shifts_in((2026, 11)) {
            for slot in &s.slots {
                let Some(p) = slot.person else { continue };
                match slot.role {
                    Role::Supervisor => assert_eq!(p, 2),
                    Role::Woman => assert!([4, 5].contains(&p)),
                    _ => {}
                }
                assert!(!(p == 4 && s.date == "2026-11-02"), "away");
            }
            let people = s.current().people();
            let unique: BTreeSet<u32> = people.iter().copied().collect();
            assert_eq!(people.len(), unique.len(), "nobody twice in a shift");
        }
        assert_eq!(r.assigned + r.empty, 15);
        // Without supervisors switched on, presets lose their supervisor places.
        d.set_supervisors_on(false);
        assert_eq!(d.shift_presets[0].needs.supervisors, 0);
    }

    #[test]
    fn nobody_fits_leaves_places_empty() {
        let mut d = setup();
        let p = d.shift_presets[0].clone();
        d.save_shift_preset(Some(p.id), p.weekday, &p.start, &p.end, &p.place, p.needs, BTreeSet::new(), true).ok();
        let r = d.fill_shift_month((2026, 11), true);
        assert_eq!((r.assigned, r.empty), (0, 15));
    }

    #[test]
    fn sending_notifies_only_the_people_concerned() {
        let mut d = setup();
        d.fill_shift_month((2026, 11), true);
        d.send_shift_month((2026, 11));
        let s = d.shifts_in((2026, 11))[0].clone();
        let people = s.current().people();
        assert!(people.iter().all(|p| d.my_notices(*p).iter().any(|n| n.record == s.id && n.kind == NoticeKind::New)));
        for p in [2, 3, 4, 5] {
            d.mark_notices_seen(p);
        }
        // Swap one person out: they hear "cancelled", the new one "new", the others nothing.
        let out = people[people.len() - 1];
        let into = [2, 3, 4, 5].into_iter().find(|p| !people.contains(p)).unwrap_or(0);
        let mut slots = s.slots.clone();
        if let Some(x) = slots.iter_mut().find(|x| x.person == Some(out)) {
            x.person = Some(into);
        }
        let draft = ShiftDraft { date: s.date.clone(), start: s.start.clone(), end: s.end.clone(), place: s.place.clone(), preset: s.preset, slots };
        assert!(d.save_shift(Some(s.id), &draft).is_ok());
        d.send_shift_month((2026, 11));
        assert_eq!(d.my_notices(out).iter().map(|n| n.kind).collect::<Vec<_>>(), vec![NoticeKind::Cancelled]);
        assert_eq!(d.my_notices(into).iter().filter(|n| n.record == s.id).map(|n| n.kind).collect::<Vec<_>>(), vec![NoticeKind::New]);
        let stayed = people.iter().find(|p| **p != out).copied().unwrap_or(0);
        assert!(d.my_notices(stayed).iter().all(|n| n.record != s.id));
        // Statistics count sent shifts.
        let (shifts, filled, places) = d.shift_totals("2026-11-01", "2026-11-30");
        assert_eq!((shifts, places), (5, 15));
        assert!(filled <= places);
        assert_eq!(d.shifts_per_person("2026-11-01", "2026-11-30").len(), 4);
    }
}
