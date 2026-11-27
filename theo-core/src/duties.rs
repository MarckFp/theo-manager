//! M-05 Duties: the *Departments* section (D-28). Audio, video and stage, attendants and
//! microphones are scheduled **per meeting** (midweek and weekend) from **duty types**, which are
//! data (I-10): a congregation can add its own (for example sign-language interpreting). Cleaning
//! goes by field service group in a rota; maintenance is a list of tasks with due dates.
//! Rules: `docs/model/m05-duties.md`.

use std::collections::BTreeMap;

use crate::assignments::AssignmentKind;
use crate::midweek::meeting_date;
use crate::people::{fold, Directory};
use crate::privileges::Privilege;
use crate::scheduling::{self, Candidate, Fit, SentPlan};
use crate::service_meetings::SendStatus;

/// The most people one duty can have at one meeting.
pub const MAX_SLOTS: u8 = 10;

/// A page of *Departments*.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize)]
pub enum Department {
    #[default]
    AudioVideo,
    Attendants,
    Microphones,
    Cleaning,
    Maintenance,
}

impl Department {
    pub const ALL: [Department; 5] = [Department::AudioVideo, Department::Attendants, Department::Microphones, Department::Cleaning, Department::Maintenance];
    /// The departments whose duties are scheduled per meeting.
    pub const SCHEDULED: [Department; 3] = [Department::AudioVideo, Department::Attendants, Department::Microphones];

    /// The page id under `/app/departments/`.
    pub fn key(self) -> &'static str {
        match self {
            Department::AudioVideo => "audio-video",
            Department::Attendants => "attendants",
            Department::Microphones => "microphones",
            Department::Cleaning => "cleaning",
            Department::Maintenance => "maintenance",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Department::AudioVideo => "Audio, video and stage",
            Department::Attendants => "Attendants",
            Department::Microphones => "Microphones",
            Department::Cleaning => "Cleaning",
            Department::Maintenance => "Maintenance",
        }
    }
}

/// Which meeting.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize)]
pub enum Meeting {
    Midweek,
    Weekend,
}

impl Meeting {
    pub fn label(self) -> &'static str {
        match self {
            Meeting::Midweek => "Midweek meeting",
            Meeting::Weekend => "Weekend meeting",
        }
    }
}

/// Who may have a duty: everyone with a privilege, or a list of people (custom duties).
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum Eligibility {
    Privilege(Privilege),
    People(Vec<u32>),
}

impl Default for Eligibility {
    fn default() -> Self {
        Eligibility::People(Vec::new())
    }
}

/// A duty type (`duty_type`): what is done at a meeting, by how many people.
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct DutyType {
    pub id: u32,
    pub department: Department,
    pub name: String,
    /// How many people at one meeting.
    pub slots: u8,
    pub eligible: Eligibility,
    /// At which meetings.
    pub midweek: bool,
    pub weekend: bool,
    /// Order inside the department.
    pub position: u32,
}

impl DutyType {
    pub fn at(&self, meeting: Meeting) -> bool {
        match meeting {
            Meeting::Midweek => self.midweek,
            Meeting::Weekend => self.weekend,
        }
    }
}

/// One filled place: `index` is 0 .. `slots` of the duty type.
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct DutySlot {
    pub duty: u32,
    pub index: u8,
    pub person: u32,
}

/// The duties of one meeting (`duty_meeting`).
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct DutyMeeting {
    pub id: u32,
    /// Monday of the week, `YYYY-MM-DD`.
    pub monday: String,
    pub meeting: Meeting,
    pub slots: Vec<DutySlot>,
    #[serde(default)]
    pub sent: Option<SentPlan>,
}

/// The cleaning rota: the field service groups take turns, `weeks_each` weeks each, from the
/// week of `start`. A week can be given to another group.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct CleaningSettings {
    /// Group ids in turn order.
    pub groups: Vec<u32>,
    /// Monday of the first group's first week.
    pub start: Option<String>,
    pub weeks_each: u8,
    /// (Monday, group) exceptions.
    pub overrides: Vec<(String, u32)>,
}

