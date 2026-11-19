//! ⚠️ MOCK helpers and sample data. Replaced by `theo-crypto` (passphrases, zxcvbn) and real
//! module data in later phases. Nothing here is cryptographically secure.

use std::sync::atomic::{AtomicU64, Ordering};
use theo_ui::{ChartPoint, Icon, ListEntry, WidgetLayout};

pub const DEMO_USER: &str = "demo";
pub const DEMO_PASSWORD: &str = "demo-password";
pub const DEMO_CONGREGATION_PASSWORD: &str = "demo";

/// A tiny word list. The real generator uses the EFF large list (7,776 words) and a CSPRNG (R-314).
const WORDS: &[&str] = &[
    "amber", "anchor", "apple", "arrow", "autumn", "badge", "bamboo", "beacon", "birch", "bloom", "breeze", "bridge",
    "canvas", "candle", "cedar", "cherry", "cloud", "clover", "comet", "copper", "coral", "crystal", "daisy", "delta",
    "ember", "falcon", "fern", "fjord", "forest", "garnet", "glacier", "harbor", "hazel", "horizon", "island", "ivory",
    "jasmine", "juniper", "lagoon", "lantern", "lemon", "lilac", "maple", "meadow", "meteor", "mint", "moss", "nectar",
    "oasis", "olive", "orbit", "pebble", "pepper", "pine", "planet", "prairie", "quartz", "raven", "ripple", "river",
    "saffron", "sierra", "spruce", "summit", "thistle", "tulip", "valley", "willow",
];

static COUNTER: AtomicU64 = AtomicU64::new(0x9E37_79B9_7F4A_7C15);

fn next_random() -> u64 {
    let nanos = web_time::SystemTime::now()
        .duration_since(web_time::UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0);
    let mut x = COUNTER.fetch_add(0x9E37_79B9_7F4A_7C15, Ordering::Relaxed) ^ nanos;
    x ^= x << 13;
    x ^= x >> 7;
    x ^= x << 17;
    x
}

/// MOCK passphrase: 6 words (NOT secure; demo only).
pub fn passphrase() -> String {
    (0..6).map(|_| WORDS[(next_random() % WORDS.len() as u64) as usize]).collect::<Vec<_>>().join("-")
}

/// MOCK strength score 0–4 (the real one is zxcvbn, R-314/R-315).
pub fn strength(pw: &str) -> u8 {
    let len = pw.chars().count();
    let classes = [
        pw.chars().any(|c| c.is_lowercase()),
        pw.chars().any(|c| c.is_uppercase()),
        pw.chars().any(|c| c.is_ascii_digit()),
        pw.chars().any(|c| !c.is_alphanumeric()),
    ]
    .iter()
    .filter(|b| **b)
    .count();
    match (len, classes) {
        (0..=5, _) => 0,
        (6..=9, 0..=2) => 1,
        (6..=9, _) => 2,
        (10..=15, 0..=1) => 2,
        (10..=15, _) => 3,
        (16..=23, _) => 3,
        _ => 4,
    }
}

pub fn default_layout() -> Vec<WidgetLayout> {
    vec![
        WidgetLayout::new("assignments", "My assignments", Icon::Assignment, 1, 2),
        WidgetLayout::new("next-meeting", "Next meeting", Icon::Meetings, 2, 1),
        WidgetLayout::new("publishers", "Publishers", Icon::Users, 1, 1),
        WidgetLayout::new("reports", "Field service reports", Icon::Footprints, 1, 1),
        WidgetLayout::new("attendance", "Meeting attendance", Icon::Chart, 2, 1),
        WidgetLayout::new("territories", "Territories", Icon::Map, 1, 1),
        WidgetLayout::new("duties", "Duties this week", Icon::Audio, 2, 1),
        WidgetLayout::new("events", "Upcoming events", Icon::Clock, 1, 1),
    ]
}

pub fn attendance() -> Vec<ChartPoint> {
    [("Jun", 92.0), ("Jul", 88.0), ("Aug", 85.0), ("Sep", 96.0), ("Oct", 101.0), ("Nov", 104.0)]
        .into_iter()
        .map(|(l, v)| ChartPoint::new(l, v))
        .collect()
}

pub fn publishers_trend() -> Vec<f64> {
    vec![78.0, 79.0, 79.0, 81.0, 80.0, 82.0, 84.0, 84.0]
}

