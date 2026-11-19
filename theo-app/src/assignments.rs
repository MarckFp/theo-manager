//! ⚠️ MOCK of "My assignments": everything a person is assigned to, and notices when something
//! sent changes for them. For now: meetings for field service (M-15). Meeting parts and duties
//! join when those modules exist. Rules: `docs/model/m15-field-service-meetings.md`, "Notices".

use crate::people::Directory;
use crate::service_meetings::MeetingVersion;

/// Where an assignment comes from.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum AssignmentKind {
    FieldServiceMeeting,
}

impl AssignmentKind {
    pub fn label(self) -> &'static str {
        match self {
            AssignmentKind::FieldServiceMeeting => "Meeting for field service",
        }
    }
}

/// One assignment of a person, as everyone sees it (sent).
#[derive(Clone, Debug, PartialEq)]
pub struct Assignment {
    pub kind: AssignmentKind,
    /// The record (for example the meeting), to match its notices.
    pub record: u32,
    /// `YYYY-MM-DD`.
    pub date: String,
    /// `HH:MM`, if it has a time.
    pub time: Option<String>,
    /// Where, or other details.
    pub detail: String,
}

/// What a notice tells.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum NoticeKind {
    New,
    Changed,
    Cancelled,
}

impl NoticeKind {
    pub fn label(self) -> &'static str {
        match self {
            NoticeKind::New => "New",
            NoticeKind::Changed => "Changed",
            NoticeKind::Cancelled => "Cancelled",
        }
    }
}

/// A notice for one person: an assignment of theirs was sent new, changed or cancelled. Only the
/// people concerned get one (for example the conductor of a meeting, not everyone).
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Notice {
    pub id: u32,
    pub person: u32,
    pub kind: NoticeKind,
    pub what: AssignmentKind,
    /// The record it is about (for example the meeting), so a newer notice replaces an older one.
    pub record: u32,
    pub date: String,
    pub time: Option<String>,
    pub detail: String,
    pub seen: bool,
}

impl Directory {
    /// The person's assignments from `today` on, by date and time (only what was sent).
    pub fn my_assignments(&self, person: u32, today: &str) -> Vec<Assignment> {
        self.all_my_assignments(person).into_iter().filter(|a| a.date.as_str() >= today).collect()
    }

    /// All the person's assignments, past and future, by date and time (only what was sent).
    pub fn all_my_assignments(&self, person: u32) -> Vec<Assignment> {
        let mut v: Vec<Assignment> = self
            .service_meetings
            .iter()
            .filter_map(|m| m.sent.as_ref().map(|s| (m.id, s)))
            .filter(|(_, s)| s.conductor == Some(person))
            .map(|(id, s)| Assignment {
                kind: AssignmentKind::FieldServiceMeeting,
                record: id,
                date: s.date.clone(),
                time: Some(s.time.clone()),
                detail: s.place.clone(),
            })
            .collect();
        v.sort_by(|a, b| (&a.date, &a.time).cmp(&(&b.date, &b.time)));
        v
    }

    /// The person's notices that are not seen yet, newest first.
    pub fn my_notices(&self, person: u32) -> Vec<&Notice> {
        let mut v: Vec<&Notice> = self.notices.iter().filter(|n| n.person == person && !n.seen).collect();
        v.sort_by_key(|n| std::cmp::Reverse(n.id));
        v
    }

    pub fn mark_notices_seen(&mut self, person: u32) {
        for n in self.notices.iter_mut().filter(|n| n.person == person) {
            n.seen = true;
        }
    }

    /// Adds a notice, replacing an unseen one of the same person about the same record.
    pub(crate) fn notify(&mut self, person: u32, kind: NoticeKind, record: u32, v: &MeetingVersion) {
        let earlier = self.notices.iter().find(|n| n.person == person && n.record == record && !n.seen).map(|n| n.kind);
        // New then changed before they looked: still new to them. New then cancelled: nothing to say.
        let kind = match (earlier, kind) {
            (Some(NoticeKind::New), NoticeKind::Changed) => NoticeKind::New,
            (Some(NoticeKind::New), NoticeKind::Cancelled) => {
                self.notices.retain(|n| !(n.person == person && n.record == record && !n.seen));
                return;
            }
            (_, k) => k,
        };
        self.notices.retain(|n| !(n.person == person && n.record == record && !n.seen));
        let id = self.notices.iter().map(|n| n.id).max().unwrap_or(0) + 1;
        self.notices.push(Notice {
            id,
            person,
            kind,
            what: AssignmentKind::FieldServiceMeeting,
            record,
            date: v.date.clone(),
            time: Some(v.time.clone()),
            detail: v.place.clone(),
            seen: false,
        });
    }
}
