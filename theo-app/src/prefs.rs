//! Display preferences (REQUIREMENTS.md R-062): date format, time format, first day of the week,
//! language and name order.
//!
//! A congregation has **defaults**; each user may **override** any of them. The effective value
//! is the user's override if set, otherwise the congregation default.

/// How dates are written.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum DateFormat {
    /// 2026-11-17
    Ymd,
    /// 11-17-2026
    Mdy,
    /// 17-11-2026
    Dmy,
}

/// How times are written.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum TimeFormat {
    /// 19:00
    H24,
    /// 7:00 PM
    H12,
}

/// First day of the week in calendars and schedules.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum WeekStart {
    Monday,
    Sunday,
}

/// How a person's name is written.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum NameOrder {
    /// John Smith
    FirstLast,
    /// Smith, John
    LastFirst,
}

/// A preference that can be shown in a select: stable value + label.
pub trait Choice: Copy + PartialEq + 'static {
    fn all() -> &'static [Self];
    fn value(self) -> &'static str;
    fn label(self) -> &'static str;
    fn parse(v: &str) -> Option<Self> {
        Self::all().iter().copied().find(|c| c.value() == v)
    }
}

impl Choice for DateFormat {
    fn all() -> &'static [Self] {
        &[DateFormat::Ymd, DateFormat::Mdy, DateFormat::Dmy]
    }
    fn value(self) -> &'static str {
        match self {
            DateFormat::Ymd => "ymd",
            DateFormat::Mdy => "mdy",
            DateFormat::Dmy => "dmy",
        }
    }
    fn label(self) -> &'static str {
        match self {
            DateFormat::Ymd => "YYYY-MM-DD",
            DateFormat::Mdy => "MM-DD-YYYY",
            DateFormat::Dmy => "DD-MM-YYYY",
        }
    }
}

impl Choice for TimeFormat {
    fn all() -> &'static [Self] {
        &[TimeFormat::H24, TimeFormat::H12]
    }
    fn value(self) -> &'static str {
        match self {
            TimeFormat::H24 => "24h",
            TimeFormat::H12 => "12h",
        }
    }
    fn label(self) -> &'static str {
        match self {
            TimeFormat::H24 => "24-hour (19:00)",
            TimeFormat::H12 => "12-hour (7:00 PM)",
        }
    }
}

impl Choice for WeekStart {
    fn all() -> &'static [Self] {
        &[WeekStart::Monday, WeekStart::Sunday]
    }
    fn value(self) -> &'static str {
        match self {
            WeekStart::Monday => "monday",
            WeekStart::Sunday => "sunday",
        }
    }
    fn label(self) -> &'static str {
        match self {
            WeekStart::Monday => "Monday",
            WeekStart::Sunday => "Sunday",
        }
    }
}

impl Choice for NameOrder {
    fn all() -> &'static [Self] {
        &[NameOrder::FirstLast, NameOrder::LastFirst]
    }
    fn value(self) -> &'static str {
        match self {
            NameOrder::FirstLast => "first-last",
            NameOrder::LastFirst => "last-first",
        }
    }
    fn label(self) -> &'static str {
        match self {
            NameOrder::FirstLast => "First name Last name",
            NameOrder::LastFirst => "Last name, First name",
        }
    }
}

/// UI languages the app offers. Translations are not written yet: every language uses the
/// English texts for now (`locales/`), but switching already works end to end.
/// Languages whose texts are translated. Until a language is here, the app shows English texts,
/// so `<html lang>` says "en" for screen readers (R-437).
pub const TRANSLATED: &[&str] = &["en"];

pub const LANGUAGES: &[(&str, &str)] = &[
    ("en", "English"),
    ("es", "Español"),
    ("fr", "Français"),
    ("de", "Deutsch"),
    ("it", "Italiano"),
    ("pt", "Português"),
];

pub fn language_label(code: &str) -> &'static str {
    LANGUAGES.iter().find(|(c, _)| *c == code).map(|(_, l)| *l).unwrap_or("English")
}

/// The device language (browser or OS locale), if the app supports it; otherwise English.
pub fn detected_language() -> String {
    let locale = sys_locale::get_locale().unwrap_or_default().to_lowercase();
    let primary = locale.split(['-', '_']).next().unwrap_or("");
    if LANGUAGES.iter().any(|(c, _)| *c == primary) { primary.to_string() } else { "en".to_string() }
}

/// A complete set of preferences (congregation defaults, or the effective values).
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Preferences {
    pub date: DateFormat,
    pub time: TimeFormat,
    pub week_start: WeekStart,
    pub language: String,
    pub name_order: NameOrder,
}

impl Default for Preferences {
    fn default() -> Self {
        Self {
            date: DateFormat::Dmy,
            time: TimeFormat::H24,
            week_start: WeekStart::Monday,
            language: detected_language(),
            name_order: NameOrder::FirstLast,
        }
    }
}

/// A user's overrides. `None` means "use the congregation default".
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Overrides {
    pub date: Option<DateFormat>,
    pub time: Option<TimeFormat>,
    pub week_start: Option<WeekStart>,
    pub language: Option<String>,
    pub name_order: Option<NameOrder>,
}

impl Preferences {
    /// Applies a user's overrides on top of these defaults.
    pub fn with(&self, o: &Overrides) -> Preferences {
        Preferences {
            date: o.date.unwrap_or(self.date),
            time: o.time.unwrap_or(self.time),
            week_start: o.week_start.unwrap_or(self.week_start),
            language: o.language.clone().unwrap_or_else(|| self.language.clone()),
            name_order: o.name_order.unwrap_or(self.name_order),
        }
    }