pub fn my_assignments() -> Vec<ListEntry> {
    vec![
        entry(Icon::Meetings, "Bible reading", "Midweek meeting", "Thu 20"),
        entry(Icon::Audio, "Audio and video", "Weekend meeting", "Sun 23"),
        entry(Icon::Users, "Attendant", "Midweek meeting", "Thu 4 Dec"),
        entry(Icon::PublicTalk, "Prayer", "Weekend meeting", "Sun 7 Dec"),
    ]
}

pub fn duties() -> Vec<ListEntry> {
    vec![
        entry(Icon::Audio, "Audio · Video", "J. Smith · M. Brown", "Thu"),
        entry(Icon::Users, "Attendants", "A. Lee · D. Clark", "Thu"),
        entry(Icon::Globe, "Zoom host", "P. Walker", "Sun"),
    ]
}

pub fn events() -> Vec<ListEntry> {
    vec![
        entry(Icon::Users, "Circuit overseer visit", "Week of 8 Dec", "3 wk"),
        entry(Icon::Globe, "Circuit assembly", "Sat 17 Jan", "9 wk"),
    ]
}

fn entry(icon: Icon, title: &str, subtitle: &str, meta: &str) -> ListEntry {
    ListEntry { icon, title: title.into(), subtitle: subtitle.into(), meta: meta.into() }
}


// ---------------------------------------------------------------------------------------------
// Synthetic people for the demo (R-022: invented names, reserved phone numbers, example.org).
// ---------------------------------------------------------------------------------------------

use crate::access::{Access, Level};
use crate::reports::{reporting_month, shift, Kind, Report, Ym, AUX_GOAL, AUX_GOAL_SHORT};
use std::collections::BTreeSet;
use crate::people::{EmergencyContact, Label, LabelColor};
use crate::tasks::CustomTask;
use crate::people::{
    AccountLink, Appointment, Directory, Family, Gender, Group, Person, PublisherType, RelationKind, Relationship,
};

/// A random-looking token for mock invite links (NOT secure).
pub fn token() -> String {
    format!("{:016x}{:016x}", next_random(), next_random())
}

const A: [Option<bool>; 6] = [Some(true); 6];
const IRR: [Option<bool>; 6] = [Some(true), Some(false), Some(true), Some(true), Some(false), Some(true)];
const INA: [Option<bool>; 6] = [Some(false); 6];
const NEW: [Option<bool>; 6] = [None, None, None, Some(true), Some(true), Some(true)];
const NONE: [Option<bool>; 6] = [None; 6];

/// (person, date) of the demo people who moved away. Henry Doyle moved away more than a year
/// ago, so the app removes him for good when the congregation is unlocked.
const MOVED_AWAY: [(u32, &str); 4] = [(33, "2026-08-15"), (34, "2026-08-15"), (35, "2025-12-20"), (36, "2025-09-30")];

type Row = (&'static str, &'static str, Gender, &'static str, PublisherType, Option<Appointment>, bool, bool, Option<&'static str>, [Option<bool>; 6]);

