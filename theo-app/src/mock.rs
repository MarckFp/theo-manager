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
    crate::widgets::default_layout()
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
        witnessing: Default::default(),
        shift_presets: Vec::new(),
        shifts: Vec::new(),
        territories: Vec::new(),
        territory_assignments: Vec::new(),
        territory_settings: Default::default(),
        territory_categories: Vec::new(),
        territory_labels: Vec::new(),
        assignment_labels: crate::territories::default_assignment_labels(),
        ministry_log: Vec::new(),
        ministry_carried: BTreeSet::new(),
        personal_goals: Default::default(),
        midweek_weeks: Vec::new(),
        midweek_settings: Default::default(),
    };
    demo_events(&mut dir);
    demo_service_meetings(&mut dir);
    demo_witnessing(&mut dir);
    demo_territories(&mut dir);
    demo_ministry(&mut dir);
    demo_midweek(&mut dir);
    dir
}

/// Midweek meeting (invented, R-022): sample weeks from four weeks ago to eight weeks ahead,
/// with **invented sample text** (never jw.org content, which may not be distributed: the real
/// workbook is downloaded on each device). Past weeks and this month are filled and sent; the
/// next weeks are filled but not sent.
fn demo_midweek(dir: &mut Directory) {
    use crate::midweek::{monday_of, PartKind, SectionKind};
    use crate::workbook::{PartContent, WeekContent};
    let today = crate::people::today_iso();
    let this = monday_of(&today);
    let ministry = [
        [PartKind::StartingConversation, PartKind::FollowingUp, PartKind::MakingDisciples],
        [PartKind::StartingConversation, PartKind::StartingConversation, PartKind::StudentTalk],
        [PartKind::StartingConversation, PartKind::FollowingUp, PartKind::ExplainingBeliefs],
    ];
    for i in -4i64..8 {
        let monday = crate::events::add_days(&this, i * 7).unwrap_or_else(|| this.clone());
        let n = (i + 8) as usize;
        let sample = |number: u8, section: SectionKind, kind: PartKind, title: &str, minutes: u8| PartContent {
            number,
            section,
            kind,
            title: title.into(),
            description: "Sample part of the demo. The real workbook is downloaded from jw.org on each device.".into(),
            minutes,
        };
        let m = ministry[n % 3];
        let content = WeekContent {
            doc: String::new(),
            monday: monday.clone(),
            label: String::new(),
            reading: format!("SAMPLE READING {}-{}", n * 2 + 1, n * 2 + 2),
            songs: [Some((n * 7 % 150 + 1) as u16), Some((n * 13 % 150 + 1) as u16), Some((n * 29 % 150 + 1) as u16)],
            parts: vec![
                sample(1, SectionKind::Treasures, PartKind::TreasuresTalk, "Talk on the week's Bible reading", 10),
                sample(2, SectionKind::Treasures, PartKind::SpiritualGems, "Spiritual Gems", 10),
                sample(3, SectionKind::Treasures, PartKind::BibleReading, "Bible Reading", 4),
                sample(4, SectionKind::Ministry, m[0], m[0].label(), 3),
                sample(5, SectionKind::Ministry, m[1], m[1].label(), 4),
                sample(6, SectionKind::Ministry, m[2], m[2].label(), 5),
                sample(7, SectionKind::Living, PartKind::Living, "Part on Christian living", 15),
                sample(8, SectionKind::Living, PartKind::BibleStudy, "Congregation Bible Study", 30),
            ],
        };
        dir.import_midweek_week(&content, None);
        // Demo weeks have no workbook on the device: the sample text is kept with the plan.
        if let Some(w) = dir.midweek_weeks.iter_mut().find(|w| w.monday == monday) {
            for p in w.sections.iter_mut().flat_map(|s| s.parts.iter_mut()) {
                if let Some(c) = content.parts.iter().find(|c| Some(c.number) == p.source) {
                    p.title = Some(c.title.clone());
                    p.description = Some(c.description.clone());
                }
            }
        }
    }
    let all: Vec<String> = dir.midweek_weeks.iter().map(|w| w.monday.clone()).collect();
    dir.autofill_midweek(&all, 3);
    let upto: Vec<String> = all.iter().filter(|m| m.as_str() <= this.as_str()).cloned().collect();
    dir.send_midweek(&upto, 3);
    // The demo starts without notices from past weeks.
    dir.notices.retain(|n| n.date >= today);
}

