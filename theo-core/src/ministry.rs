//! *My ministry*: a publisher logs each day's time, credit hours and Bible studies, and
//! at the end of the month sends the totals as their field service report (M-10). Only the
//! publisher sees their log. Rules: `docs/model/m10-field-service-reports.md`, "My ministry".

use crate::people::{parse_date, Directory};
use crate::reports::{reporting_month, Kind, Report, ReportDraft, Ym};

/// The ministry of one day.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct MinistryDay {
    pub person: u32,
    /// `YYYY-MM-DD`.
    pub date: String,
    /// Time in the ministry, in minutes.
    pub minutes: u32,
    /// Credit hours (pioneers and continuous auxiliary pioneers), in minutes.
    pub credit_minutes: u32,
    /// Bible studies conducted that day.
    pub studies: u32,
    pub note: Option<String>,
}

/// The totals of a month.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct MonthLog {
    pub minutes: u32,
    pub credit_minutes: u32,
    /// The most Bible studies in one week (Monday to Sunday): a first guess of the different
    /// people studied with in the month.
    pub week_studies: u32,
    /// Days with something logged.
    pub days: usize,
}

impl MonthLog {
    /// Whole hours (reports use whole hours).
    pub fn hours(&self) -> u32 {
        self.minutes / 60
    }

    pub fn credit_hours(&self) -> u32 {
        self.credit_minutes / 60
    }
}

/// "2:30" (hours and minutes); "" for 0.
pub fn hm(minutes: u32) -> String {
    if minutes == 0 { String::new() } else { format!("{}:{:02}", minutes / 60, minutes % 60) }
}

/// "12 h 30 min", "45 min", "3 h".
pub fn hm_long(minutes: u32) -> String {
    match (minutes / 60, minutes % 60) {
        (0, m) => format!("{m} min"),
        (h, 0) => format!("{h} h"),
        (h, m) => format!("{h} h {m} min"),
    }
}

fn month_of(date: &str) -> Option<Ym> {
    parse_date(date).map(|(y, m, _)| (y, m))
}

/// The month in progress.
pub fn this_month() -> Ym {
    let (y, m, _) = crate::prefs::today();
    (y, m)
}

impl Directory {
    pub fn ministry_day(&self, person: u32, date: &str) -> Option<&MinistryDay> {
        self.ministry_log.iter().find(|d| d.person == person && d.date == date)
    }

    /// The days of a month, by date.
    pub fn ministry_days(&self, person: u32, ym: Ym) -> Vec<&MinistryDay> {
        let mut v: Vec<&MinistryDay> = self.ministry_log.iter().filter(|d| d.person == person && month_of(&d.date) == Some(ym)).collect();
        v.sort_by(|a, b| a.date.cmp(&b.date));
        v
    }

    pub fn month_log(&self, person: u32, ym: Ym) -> MonthLog {
        let days = self.ministry_days(person, ym);
        // Weeks from Monday: day 0 (1970-01-01) was a Thursday.
        let mut weeks: std::collections::BTreeMap<i64, u32> = std::collections::BTreeMap::new();
        for d in &days {
            if let Some(date) = parse_date(&d.date) {
                *weeks.entry((crate::prefs::days_from_ymd(date) + 3).div_euclid(7)).or_default() += d.studies;
            }
        }
        MonthLog {
            minutes: days.iter().map(|d| d.minutes).sum(),
            credit_minutes: days.iter().map(|d| d.credit_minutes).sum(),
            week_studies: weeks.values().copied().max().unwrap_or(0),
            days: days.len(),
        }
    }

    /// Saves (or, when everything is 0 and there is no note, removes) a day.
    pub fn save_ministry_day(&mut self, person: u32, date: &str, minutes: u32, credit_minutes: u32, studies: u32, note: &str) -> Result<(), String> {
        if parse_date(date).is_none() {
            return Err("The date is not valid.".into());
        }
        if date > crate::people::today_iso().as_str() {
            return Err("You can log days up to today.".into());
        }
        if minutes + credit_minutes > 24 * 60 {
            return Err("A day has at most 24 hours.".into());
        }
        self.ministry_log.retain(|d| !(d.person == person && d.date == date));
        let note = note.trim();
        if minutes > 0 || credit_minutes > 0 || studies > 0 || !note.is_empty() {
            self.ministry_log.push(MinistryDay { person, date: date.into(), minutes, credit_minutes, studies, note: (!note.is_empty()).then(|| note.to_string()) });
        }
        Ok(())
    }