/// The demo congregation's people, families and groups. `size` limits the id of people.
pub fn demo_directory(size: usize) -> Directory {
    use PublisherType as T;
    let elder = Some(Appointment::Elder);
    let ms = Some(Appointment::MinisterialServant);
    #[rustfmt::skip]
    let rows: Vec<Row> = vec![
        // first, last, gender, birthday, type, appointment, anointed, infirm, consent, reports
        ("John", "Smith", Gender::Male, "1968-04-12", T::BaptisedPublisher, elder, false, false, Some("2025-01-15"), A),
        ("Mary", "Smith", Gender::Female, "1970-09-03", T::RegularPioneer, None, false, false, Some("2025-01-15"), A),
        ("Tom", "Smith", Gender::Male, "2006-02-20", T::UnbaptisedPublisher, None, false, false, Some("2025-01-15"), IRR),
        ("Emma", "Smith", Gender::Female, "2012-07-08", T::Student, None, false, false, None, NONE),
        ("Michael", "Brown", Gender::Male, "1975-11-30", T::BaptisedPublisher, ms, false, false, Some("2024-06-02"), A),
        ("Sarah", "Brown", Gender::Female, "1977-05-14", T::ContinuousAuxiliaryPioneer, None, false, false, None, A),
        ("Luis", "Garcia", Gender::Male, "1965-03-22", T::BaptisedPublisher, elder, false, false, Some("2024-03-10"), A),
        ("Ana", "Garcia", Gender::Female, "1966-08-19", T::BaptisedPublisher, None, false, false, Some("2024-03-10"), A),
        ("Sofia", "Garcia", Gender::Female, "1998-12-01", T::RegularPioneer, None, false, false, Some("2024-03-10"), A),
        ("David", "Clark", Gender::Male, "1982-01-25", T::BaptisedPublisher, ms, false, false, Some("2025-09-01"), A),
        ("Grace", "Clark", Gender::Female, "1984-04-04", T::BaptisedPublisher, None, false, false, None, IRR),
        ("Paul", "Walker", Gender::Male, "1958-10-10", T::BaptisedPublisher, elder, true, false, Some("2023-11-20"), A),
        ("Andrew", "Lee", Gender::Male, "1990-06-17", T::RegularPioneer, ms, false, false, Some("2025-02-02"), A),
        ("Hannah", "Lee", Gender::Female, "1992-02-09", T::RegularPioneer, None, false, false, Some("2025-02-02"), A),
        ("Peter", "Novak", Gender::Male, "1949-07-30", T::BaptisedPublisher, None, false, true, Some("2023-05-05"), A),
        ("Margaret", "Novak", Gender::Female, "1951-03-12", T::BaptisedPublisher, None, false, true, Some("2023-05-05"), IRR),
        ("James", "Wilson", Gender::Male, "1988-09-21", T::BaptisedPublisher, None, false, false, None, INA),
        ("Olivia", "Martin", Gender::Female, "1995-01-11", T::BaptisedPublisher, None, false, false, Some("2025-07-19"), A),
        ("Daniel", "Kowalski", Gender::Male, "1979-12-24", T::SpecialPioneer, elder, false, false, Some("2024-01-08"), A),
        ("Chloe", "Martin", Gender::Female, "2001-10-02", T::UnbaptisedPublisher, None, false, false, None, NEW),
        ("Ethan", "Taylor", Gender::Male, "1985-08-08", T::BaptisedPublisher, None, false, false, Some("2024-10-10"), IRR),
        ("Isabel", "Rossi", Gender::Female, "1972-06-06", T::BaptisedPublisher, None, false, false, Some("2022-12-12"), INA),
        ("Noah", "Fischer", Gender::Male, "2003-03-03", T::Student, None, false, false, None, NONE),
        ("Lucy", "Fischer", Gender::Female, "1974-04-28", T::Visitor, None, false, false, None, NONE),
        ("Samuel", "Okafor", Gender::Male, "1980-02-14", T::Missionary, elder, false, false, Some("2025-03-03"), A),
        ("Ruth", "Okafor", Gender::Female, "1982-09-09", T::Missionary, None, false, false, Some("2025-03-03"), A),
        ("Victor", "Hughes", Gender::Male, "1960-05-27", T::BaptisedPublisher, None, false, false, None, A),
        ("Elena", "Petrova", Gender::Female, "1993-11-16", T::RegularPioneer, None, false, false, Some("2024-08-30"), A),
        ("George", "Adams", Gender::Male, "1955-01-31", T::BaptisedPublisher, elder, false, false, Some("2023-02-14"), A),
        ("Lily", "Adams", Gender::Female, "2010-12-20", T::Student, None, false, false, None, NONE),
        ("Jos\u{e9}", "\u{c1}lvarez", Gender::Male, "1999-07-07", T::UnbaptisedPublisher, None, false, false, None, IRR),
        ("Luc\u{ed}a", "\u{c1}lvarez", Gender::Female, "1968-02-02", T::Visitor, None, false, false, None, NONE),
        // Moved away (see MOVED_AWAY below).
        ("Oscar", "Lind", Gender::Male, "1971-05-05", T::BaptisedPublisher, ms, false, false, Some("2024-02-02"), NONE),
        ("Freya", "Lind", Gender::Female, "1973-08-18", T::BaptisedPublisher, None, false, false, Some("2024-02-02"), NONE),
        ("Marta", "Kovac", Gender::Female, "1987-03-29", T::RegularPioneer, None, false, false, None, NONE),
        ("Henry", "Doyle", Gender::Male, "1962-12-01", T::BaptisedPublisher, None, false, false, None, NONE),
    ];
    // (person, type, activity pattern of the last 6 months) for the demo reports.
    let mut patterns: Vec<(u32, PublisherType, [Option<bool>; 6])> = Vec::new();
    let mut pioneer_numbers = ["4410023", "4410187", "4410254", "4410391", "4410402", "4410566", "4410671", "4410733"].iter();
    let people: Vec<Person> = rows
        .into_iter()
        .take(size)
        .enumerate()
        .map(|(i, (first, last, gender, birthday, ty, appointment, anointed, infirm, consent, reports))| {
            let n = i as u32 + 1;
            if MOVED_AWAY.iter().all(|(id, _)| *id != n) {
                patterns.push((n, ty, reports));
            }
            let local = fold_ascii(&format!("{first}.{last}"));
            let born: i32 = birthday[..4].parse().unwrap_or(1990);
            Person {
                id: n,
                first_name: first.into(),
                last_name: last.into(),
                birthday: Some(birthday.into()),
                gender,
                email: (!n.is_multiple_of(3)).then(|| format!("{local}@example.org")),
                telephone: n.is_multiple_of(4).then(|| format!("+44 1632 960{:03}", 100 + n)),
                mobile_phone: (!n.is_multiple_of(5)).then(|| format!("+44 7700 900{:03}", 100 + n)),
                baptism_date: ty.is_baptised().then(|| format!("{}-0{}-1{}", born + 17, 1 + n % 9, n % 10)),
                publisher_type: ty,
                infirm,
                pioneer_number: if ty.has_pioneer_number() { pioneer_numbers.next().map(|s| s.to_string()) } else { None },
                appointment,
                anointed,
                address_line_1: Some(format!("{} Example Street", 2 + n * 3)),
                address_line_2: n.is_multiple_of(7).then(|| "Flat 2".to_string()),
                postal_code: Some(format!("ZZ{} {}AB", 1 + n % 9, n % 10)),
                city: Some(if n.is_multiple_of(2) { "Riverside" } else { "Millbrook" }.into()),
                consent_date: consent.map(str::to_string),
                notes: (n == 15).then(|| "Prefers phone calls in the morning.".to_string()),
                // Joined the app: the demo account (John, who created the congregation) and a few
                // others. Mary has an open invite; everyone else is managed by others.
                account: match n {
                    1 => AccountLink::Linked { username: "demo".into() },
                    5 => AccountLink::Linked { username: "mbrown".into() },
                    7 => AccountLink::Linked { username: "lgarcia".into() },
                    10 => AccountLink::Linked { username: "dclark".into() },
                    12 => AccountLink::Linked { username: "pwalker".into() },
                    13 => AccountLink::Linked { username: "alee".into() },
                    2 => AccountLink::Invited { link: format!("https://app.example.org/join#t={}", token()) },
                    _ => AccountLink::None,
                },
                moved_away: MOVED_AWAY.iter().find(|(id, _)| *id == n).map(|(_, d)| d.to_string()),
                privileges: BTreeSet::new(),
                emergency_contacts: demo_emergency_contacts(n),
                labels: match n {
                    15 | 16 | 27 => [1].into_iter().collect(),
                    31 | 32 | 8 => [2].into_iter().collect(),
                    20 | 21 => [3].into_iter().collect(),
                    _ => BTreeSet::new(),
                },
            }
        })
        .collect();
    let count = people.len() as u32;
    let have = |n: u32| n <= count;
    let rel = |from, to, kind| Relationship { from, to, kind };
    #[rustfmt::skip]
    let families: Vec<Family> = vec![
        (1, vec![1, 2, 3, 4], vec![rel(2, 1, RelationKind::Spouse), rel(1, 3, RelationKind::Father), rel(1, 4, RelationKind::Father), rel(2, 3, RelationKind::Mother)]),
        (5, vec![5, 6], vec![rel(6, 5, RelationKind::Spouse)]),
        (7, vec![7, 8, 9], vec![rel(8, 7, RelationKind::Spouse), rel(9, 7, RelationKind::Daughter)]),
        (10, vec![10, 11], vec![rel(11, 10, RelationKind::Spouse)]),
        (13, vec![13, 14], vec![rel(14, 13, RelationKind::Spouse)]),
        (15, vec![15, 16], vec![rel(16, 15, RelationKind::Spouse)]),
        (18, vec![18, 20], vec![rel(20, 18, RelationKind::Sister)]),
        (24, vec![24, 23], vec![rel(24, 23, RelationKind::Mother)]),
        (25, vec![25, 26], vec![rel(26, 25, RelationKind::Spouse)]),
        (29, vec![29, 30], vec![rel(30, 29, RelationKind::Other)]),
        (32, vec![32, 31], vec![rel(32, 31, RelationKind::Mother)]),
        (33, vec![33, 34], vec![rel(34, 33, RelationKind::Spouse)]),
    ]
    .into_iter()
    .filter(|(_, members, _)| members.iter().all(|m| have(*m)))
    .enumerate()
    .map(|(i, (head, members, relationships))| Family { id: i as u32 + 1, head, members, relationships })
    .collect();
    let keep = |v: Vec<u32>| v.into_iter().filter(|m| have(*m)).collect::<Vec<_>>();
    let lead = |n: u32| have(n).then_some(n);
    let groups = vec![
        Group { id: 1, name: "North".into(), overseer: lead(1), assistant: lead(5), members: keep(vec![2, 3, 6, 17, 21, 22, 28, 29]) },
        Group { id: 2, name: "South".into(), overseer: lead(7), assistant: lead(10), members: keep(vec![8, 9, 11, 18, 20, 31]) },
        Group { id: 3, name: "East".into(), overseer: lead(12), assistant: lead(13), members: keep(vec![14, 15, 16, 19]) },
        // No assistant yet: shows the warnings on the groups page.
        Group { id: 4, name: "Town centre".into(), overseer: lead(25), assistant: None, members: keep(vec![26]) },
    ];
    let mut people = people;
    demo_privileges(&mut people);
    let (reports, aux_short_months) = demo_reports(&patterns);
    let mut aux_plans: BTreeSet<(u32, Ym)> = reports.iter().filter(|r| r.auxiliary).map(|r| (r.person, r.month)).collect();
    // Planned ahead: George (29) this month, Olivia (18) the month after.
    for (person, ahead) in [(29, 1), (18, 2)] {
        if (person as usize) <= size {
            aux_plans.insert((person, shift(reporting_month(), ahead)));
        }
    }
    let access = demo_access(count);
    let labels = vec![
        Label { id: 1, name: "Needs a ride".into(), color: LabelColor::Yellow },
        Label { id: 2, name: "Speaks Spanish".into(), color: LabelColor::Blue },
        Label { id: 3, name: "New here".into(), color: LabelColor::Green },
    ];
    let absences = demo_absences(count);
    let custom_tasks = vec![
        CustomTask { id: 1, name: "Literature desk".into(), description: Some("Hands out publications before and after the meetings.".into()), people: [8, 11, 18].into_iter().filter(|p| *p <= count).collect() },
        CustomTask { id: 2, name: "Accounts".into(), description: Some("Counts the contributions and keeps the accounts.".into()), people: [7, 10].into_iter().filter(|p| *p <= count).collect() },
        CustomTask { id: 3, name: "Garden".into(), description: Some("Keeps the Kingdom Hall garden tidy.".into()), people: vec![] },
    ];
    let attendance = demo_attendance(size);
    let mut dir = Directory {
        people,
        families,
        groups,
        reports,
        aux_short_months,
        aux_plans,
        access,
        owner: Some(1),
        log: Vec::new(),
        labels,
        absences,
        custom_tasks,
        attendance,
        events: Vec::new(),
        service_presets: Vec::new(),
        service_meetings: Vec::new(),
        notices: Vec::new(),
    };
    demo_events(&mut dir);
    demo_service_meetings(&mut dir);
    dir
}

