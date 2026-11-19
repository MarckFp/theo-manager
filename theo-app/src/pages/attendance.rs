//! *Meetings › Meeting attendance*: the months of a service year, one month's record (five
//! weeks × midweek and weekend meeting) and statistics. Rules: `docs/model/m11-meeting-attendance.md`.

use crate::attendance::{meeting_days, show_average, Meeting, MonthAttendance, Totals, MAX_ATTENDANCE, WEEKS};
use crate::export::{Format, Table};
use crate::people::Directory;
use crate::prefs;
use crate::reports::{month_label, service_year, service_year_label, short_month, Ym};
use crate::state::use_app_state;
use crate::Route;
use dioxus::prelude::*;
use theo_ui::*;

fn this_month() -> Ym {
    let (y, m, _) = prefs::today();
    (y, m)
}

/// `2026-11` for a route.
fn month_param((y, m): Ym) -> String {
    format!("{y:04}-{m:02}")
}

fn parse_month(s: &str) -> Option<Ym> {
    let (y, m) = s.split_once('-')?;
    let (y, m) = (y.parse::<i32>().ok()?, m.parse::<u32>().ok()?);
    (1..=12).contains(&m).then_some((y, m))
}

/// The weekday of each meeting, from the congregation (`"Thursday"`, `"Sunday"`).
fn meeting_weekday(state: &crate::state::AppState, m: Meeting) -> String {
    state
        .active_congregation()
        .map(|c| match m {
            Meeting::Midweek => c.midweek.day,
            Meeting::Weekend => c.weekend.day,
        })
        .unwrap_or_default()
}

/// Signed difference for a stat tile, with its direction.
fn delta(now: Option<f64>, before: Option<f64>, against: &str) -> (Option<String>, Trend) {
    match (now, before) {
        (Some(a), Some(b)) => {
            let d = a.round() as i64 - b.round() as i64;
            let trend = match d.signum() {
                1 => Trend::Up,
                -1 => Trend::Down,
                _ => Trend::Flat,
            };
            (Some(format!("{}{d} vs {against}", if d > 0 { "+" } else { "" })), trend)
        }
        _ => (None, Trend::Flat),
    }
}

/// The yearly record (like the paper summary): per month, meetings, total and average of each meeting.
fn year_rows(dir: &Directory, start: i32) -> (Vec<Vec<String>>, Vec<String>) {
    let now = this_month();
    let cells = |t: Totals| {
        if t.meetings == 0 {
            vec!["–".to_string(), "–".into(), "–".into()]
        } else {
            vec![t.meetings.to_string(), t.total.to_string(), show_average(t.average())]
        }
    };
    let rows = dir
        .attendance_year(start)
        .iter()
        .filter(|a| a.month <= now)
        .map(|a| {
            let mut row = vec![month_label(a.month)];
            row.extend(cells(a.stats(Meeting::Midweek)));
            row.extend(cells(a.stats(Meeting::Weekend)));
            row
        })
        .collect();
    let mut footer = vec!["Service year".to_string()];
    footer.extend(cells(dir.year_totals(start, Meeting::Midweek)));
    footer.extend(cells(dir.year_totals(start, Meeting::Weekend)));
    (rows, footer)
}

// ---------------------------------------------------------------------------------------------
// The page: months and statistics of a service year
// ---------------------------------------------------------------------------------------------

/// `/app/meetings/attendance`
#[component]
pub fn AttendancePage() -> Element {
    let mut state = use_app_state();
    let toaster = use_toaster();
    let congregation = state.active_congregation().map(|c| c.name).unwrap_or_default();
    let dir = state.directory();
    let tab = (state.attendance_tab)();
    let current = service_year(this_month());
    let first = dir.first_attendance_year(current);
    let year = (state.attendance_year)().unwrap_or(current).clamp(first, current);

    let on_export = {
        let (dir, congregation) = (dir.clone(), congregation.clone());
        move |id: String| {
            let Some(format) = Format::parse(&id) else { return };
            let (mut rows, footer) = year_rows(&dir, year);
            rows.push(footer);
            let columns = ["Month", "Midweek meetings", "Midweek total", "Midweek average", "Weekend meetings", "Weekend total", "Weekend average"];
            let table = Table {
                title: format!("{congregation} · Meeting attendance"),
                subtitle: format!("Service year {}", service_year_label(year)),
                columns: columns.iter().map(|c| c.to_string()).collect(),
                widths: vec![2.0, 1.2, 1.2, 1.2, 1.2, 1.2, 1.2],
                rows,
            };
            super::users::save_export(&table, format, &congregation, &format!("attendance-{}", service_year_label(year).replace('/', "-")), toaster);
        }
    };

    rsx! {
        Page {
            title: "Meeting attendance",
            subtitle: "{congregation} · How many attend each meeting, month by month",
            actions: rsx! {
                super::users::ExportMenu { csv_description: "The yearly record, for spreadsheets", onselect: on_export }
            },
            StickyBar {
                Segmented {
                    value: tab.clone(),
                    options: vec![
                        Segment { value: "months".into(), label: "Months".into(), icon: Some(Icon::Calendar) },
                        Segment { value: "statistics".into(), label: "Statistics".into(), icon: Some(Icon::Chart) },
                    ],
                    onchange: move |v: String| state.attendance_tab.set(v),
                }
                YearPicker {
                    value: year,
                    min: first,
                    max: current,
                    current,
                    label: format!("Service year {}", service_year_label(year)),
                    full_width: true,
                    onchange: move |y| state.attendance_year.set(Some(y)),
                }
            }
            if tab == "statistics" {
                StatisticsTab { year }
            } else {
                MonthsTab { year }
            }
        }
    }
}

