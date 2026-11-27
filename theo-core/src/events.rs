//! Events (M-12): conventions, assemblies, the Memorial, the circuit overseer's visit
//! and custom events, for everyone or for chosen people, optionally with absences.
//! Rules: `docs/model/m12-events.md`.

use crate::absences::{Absence, When};
use crate::people::Directory;

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum EventKind {
    RegionalConvention,
    CircuitAssembly,
    Memorial,
    CircuitOverseerVisit,
    Custom,
}

impl EventKind {
    pub const ALL: [EventKind; 5] =
        [EventKind::RegionalConvention, EventKind::CircuitAssembly, EventKind::Memorial, EventKind::CircuitOverseerVisit, EventKind::Custom];

    pub fn value(self) -> &'static str {
        match self {
            EventKind::RegionalConvention => "regional-convention",
            EventKind::CircuitAssembly => "circuit-assembly",
            EventKind::Memorial => "memorial",
            EventKind::CircuitOverseerVisit => "circuit-overseer-visit",
            EventKind::Custom => "custom",
        }
    }

    pub fn parse(s: &str) -> EventKind {
        Self::ALL.into_iter().find(|k| k.value() == s).unwrap_or(EventKind::Custom)
    }

    pub fn label(self) -> &'static str {
        match self {
            EventKind::RegionalConvention => "Regional convention",
            EventKind::CircuitAssembly => "Circuit assembly",
            EventKind::Memorial => "Memorial",
            EventKind::CircuitOverseerVisit => "Circuit overseer's visit",
            EventKind::Custom => "Custom event",
        }
    }

    /// The title a new event of this kind starts with (empty for a custom event).
    pub fn default_title(self) -> &'static str {
        match self {
            EventKind::Custom => "",
            k => k.label(),
        }
    }

    /// How many days an event of this kind usually lasts.
    pub fn default_days(self) -> i64 {
        match self {
            EventKind::RegionalConvention => 3,
            EventKind::CircuitOverseerVisit => 6,
            _ => 1,
        }
    }

    /// Whether the congregation is usually away from its own meetings: yes for conventions,
    /// assemblies and the Memorial (it replaces the meeting of that day).
    pub fn default_absence(self) -> bool {
        matches!(self, EventKind::RegionalConvention | EventKind::CircuitAssembly | EventKind::Memorial)
    }
}

/// An event. `people` None: the whole congregation; otherwise only those people (and the
/// administrators) see it.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Event {
    pub id: u32,
    pub kind: EventKind,
    pub title: String,
    pub description: Option<String>,
    /// `YYYY-MM-DD`.
    pub from: String,
    /// `YYYY-MM-DD`, not before `from`.
    pub to: String,
    pub people: Option<Vec<u32>>,
    /// The event creates absences on its days: one for the whole congregation, or one per person.
    pub absence: bool,
}

impl Event {
    pub fn when(&self, today: &str) -> When {
        if self.to.as_str() < today {
            When::Past
        } else if self.from.as_str() > today {
            When::Upcoming
        } else {
            When::Now
        }
    }

    pub fn is_private(&self) -> bool {
        self.people.is_some()
    }
}

/// The event form.
#[derive(Clone, Debug, PartialEq)]
pub struct EventDraft {
    pub kind: EventKind,
    pub title: String,
    pub description: String,
    pub from: String,
    pub to: String,
    pub everyone: bool,
    pub people: Vec<u32>,
    pub absence: bool,
}

impl EventDraft {
    pub fn new(kind: EventKind, from: &str) -> Self {
        let to = add_days(from, kind.default_days() - 1).unwrap_or_else(|| from.to_string());
        EventDraft {
            kind,
            title: kind.default_title().into(),
            description: String::new(),
            from: from.into(),
            to,
            everyone: true,
            people: Vec::new(),
            absence: kind.default_absence(),
        }
    }

    pub fn from_event(e: &Event) -> Self {
        EventDraft {
            kind: e.kind,
            title: e.title.clone(),
            description: e.description.clone().unwrap_or_default(),
            from: e.from.clone(),
            to: e.to.clone(),
            everyone: e.people.is_none(),
            people: e.people.clone().unwrap_or_default(),
            absence: e.absence,
        }
    }