/// Meetings for field service (invented, R-022): four weekly presets, each with the brothers
/// who said they can conduct it; this month filled, assigned and sent, next month filled and
/// assigned but not sent yet.
fn demo_service_meetings(dir: &mut Directory) {
    let conductors: Vec<u32> = dir.conductors().iter().map(|p| p.id).collect();
    let all_but = |ids: &[usize]| {
        conductors.iter().enumerate().filter(|(i, _)| !ids.contains(i)).map(|(_, c)| *c).collect::<BTreeSet<u32>>()
    };
    for (weekday, time, place, available) in [
        (1, "10:00", "Kingdom Hall", all_but(&[0, 4])),
        (3, "18:30", "Kingdom Hall", all_but(&[1, 2, 5])),
        (6, "09:30", "Kingdom Hall", all_but(&[])),
        (0, "09:00", "Smith family home, 5 Example Street", all_but(&[3, 6, 7])),
    ] {
        let _ = dir.save_service_preset(None, weekday, time, place, available);
    }
    let (y, m, _) = crate::prefs::today();
    dir.fill_service_month((y, m), true);
    dir.send_service_month((y, m));
    dir.fill_service_month(crate::reports::shift((y, m), 1), true);
}

/// A few events around today (invented, R-022): the circuit overseer's visit, a circuit assembly,
/// the Memorial, regional conventions, and two custom events, one only for the elders.
fn demo_events(dir: &mut Directory) {
    use crate::events::{add_days, EventDraft, EventKind};
    let today = crate::people::today_iso();
    let day = |n: i64| add_days(&today, n).unwrap_or_else(|| today.clone());
    let mut add = |kind: EventKind, start: i64, title: &str, description: &str, people: Option<Vec<u32>>| {
        let mut d = EventDraft::new(kind, &day(start));
        if !title.is_empty() {
            d.title = title.into();
        }
        d.description = description.into();
        if let Some(p) = people {
            d.everyone = false;
            d.people = p.into_iter().filter(|p| dir.person(*p).is_some()).collect();
            d.absence = false;
        }
        let _ = dir.save_event(None, &d);
    };
    add(EventKind::RegionalConvention, -130, "", "Three days at the assembly hall.", None);
    add(EventKind::CircuitOverseerVisit, 12, "", "Meetings, field service and visits all week.", None);
    add(EventKind::Custom, 5, "Kingdom Hall cleaning day", "Everyone is welcome to help, from 9:00.", None);
    add(EventKind::Custom, 9, "Elders' meeting", "Before the circuit overseer's visit.", Some(vec![1, 7, 12]));
    add(EventKind::CircuitAssembly, 38, "", "", None);
    add(EventKind::Memorial, 120, "", "Invite everyone you know.", None);
    add(EventKind::RegionalConvention, 235, "", "", None);
}