impl Default for CleaningSettings {
    fn default() -> Self {
        CleaningSettings { groups: Vec::new(), start: None, weeks_each: 1, overrides: Vec::new() }
    }
}

/// A Kingdom Hall maintenance task (`maintenance_task`), one-off or repeating.
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct MaintenanceTask {
    pub id: u32,
    pub title: String,
    pub notes: Option<String>,
    /// Repeats every this many days after it is done; None: once.
    pub every_days: Option<u16>,
    /// When it is due next; None once a one-off task is done.
    pub due: Option<String>,
    pub last_done: Option<String>,
    /// Who looks after it.
    pub people: Vec<u32>,
}

/// The duty-type form.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct DutyTypeDraft {
    pub department: Department,
    pub name: String,
    pub slots: u8,
    pub eligible: Eligibility,
    pub midweek: bool,
    pub weekend: bool,
}

/// The maintenance-task form.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct MaintenanceDraft {
    pub title: String,
    pub notes: String,
    pub every_days: Option<u16>,
    pub due: String,
    pub people: Vec<u32>,
}

/// The duty types a new congregation starts with.
pub fn default_duty_types() -> Vec<DutyType> {
    use Department as D;
    use Privilege as P;
    let rows: [(D, &str, u8, P, bool, bool); 8] = [
        (D::AudioVideo, "Audio", 1, P::Audio, true, true),
        (D::AudioVideo, "Video", 1, P::Video, true, true),
        (D::AudioVideo, "Stage", 1, P::Stage, true, true),
        (D::AudioVideo, "Zoom host", 1, P::ZoomAttendant, true, true),
        (D::AudioVideo, "Zoom attendant", 1, P::ZoomAttendant, true, true),
        (D::Attendants, "Entrance", 1, P::EntranceAttendant, true, true),
        (D::Attendants, "Hall", 2, P::Attendant, true, true),
        (D::Microphones, "Roving microphones", 2, P::Microphones, true, true),
    ];
    rows.iter()
        .enumerate()
        .map(|(i, (department, name, slots, p, midweek, weekend))| DutyType {
            id: i as u32 + 1,
            department: *department,
            name: (*name).into(),
            slots: *slots,
            eligible: Eligibility::Privilege(*p),
            midweek: *midweek,
            weekend: *weekend,
            position: i as u32,
        })
        .collect()
}

impl Directory {
    // ----- duty types -----

    /// Adds the default duty types when the congregation has none yet.
    pub fn ensure_duty_types(&mut self) {
        if self.duty_types.is_empty() {
            self.duty_types = default_duty_types();
        }
    }

    pub fn duty_type(&self, id: u32) -> Option<&DutyType> {
        self.duty_types.iter().find(|t| t.id == id)
    }

    /// The duty types of a department, in order.
    pub fn duty_types_of(&self, department: Department) -> Vec<&DutyType> {
        let mut v: Vec<&DutyType> = self.duty_types.iter().filter(|t| t.department == department).collect();
        v.sort_by_key(|t| (t.position, t.id));
        v
    }

