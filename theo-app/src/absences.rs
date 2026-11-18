//! ⚠️ MOCK of absences (M-01, R-017): who is away when. Schedules will not assign people during
//! an absence (R-012). Rules: `docs/model/m01-people.md`, "Absences".

use crate::people::Directory;

/// One absence of one person (or of the whole congregation), from and to a date (both included).
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Absence {
    pub id: u32,
    /// `None`: the whole congregation is away (for example at a convention).
    pub person: Option<u32>,
    /// `YYYY-MM-DD`.
    pub from: String,
    /// `YYYY-MM-DD`, not before `from`.
    pub to: String,
    /// Free notes, shown only when the absence is opened.
    #[serde(alias = "note")]
    pub notes: Option<String>,
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
    /// Does this absence cover the person (their own, or one of the whole congregation)?
    pub fn covers(&self, person: u32) -> bool {
        self.person.is_none_or(|p| p == person)
    }

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
    /// What is wrong with an absence, if anything (empty: it can be saved).
    pub fn absence_errors(&self, person: Option<u32>, from: &str, to: &str) -> Vec<(&'static str, String)> {
        let mut e = Vec::new();
        if person.is_some_and(|p| self.person(p).is_none()) {
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
        e
    }

    /// Adds (`id` = None) or changes an absence. `person` None: the whole congregation.
    pub fn save_absence(&mut self, id: Option<u32>, person: Option<u32>, from: &str, to: &str, notes: &str) -> Result<u32, Vec<(&'static str, String)>> {
        let e = self.absence_errors(person, from, to);
        if !e.is_empty() {
            return Err(e);
        }
        let notes = Some(notes.trim().to_string()).filter(|n| !n.is_empty());
        let new_id = id.unwrap_or_else(|| self.absences.iter().map(|a| a.id).max().unwrap_or(0) + 1);
        let a = Absence { id: new_id, person, from: from.into(), to: to.into(), notes };
        match self.absences.iter_mut().find(|x| x.id == new_id) {
            Some(slot) => *slot = a,
            None => self.absences.push(a),
        }
        Ok(new_id)
    }

    pub fn delete_absence(&mut self, id: u32) {
        self.absences.retain(|a| a.id != id);
    }

    /// Is the person away on that day (their own absence or one of the whole congregation)?
    pub fn is_away(&self, person: u32, day: &str) -> bool {
        self.absences.iter().any(|a| a.covers(person) && a.from.as_str() <= day && day <= a.to.as_str())
    }

    /// The other current members of the person's family.
    pub fn family_others(&self, person: u32) -> Vec<u32> {
        self.family_of(person)
            .map(|f| {
                f.members.iter().copied().filter(|m| *m != person && self.person(*m).is_some_and(|p| p.moved_away.is_none())).collect()
            })
            .unwrap_or_default()
    }

    /// The same absence (same days) of other family members: (absence id, person).
    pub fn family_matches(&self, absence: u32) -> Vec<(u32, u32)> {
        let Some(a) = self.absences.iter().find(|a| a.id == absence) else { return Vec::new() };
        let Some(person) = a.person else { return Vec::new() };
        let others = self.family_others(person);
        self.absences
            .iter()
            .filter(|b| b.id != a.id && b.from == a.from && b.to == a.to)
            .filter_map(|b| b.person.filter(|p| others.contains(p)).map(|p| (b.id, p)))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn absences_are_checked_and_found() {
        let mut d = crate::mock::demo_directory(14);
        d.absences.clear();
        assert!(d.save_absence(None, Some(3), "2026-12-10", "2026-12-01", "").is_err());
        let id = d.save_absence(None, Some(3), "2026-12-01", "2026-12-10", " Holiday ").unwrap_or(0);
        assert!(d.is_away(3, "2026-12-05") && !d.is_away(3, "2026-12-11"));
        assert_eq!(d.absences.iter().find(|a| a.id == id).and_then(|a| a.notes.clone()), Some("Holiday".into()));
        // The whole congregation.
        assert!(d.save_absence(None, None, "2027-01-08", "2027-01-10", "Convention").is_ok());
        assert!(d.is_away(5, "2027-01-09") && d.is_away(3, "2027-01-10"));
        let a = d.absences.iter().find(|a| a.id == id).cloned();
        assert!(a.is_some_and(|a| a.when("2026-11-18") == When::Upcoming));
    }

    #[test]
    fn family_absences_are_found() {
        let mut d = crate::mock::demo_directory(40);
        d.absences.clear();
        let Some(f) = d.families.iter().find(|f| f.members.len() >= 2).cloned() else { return };
        let (a, b) = (f.members[0], f.members[1]);
        assert!(d.family_others(a).contains(&b));
        let first = d.save_absence(None, Some(a), "2027-02-01", "2027-02-03", "").unwrap_or(0);
        let second = d.save_absence(None, Some(b), "2027-02-01", "2027-02-03", "").unwrap_or(0);
        d.save_absence(None, Some(b), "2027-03-01", "2027-03-03", "").ok();
        assert_eq!(d.family_matches(first), vec![(second, b)]);
    }
}