/// Plausible meeting attendance (invented, R-022) from the last service year up to today, for
/// meetings on Thursday and Sunday: a bit more at the weekend, a dip in summer, a week without
/// a meeting now and then (assemblies), and nothing for meetings still to come.
fn demo_attendance(size: usize) -> Vec<crate::attendance::MonthAttendance> {
    use crate::attendance::{meeting_days, Meeting, MonthAttendance};
    use crate::reports::{service_year, service_year_months};
    let (y, m, d) = crate::prefs::today();
    let start = service_year((y, m)) - 1;
    let base = size as f64;
    let mut out = Vec::new();
    for ym in service_year_months(start).into_iter().chain(service_year_months(start + 1)) {
        if ym > (y, m) {
            break;
        }
        let mut a = MonthAttendance::new(ym);
        for (meeting, day, factor) in [(Meeting::Midweek, "Thursday", 0.95), (Meeting::Weekend, "Sunday", 1.1)] {
            for (w, day_of_month) in meeting_days(ym, day).into_iter().enumerate() {
                if ym == (y, m) && day_of_month > d {
                    continue;
                }
                // A week without a meeting: the circuit assembly weekend and the convention week.
                if (ym.1 == 2 && w == 2 && meeting == Meeting::Weekend) || (ym.1 == 7 && w == 1) {
                    continue;
                }
                let season = match ym.1 {
                    7 | 8 => 0.9,
                    12 | 1 => 0.95,
                    4 => 1.08,
                    _ => 1.0,
                };
                // Small week-to-week changes, the same every time the demo opens.
                let wobble = ((ym.0 as u32 * 31 + ym.1 * 7 + w as u32 * 13 + (factor * 100.0) as u32) % 9) as f64 - 4.0;
                let trend = if ym.0 * 12 + ym.1 as i32 >= (start + 1) * 12 + 9 { 1.04 } else { 1.0 };
                let count = (base * factor * season * trend + wobble).round().max(1.0) as u32;
                a.midweek[w] = if meeting == Meeting::Midweek { Some(count) } else { a.midweek[w] };
                a.weekend[w] = if meeting == Meeting::Weekend { Some(count) } else { a.weekend[w] };
            }
        }
        if !a.is_empty() {
            out.push(a);
        }
    }
    out
}