    /// What kind of report the person makes for a month.
    pub fn my_kind(&self, person: u32, ym: Ym) -> Option<Kind> {
        let p = self.person(person)?;
        p.publisher_type.is_publisher().then(|| self.kind_for(p, ym))
    }

    /// The report the month's log gives, with `studies` different Bible studies.
    pub fn draft_from_log(&self, person: u32, ym: Ym, studies: u32) -> Option<ReportDraft> {
        let kind = self.my_kind(person, ym)?;
        let log = self.month_log(person, ym);
        let took_part = log.minutes > 0 || log.credit_minutes > 0 || studies > 0;
        Some(ReportDraft {
            took_part: if took_part { "yes".into() } else { "no".into() },
            hours: if kind.reports_hours() { log.hours().to_string() } else { String::new() },
            credit: if kind.has_credit() { log.credit_hours().min(crate::reports::CREDIT_MAX).to_string() } else { String::new() },
            studies: studies.to_string(),
            auxiliary: kind == Kind::AuxiliaryMonth,
            notes: String::new(),
        })
    }

    /// Sends the month as the person's report (it replaces one sent before). The month in
    /// progress can be sent too, for example on its last day.
    pub fn send_my_report(&mut self, person: u32, ym: Ym, studies: u32) -> Result<(), String> {
        if ym > this_month() {
            return Err("This month has not started yet.".into());
        }
        let d = self.draft_from_log(person, ym, studies).ok_or("Only publishers send a report.")?;
        // The secretary's form stops at the reporting month; the publisher may send the month in
        // progress, so it is saved as the reporting month would be.
        if ym > reporting_month() { self.save_report_unchecked(person, ym, &d) } else { self.save_report(person, ym, &d).map_err(|e| e.first().map(|x| x.1.clone()).unwrap_or_default()) }
    }

    fn save_report_unchecked(&mut self, person: u32, ym: Ym, d: &ReportDraft) -> Result<(), String> {
        let p = self.person(person).ok_or("Unknown person.")?;
        let kind = Kind::of(p.type_in(ym), d.auxiliary);
        if let Some(e) = d.validate(kind).first() {
            return Err(e.1.clone());
        }
        let n = |s: &str| s.trim().parse::<u32>().ok();
        let hours = kind.reports_hours().then(|| n(&d.hours).unwrap_or(0));
        let credit = if kind.has_credit() { n(&d.credit).filter(|c| *c > 0) } else { None };
        let mut report = Report { person, month: ym, took_part: d.took_part == "yes", hours, credit, studies: n(&d.studies).unwrap_or(0), auxiliary: d.auxiliary, notes: None };
        if report.hours.is_some() {
            report.took_part = report.total() > 0;
        }
        self.reports.retain(|r| !(r.person == person && r.month == ym));
        self.reports.push(report);
        Ok(())
    }

    /// The sent report differs from what the log gives now (log changed after sending).
    pub fn log_changed_since_sent(&self, person: u32, ym: Ym) -> bool {
        let (Some(r), Some(kind)) = (self.report(person, ym), self.my_kind(person, ym)) else { return false };
        let log = self.month_log(person, ym);
        if log.days == 0 {
            return false;
        }
        if kind.reports_hours() {
            r.hours.unwrap_or(0) != log.hours() || (kind.has_credit() && r.credit.unwrap_or(0) != log.credit_hours().min(crate::reports::CREDIT_MAX))
        } else {
            r.took_part != (log.minutes > 0 || log.credit_minutes > 0 || log.week_studies > 0)
        }
    }

    /// Minutes that whole hours leave over in a month, unless they were already carried over.
    pub fn leftover_minutes(&self, person: u32, ym: Ym) -> u32 {
        if self.ministry_carried.contains(&(person, ym)) { 0 } else { self.month_log(person, ym).minutes % 60 }
    }