    /// Adds (`id` None) or changes a duty type. Names are unique in a department; scheduled
    /// departments only; 1 to 10 people; at least one meeting.
    pub fn save_duty_type(&mut self, id: Option<u32>, d: &DutyTypeDraft) -> Result<u32, String> {
        let name = d.name.trim();
        if name.is_empty() {
            return Err("Enter a name for the duty.".into());
        }
        if !Department::SCHEDULED.contains(&d.department) {
            return Err("Cleaning and maintenance are not scheduled per meeting.".into());
        }
        if self.duty_types.iter().any(|t| Some(t.id) != id && t.department == d.department && fold(&t.name) == fold(name)) {
            return Err("This department already has a duty with this name.".into());
        }
        if !(1..=MAX_SLOTS).contains(&d.slots) {
            return Err(format!("A duty has 1 to {MAX_SLOTS} people per meeting."));
        }
        if !d.midweek && !d.weekend {
            return Err("Choose at least one meeting.".into());
        }
        let eligible = match &d.eligible {
            Eligibility::People(list) => Eligibility::People(list.iter().copied().filter(|p| self.person(*p).is_some_and(|x| x.moved_away.is_none())).collect()),
            e => e.clone(),
        };
        let new_id = id.unwrap_or_else(|| self.duty_types.iter().map(|t| t.id).max().unwrap_or(0) + 1);
        let position = self.duty_type(new_id).map(|t| t.position).unwrap_or_else(|| self.duty_types.iter().map(|t| t.position + 1).max().unwrap_or(0));
        let t = DutyType { id: new_id, department: d.department, name: name.into(), slots: d.slots, eligible, midweek: d.midweek, weekend: d.weekend, position };
        match self.duty_types.iter_mut().find(|x| x.id == new_id) {
            Some(slot) => *slot = t,
            None => self.duty_types.push(t),
        }
        // Fewer places: the extra people of meetings not sent yet are removed.
        let slots = d.slots;
        for m in self.duty_meetings.iter_mut().filter(|m| m.sent.is_none()) {
            m.slots.retain(|s| s.duty != new_id || s.index < slots);
        }
        Ok(new_id)
    }

    /// Deletes a duty type and its places in meetings not sent yet.
    pub fn delete_duty_type(&mut self, id: u32) {
        self.duty_types.retain(|t| t.id != id);
        for m in self.duty_meetings.iter_mut().filter(|m| m.sent.is_none()) {
            m.slots.retain(|s| s.duty != id);
        }
    }

    /// Moves a duty type up (`delta` -1) or down (+1) in its department.
    pub fn move_duty_type(&mut self, id: u32, delta: i32) {
        let Some(dept) = self.duty_type(id).map(|t| t.department) else { return };
        let order: Vec<u32> = self.duty_types_of(dept).iter().map(|t| t.id).collect();
        let Some(i) = order.iter().position(|x| *x == id) else { return };
        let j = i as i64 + i64::from(delta);
        if j < 0 || j as usize >= order.len() {
            return;
        }
        let mut order = order;
        order.swap(i, j as usize);
        for (pos, tid) in order.iter().enumerate() {
            if let Some(t) = self.duty_types.iter_mut().find(|t| t.id == *tid) {
                t.position = pos as u32;
            }
        }
    }

    /// May the person do the duty: the privilege, or on the list; never someone who moved away.
    pub fn may_do_duty(&self, t: &DutyType, person: u32) -> bool {
        let Some(p) = self.person(person).filter(|p| p.moved_away.is_none()) else { return false };
        match &t.eligible {
            Eligibility::Privilege(x) => p.privileges.contains(x),
            Eligibility::People(list) => list.contains(&person),
        }
    }

    // ----- meetings -----

    pub fn duty_meeting(&self, monday: &str, meeting: Meeting) -> Option<&DutyMeeting> {
        self.duty_meetings.iter().find(|m| m.monday == monday && m.meeting == meeting)
    }

    fn duty_meeting_mut(&mut self, monday: &str, meeting: Meeting) -> &mut DutyMeeting {
        if let Some(i) = self.duty_meetings.iter().position(|m| m.monday == monday && m.meeting == meeting) {
            return &mut self.duty_meetings[i];
        }
        let id = self.duty_meetings.iter().map(|m| m.id).max().unwrap_or(0) + 1;
        self.duty_meetings.push(DutyMeeting { id, monday: monday.into(), meeting, slots: Vec::new(), sent: None });
        let last = self.duty_meetings.len() - 1;
        &mut self.duty_meetings[last]
    }