/// Permissions of the demo accounts: a secretary, a group overseer, an audio coordinator, and
/// two publishers (one with nothing).
fn demo_access(count: u32) -> std::collections::BTreeMap<u32, Access> {
    let page = |pairs: &[(&str, Level)]| pairs.iter().map(|(p, l)| (p.to_string(), *l)).collect();
    let mut out = std::collections::BTreeMap::new();
    let mut add = |id: u32, a: Access| {
        if id <= count {
            out.insert(id, a);
        }
    };
    // Luis (7): secretary.
    add(7, Access {
        pages: page(&[("users/all", Level::Write), ("users/privileges", Level::Write), ("ministry/reports", Level::Write), ("ministry/groups", Level::Write), ("users/families", Level::Read)]),
        all_groups_read: true,
        ..Default::default()
    });
    // Paul (12): overseer of East.
    add(12, Access { own_group_reports: true, own_group_read: true, pages: page(&[("meetings/midweek", Level::Read), ("meetings/weekend", Level::Read)]), ..Default::default() });
    // Michael (5): audio and video.
    add(5, Access { pages: page(&[("departments/audio-video", Level::Write), ("departments/microphones", Level::Write), ("departments/attendants", Level::Read)]), ..Default::default() });
    // David (10): reads the meeting schedules.
    add(10, Access { pages: page(&[("meetings/midweek", Level::Read), ("meetings/weekend", Level::Read), ("meetings/events", Level::Read)]), ..Default::default() });
    out
}