/// *My ministry* (invented, R-022): John Smith (the demo user) is an auxiliary pioneer this
/// month and has logged most days so far.
fn demo_ministry(dir: &mut Directory) {
    let (y, m, today) = crate::prefs::today();
    dir.aux_plans.insert((1, (y, m)));
    for d in 1..=today {
        let (minutes, studies) = match d % 7 {
            0 => (0, 0),
            1 => (150, 1),
            2 => (60, 0),
            3 => (105, 2),
            4 => (0, 0),
            5 => (90, 1),
            _ => (120, 0),
        };
        if minutes > 0 {
            let _ = dir.save_ministry_day(1, &format!("{y:04}-{m:02}-{d:02}"), minutes, 0, studies, "");
        }
    }
}

/// Territories (invented, R-022): a grid of blocks around a town centre, with categories,
/// labels, markers, do-not-calls and two years of registry; some assigned now, two of them late.
fn demo_territories(dir: &mut Directory) {
    use crate::people::LabelColor;
    use crate::territories::{DoNotCall, MapView, Point, Shape, ShapeKind, TagKind, TerritoryDraft};
    let cat = |d: &mut Directory, n: &str, c: LabelColor| d.save_territory_tag(TagKind::Category, None, n, c).unwrap_or(0);
    let residential = cat(dir, "Residential", LabelColor::Blue);
    let business = cat(dir, "Business", LabelColor::Yellow);
    let rural = cat(dir, "Rural", LabelColor::Green);
    let lab = |d: &mut Directory, n: &str, c: LabelColor| d.save_territory_tag(TagKind::Label, None, n, c).unwrap_or(0);
    let elevator = lab(dir, "Has an elevator", LabelColor::Gray);
    let difficult = lab(dir, "Difficult access", LabelColor::Red);
    let gated = lab(dir, "Gated", LabelColor::Yellow);
    let apartments = lab(dir, "Apartments", LabelColor::Blue);
    // A 4 × 3 grid of blocks; corners a little uneven so it looks drawn by hand.
    let (lat0, lon0, dlat, dlon) = (33.9862, -117.3785, 0.0046, 0.0056);
    let corner = |r: usize, c: usize| {
        let j = ((r * 7 + c * 3) % 5) as f64 * 0.00012;
        Point::new(lat0 - r as f64 * dlat + j, lon0 + c as f64 * dlon - j * 0.6)
    };
    dir.territory_settings.maps_on = true;
    dir.territory_settings.home = Some(MapView { center: Point::new(lat0 - 1.5 * dlat, lon0 + 2.0 * dlon), zoom: 14 });
    for i in 0..12usize {
        let (r, c) = (i / 4, i % 4);
        let mut shapes = vec![Shape { kind: ShapeKind::Area, points: vec![corner(r, c), corner(r, c + 1), corner(r + 1, c + 1), corner(r + 1, c)], title: None, color: None }];
        let mid = Point::new(lat0 - (r as f64 + 0.5) * dlat, lon0 + (c as f64 + 0.5) * dlon);
        let (category, labels, dnc, description): (u32, Vec<u32>, Vec<DoNotCall>, &str) = match i {
            0 => (business, vec![elevator], vec![DoNotCall { address: "14 Market Street, flat 3B".into(), note: Some("Asked not to be visited".into()), date: Some("2026-03-02".into()) }], "Shops and offices on the ground floors, flats above. Most entrances have an intercom."),
            1 => (business, vec![elevator, apartments], vec![], "The station area. Best on weekday mornings."),
            2 => (residential, vec![apartments], vec![DoNotCall { address: "220 Lime Avenue".into(), note: None, date: Some("2025-11-20".into()) }, DoNotCall { address: "231 Lime Avenue, 2nd floor".into(), note: Some("Elderly man, asked by letter".into()), date: Some("2026-06-15".into()) }], ""),
            3 => (residential, vec![gated], vec![], "Gated houses: ring at the main gate."),
            5 => (residential, vec![apartments, elevator], vec![DoNotCall { address: "8 Orange Grove, flat 12".into(), note: None, date: None }], "Three blocks of flats around a park."),
            7 => (residential, vec![difficult], vec![], "Steep streets. A car helps."),
            8 => (rural, vec![difficult], vec![], "Farms along the river road, far apart."),
            10 => (rural, vec![], vec![DoNotCall { address: "Farm at the end of Mill Lane".into(), note: Some("Dogs, do not enter".into()), date: Some("2026-08-01".into()) }], ""),
            11 => (residential, vec![gated], vec![], ""),
            _ => (residential, vec![], vec![], ""),
        };
        if i == 0 || i == 5 {
            shapes.push(Shape { kind: ShapeKind::Marker, points: vec![mid], title: Some(if i == 0 { "Intercom at the side door" } else { "Park entrance" }.into()), color: Some(if i == 0 { LabelColor::Yellow } else { LabelColor::Green }) });
        }
        if i == 3 {
            shapes.push(Shape { kind: ShapeKind::Marker, points: vec![Point::new(mid.lat + dlat * 0.3, mid.lon - dlon * 0.3)], title: Some("Main gate".into()), color: Some(LabelColor::Red) });
        }
        if i == 6 {
            // Only one side of the boundary street.
            let (a, b) = (corner(r, c), corner(r + 1, c));
            shapes.push(Shape { kind: ShapeKind::Line, points: vec![Point::new(a.lat - 0.0004, a.lon + 0.0005), Point::new(b.lat + 0.0004, b.lon + 0.0005)], title: None, color: None });
        }
        let draft = TerritoryDraft {
            title: (i + 1).to_string(),
            category: Some(category),
            labels: labels.into_iter().collect(),
            shapes,
            view: None,
            description: description.into(),
            do_not_calls: dnc,
            url: if i == 4 { "https://example.org/territories/5".into() } else { String::new() },
        };
        let _ = dir.save_territory(None, &draft);
    }
    // Registry: two years of assignments, then who has which territory now.
    let today = crate::people::today_iso();
    let ago = |days: i64| crate::events::add_days(&today, -days).unwrap_or_else(|| today.clone());
    let people: Vec<u32> = dir.territory_people().iter().map(|p| p.id).collect();
    if people.is_empty() {
        return;
    }
    let ids: Vec<u32> = dir.territories_sorted().iter().map(|t| t.id).collect();
    for (i, t) in ids.iter().enumerate() {
        // Territories 9 and 12 were never worked.
        if i == 8 || i == 11 {
            continue;
        }
        let mut start = 700 - (i as i64 * 23) % 120;
        let mut k = i * 5;
        while start > 220 {
            let length = 45 + ((k * 37) % 90) as i64;
            let person = people[(k * 7 + 3) % people.len()];
            if let Ok(a) = dir.assign_territory(*t, person, &ago(start)) {
                let _ = dir.complete_territory(a, &ago(start - length));
            }
            start -= length + 20 + ((k * 13) % 60) as i64;
            k += 1;
        }
    }
    // Campaign labels: assignments that started in March or April (Memorial), June or July
    // (regional convention).
    for a in dir.territory_assignments.clone() {
        let month = a.assigned.get(5..7).unwrap_or("");
        let label = match month {
            "03" | "04" => Some(1),
            "06" | "07" => Some(2),
            _ => None,
        };
        if let Some(l) = label {
            dir.set_assignment_labels(a.id, &std::iter::once(l).collect());
        }
    }
    // Now: person 1 has two (one late); two are late in all.
    for (i, (person, days)) in [(1u32, 40i64), (5, 70), (7, 150), (1, 185), (13, 20), (19, 95), (25, 10)].into_iter().enumerate() {
        if let Some(t) = ids.get([0usize, 2, 3, 5, 6, 9, 10][i]) {
            let p = if people.contains(&person) { person } else { people[i % people.len()] };
            let _ = dir.assign_territory(*t, p, &ago(days));
        }
    }
}