    /// No meeting that week: the week of that meeting says so.
    pub fn no_duty_meeting(&self, monday: &str, meeting: Meeting) -> bool {
        match meeting {
            Meeting::Midweek => self.midweek_week(monday).is_some_and(|w| w.no_meeting),
            Meeting::Weekend => self.weekend_week(monday).is_some_and(|w| w.no_meeting),
        }
    }

    /// Every place of a meeting, (duty, index, person), in department and duty order.
    pub fn duty_places(&self, monday: &str, meeting: Meeting) -> Vec<(u32, u8, Option<u32>)> {
        if self.no_duty_meeting(monday, meeting) {
            return Vec::new();
        }
        let filled = self.duty_meeting(monday, meeting);
        let mut v = Vec::new();
        for dept in Department::SCHEDULED {
            for t in self.duty_types_of(dept).into_iter().filter(|t| t.at(meeting)) {
                for i in 0..t.slots {
                    let person = filled.and_then(|m| m.slots.iter().find(|s| s.duty == t.id && s.index == i)).map(|s| s.person);
                    v.push((t.id, i, person));
                }
            }
        }
        v
    }

    /// "Audio" or, for a duty with more people, "Hall attendant 2".
    pub fn duty_label(&self, duty: u32, index: u8) -> String {
        match self.duty_type(duty) {
            Some(t) if t.slots > 1 => format!("{} {}", t.name, index + 1),
            Some(t) => t.name.clone(),
            None => "Removed duty".into(),
        }
    }

    /// Fills or empties a place. Refused for people who may not do the duty.
    pub fn assign_duty(&mut self, monday: &str, meeting: Meeting, duty: u32, index: u8, person: Option<u32>) -> Result<(), String> {
        let t = self.duty_type(duty).cloned().ok_or("This duty no longer exists.")?;
        if index >= t.slots {
            return Err("This duty has fewer places.".into());
        }
        if let Some(p) = person.filter(|p| !self.may_do_duty(&t, *p)) {
            let name = self.person(p).map(|x| format!("{} {}", x.first_name, x.last_name)).unwrap_or_default();
            return Err(format!("{name} may not do this duty."));
        }
        let m = self.duty_meeting_mut(monday, meeting);
        m.slots.retain(|s| !(s.duty == duty && s.index == index));
        if let Some(p) = person {
            m.slots.push(DutySlot { duty, index, person: p });
        }
        Ok(())
    }

    /// The duties a person has at a meeting ("Audio", "Hall 2").
    pub fn duty_roles_of(&self, monday: &str, meeting: Meeting, person: u32) -> Vec<String> {
        self.duty_places(monday, meeting).into_iter().filter(|(_, _, p)| *p == Some(person)).map(|(d, i, _)| self.duty_label(d, i)).collect()
    }

    /// What else a person does at that meeting: other duties and parts of the meeting.
    fn duty_busy(&self, monday: &str, meeting: Meeting, duty: u32, index: u8, person: u32) -> Vec<String> {
        let mut busy: Vec<String> =
            self.duty_places(monday, meeting).into_iter().filter(|(d, i, p)| *p == Some(person) && (*d, *i) != (duty, index)).map(|(d, i, _)| self.duty_label(d, i)).collect();
        match meeting {
            Meeting::Midweek => {
                if let Some(w) = self.midweek_week(monday) {
                    busy.extend(w.places().into_iter().filter(|(_, p)| *p == Some(person)).map(|(r, _)| w.role_label(r)));
                }
            }
            Meeting::Weekend => {
                if let Some(w) = self.weekend_week(monday) {
                    busy.extend(w.places_with(&self.weekend_settings).into_iter().filter(|(_, p)| *p == Some(person)).map(|(r, _)| r.label().to_string()));
                }
            }
        }
        busy
    }