    pub fn format_date(&self, (y, m, d): (i32, u32, u32)) -> String {
        match self.date {
            DateFormat::Ymd => format!("{y:04}-{m:02}-{d:02}"),
            DateFormat::Mdy => format!("{m:02}-{d:02}-{y:04}"),
            DateFormat::Dmy => format!("{d:02}-{m:02}-{y:04}"),
        }
    }

    pub fn format_time(&self, hour: u32, minute: u32) -> String {
        match self.time {
            TimeFormat::H24 => format!("{hour:02}:{minute:02}"),
            TimeFormat::H12 => {
                let suffix = if hour < 12 { "AM" } else { "PM" };
                let h = match hour % 12 {
                    0 => 12,
                    h => h,
                };
                format!("{h}:{minute:02} {suffix}")
            }
        }
    }

    pub fn format_name(&self, first: &str, last: &str) -> String {
        match self.name_order {
            NameOrder::FirstLast => format!("{first} {last}"),
            NameOrder::LastFirst => format!("{last}, {first}"),
        }
    }

    /// Formats an "HH:MM" string (as stored for meeting times).
    pub fn format_hhmm(&self, hhmm: &str) -> String {
        let mut it = hhmm.split(':').map(|p| p.parse::<u32>().unwrap_or(0));
        let (h, m) = (it.next().unwrap_or(0), it.next().unwrap_or(0));
        self.format_time(h, m)
    }
}

/// Today's date in UTC (the time zone of the congregation is applied later, with theo-core).
pub fn today() -> (i32, u32, u32) {
    let secs = web_time::SystemTime::now()
        .duration_since(web_time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    civil_from_days((secs / 86_400) as i64)
}

/// Day number (days since 1970-01-01) of a date.
pub fn days_from_ymd(d: (i32, u32, u32)) -> i64 {
    days_from_civil(d)
}

/// The date of a day number.
pub fn ymd_from_days(z: i64) -> (i32, u32, u32) {
    civil_from_days(z)
}

/// Now in UTC as `YYYY-MM-DD HH:MM` (activity log).
pub fn now_stamp() -> String {
    let secs = web_time::SystemTime::now().duration_since(web_time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
    let (y, m, d) = civil_from_days((secs / 86_400) as i64);
    let rest = secs % 86_400;
    format!("{y:04}-{m:02}-{d:02} {:02}:{:02}", rest / 3600, rest % 3600 / 60)
}

/// (year, month, day) → days since 1970-01-01. Howard Hinnant's algorithm.
fn days_from_civil((y, m, d): (i32, u32, u32)) -> i64 {
    let y = i64::from(y) - i64::from(m <= 2);
    let era = y.div_euclid(400);
    let yoe = y.rem_euclid(400);
    let m = i64::from(m);
    let doy = (153 * (if m > 2 { m - 3 } else { m + 9 }) + 2) / 5 + i64::from(d) - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

/// Whole and partial years from `from` to today, rounded down to one decimal (23.5).
/// `None` for a date in the future.
pub fn years_since(from: (i32, u32, u32)) -> Option<f64> {
    years_between(from, today())
}

fn years_between(from: (i32, u32, u32), to: (i32, u32, u32)) -> Option<f64> {
    let days = days_from_civil(to) - days_from_civil(from);
    (days >= 0).then(|| (days as f64 / 365.2425 * 10.0).floor() / 10.0)
}

/// Days since 1970-01-01 → (year, month, day). Howard Hinnant's algorithm.
fn civil_from_days(z: i64) -> (i32, u32, u32) {
    let z = z + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    ((if m <= 2 { y + 1 } else { y }) as i32, m, d)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn prefs() -> Preferences {
        Preferences {
            date: DateFormat::Dmy,
            time: TimeFormat::H24,
            week_start: WeekStart::Monday,
            language: "en".into(),
            name_order: NameOrder::FirstLast,
        }
    }

    #[test]
    fn overrides_win_over_defaults() {
        let o = Overrides { time: Some(TimeFormat::H12), ..Default::default() };
        let p = prefs().with(&o);
        assert_eq!(p.time, TimeFormat::H12);
        assert_eq!(p.date, DateFormat::Dmy);
    }

    #[test]
    fn formats() {
        let mut p = prefs();
        assert_eq!(p.format_date((2026, 11, 7)), "07-11-2026");
        p.date = DateFormat::Ymd;
        assert_eq!(p.format_date((2026, 11, 7)), "2026-11-07");
        p.time = TimeFormat::H12;
        assert_eq!(p.format_time(0, 5), "12:05 AM");
        assert_eq!(p.format_time(19, 0), "7:00 PM");
        p.name_order = NameOrder::LastFirst;
        assert_eq!(p.format_name("John", "Smith"), "Smith, John");
    }

    #[test]
    fn years_have_one_decimal() {
        assert_eq!(days_from_civil((2026, 11, 17)), 20_774);
        assert_eq!(years_between((2003, 5, 17), (2026, 11, 17)), Some(23.5));
        assert_eq!(years_between((2026, 11, 18), (2026, 11, 17)), None);
    }

    #[test]
    fn civil_dates() {
        assert_eq!(civil_from_days(0), (1970, 1, 1));
        assert_eq!(civil_from_days(20_774), (2026, 11, 17));
    }
}