#[component]
fn MonthsTab(year: i32) -> Element {
    let state = use_app_state();
    let navigator = navigator();
    let dir = state.directory();
    let now = this_month();
    // Newest first: the month to fill in is on top.
    let mut months: Vec<MonthAttendance> = dir.attendance_year(year).into_iter().filter(|a| a.month <= now).collect();
    months.reverse();
    rsx! {
        Grid { cols: 3, gap: Gap::Sm,
            for a in months {
                {
                    let ym = a.month;
                    let (mid, wk) = (a.stats(Meeting::Midweek), a.stats(Meeting::Weekend));
                    let held = mid.meetings + wk.meetings;
                    let subtitle = if held == 0 {
                        "Nothing entered yet".to_string()
                    } else {
                        format!("Average: midweek {} · weekend {}", show_average(mid.average()), show_average(wk.average()))
                    };
                    let tags = if held == 0 && ym < now {
                        vec![Tag::new("Not entered", Tone::Warning).with_icon(Icon::Warning)]
                    } else if held == 0 {
                        vec![Tag::new("This month", Tone::Info).with_icon(Icon::Calendar)]
                    } else {
                        vec![Tag::new(if held == 1 { "1 meeting".to_string() } else { format!("{held} meetings") }, Tone::Neutral).with_icon(Icon::Users)]
                    };
                    rsx! {
                        RecordCard {
                            key: "{ym.0}-{ym.1}",
                            title: month_label(ym),
                            avatar: theo_ui::month_name(ym.1).to_string(),
                            subtitle,
                            tags,
                            onclick: move |_| { navigator.push(Route::AttendanceMonth { month: month_param(ym) }); },
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn StatisticsTab(year: i32) -> Element {
    let state = use_app_state();
    let prefs = state.preferences();
    let dir = state.directory();
    let mid = dir.year_totals(year, Meeting::Midweek);
    let wk = dir.year_totals(year, Meeting::Weekend);
    if mid.meetings + wk.meetings == 0 {
        return rsx! {
            EmptyState { icon: Icon::Chart, title: "No attendance in this service year", description: "Enter the attendance of a month in the Months tab." }
        };
    }
    let before = service_year_label(year - 1);
    let (mid_d, mid_t) = delta(mid.average(), dir.year_totals(year - 1, Meeting::Midweek).average(), &before);
    let (wk_d, wk_t) = delta(wk.average(), dir.year_totals(year - 1, Meeting::Weekend).average(), &before);
    let peak = dir.peak(year);
    let peak_when = peak.map(|p| {
        let day = meeting_days(p.month, &meeting_weekday(&state, p.meeting)).get(p.week).copied();
        match day {
            Some(d) => format!("{} · {}", p.meeting.short(), prefs.format_date((p.month.0, p.month.1, d))),
            None => format!("{} · {}, week {}", p.meeting.short(), month_label(p.month), p.week + 1),
        }
    });
    let publishers = dir.reporters().len();
    let share = wk.average().filter(|_| publishers > 0).map(|a| a / publishers as f64 * 100.0);

    // Per month (September to August) for the chart and the ranking.
    let months = dir.attendance_year(year);
    let labels: Vec<String> = months.iter().map(|a| short_month(a.month)).collect();
    let avg = |a: &MonthAttendance, m: Meeting| a.stats(m).average().map(f64::round);
    let series = vec![
        ChartSeries::new("Midweek", months.iter().map(|a| avg(a, Meeting::Midweek)).collect()),
        ChartSeries::new("Weekend", months.iter().map(|a| avg(a, Meeting::Weekend)).collect()),
    ];
    // Months ranked by the average of all their meetings.
    let mut ranked: Vec<(Ym, f64, String)> = months
        .iter()
        .filter_map(|a| {
            let both = a.stats(Meeting::Midweek).add(a.stats(Meeting::Weekend));
            both.average().map(|v| {
                (a.month, v, format!("Midweek {} · weekend {}", show_average(a.stats(Meeting::Midweek).average()), show_average(a.stats(Meeting::Weekend).average())))
            })
        })
        .collect();
    ranked.sort_by(|a, b| b.1.total_cmp(&a.1));
    let top = ranked.first().map(|r| r.1).unwrap_or(1.0);
    let gap = match (mid.average(), wk.average()) {
        (Some(m), Some(w)) if m > 0.0 => Some((w - m, (w - m) / m * 100.0)),
        _ => None,
    };
    let (rows, footer) = year_rows(&dir, year);

    rsx! {
        Grid { cols: 4, gap: Gap::Sm, compact: true,
            Card { StatTile { label: "Midweek meeting (average)", value: show_average(mid.average()), icon: Icon::BookOpen, delta: mid_d, trend: mid_t } }
            Card { StatTile { label: "Weekend meeting (average)", value: show_average(wk.average()), icon: Icon::Talk, delta: wk_d, trend: wk_t } }
            Card { StatTile { label: "Highest attendance", value: peak.map(|p| p.count.to_string()).unwrap_or_else(|| "–".into()), icon: Icon::Sparkles, delta: peak_when } }
            Card {
                StatTile {
                    label: "Weekend average vs publishers",
                    value: share.map(|s| format!("{s:.0}%")).unwrap_or_else(|| "–".into()),
                    icon: Icon::Users,
                    delta: format!("{publishers} publishers"),
                }
            }
        }
        Grid { cols: 2,
            Card { title: "Average attendance per month", icon: Icon::Chart,
                LineChart { labels, series }
            }
            Card { title: "Months by average attendance", icon: Icon::Calendar,
                Stack { gap: Gap::Sm,
                    Text { tone: TextTone::Small, "Both meetings together, highest first." }
                    for (ym, v, detail) in ranked.iter() {
                        ProgressRow { key: "{ym.0}-{ym.1}", label: month_label(*ym), detail: format!("{}", v.round() as u32), value: *v, max: top, note: detail.clone() }
                    }
                }
            }
        }
        Card { title: "Highlights", icon: Icon::Sparkles,
            ItemList {
                items: {
                    let mut items = Vec::new();
                    if let (Some(best), Some(low)) = (ranked.first(), ranked.last()) {
                        items.push(ListEntry { icon: Icon::ArrowRight, title: format!("Best month: {}", month_label(best.0)), subtitle: format!("Average {} · {}", best.1.round() as u32, best.2), meta: String::new() });
                        if ranked.len() > 1 {
                            items.push(ListEntry { icon: Icon::ArrowRight, title: format!("Lowest month: {}", month_label(low.0)), subtitle: format!("Average {} · {}", low.1.round() as u32, low.2), meta: String::new() });
                        }
                    }
                    if let Some((d, pct)) = gap {
                        let more = if d >= 0.0 { "more" } else { "fewer" };
                        items.push(ListEntry {
                            icon: Icon::Users,
                            title: format!("{} {more} people at the weekend meeting", d.abs().round() as u32),
                            subtitle: format!("{:+.0}% compared with the midweek meeting, on average", pct),
                            meta: String::new(),
                        });
                    }
                    items.push(ListEntry {
                        icon: Icon::Calendar,
                        title: format!("{} meetings recorded", mid.meetings + wk.meetings),
                        subtitle: format!("{} midweek, {} weekend · {} people in total", mid.meetings, wk.meetings, mid.total + wk.total),
                        meta: String::new(),
                    });
                    items
                },
            }
        }
        Card { title: "Yearly record", icon: Icon::Report,
            DataTable {
                caption: format!("Meeting attendance, service year {}", service_year_label(year)),
                groups: vec![(String::new(), 1), ("Midweek meeting".into(), 3), ("Weekend meeting".into(), 3)],
                columns: ["Month", "Meetings", "Total", "Average", "Meetings", "Total", "Average"].iter().map(|c| c.to_string()).collect(),
                rows,
                footer,
            }
        }
    }
}

// ---------------------------------------------------------------------------------------------
// One month
// ---------------------------------------------------------------------------------------------

/// `/app/meetings/attendance/<YYYY-MM>`
#[component]
pub fn AttendanceMonth(month: String) -> Element {
    let mut state = use_app_state();
    let navigator = navigator();
    let congregation = state.active_congregation().map(|c| c.name).unwrap_or_default();
    let dir = state.directory();
    let now = this_month();
    let back = ("Meeting attendance".to_string(), EventHandler::new(move |_| { navigator.push(Route::AttendancePage {}); }));
    let Some(ym) = parse_month(&month).filter(|m| *m <= now) else {
        return rsx! {
            Page { title: "Month not found", back,
                EmptyState { icon: Icon::Calendar, title: "There is no record for this month", description: "Choose a month from the list." }
            }
        };
    };
    // Coming back to the list shows this month's service year.
    use_effect(move || state.attendance_year.set(Some(service_year(ym))));

    let min = (dir.first_attendance_year(service_year(now)) - 1, 9);
    let a = dir.attendance(ym);
    let prev = dir.attendance(crate::reports::shift(ym, -1));
    let last_year = dir.attendance((ym.0 - 1, ym.1));
    let columns: Vec<GridColumn> = (1..=WEEKS).map(|w| GridColumn { label: format!("Week {w}"), hint: None }).collect();
    let rows: Vec<GridRow> = Meeting::ALL
        .iter()
        .map(|m| {
            let t = a.stats(*m);
            let weekday = meeting_weekday(&state, *m);
            let short: String = weekday.chars().take(3).collect();
            let days = meeting_days(ym, &weekday);
            GridRow {
                label: m.label().into(),
                icon: match m {
                    Meeting::Midweek => Icon::BookOpen,
                    Meeting::Weekend => Icon::Talk,
                },
                values: a.row(*m).to_vec(),
                summary: vec![t.meetings.to_string(), t.total.to_string(), show_average(t.average())],
                placeholders: (0..WEEKS).map(|w| days.get(w).map(|d| format!("{short} {d}")).unwrap_or_default()).collect(),
            }
        })
        .collect();
    let prev_label = theo_ui::month_name(crate::reports::shift(ym, -1).1).to_string();
    let compare = |m: Meeting| {
        let now = a.stats(m).average();
        let (d1, t1) = delta(now, prev.stats(m).average(), &prev_label);
        let (d2, t2) = delta(now, last_year.stats(m).average(), &format!("{} {}", &theo_ui::month_name(ym.1)[..3], ym.0 - 1));
        // One colour for both changes: up or down only when they agree.
        let trend = match (d1.is_some(), d2.is_some()) {
            (true, true) if t1 != t2 => Trend::Flat,
            (true, _) => t1,
            _ => t2,
        };
        let text = [d1, d2].into_iter().flatten().collect::<Vec<_>>().join(" · ");
        (show_average(now), (!text.is_empty()).then_some(text), trend)
    };
    let (mid_v, mid_d, mid_t) = compare(Meeting::Midweek);
    let (wk_v, wk_d, wk_t) = compare(Meeting::Weekend);

    rsx! {
        Page {
            title: month_label(ym),
            subtitle: "{congregation} · Meeting attendance",
            back,
            StickyBar {
                MonthPicker { value: ym, min, max: now, today: now, full_width: true, onchange: move |v: Ym| { navigator.replace(Route::AttendanceMonth { month: month_param(v) }); } }
            }
            Card { title: "Attendance", icon: Icon::Users,
                Text { tone: TextTone::Muted,
                    "Count everyone at each meeting, in person and by video conference together. Leave a week empty when there was no meeting (for example during an assembly)."
                }
                NumberGrid {
                    columns,
                    summary_columns: vec!["Meetings".into(), "Total".into(), "Average".into()],
                    rows,
                    max: MAX_ATTENDANCE,
                    onchange: move |(r, w, v): (usize, usize, Option<u32>)| {
                        if let Some(m) = Meeting::ALL.get(r) {
                            state.update_directory(|d| d.set_attendance(ym, *m, w, v));
                        }
                    },
                }
                Text { tone: TextTone::Small, "Changes are saved automatically." }
            }
            Grid { cols: 2, gap: Gap::Sm, compact: true,
                Card { StatTile { label: "Midweek meeting (average)", value: mid_v, icon: Icon::BookOpen, delta: mid_d, trend: mid_t } }
                Card { StatTile { label: "Weekend meeting (average)", value: wk_v, icon: Icon::Talk, delta: wk_d, trend: wk_t } }
            }
        }
    }
}
