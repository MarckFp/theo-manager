//! ⚠️ MOCK of M-10 Field service reports, kept in memory with the rest of the [`Directory`].
//! Rules: `docs/model/m10-field-service-reports.md`. The real types go to `theo-core`.

use crate::people::{fold, Activity, Directory, Person, PublisherType};
use crate::prefs;
use std::collections::BTreeSet;

/// A month: (year, 1..=12).
pub type Ym = (i32, u32);

/// Credit hours per month at most.
pub const CREDIT_MAX: u32 = 50;
/// Yearly goal of regular pioneers, special pioneers and missionaries.
pub const YEARLY_GOAL: u32 = 600;
/// Monthly goal of auxiliary pioneers; `AUX_GOAL_SHORT` in months the congregation marks.
pub const AUX_GOAL: u32 = 30;
pub const AUX_GOAL_SHORT: u32 = 15;
const NOTES_MAX: usize = 1000;
/// Day of the month from which a missing report of the reporting month gets a warning.
pub const WARN_DAY: u32 = 5;
/// Day of the month from which a missing report of the reporting month is late.
pub const LATE_DAY: u32 = 15;

/// Where a person's report for a month stands.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReportStatus {
    Received,
    /// Reporting month, before the 5th.
    NotYet,
    /// Reporting month, from the 5th.
    Missing,
    /// Reporting month from the 15th, or any earlier month.
    Late,
}

impl ReportStatus {
    pub fn value(self) -> &'static str {
        match self {
            ReportStatus::Received => "received",
            ReportStatus::NotYet => "not-yet",
            ReportStatus::Missing => "missing",
            ReportStatus::Late => "late",
        }
    }
}

/// The status of a missing report of `ym` on day `day` of the current month.
fn missing_status(ym: Ym, day: u32) -> ReportStatus {
    if ym < reporting_month() || day >= LATE_DAY {
        ReportStatus::Late
    } else if day >= WARN_DAY {
        ReportStatus::Missing
    } else {
        ReportStatus::NotYet
    }
}

pub fn shift((y, m): Ym, by: i32) -> Ym {
    let idx = y * 12 + m as i32 - 1 + by;
    (idx.div_euclid(12), idx.rem_euclid(12) as u32 + 1)
}

/// The month publishers report now: the month before today.
pub fn reporting_month() -> Ym {
    let (y, m, _) = prefs::today();
    shift((y, m), -1)
}

/// Service years run from September to August; this returns the starting year.
pub fn service_year((y, m): Ym) -> i32 {
    if m >= 9 { y } else { y - 1 }
}

pub fn service_year_months(start: i32) -> Vec<Ym> {
    (0..12).map(|i| shift((start, 9), i)).collect()
}

pub fn service_year_label(start: i32) -> String {
    format!("{start}/{}", start + 1)
}

pub fn month_label((y, m): Ym) -> String {
    format!("{} {y}", theo_ui::month_name(m))
}

pub fn short_month((_, m): Ym) -> String {
    theo_ui::month_name(m)[..3].to_string()
}

// ---------------------------------------------------------------------------------------------
// Report
// ---------------------------------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq)]
pub struct Report {
    pub person: u32,
    pub month: Ym,
    pub took_part: bool,
    pub hours: Option<u32>,
    pub credit: Option<u32>,
    pub studies: u32,
    pub auxiliary: bool,
    pub notes: Option<String>,
}

impl Report {
    /// Hours + credit hours.
    pub fn total(&self) -> u32 {
        self.hours.unwrap_or(0) + self.credit.unwrap_or(0)
    }

    /// Took part in the ministry (for people who report hours: more than 0 hours).
    pub fn participated(&self) -> bool {
        if self.hours.is_some() { self.total() > 0 } else { self.took_part }
    }
}

/// What a person reports, from their type (and the monthly auxiliary flag).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Publisher,
    AuxiliaryMonth,
    ContinuousAuxiliary,
    Pioneer,
}

impl Kind {
    pub fn of(ty: PublisherType, auxiliary_this_month: bool) -> Kind {
        match ty {
            PublisherType::ContinuousAuxiliaryPioneer => Kind::ContinuousAuxiliary,
            PublisherType::RegularPioneer | PublisherType::SpecialPioneer | PublisherType::Missionary => Kind::Pioneer,
            PublisherType::BaptisedPublisher if auxiliary_this_month => Kind::AuxiliaryMonth,
            _ => Kind::Publisher,
        }
    }
    pub fn value(self) -> &'static str {
        match self {
            Kind::Publisher => "publisher",
            Kind::AuxiliaryMonth | Kind::ContinuousAuxiliary => "auxiliary",
            Kind::Pioneer => "pioneer",
        }
    }
    pub fn reports_hours(self) -> bool {
        self != Kind::Publisher
    }
    pub fn has_credit(self) -> bool {
        matches!(self, Kind::ContinuousAuxiliary | Kind::Pioneer)
    }
}