    /// When each person last had each duty type, and any duty, before `monday` (both meetings).
    fn duty_history(&self, monday: &str) -> BTreeMap<u32, (String, BTreeMap<u32, String>)> {
        let mut out: BTreeMap<u32, (String, BTreeMap<u32, String>)> = BTreeMap::new();
        for m in self.duty_meetings.iter().filter(|m| m.monday.as_str() < monday) {
            for s in &m.slots {
                let e = out.entry(s.person).or_default();
                if e.0 < m.monday {
                    e.0 = m.monday.clone();
                }
                let last = e.1.entry(s.duty).or_default();
                if *last < m.monday {
                    *last = m.monday.clone();
                }
            }
        }
        out
    }

    /// Who can take a duty place, sorted (see [`scheduling::sort`]). `weekday` is the meeting's
    /// day from Monday (0 = Monday).
    pub fn duty_candidates(&self, monday: &str, meeting: Meeting, duty: u32, index: u8, weekday: i64) -> Vec<Candidate> {
        let Some(t) = self.duty_type(duty) else { return Vec::new() };
        let date = meeting_date(monday, weekday);
        let history = self.duty_history(monday);
        let mut v: Vec<Candidate> = self
            .people
            .iter()
            .filter(|p| self.may_do_duty(t, p.id))
            .map(|p| {
                let h = history.get(&p.id);
                Candidate {
                    person: p.id,
                    fit: if self.is_away(p.id, &date) { Fit::Away } else { Fit::Available },
                    busy: self.duty_busy(monday, meeting, duty, index, p.id),
                    last_this: h.and_then(|h| h.1.get(&duty).cloned()),
                    last_any: h.map(|h| h.0.clone()),
                }
            })
            .collect();
        scheduling::sort(self, &mut v);
        v
    }

    /// Fills the empty duty places of the weeks with the first free candidate. Returns (filled,
    /// still empty).
    pub fn autofill_duties(&mut self, mondays: &[String], meeting: Meeting, weekday: i64) -> (usize, usize) {
        let (mut filled, mut left) = (0, 0);
        let mut order = mondays.to_vec();
        order.sort();
        for monday in order {
            for (duty, index, _) in self.duty_places(&monday, meeting).into_iter().filter(|(_, _, p)| p.is_none()) {
                match self.duty_candidates(&monday, meeting, duty, index, weekday).into_iter().find(Candidate::is_free) {
                    Some(c) if self.assign_duty(&monday, meeting, duty, index, Some(c.person)).is_ok() => filled += 1,
                    _ => left += 1,
                }
            }
        }
        (filled, left)
    }

    /// Empties every place of the weeks (meetings not sent keep nothing; sent ones show the change).
    pub fn clear_duties(&mut self, mondays: &[String], meeting: Meeting) {
        for m in self.duty_meetings.iter_mut().filter(|m| m.meeting == meeting && mondays.contains(&m.monday)) {
            m.slots.clear();
        }
    }

    fn duty_roles(&self, monday: &str, meeting: Meeting) -> Vec<(u32, String)> {
        self.duty_places(monday, meeting).into_iter().filter_map(|(d, i, p)| p.map(|p| (p, self.duty_label(d, i)))).collect()
    }

    pub fn duty_status(&self, monday: &str, meeting: Meeting, weekday: i64) -> SendStatus {
        let sent = self.duty_meeting(monday, meeting).and_then(|m| m.sent.as_ref());
        scheduling::status(sent, &self.duty_roles(monday, meeting), self.no_duty_meeting(monday, meeting), &meeting_date(monday, weekday))
    }

