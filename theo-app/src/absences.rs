//! ⚠️ MOCK of absences (M-01, R-017): who is away when. Schedules will not assign people during
//! an absence (R-012). Rules: `docs/model/m01-people.md`, "Absences".

use crate::people::Directory;

/// One absence of one person, from and to a date (both included).
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Absence {
    pub id: u32,
    pub person: u32,
    /// `YYYY-MM-DD`.
    pub from: String,
    /// `YYYY-MM-DD`, not before `from`.
    pub to: String,
    pub note: Option<String>,
}

/// Where an absence is in time, compared with today.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum When {
    Now,
    Upcoming,
    Past,
}

impl When {
    pub fn value(self) -> &'static str {
        match self {
            When::Now => "now",
            When::Upcoming => "upcoming",
            When::Past => "past",
        }
    }
}

impl Absence {
    pub fn when(&self, today: &str) -> When {
        if self.to.as_str() < today {
            When::Past
        } else if self.from.as_str() > today {
            When::Upcoming
        } else {
            When::Now
        }
    }
}

fn valid(d: &str) -> bool {
    crate::people::parse_date(d).is_some() && d.len() == 10
}

impl Directory {
    /// Adds (`id` = None) or changes an absence.
    pub fn save_absence(&mut self, id: Option<u32>, person: u32, from: &str, to: &str, note: &str) -> Result<u32, Vec<(&'static str, String)>> {
        let mut e = Vec::new();
        if self.person(person).is_none() {
            e.push(("person", "Choose a person.".to_string()));
        }
        if !valid(from) {
            e.push(("from", "Enter the first day.".into()));
        }
        if !valid(to) {
            e.push(("to", "Enter the last day.".into()));
        } else if valid(from) && to < from {
            e.push(("to", "The last day cannot be before the first day.".into()));
        }
        if !e.is_empty() {
            return Err(e);
        }
        let note = Some(note.trim().to_string()).filter(|n| !n.is_empty());
        let new_id = id.unwrap_or_else(|| self.absences.iter().map(|a| a.id).max().unwrap_or(0) + 1);
        let a = Absence { id: new_id, person, from: from.into(), to: to.into(), note };
        match self.absences.iter_mut().find(|x| x.id == new_id) {
            Some(slot) => *slot = a,
            None => self.absences.push(a),
        }
        Ok(new_id)
    }

    pub fn delete_absence(&mut self, id: u32) {
        self.absences.retain(|a| a.id != id);
    }

    /// Is the person away on that day?
    pub fn is_away(&self, person: u32, day: &str) -> bool {
        self.absences.iter().any(|a| a.person == person && a.from.as_str() <= day && day <= a.to.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn absences_are_checked_and_found() {
        let mut d = crate::mock::demo_directory(14);
        assert!(d.save_absence(None, 3, "2026-12-10", "2026-12-01", "").is_err());
        let id = d.save_absence(None, 3, "2026-12-01", "2026-12-10", " Holiday ").unwrap_or(0);
        assert!(d.is_away(3, "2026-12-05") && !d.is_away(3, "2026-12-11"));
        assert_eq!(d.absences.iter().find(|a| a.id == id).and_then(|a| a.note.clone()), Some("Holiday".into()));
        let a = d.absences.iter().find(|a| a.id == id).cloned();
        assert!(a.is_some_and(|a| a.when("2026-11-18") == When::Upcoming));
    }
}
