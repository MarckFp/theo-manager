//! ⚠️ MOCK of meeting attendance (M-11): how many people attended each meeting, like the
//! paper record: per month, five weeks × the midweek and weekend meeting.
//! Rules: `docs/model/m11-meeting-attendance.md`.

use crate::people::Directory;
use crate::reports::{service_year_months, shift, Ym};

/// The highest number a meeting can have (a typing guard, not a rule).
pub const MAX_ATTENDANCE: u32 = 9999;
/// Columns of the monthly record: week 1 to week 5.
pub const WEEKS: usize = 5;

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Meeting {
    Midweek,
    Weekend,
}

impl Meeting {
    pub const ALL: [Meeting; 2] = [Meeting::Midweek, Meeting::Weekend];

    pub fn label(self) -> &'static str {
        match self {
            Meeting::Midweek => "Midweek meeting",
            Meeting::Weekend => "Weekend meeting",
        }
    }

    pub fn short(self) -> &'static str {
        match self {
            Meeting::Midweek => "Midweek",
            Meeting::Weekend => "Weekend",
        }
    }
}

/// The record of one month: the number of people at each meeting, week by week. Empty: no
/// meeting that week (for example during an assembly) or not entered yet.
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct MonthAttendance {
    pub month: Ym,
    pub midweek: [Option<u32>; WEEKS],
    pub weekend: [Option<u32>; WEEKS],
}

impl MonthAttendance {
    pub fn new(month: Ym) -> Self {
        Self { month, ..Default::default() }
    }

    pub fn row(&self, m: Meeting) -> &[Option<u32>; WEEKS] {
        match m {
            Meeting::Midweek => &self.midweek,
            Meeting::Weekend => &self.weekend,
        }
    }

    fn row_mut(&mut self, m: Meeting) -> &mut [Option<u32>; WEEKS] {
        match m {
            Meeting::Midweek => &mut self.midweek,
            Meeting::Weekend => &mut self.weekend,
        }
    }

    pub fn stats(&self, m: Meeting) -> Totals {
        Totals::of(self.row(m).iter().flatten().copied())
    }

    pub fn is_empty(&self) -> bool {
        self.midweek.iter().chain(self.weekend.iter()).all(Option::is_none)
    }
}

/// Meetings held, total attendance, and the average per meeting held (as on the paper record:
/// total ÷ number of meetings, so weeks without a meeting do not lower it).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Totals {
    pub meetings: u32,
    pub total: u32,
}

impl Totals {
    pub fn of(values: impl Iterator<Item = u32>) -> Self {
        values.fold(Totals::default(), |t, v| Totals { meetings: t.meetings + 1, total: t.total + v })
    }

    pub fn add(self, o: Totals) -> Totals {
        Totals { meetings: self.meetings + o.meetings, total: self.total + o.total }
    }

    pub fn average(&self) -> Option<f64> {
        (self.meetings > 0).then(|| self.total as f64 / self.meetings as f64)
    }
}

/// An average as shown: whole people.
pub fn show_average(a: Option<f64>) -> String {
    a.map(|a| format!("{}", a.round() as u32)).unwrap_or_else(|| "–".into())
}

/// The highest attendance of a period: which meeting, when.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Peak {
    pub month: Ym,
    pub meeting: Meeting,
    /// 0-based week.
    pub week: usize,
    pub count: u32,
}

/// Weekday number (0 = Sunday) of an English weekday name, as stored in the congregation.
fn weekday_index(name: &str) -> Option<i64> {
    ["sunday", "monday", "tuesday", "wednesday", "thursday", "friday", "saturday"]
        .iter()
        .position(|d| d.eq_ignore_ascii_case(name.trim()))
        .map(|i| i as i64)
}

/// The days of the month on which a weekly meeting on `weekday` falls (4 or 5 of them).
pub fn meeting_days((y, m): Ym, weekday: &str) -> Vec<u32> {
    let Some(want) = weekday_index(weekday) else { return Vec::new() };
    let first = crate::prefs::days_from_ymd((y, m, 1));
    // 1970-01-01 was a Thursday (4).
    let first_wd = (first + 4).rem_euclid(7);
    let offset = (want - first_wd).rem_euclid(7) as u32;
    let (ny, nm) = shift((y, m), 1);
    let len = (crate::prefs::days_from_ymd((ny, nm, 1)) - first) as u32;
    (0..WEEKS as u32).map(|w| 1 + offset + 7 * w).filter(|d| *d <= len).collect()
}

