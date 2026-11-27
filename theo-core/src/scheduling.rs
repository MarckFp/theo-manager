//! What every schedule shares (the weekend meeting, duties, outgoing talks): what was **sent**,
//! the **send status**, **candidates** sorted the same way everywhere, and the **notices** for
//! the people whose assignments are new, changed or cancelled.
//!
//! A plan is private to the planners until it is sent. Sending copies it into a [`SentPlan`]:
//! that copy is what everyone sees, and the difference between two sent copies decides the notices.

use crate::assignments::{AssignmentKind, NoticeKind};
use crate::people::Directory;
use crate::service_meetings::SendStatus;

/// What everyone sees after a plan is sent.
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct SentPlan {
    /// The day it happens, `YYYY-MM-DD`.
    pub date: String,
    /// There is no meeting that day (for example a convention).
    pub no_meeting: bool,
    /// (person, what they do), in display order.
    pub roles: Vec<(u32, String)>,
    /// (stable place key, person), to show the plan exactly as it was sent.
    pub places: Vec<(String, u32)>,
}

/// The send status of a plan whose people and roles are `roles` now.
pub fn status(sent: Option<&SentPlan>, roles: &[(u32, String)], no_meeting: bool, date: &str) -> SendStatus {
    match sent {
        None => SendStatus::NotSent,
        Some(s) if s.roles == roles && s.no_meeting == no_meeting && s.date == date => SendStatus::Sent,
        Some(_) => SendStatus::Changed,
    }
}

/// Can a person take a place on that day.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Fit {
    Available,
    /// Has an absence on the day (M-01, "Absences").
    Away,
}

/// One person who can take a place, and what helps to choose.
#[derive(Clone, Debug, PartialEq)]
pub struct Candidate {
    pub person: u32,
    pub fit: Fit,
    /// What else they do that day.
    pub busy: Vec<String>,
    /// The last date they had **this kind** of assignment (before this one).
    pub last_this: Option<String>,
    /// The last date they had **any** assignment of this schedule (before this one).
    pub last_any: Option<String>,
}

impl Candidate {
    /// Free: available and nothing else that day. Autofill only picks free people.
    pub fn is_free(&self) -> bool {
        self.fit == Fit::Available && self.busy.is_empty()
    }
}

/// Sorts candidates: available first, then not busy, then the longest without this kind of
/// assignment (never first), then the longest without any, then by last and first name.
pub fn sort(dir: &Directory, v: &mut [Candidate]) {
    let name = |id: u32| dir.person(id).map(|p| (p.last_name.to_lowercase(), p.first_name.to_lowercase())).unwrap_or_default();
    v.sort_by_cached_key(|c| (c.fit != Fit::Available, !c.busy.is_empty(), c.last_this.clone().unwrap_or_default(), c.last_any.clone().unwrap_or_default(), name(c.person)));
}

impl Directory {
    /// Sends one plan: stores `now` as what everyone sees and notifies only the people whose
    /// assignments are new, changed (also a new date) or cancelled. Returns the new sent copy.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn send_plan(
        &mut self,
        what: AssignmentKind,
        record: u32,
        before: Option<&SentPlan>,
        date: &str,
        no_meeting: bool,
        roles: Vec<(u32, String)>,
        places: Vec<(String, u32)>,
        title: &str,
    ) -> SentPlan {
        let (was_date, was_roles) = before.map(|s| (s.date.clone(), if s.no_meeting { Vec::new() } else { s.roles.clone() })).unwrap_or_default();
        let now_roles = if no_meeting { Vec::new() } else { roles.clone() };
        let people: std::collections::BTreeSet<u32> = now_roles.iter().chain(was_roles.iter()).map(|(p, _)| *p).collect();
        for p in people {
            let mine = |list: &[(u32, String)]| list.iter().filter(|(x, _)| *x == p).map(|(_, r)| r.clone()).collect::<Vec<_>>();
            let (was, is) = (mine(&was_roles), mine(&now_roles));
            let kind = match (was.is_empty(), is.is_empty()) {
                (true, false) => Some(NoticeKind::New),
                (false, true) => Some(NoticeKind::Cancelled),
                (false, false) if was != is || was_date != date => Some(NoticeKind::Changed),
                _ => None,
            };
            if let Some(k) = kind {
                let (d, list) = if k == NoticeKind::Cancelled { (was_date.clone(), was) } else { (date.to_string(), is) };
                self.notify(p, k, what, record, &d, None, format!("{title} · {}", list.join(", ")));
            }
        }
        SentPlan { date: date.into(), no_meeting, roles, places }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::assignments::NoticeKind;

    #[test]
    fn only_people_concerned_get_notices() {
        let mut d = crate::demo::demo_directory(10);
        let first = d.send_plan(AssignmentKind::Duty, 1, None, "2026-12-03", false, vec![(1, "Audio".into()), (2, "Video".into())], vec![], "Duties");
        assert_eq!(d.my_notices(1).first().map(|n| n.kind), Some(NoticeKind::New));
        d.mark_notices_seen(1);
        d.mark_notices_seen(2);
        // 1 keeps the same place, 2 is replaced by 3.
        d.send_plan(AssignmentKind::Duty, 1, Some(&first), "2026-12-03", false, vec![(1, "Audio".into()), (3, "Video".into())], vec![], "Duties");
        assert!(d.my_notices(1).is_empty());
        assert_eq!(d.my_notices(2).first().map(|n| n.kind), Some(NoticeKind::Cancelled));
        assert_eq!(d.my_notices(3).first().map(|n| n.kind), Some(NoticeKind::New));
        assert_eq!(status(Some(&first), &first.roles, false, "2026-12-03"), SendStatus::Sent);
        assert_eq!(status(Some(&first), &[], false, "2026-12-03"), SendStatus::Changed);
        assert_eq!(status(None, &[], false, "2026-12-03"), SendStatus::NotSent);
    }
}