/// The report form. Text as typed.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ReportDraft {
    /// "yes", "no" or "" (not chosen yet).
    pub took_part: String,
    pub hours: String,
    pub credit: String,
    pub studies: String,
    pub auxiliary: bool,
    pub notes: String,
}

impl ReportDraft {
    pub fn from_report(r: &Report) -> Self {
        Self {
            took_part: if r.took_part { "yes" } else { "no" }.into(),
            hours: r.hours.map(|h| h.to_string()).unwrap_or_default(),
            credit: r.credit.filter(|c| *c > 0).map(|c| c.to_string()).unwrap_or_default(),
            studies: r.studies.to_string(),
            auxiliary: r.auxiliary,
            notes: r.notes.clone().unwrap_or_default(),
        }
    }

    pub fn validate(&self, kind: Kind) -> Vec<(&'static str, String)> {
        let mut e = Vec::new();
        let num = |s: &str| -> Option<Result<u32, ()>> {
            let t = s.trim();
            (!t.is_empty()).then(|| t.parse::<u32>().map_err(|_| ()))
        };
        if kind.reports_hours() {
            match num(&self.hours) {
                None => e.push(("hours", "Enter the hours.".to_string())),
                Some(Err(())) => e.push(("hours", "Enter whole hours, for example 12.".into())),
                Some(Ok(h)) if h > 744 => e.push(("hours", "A month has at most 744 hours.".into())),
                _ => {}
            }
        } else if self.took_part.is_empty() {
            e.push(("took_part", "Choose whether they took part in the ministry.".into()));
        }
        if kind.has_credit() {
            match num(&self.credit) {
                Some(Err(())) => e.push(("credit", "Enter whole hours, for example 5.".into())),
                Some(Ok(c)) if c > CREDIT_MAX => e.push(("credit", format!("Credit hours are at most {CREDIT_MAX} per month."))),
                _ => {}
            }
        }
        match num(&self.studies) {
            None => e.push(("studies", "Enter the number of Bible studies (0 if none).".into())),
            Some(Err(())) => e.push(("studies", "Enter a whole number.".into())),
            _ => {}
        }
        if self.notes.chars().count() > NOTES_MAX {
            e.push(("notes", format!("Notes can have at most {NOTES_MAX} characters.")));
        }
        e
    }
}

// ---------------------------------------------------------------------------------------------
// Statistics types
// ---------------------------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct MonthSummary {
    pub expected: usize,
    pub received: usize,
    pub took_part: usize,
    pub hours: u32,
    pub studies: u32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct GoalProgress {
    pub person: u32,
    pub kind: Kind,
    /// Total hours so far in the period.
    pub hours: u32,
    /// The goal of the whole period (year, or the sum of the monthly goals so far).
    pub goal: u32,
    /// Where the person should be by now.
    pub expected: u32,
    /// Months in the period that met their monthly goal (auxiliary pioneers).
    pub months_met: usize,
    pub months: usize,
}