impl Directory {
    /// The record of a month (empty when nothing was entered).
    pub fn attendance(&self, ym: Ym) -> MonthAttendance {
        self.attendance.iter().find(|a| a.month == ym).cloned().unwrap_or_else(|| MonthAttendance::new(ym))
    }

    /// Sets (or empties, with `None`) the attendance of one meeting. Months left empty are removed.
    pub fn set_attendance(&mut self, ym: Ym, meeting: Meeting, week: usize, count: Option<u32>) {
        if week >= WEEKS {
            return;
        }
        let count = count.map(|c| c.min(MAX_ATTENDANCE));
        match self.attendance.iter_mut().find(|a| a.month == ym) {
            Some(a) => a.row_mut(meeting)[week] = count,
            None if count.is_some() => {
                let mut a = MonthAttendance::new(ym);
                a.row_mut(meeting)[week] = count;
                self.attendance.push(a);
                self.attendance.sort_by_key(|a| a.month);
            }
            None => {}
        }
        self.attendance.retain(|a| !a.is_empty());
    }

    /// The 12 months of a service year (September to August), entered or not.
    pub fn attendance_year(&self, start: i32) -> Vec<MonthAttendance> {
        service_year_months(start).into_iter().map(|ym| self.attendance(ym)).collect()
    }

    /// Totals of one meeting over a service year.
    pub fn year_totals(&self, start: i32, m: Meeting) -> Totals {
        self.attendance_year(start).iter().fold(Totals::default(), |t, a| t.add(a.stats(m)))
    }

    /// The highest attendance of a service year (the first one, when several are equal).
    pub fn peak(&self, start: i32) -> Option<Peak> {
        let mut best: Option<Peak> = None;
        for a in self.attendance_year(start) {
            for m in Meeting::ALL {
                for (week, c) in a.row(m).iter().enumerate() {
                    if let Some(count) = *c
                        && best.is_none_or(|b| count > b.count)
                    {
                        best = Some(Peak { month: a.month, meeting: m, week, count });
                    }
                }
            }
        }
        best
    }

    /// The first service year with attendance (or the current one).
    pub fn first_attendance_year(&self, current: i32) -> i32 {
        self.attendance.iter().map(|a| crate::reports::service_year(a.month)).min().unwrap_or(current).min(current)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn meeting_days_follow_the_weekday() {
        // November 2026 starts on a Sunday: Sundays 1, 8, 15, 22, 29; Thursdays 5, 12, 19, 26.
        assert_eq!(meeting_days((2026, 11), "Sunday"), vec![1, 8, 15, 22, 29]);
        assert_eq!(meeting_days((2026, 11), "Thursday"), vec![5, 12, 19, 26]);
        // February 2027 has 28 days.
        assert_eq!(meeting_days((2027, 2), "monday"), vec![1, 8, 15, 22]);
        assert!(meeting_days((2026, 11), "Someday").is_empty());
    }

    #[test]
    fn averages_count_only_meetings_held() {
        let mut d = Directory::default();
        d.set_attendance((2026, 11), Meeting::Midweek, 0, Some(50));
        d.set_attendance((2026, 11), Meeting::Midweek, 1, Some(54));
        d.set_attendance((2026, 11), Meeting::Weekend, 4, Some(99_999));
        let a = d.attendance((2026, 11));
        assert_eq!(a.stats(Meeting::Midweek), Totals { meetings: 2, total: 104 });
        assert_eq!(a.stats(Meeting::Midweek).average(), Some(52.0));
        assert_eq!(a.weekend[4], Some(MAX_ATTENDANCE));
        assert_eq!(d.peak(2026).map(|p| (p.meeting, p.week)), Some((Meeting::Weekend, 4)));
        // Emptying every value removes the month.
        d.set_attendance((2026, 11), Meeting::Midweek, 0, None);
        d.set_attendance((2026, 11), Meeting::Midweek, 1, None);
        d.set_attendance((2026, 11), Meeting::Weekend, 4, None);
        assert!(d.attendance.is_empty());
        assert_eq!(d.year_totals(2026, Meeting::Weekend).average(), None);
    }
}