/// Emergency contacts for some people (invented, R-022).
fn demo_emergency_contacts(n: u32) -> Vec<EmergencyContact> {
    let c = |first: &str, last: &str, mobile: &str, city: &str, notes: Option<&str>| EmergencyContact {
        first_name: first.into(),
        last_name: last.into(),
        mobile_phone: Some(mobile.into()),
        city: Some(city.into()),
        notes: notes.map(str::to_string),
        ..Default::default()
    };
    match n {
        15 | 16 => vec![c("Laura", "Novak", "+44 7700 900801", "Riverside", Some("Daughter. Lives nearby."))],
        22 => vec![c("Marco", "Rossi", "+44 7700 900802", "Millbrook", Some("Brother")), c("Giulia", "Rossi", "+44 7700 900803", "Millbrook", None)],
        27 => vec![c("Kevin", "Hughes", "+44 7700 900804", "Riverside", Some("Son"))],
        4 => vec![c("Anna", "Smith", "+44 7700 900805", "Riverside", Some("Aunt, when the parents cannot be reached"))],
        _ => Vec::new(),
    }
}

/// A few absences, relative to today: now, coming up and past.
fn demo_absences(count: u32) -> Vec<crate::absences::Absence> {
    let (y, m, d) = crate::prefs::today();
    let day = |offset: i64| {
        // Simple date arithmetic through the Unix day number.
        let base = crate::prefs::days_from_ymd((y, m, d)) + offset;
        let (yy, mm, dd) = crate::prefs::ymd_from_days(base);
        format!("{yy:04}-{mm:02}-{dd:02}")
    };
    let a = |id: u32, person: Option<u32>, from: i64, to: i64, notes: &str| crate::absences::Absence {
        id,
        person,
        from: day(from),
        to: day(to),
        notes: (!notes.is_empty()).then(|| notes.to_string()),
        event: None,
    };
    vec![
        a(1, Some(5), -2, 5, "Holiday"),
        a(2, Some(13), 10, 24, "Visiting family abroad"),
        a(3, Some(9), 3, 4, "Pioneer school"),
        a(4, Some(21), -30, -20, "Work trip"),
    ]
    .into_iter()
    .filter(|x| x.person.is_none_or(|p| p <= count))
    .collect()
}

/// Plausible privileges: elders take most parts, ministerial servants readings and duties,
/// sisters the student parts, and so on. Some are left out so the lists show variety.
fn demo_privileges(people: &mut [Person]) {
    use crate::privileges::Privilege as P;
    for p in people.iter_mut().filter(|p| p.moved_away.is_none()) {
        let elder = p.appointment == Some(Appointment::Elder);
        let ms = p.appointment == Some(Appointment::MinisterialServant);
        let brother = p.gender == Gender::Male;
        let mut wanted: Vec<P> = vec![P::StartingConversation, P::FollowingUp, P::MakingDisciples, P::Assistant, P::Cleaning, P::BibleReading];
        if p.publisher_type.is_baptised() {
            wanted.extend([P::Attendant, P::ZoomAttendant, P::EntranceAttendant, P::Microphones, P::WatchtowerReader, P::PublicWitnessing]);
            // Meetings for field service: elders, ministerial servants and brothers who pioneer.
            if brother && (elder || ms || p.publisher_type.is_pioneer()) {
                wanted.push(P::FieldServiceMeeting);
            }
            if brother {
                wanted.extend([P::Prayer, P::StudentTalk, P::Stage, P::Audio, P::Video, P::BibleStudyReader]);
            } else {
                wanted.push(P::Hospitality);
            }
        }
        if elder {
            wanted.extend([
                P::MidweekChairman, P::AuxiliaryChairman, P::Treasures, P::SpiritualGems, P::FieldMinistryDiscussion, P::LivingAsChristians,
                P::BibleStudyConductor, P::WeekendChairman, P::PublicTalk, P::PublicTalkAway, P::WatchtowerConductor,
            ]);
        }
        if ms {
            wanted.extend([P::SpiritualGems, P::FieldMinistryDiscussion, P::PublicTalk]);
        }
        if p.infirm {
            wanted.retain(|x| matches!(x, P::Prayer | P::StartingConversation | P::FollowingUp));
        }
        if p.id % 4 == 0 {
            wanted.push(P::ElderlyCare);
        }
        if p.id == 9 || p.id == 26 {
            wanted.push(P::Interpreter);
        }
        // Leave a few out so not everyone has everything.
        for (i, x) in wanted.into_iter().enumerate() {
            if !(p.id as usize + i).is_multiple_of(7) && x.allowed_for(p.publisher_type, p.gender) {
                p.privileges.insert(x);
            }
        }
    }
}