    /// Sends the duties of the weeks; only the people whose duties changed get a notice.
    pub fn send_duties(&mut self, mondays: &[String], meeting: Meeting, weekday: i64) -> usize {
        let todo: Vec<String> = mondays.iter().filter(|m| self.duty_status(m, meeting, weekday) != SendStatus::Sent).cloned().collect();
        for monday in &todo {
            let roles = self.duty_roles(monday, meeting);
            let places = self.duty_places(monday, meeting).into_iter().filter_map(|(d, i, p)| p.map(|p| (format!("{d}:{i}"), p))).collect();
            let no_meeting = self.no_duty_meeting(monday, meeting);
            let date = meeting_date(monday, weekday);
            let (id, before) = {
                let m = self.duty_meeting_mut(monday, meeting);
                (m.id, m.sent.clone())
            };
            let sent = self.send_plan(AssignmentKind::Duty, id, before.as_ref(), &date, no_meeting, roles, places, &format!("Duties · {}", meeting.label()));
            self.duty_meeting_mut(monday, meeting).sent = Some(sent);
        }
        todo.len()
    }

    // ----- cleaning -----

    /// Saves the cleaning rota. Unknown groups are left out; at least one week each.
    pub fn set_cleaning(&mut self, mut c: CleaningSettings) -> Result<(), String> {
        c.groups.retain(|g| self.groups.iter().any(|x| x.id == *g));
        if c.weeks_each == 0 {
            return Err("Each group cleans at least one week.".into());
        }
        if let Some(s) = &c.start {
            if crate::people::parse_date(s).is_none() {
                return Err("Enter the first week of the rota.".into());
            }
            c.start = Some(crate::midweek::monday_of(s));
        }
        c.overrides.retain(|(_, g)| self.groups.iter().any(|x| x.id == *g));
        self.cleaning = c;
        Ok(())
    }

    /// The group that cleans in the week of `monday`: an exception, or the rota. None before the
    /// rota starts or without groups.
    pub fn cleaning_group(&self, monday: &str) -> Option<u32> {
        let c = &self.cleaning;
        if let Some((_, g)) = c.overrides.iter().find(|(m, _)| m == monday) {
            return Some(*g);
        }
        let start = c.start.as_deref()?;
        if c.groups.is_empty() || monday < start {
            return None;
        }
        let weeks = crate::events::days_between(start, monday)? / 7;
        let turn = weeks / i64::from(c.weeks_each.max(1));
        c.groups.get((turn % c.groups.len() as i64) as usize).copied()
    }

    // ----- maintenance -----

    pub fn maintenance_task(&self, id: u32) -> Option<&MaintenanceTask> {
        self.maintenance_tasks.iter().find(|t| t.id == id)
    }

    pub fn save_maintenance_task(&mut self, id: Option<u32>, d: &MaintenanceDraft) -> Result<u32, String> {
        let title = d.title.trim();
        if title.is_empty() {
            return Err("Enter what has to be done.".into());
        }
        let due = Some(d.due.trim().to_string()).filter(|s| !s.is_empty());
        if due.as_deref().is_some_and(|s| crate::people::parse_date(s).is_none()) {
            return Err("Enter the due date as a date.".into());
        }
        if d.every_days == Some(0) {
            return Err("A repeating task repeats after at least one day.".into());
        }
        let people = d.people.iter().copied().filter(|p| self.person(*p).is_some_and(|x| x.moved_away.is_none())).collect();
        let new_id = id.unwrap_or_else(|| self.maintenance_tasks.iter().map(|t| t.id).max().unwrap_or(0) + 1);
        let last_done = self.maintenance_task(new_id).and_then(|t| t.last_done.clone());
        let t = MaintenanceTask { id: new_id, title: title.into(), notes: Some(d.notes.trim().to_string()).filter(|n| !n.is_empty()), every_days: d.every_days, due, last_done, people };
        match self.maintenance_tasks.iter_mut().find(|x| x.id == new_id) {
            Some(slot) => *slot = t,
            None => self.maintenance_tasks.push(t),
        }
        Ok(new_id)
    }

    pub fn delete_maintenance_task(&mut self, id: u32) {
        self.maintenance_tasks.retain(|t| t.id != id);
    }