    /// Another kind: the title, the length and the absence follow the preset (a title the user
    /// typed is kept).
    pub fn with_kind(mut self, kind: EventKind) -> Self {
        if self.title.trim().is_empty() || self.title == self.kind.default_title() {
            self.title = kind.default_title().into();
        }
        if let Some(to) = add_days(&self.from, kind.default_days() - 1) {
            self.to = to;
        }
        self.absence = kind.default_absence();
        self.kind = kind;
        self
    }

    /// Another first day: the event keeps its length.
    pub fn with_from(mut self, from: String) -> Self {
        let length = days_between(&self.from, &self.to);
        if let (Some(len), Some(to)) = (length, length.and_then(|l| add_days(&from, l)))
            && len >= 0
        {
            self.to = to;
        }
        self.from = from;
        self
    }
}

fn day_number(iso: &str) -> Option<i64> {
    crate::people::parse_date(iso).map(crate::prefs::days_from_ymd)
}

/// `YYYY-MM-DD` plus `n` days.
pub fn add_days(iso: &str, n: i64) -> Option<String> {
    let (y, m, d) = crate::prefs::ymd_from_days(day_number(iso)? + n);
    Some(format!("{y:04}-{m:02}-{d:02}"))
}

/// Days from `a` to `b` (negative when `b` is earlier).
pub(crate) fn days_between(a: &str, b: &str) -> Option<i64> {
    Some(day_number(b)? - day_number(a)?)
}

/// Number of days of an event (both ends included).
pub fn length_in_days(e: &Event) -> i64 {
    days_between(&e.from, &e.to).map(|d| d + 1).unwrap_or(1)
}

impl Directory {
    pub fn event(&self, id: u32) -> Option<&Event> {
        self.events.iter().find(|e| e.id == id)
    }

    /// Can this viewer see the event: everyone sees events of the whole congregation; private
    /// events only their people and administrators.
    pub fn can_see_event(&self, e: &Event, viewer: Option<u32>, admin: bool) -> bool {
        admin || e.people.as_ref().is_none_or(|p| viewer.is_some_and(|v| p.contains(&v)))
    }

    /// The events this viewer can see, by first day.
    pub fn visible_events(&self, viewer: Option<u32>, admin: bool) -> Vec<&Event> {
        let mut v: Vec<&Event> = self.events.iter().filter(|e| self.can_see_event(e, viewer, admin)).collect();
        v.sort_by(|a, b| (&a.from, &a.to).cmp(&(&b.from, &b.to)));
        v
    }

    /// Adds (`id` None) or changes an event, and brings its absences in line with it.
    pub fn save_event(&mut self, id: Option<u32>, d: &EventDraft) -> Result<u32, Vec<(&'static str, String)>> {
        let mut e = Vec::new();
        if d.title.trim().is_empty() {
            e.push(("title", "Enter a title.".to_string()));
        }
        let from_ok = day_number(&d.from).is_some() && d.from.len() == 10;
        let to_ok = day_number(&d.to).is_some() && d.to.len() == 10;
        if !from_ok {
            e.push(("from", "Enter the first day.".into()));
        }
        if !to_ok {
            e.push(("to", "Enter the last day.".into()));
        } else if from_ok && d.to < d.from {
            e.push(("to", "The last day cannot be before the first day.".into()));
        }
        let people: Vec<u32> = d.people.iter().copied().filter(|p| self.person(*p).is_some()).collect();
        if !d.everyone && people.is_empty() {
            e.push(("people", "Choose at least one person, or make it for the whole congregation.".into()));
        }
        if !e.is_empty() {
            return Err(e);
        }
        let new_id = id.unwrap_or_else(|| self.events.iter().map(|x| x.id).max().unwrap_or(0) + 1);
        let event = Event {
            id: new_id,
            kind: d.kind,
            title: d.title.trim().into(),
            description: Some(d.description.trim().to_string()).filter(|s| !s.is_empty()),
            from: d.from.clone(),
            to: d.to.clone(),
            people: (!d.everyone).then_some(people),
            absence: d.absence,
        };
        match self.events.iter_mut().find(|x| x.id == new_id) {
            Some(slot) => *slot = event,
            None => self.events.push(event),
        }
        self.sync_event_absences(new_id);
        Ok(new_id)
    }