/// Field service reports for the last 14 months, relative to today, so the demo always looks
/// current. The last 6 months follow each person's activity pattern.
fn demo_reports(patterns: &[(u32, PublisherType, [Option<bool>; 6])]) -> (Vec<Report>, BTreeSet<Ym>) {
    let last = reporting_month();
    // 15-hour auxiliary goal months: Memorial (March), regional convention (July) and the
    // circuit overseer's visit (December).
    let short: BTreeSet<Ym> = (last.0 - 2..=last.0 + 1).flat_map(|y| [(y, 3), (y, 7), (y, 12)]).collect();
    let goal = |ym: Ym| if short.contains(&ym) { AUX_GOAL_SHORT } else { AUX_GOAL };
    let wobble = |id: u32, m: u32, span: u32| ((id * 7 + m * 13) % span) as i32 - (span / 2) as i32;
    let pioneer_base = |id: u32| match id {
        9 => 56,
        13 => 43, // behind on the yearly goal
        19 => 55,
        28 => 47,
        _ => 51,
    };
    let mut out = Vec::new();
    for (id, ty, pattern) in patterns.iter().copied() {
        if !ty.is_publisher() {
            continue;
        }
        let inactive = pattern.iter().all(|p| *p == Some(false));
        for back in (0..14).rev() {
            let ym = shift(last, -back);
            let took: Option<bool> = if back <= 5 {
                match pattern[(5 - back) as usize] {
                    None => None,
                    Some(true) => Some(true),
                    // Inactive: James (17) reports "did not take part"; others send nothing.
                    Some(false) if inactive && id == 17 => Some(false),
                    Some(false) => None,
                }
            } else if pattern[0].is_none() {
                None // new publisher: no reports before
            } else {
                Some(true)
            };
            // Not in yet for the reporting month.
            let took = if back == 0 && (id == 15 || id == 27) { None } else { took };
            let Some(took_part) = took else { continue };
            let auxiliary = ty == PublisherType::BaptisedPublisher && took_part && ((id == 18 && ym.1 == 3) || (id == 21 && ym.1 == 12));
            let kind = Kind::of(ty, auxiliary);
            let hours = match kind {
                Kind::Pioneer => Some((pioneer_base(id) + wobble(id, ym.1, 11)).max(0) as u32),
                Kind::ContinuousAuxiliary => Some((goal(ym) as i32 + wobble(id, ym.1, 7)).max(0) as u32),
                Kind::AuxiliaryMonth => Some(goal(ym) + 2),
                Kind::Publisher => None,
            };
            let credit = match (id, ym.1) {
                (14, m) if m % 3 == 0 => Some(8),
                (25, 12) => Some(12),
                _ => None,
            };
            let studies = match kind {
                Kind::Pioneer => 1 + (id + ym.1) % 4,
                Kind::ContinuousAuxiliary | Kind::AuxiliaryMonth => 1,
                Kind::Publisher if took_part => u32::from((id + ym.1).is_multiple_of(3)),
                Kind::Publisher => 0,
            };
            out.push(Report {
                person: id,
                month: ym,
                took_part,
                hours,
                credit: if kind.has_credit() { credit } else { None },
                studies,
                auxiliary,
                notes: (id == 16 && back == 2).then(|| "Witnessed by letter while recovering from surgery.".to_string()),
            });
        }
    }
    (out, short)
}

fn fold_ascii(s: &str) -> String {
    crate::people::fold(s).chars().filter(|c| c.is_ascii_alphanumeric() || *c == '.').collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn passphrase_has_six_words() {
        assert_eq!(passphrase().split('-').count(), 6);
    }

    #[test]
    fn strength_grows_with_length() {
        assert!(strength("abc") < strength("correct-horse-battery-staple"));
    }
}