impl GoalProgress {
    pub fn on_track(&self) -> bool {
        self.hours >= self.expected
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct GroupStats {
    pub group: Option<u32>,
    pub name: String,
    pub members: usize,
    /// Average per month over the months with data.
    pub avg_took_part: f64,
    pub avg_studies: f64,
    pub hours: u32,
    pub received_pct: f64,
}

// ---------------------------------------------------------------------------------------------
// Directory: reports
// ---------------------------------------------------------------------------------------------

impl Directory {
    /// Current publishers, sorted by first name then last name (callers sort by display name).
    pub fn reporters(&self) -> Vec<&Person> {
        self.current_publishers()
    }

    pub fn report(&self, person: u32, ym: Ym) -> Option<&Report> {
        self.reports.iter().find(|r| r.person == person && r.month == ym)
    }

    /// Auxiliary pioneer that month: planned, or marked in the report.
    pub fn is_aux(&self, person: u32, ym: Ym) -> bool {
        self.aux_plans.contains(&(person, ym)) || self.report(person, ym).is_some_and(|r| r.auxiliary)
    }

    pub fn kind_for(&self, p: &Person, ym: Ym) -> Kind {
        Kind::of(p.publisher_type, self.is_aux(p.id, ym))
    }

    /// Plans (or removes) monthly auxiliary pioneering. Baptised publishers only.
    pub fn set_aux_plan(&mut self, person: u32, ym: Ym, on: bool) {
        let ok = self.person(person).is_some_and(|p| p.publisher_type == PublisherType::BaptisedPublisher);
        if on && ok {
            self.aux_plans.insert((person, ym));
        } else {
            self.aux_plans.remove(&(person, ym));
            if let Some(r) = self.reports.iter_mut().find(|r| r.person == person && r.month == ym) {
                r.auxiliary = false;
            }
        }
    }

    /// Months with planned auxiliary pioneering from the reporting month on.
    pub fn planned_aux(&self, person: u32) -> Vec<Ym> {
        let from = reporting_month();
        self.aux_plans.iter().filter(|(p, m)| *p == person && *m >= from).map(|(_, m)| *m).collect()
    }

    pub fn report_status(&self, person: u32, ym: Ym) -> ReportStatus {
        if self.report(person, ym).is_some() {
            ReportStatus::Received
        } else {
            missing_status(ym, prefs::today().2)
        }
    }

    fn first_report(&self, person: u32) -> Option<Ym> {
        self.reports.iter().filter(|r| r.person == person).map(|r| r.month).min()
    }

    /// The months that have at least one report, oldest and newest (for month pickers).
    pub fn report_range(&self) -> (Ym, Ym) {
        let last = reporting_month();
        let first = self.reports.iter().map(|r| r.month).min().unwrap_or(last).min(last);
        (shift(first, -12).max((first.0 - 1, 9)), last)
    }

    pub fn save_report(&mut self, person: u32, ym: Ym, d: &ReportDraft) -> Result<(), Vec<(&'static str, String)>> {
        let Some(p) = self.person(person) else { return Err(vec![("person", "Unknown person.".into())]) };
        if ym > reporting_month() {
            return Err(vec![("month", "Reports are for past months only.".into())]);
        }
        let auxiliary = d.auxiliary && p.publisher_type == PublisherType::BaptisedPublisher;
        let kind = Kind::of(p.publisher_type, auxiliary);
        let errors = d.validate(kind);
        if !errors.is_empty() {
            return Err(errors);
        }
        let n = |s: &str| s.trim().parse::<u32>().ok();
        let hours = kind.reports_hours().then(|| n(&d.hours).unwrap_or(0));
        let credit = if kind.has_credit() { n(&d.credit).filter(|c| *c > 0) } else { None };
        let mut report = Report {
            person,
            month: ym,
            took_part: d.took_part == "yes",
            hours,
            credit,
            studies: n(&d.studies).unwrap_or(0),
            auxiliary,
            notes: (!d.notes.trim().is_empty()).then(|| d.notes.trim().to_string()),
        };
        if report.hours.is_some() {
            report.took_part = report.total() > 0;
        }
        self.reports.retain(|r| !(r.person == person && r.month == ym));
        self.reports.push(report);
        if auxiliary {
            self.aux_plans.insert((person, ym));
        } else {
            self.aux_plans.remove(&(person, ym));
        }
        Ok(())
    }

    pub fn delete_report(&mut self, person: u32, ym: Ym) {
        self.reports.retain(|r| !(r.person == person && r.month == ym));
    }

    // ----- goals -----

    pub fn aux_goal(&self, ym: Ym) -> u32 {
        if self.aux_short_months.contains(&ym) { AUX_GOAL_SHORT } else { AUX_GOAL }
    }

    pub fn set_aux_goal(&mut self, ym: Ym, short: bool) {
        if short {
            self.aux_short_months.insert(ym);
        } else {
            self.aux_short_months.remove(&ym);
        }
    }

    /// Goal progress of a pioneer or auxiliary pioneer in a service year, up to the reporting
    /// month. `None` for people without a goal in that year.
    pub fn goal_progress(&self, person: u32, start: i32) -> Option<GoalProgress> {
        let p = self.person(person)?;
        let last = reporting_month();
        let months: Vec<Ym> = service_year_months(start).into_iter().filter(|m| *m <= last).collect();
        if months.is_empty() {
            return None;
        }
        let kind = Kind::of(p.publisher_type, false);
        let reports: Vec<&Report> = self.reports.iter().filter(|r| r.person == person && months.contains(&r.month)).collect();
        let hours: u32 = reports.iter().map(|r| r.total()).sum();
        match kind {
            Kind::Pioneer => {
                let expected = YEARLY_GOAL * months.len() as u32 / 12;
                Some(GoalProgress { person, kind, hours, goal: YEARLY_GOAL, expected, months_met: 0, months: months.len() })
            }
            Kind::ContinuousAuxiliary => {
                let goal: u32 = months.iter().map(|m| self.aux_goal(*m)).sum();
                let met = months.iter().filter(|m| reports.iter().any(|r| r.month == **m && r.total() >= self.aux_goal(**m))).count();
                Some(GoalProgress { person, kind, hours, goal, expected: goal, months_met: met, months: months.len() })
            }
            _ => {
                // Monthly auxiliary pioneering: only the months they were auxiliary pioneers.
                let aux: Vec<&&Report> = reports.iter().filter(|r| r.auxiliary).collect();
                if aux.is_empty() {
                    return None;
                }
                let hours = aux.iter().map(|r| r.total()).sum();
                let goal = aux.iter().map(|r| self.aux_goal(r.month)).sum();
                let met = aux.iter().filter(|r| r.total() >= self.aux_goal(r.month)).count();
                Some(GoalProgress { person, kind: Kind::AuxiliaryMonth, hours, goal, expected: goal, months_met: met, months: aux.len() })
            }
        }
    }

    // ----- activity (M-01, derived) -----

    /// Activity over the last 6 complete months. `None` for non-publishers, people who moved
    /// away, and publishers without any report yet.
    pub fn activity(&self, p: &Person) -> Option<Activity> {
        if !p.publisher_type.is_publisher() || p.moved_away.is_some() {
            return None;
        }
        let counted: Vec<bool> = self.last_six(p.id).into_iter().flatten().collect();
        if counted.is_empty() {
            None
        } else if counted.iter().all(|x| *x) {
            Some(Activity::Active)
        } else if counted.iter().any(|x| *x) {
            Some(Activity::Irregular)
        } else {
            Some(Activity::Inactive)
        }
    }

    /// The last 6 complete months, oldest first: `Some(true)` took part, `Some(false)` no report
    /// or did not take part, `None` before the first report.
    pub fn last_six(&self, person: u32) -> [Option<bool>; 6] {
        let last = reporting_month();
        let first = self.first_report(person);
        std::array::from_fn(|i| {
            let ym = shift(last, i as i32 - 5);
            match first {
                Some(f) if ym >= f => Some(self.report(person, ym).is_some_and(Report::participated)),
                _ => None,
            }
        })
    }

    // ----- statistics -----

    pub fn month_summary(&self, ym: Ym) -> MonthSummary {
        let people: BTreeSet<u32> = self.reporters().iter().map(|p| p.id).collect();
        let mut s = MonthSummary { expected: people.len(), ..Default::default() };
        for r in self.reports.iter().filter(|r| r.month == ym && people.contains(&r.person)) {
            s.received += 1;
            s.took_part += usize::from(r.participated());
            s.hours += r.total();
            s.studies += r.studies;
        }
        s
    }

    /// Per group (and "No group") over the given months.
    pub fn group_stats(&self, months: &[Ym]) -> Vec<GroupStats> {
        let reporters = self.reporters();
        let mut groups: Vec<(Option<u32>, String)> = self.groups_sorted().iter().map(|g| (Some(g.id), g.name.clone())).collect();
        groups.push((None, "No group".into()));
        groups
            .into_iter()
            .filter_map(|(gid, name)| {
                let members: Vec<u32> = reporters
                    .iter()
                    .filter(|p| match gid {
                        Some(g) => self.home_group(p.id).is_some_and(|h| h.id == g),
                        None => self.groups_of(p.id).is_empty(),
                    })
                    .map(|p| p.id)
                    .collect();
                if members.is_empty() {
                    return None;
                }
                let reports: Vec<&Report> = self.reports.iter().filter(|r| members.contains(&r.person) && months.contains(&r.month)).collect();
                let n = months.len().max(1) as f64;
                Some(GroupStats {
                    group: gid,
                    name,
                    members: members.len(),
                    avg_took_part: reports.iter().filter(|r| r.participated()).count() as f64 / n,
                    avg_studies: reports.iter().map(|r| f64::from(r.studies)).sum::<f64>() / n,
                    hours: reports.iter().map(|r| r.total()).sum(),
                    received_pct: reports.len() as f64 / (members.len() as f64 * n) * 100.0,
                })
            })
            .collect()
    }

    /// Everyone with a goal in the service year, pioneers first, then by name.
    pub fn goals(&self, start: i32) -> Vec<GoalProgress> {
        let mut out: Vec<GoalProgress> = self.reporters().iter().filter_map(|p| self.goal_progress(p.id, start)).collect();
        out.sort_by_cached_key(|g| {
            let rank = match g.kind {
                Kind::Pioneer => 0,
                Kind::ContinuousAuxiliary => 1,
                _ => 2,
            };
            (rank, self.person(g.person).map(|p| fold(&format!("{} {}", p.first_name, p.last_name))).unwrap_or_default())
        });
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dir() -> Directory {
        crate::mock::demo_directory(40)
    }

    fn draft(took_part: &str, hours: &str, credit: &str, studies: &str) -> ReportDraft {
        ReportDraft { took_part: took_part.into(), hours: hours.into(), credit: credit.into(), studies: studies.into(), ..Default::default() }
    }

    #[test]
    fn publishers_say_yes_or_no_and_pioneers_give_hours() {
        assert!(draft("", "", "", "0").validate(Kind::Publisher).iter().any(|(f, _)| *f == "took_part"));
        assert!(draft("yes", "", "", "1").validate(Kind::Publisher).is_empty());
        assert!(draft("", "", "", "1").validate(Kind::Pioneer).iter().any(|(f, _)| *f == "hours"));
        assert!(draft("", "50", "", "1").validate(Kind::Pioneer).is_empty());
        // Credit hours: at most 50, and only for pioneers.
        assert!(draft("", "40", "51", "1").validate(Kind::Pioneer).iter().any(|(f, _)| *f == "credit"));
        assert!(draft("", "40", "50", "1").validate(Kind::Pioneer).is_empty());
        assert!(!Kind::AuxiliaryMonth.has_credit());
    }

    #[test]
    fn total_hours_include_credit_and_drive_took_part() {
        let mut d = dir();
        let ym = reporting_month();
        // Mary (2) is a regular pioneer.
        assert!(d.save_report(2, ym, &draft("", "40", "10", "2")).is_ok());
        let r = d.report(2, ym).cloned();
        assert!(r.is_some_and(|r| r.total() == 50 && r.took_part && r.credit == Some(10)));
        // A publisher's credit is ignored; a monthly auxiliary pioneer reports hours.
        let mut aux = draft("", "16", "5", "0");
        aux.auxiliary = true;
        assert!(d.save_report(21, ym, &aux).is_ok());
        assert!(d.report(21, ym).is_some_and(|r| r.auxiliary && r.hours == Some(16) && r.credit.is_none()));
        // Future months are refused.
        assert!(d.save_report(2, shift(ym, 1), &draft("", "1", "", "0")).is_err());
    }

    #[test]
    fn goals_use_the_service_year_and_short_months() {
        let mut d = dir();
        let sy = service_year(reporting_month());
        let months = service_year_months(sy).into_iter().filter(|m| *m <= reporting_month()).count() as u32;
        let g = d.goal_progress(2, sy);
        assert!(g.as_ref().is_some_and(|g| g.goal == YEARLY_GOAL && g.expected == YEARLY_GOAL * months / 12));
        // Continuous auxiliary pioneer (Sarah, 6): the goal is the sum of the monthly goals.
        let first = service_year_months(sy)[0];
        d.set_aux_goal(first, true);
        let g = d.goal_progress(6, sy);
        assert!(g.is_some_and(|g| g.goal == AUX_GOAL * months - (AUX_GOAL - AUX_GOAL_SHORT)));
    }

    #[test]
    fn missing_reports_warn_from_the_5th_and_are_late_from_the_15th() {
        let now = reporting_month();
        assert_eq!(missing_status(now, 4), ReportStatus::NotYet);
        assert_eq!(missing_status(now, 5), ReportStatus::Missing);
        assert_eq!(missing_status(now, 14), ReportStatus::Missing);
        assert_eq!(missing_status(now, 15), ReportStatus::Late);
        assert_eq!(missing_status(shift(now, -1), 2), ReportStatus::Late);
    }

    #[test]
    fn auxiliary_pioneering_can_be_planned_ahead() {
        let mut d = dir();
        let next = shift(reporting_month(), 2);
        // Ethan (21) is a baptised publisher; Mary (2) is a regular pioneer.
        d.set_aux_plan(21, next, true);
        d.set_aux_plan(2, next, true);
        assert!(d.planned_aux(21).contains(&next));
        assert!(d.planned_aux(2).is_empty());
        let ethan = d.person(21).cloned();
        assert!(ethan.is_some_and(|p| d.kind_for(&p, next) == Kind::AuxiliaryMonth));
    }

    #[test]
    fn service_years_start_in_september() {
        assert_eq!(service_year((2026, 8)), 2025);
        assert_eq!(service_year((2026, 9)), 2026);
        assert_eq!(service_year_months(2025)[0], (2025, 9));
        assert_eq!(service_year_months(2025)[11], (2026, 8));
    }
}