    /// Deletes an event and the absences it created.
    pub fn delete_event(&mut self, id: u32) {
        self.events.retain(|e| e.id != id);
        self.absences.retain(|a| a.event != Some(id));
    }

    /// Replaces the absences of an event: one for the whole congregation, or one per chosen
    /// person, on the event's days (none when the event does not create absences).
    pub fn sync_event_absences(&mut self, id: u32) {
        self.absences.retain(|a| a.event != Some(id));
        let Some(e) = self.event(id).cloned() else { return };
        if !e.absence {
            return;
        }
        // A private event's title stays private: its absences carry no notes.
        let notes = (!e.is_private()).then(|| e.title.clone());
        let who: Vec<Option<u32>> = match &e.people {
            None => vec![None],
            Some(p) => p.iter().map(|p| Some(*p)).collect(),
        };
        let first = self.absences.iter().map(|a| a.id).max().unwrap_or(0) + 1;
        for (next, person) in (first..).zip(who) {
            self.absences.push(Absence { id: next, person, from: e.from.clone(), to: e.to.clone(), notes: notes.clone(), event: Some(id) });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn presets_fill_the_form() {
        let d = EventDraft::new(EventKind::RegionalConvention, "2027-07-09");
        assert_eq!((d.title.as_str(), d.to.as_str(), d.absence), ("Regional convention", "2027-07-11", true));
        let d = d.with_kind(EventKind::Memorial);
        assert_eq!((d.title.as_str(), d.to.as_str(), d.absence), ("Memorial", "2027-07-09", true));
        let d = d.with_kind(EventKind::Custom);
        assert_eq!((d.title.as_str(), d.absence), ("", false));
        // A typed title is kept; a new first day keeps the length.
        let mut d = EventDraft::new(EventKind::CircuitOverseerVisit, "2027-01-12");
        d.title = "Visit of brother Lee".into();
        let d = d.with_kind(EventKind::CircuitAssembly).with_kind(EventKind::CircuitOverseerVisit).with_from("2027-02-02".into());
        assert_eq!((d.title.as_str(), d.from.as_str(), d.to.as_str()), ("Visit of brother Lee", "2027-02-02", "2027-02-07"));
    }

    #[test]
    fn events_create_and_remove_their_absences() {
        let mut dir = crate::demo::demo_directory(20);
        dir.events.clear();
        dir.absences.clear();
        let mut d = EventDraft::new(EventKind::CircuitAssembly, "2027-03-06");
        let id = dir.save_event(None, &d).unwrap_or(0);
        assert!(dir.is_away(4, "2027-03-06") && dir.absences.len() == 1);
        // Private: only the chosen people are away, and only they (and admins) see it.
        d.everyone = false;
        d.people = vec![3, 4];
        assert!(dir.save_event(Some(id), &d).is_ok());
        assert!(dir.is_away(3, "2027-03-06") && !dir.is_away(5, "2027-03-06"));
        let e = dir.event(id).cloned();
        assert!(e.as_ref().is_some_and(|e| dir.can_see_event(e, Some(3), false)));
        assert!(e.as_ref().is_some_and(|e| !dir.can_see_event(e, Some(5), false) && dir.can_see_event(e, Some(5), true)));
        assert!(dir.absences.iter().all(|a| a.notes.is_none()));
        // Without people it cannot be private; without a title it cannot be saved.
        d.people.clear();
        assert!(dir.save_event(Some(id), &d).is_err());
        d.everyone = true;
        d.title = " ".into();
        assert!(dir.save_event(Some(id), &d).is_err());
        dir.delete_event(id);
        assert!(dir.absences.is_empty() && dir.events.is_empty());
    }
}