    /// Marks a task done on `date`: a repeating task is due again `every_days` later.
    pub fn maintenance_done(&mut self, id: u32, date: &str) {
        if let Some(t) = self.maintenance_tasks.iter_mut().find(|t| t.id == id) {
            t.last_done = Some(date.into());
            t.due = t.every_days.and_then(|n| crate::events::add_days(date, i64::from(n)));
        }
    }

    /// Tasks due on or before `today`, the most overdue first.
    pub fn maintenance_due(&self, today: &str) -> Vec<&MaintenanceTask> {
        let mut v: Vec<&MaintenanceTask> = self.maintenance_tasks.iter().filter(|t| t.due.as_deref().is_some_and(|d| d <= today)).collect();
        v.sort_by(|a, b| a.due.cmp(&b.due));
        v
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::assignments::NoticeKind;

    const THURSDAY: i64 = 3;

    fn setup() -> Directory {
        let mut d = crate::demo::demo_directory(30);
        d.duty_types.clear();
        d.duty_meetings.clear();
        d.maintenance_tasks.clear();
        d.weekend_weeks.clear();
        d.notices.clear();
        d.ensure_duty_types();
        d
    }

    #[test]
    fn duty_types_are_data_and_checked() {
        let mut d = setup();
        assert_eq!(d.duty_types_of(Department::AudioVideo).len(), 5);
        let sister = d.people.iter().find(|p| p.gender == crate::people::Gender::Female).map(|p| p.id).unwrap_or(0);
        let draft = DutyTypeDraft { department: Department::AudioVideo, name: "Sign language".into(), slots: 1, eligible: Eligibility::People(vec![sister, 999]), midweek: true, weekend: true };
        let id = d.save_duty_type(None, &draft).unwrap_or(0);
        assert_eq!(d.duty_type(id).map(|t| t.eligible.clone()), Some(Eligibility::People(vec![sister])));
        assert!(d.save_duty_type(None, &DutyTypeDraft { name: " sign LANGUAGE ".into(), ..draft.clone() }).is_err());
        assert!(d.save_duty_type(None, &DutyTypeDraft { name: "x".into(), slots: 0, ..draft.clone() }).is_err());
        assert!(d.save_duty_type(None, &DutyTypeDraft { name: "x".into(), midweek: false, weekend: false, ..draft.clone() }).is_err());
        assert!(d.save_duty_type(None, &DutyTypeDraft { name: "x".into(), department: Department::Cleaning, ..draft.clone() }).is_err());
        // Moving it up puts it before the last default type.
        d.move_duty_type(id, -1);
        assert_eq!(d.duty_types_of(Department::AudioVideo).iter().map(|t| t.name.as_str()).nth(4), Some("Sign language"));
        // Only the people on the list may do it.
        assert!(d.assign_duty("2026-12-07", Meeting::Midweek, id, 0, Some(sister)).is_ok());
        let other = d.people.iter().find(|p| p.id != sister).map(|p| p.id).unwrap_or(0);
        assert!(d.assign_duty("2026-12-07", Meeting::Midweek, id, 0, Some(other)).is_err());
    }

    #[test]
    fn autofill_fills_each_meeting_without_doubles_and_sends() {
        let mut d = setup();
        let weeks: Vec<String> = ["2026-12-07", "2026-12-14"].iter().map(|s| s.to_string()).collect();
        let (filled, _) = d.autofill_duties(&weeks, Meeting::Midweek, THURSDAY);
        assert!(filled > 0);
        for w in &weeks {
            let people: Vec<u32> = d.duty_places(w, Meeting::Midweek).iter().filter_map(|(_, _, p)| *p).collect();
            let unique: std::collections::BTreeSet<u32> = people.iter().copied().collect();
            assert_eq!(people.len(), unique.len(), "one duty per person per meeting");
            for (duty, _, p) in d.duty_places(w, Meeting::Midweek) {
                if let (Some(p), Some(t)) = (p, d.duty_type(duty)) {
                    assert!(d.may_do_duty(t, p));
                }
            }
        }
        assert_eq!(d.send_duties(&weeks, Meeting::Midweek, THURSDAY), 2);
        assert_eq!(d.duty_status(&weeks[0], Meeting::Midweek, THURSDAY), SendStatus::Sent);
        let (duty, index, person) = d.duty_places(&weeks[0], Meeting::Midweek).into_iter().find(|(_, _, p)| p.is_some()).unwrap_or_default();
        let person = person.unwrap_or(0);
        assert!(d.my_notices(person).iter().any(|n| n.kind == NoticeKind::New && n.what == AssignmentKind::Duty));
        assert!(d.all_my_assignments(person).iter().any(|a| a.kind == AssignmentKind::Duty && a.date == "2026-12-10"));
        // A change after sending shows as changed.
        let _ = d.assign_duty(&weeks[0], Meeting::Midweek, duty, index, None);
        assert_eq!(d.duty_status(&weeks[0], Meeting::Midweek, THURSDAY), SendStatus::Changed);
    }

    #[test]
    fn no_meeting_weeks_have_no_duties() {
        let mut d = setup();
        d.add_weekend_week("2026-12-07", 6);
        d.set_weekend_flags("2026-12-07", true, false);
        assert!(d.duty_places("2026-12-07", Meeting::Weekend).is_empty());
        assert!(!d.duty_places("2026-12-07", Meeting::Midweek).is_empty());
    }

    #[test]
    fn cleaning_goes_round_the_groups() {
        let mut d = setup();
        let groups: Vec<u32> = d.groups.iter().map(|g| g.id).take(2).collect();
        assert!(d.set_cleaning(CleaningSettings { groups: groups.clone(), start: Some("2026-12-09".into()), weeks_each: 2, overrides: vec![] }).is_ok());
        assert_eq!(d.cleaning.start.as_deref(), Some("2026-12-07"));
        assert_eq!(d.cleaning_group("2026-11-30"), None);
        assert_eq!(d.cleaning_group("2026-12-07"), groups.first().copied());
        assert_eq!(d.cleaning_group("2026-12-14"), groups.first().copied());
        assert_eq!(d.cleaning_group("2026-12-21"), groups.get(1).copied());
        assert_eq!(d.cleaning_group("2027-01-04"), groups.first().copied());
        let mut c = d.cleaning.clone();
        c.overrides.push(("2026-12-07".into(), groups[1]));
        assert!(d.set_cleaning(c).is_ok());
        assert_eq!(d.cleaning_group("2026-12-07"), groups.get(1).copied());
        assert!(d.set_cleaning(CleaningSettings { weeks_each: 0, ..Default::default() }).is_err());
    }

    #[test]
    fn maintenance_tasks_repeat_after_they_are_done() {
        let mut d = setup();
        let id = d.save_maintenance_task(None, &MaintenanceDraft { title: "Check the fire extinguishers".into(), every_days: Some(180), due: "2026-12-01".into(), ..Default::default() }).unwrap_or(0);
        let once = d.save_maintenance_task(None, &MaintenanceDraft { title: "Fix the door".into(), due: "2026-11-20".into(), ..Default::default() }).unwrap_or(0);
        assert!(d.save_maintenance_task(None, &MaintenanceDraft { title: "x".into(), due: "soon".into(), ..Default::default() }).is_err());
        assert_eq!(d.maintenance_due("2026-12-05").iter().map(|t| t.id).collect::<Vec<_>>(), vec![once, id]);
        d.maintenance_done(id, "2026-12-05");
        d.maintenance_done(once, "2026-12-05");
        assert_eq!(d.maintenance_task(id).and_then(|t| t.due.clone()).as_deref(), Some("2027-06-03"));
        assert_eq!(d.maintenance_task(once).and_then(|t| t.due.clone()), None);
        assert!(d.maintenance_due("2027-01-01").is_empty());
    }
}