    /// Moves the minutes left over to the first day of the next month (once per month). Returns
    /// the next month and the minutes.
    pub fn carry_over(&mut self, person: u32, ym: Ym) -> Result<(Ym, u32), String> {
        let minutes = self.leftover_minutes(person, ym);
        if minutes == 0 {
            return Err("There are no minutes left over.".into());
        }
        let next = crate::reports::shift(ym, 1);
        let date = format!("{:04}-{:02}-01", next.0, next.1);
        if date.as_str() > crate::people::today_iso().as_str() {
            return Err("Carry the minutes over once the month is over.".into());
        }
        let (m, c, s, note) = match self.ministry_day(person, &date) {
            Some(d) => (d.minutes, d.credit_minutes, d.studies, d.note.clone().unwrap_or_default()),
            None => (0, 0, 0, String::new()),
        };
        let note = if note.is_empty() { format!("{minutes} min carried over") } else { format!("{note} · {minutes} min carried over") };
        self.save_ministry_day(person, &date, m + minutes, c, s, &note)?;
        self.ministry_carried.insert((person, ym));
        Ok((next, minutes))
    }

    /// A personal goal of hours per month (anyone, also when not pioneering).
    pub fn personal_goal(&self, person: u32) -> Option<u32> {
        self.personal_goals.get(&person).copied()
    }

    pub fn set_personal_goal(&mut self, person: u32, hours: Option<u32>) {
        match hours.filter(|h| *h > 0) {
            Some(h) => {
                self.personal_goals.insert(person, h.min(744));
            }
            None => {
                self.personal_goals.remove(&person);
            }
        }
    }

    /// Months with a log or a report, newest first (the history).
    pub fn my_ministry_months(&self, person: u32) -> Vec<Ym> {
        let mut v: Vec<Ym> = self.ministry_log.iter().filter(|d| d.person == person).filter_map(|d| month_of(&d.date)).chain(self.reports.iter().filter(|r| r.person == person).map(|r| r.month)).collect();
        v.sort();
        v.dedup();
        v.reverse();
        v
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn days_add_up_and_send_whole_hours() {
        let mut d = crate::demo::demo_directory(36);
        d.ministry_log.clear();
        // September 2026: the 8th and 9th are in one week (Tuesday, Wednesday), the 14th in the next.
        let ym = (2026, 9);
        let day = |n: u32| format!("2026-09-{n:02}");
        // Person 2 is a regular pioneer in the demo.
        assert!(d.save_ministry_day(2, &day(8), 150, 0, 1, "").is_ok());
        assert!(d.save_ministry_day(2, &day(9), 100, 60, 2, "").is_ok());
        assert!(d.save_ministry_day(2, &day(14), 0, 0, 1, "").is_ok());
        let log = d.month_log(2, ym);
        assert_eq!((log.minutes, log.hours(), log.credit_hours(), log.week_studies, log.days), (250, 4, 1, 3, 3));
        assert!(d.save_ministry_day(2, &day(14), 0, 0, 0, "").is_ok());
        assert!(d.send_my_report(2, ym, 2).is_ok());
        let r = d.report(2, ym).cloned();
        assert_eq!(r.as_ref().and_then(|r| r.hours), Some(4));
        assert_eq!(r.as_ref().map(|r| r.studies), Some(2));
        assert!(!d.log_changed_since_sent(2, ym));
        assert!(d.save_ministry_day(2, &day(5), 120, 0, 0, "").is_ok());
        assert!(d.log_changed_since_sent(2, ym));
        // Removing everything removes the day.
        assert!(d.save_ministry_day(2, &day(5), 0, 0, 0, "").is_ok());
        assert_eq!(d.month_log(2, ym).days, 2);
        assert!(d.save_ministry_day(2, "2999-01-01", 60, 0, 0, "").is_err());
        // 10 min left over go to 1 October, once.
        assert_eq!(d.leftover_minutes(2, ym), 10);
        assert_eq!(d.carry_over(2, ym), Ok(((2026, 10), 10)));
        assert_eq!(d.ministry_day(2, "2026-10-01").map(|x| x.minutes), Some(10));
        assert_eq!(d.leftover_minutes(2, ym), 0);
        assert!(d.carry_over(2, ym).is_err());
    }

    #[test]
    fn hours_and_minutes_read_well() {
        assert_eq!(hm(150), "2:30");
        assert_eq!(hm_long(45), "45 min");
        assert_eq!(hm_long(180), "3 h");
        assert_eq!(hm_long(750), "12 h 30 min");
    }
}