/// Public witnessing (invented, R-022): shift supervisors on (the elders and ministerial
/// servants among the witnesses), three weekly presets with their availability; this month
/// filled, assigned and sent, next month filled and assigned but not sent.
fn demo_witnessing(dir: &mut Directory) {
    use crate::public_witnessing::Needs;
    let witnesses: Vec<(u32, bool)> = dir.witnesses().iter().map(|p| (p.id, p.appointment.is_some())).collect();
    dir.set_supervisors_on(true);
    dir.set_supervisors(witnesses.iter().filter(|(_, appointed)| *appointed).map(|(id, _)| *id).collect());
    // Every `every`-th witness cannot come at that time.
    let some = |every: usize| witnesses.iter().enumerate().filter(|(i, _)| i % every != 0).map(|(_, (id, _))| *id).collect::<BTreeSet<u32>>();
    for (weekday, start, end, place, needs, available) in [
        (2, "09:00", "11:00", "Market square", Needs { supervisors: 1, men: 0, women: 1, anyone: 1 }, some(5)),
        (4, "17:00", "19:00", "Riverside station", Needs { supervisors: 1, men: 1, women: 1, anyone: 0 }, some(2)),
        (6, "10:00", "12:00", "Hospital entrance", Needs { supervisors: 0, men: 0, women: 0, anyone: 2 }, some(3)),
    ] {
        let _ = dir.save_shift_preset(None, weekday, start, end, place, needs, available, true);
    }
    let (y, m, _) = crate::prefs::today();
    for back in [3, 2, 1] {
        let ym = crate::reports::shift((y, m), -back);
        dir.fill_shift_month(ym, true);
        dir.send_shift_month(ym);
    }
    dir.fill_shift_month((y, m), true);
    dir.send_shift_month((y, m));
    dir.fill_shift_month(crate::reports::shift((y, m), 1), true);
    // The demo starts without notices from past months.
    let today = crate::people::today_iso();
    dir.notices.retain(|n| n.date >= today);
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
