//! A month calendar for things that last one or more days (events, absences, schedules).
//!
//! Larger screens: a month grid; things that last several days are bars across the days (cut at
//! the end of a week). Phones: a compact grid with a dot per thing on each day, and below it the
//! list of the month (or of the day you tap).

use crate::feedback::Tone;
use crate::icon::{Icon, IconSize, IconView};
use crate::period::{MonthPicker, YearMonth};
use dioxus::prelude::*;

/// A calendar date (year, month, day).
pub type Date = (i32, u32, u32);

/// One thing on a [`Calendar`].
#[derive(Clone, Debug, PartialEq)]
pub struct CalendarEntry {
    /// Sent to `on_select`.
    pub id: String,
    /// Title on the bar and in the list.
    pub title: String,
    /// When, as the user reads it ("21–23 November"): shown in the list.
    pub subtitle: String,
    /// First day.
    pub from: Date,
    /// Last day (included).
    pub to: Date,
    /// Colour of the bar or dot.
    pub tone: Tone,
    /// Icon in the list.
    pub icon: Icon,
    /// A second line on the bars of large screens (for example the place).
    pub detail: Option<String>,
}

impl CalendarEntry {
    fn covers(&self, d: Date) -> bool {
        self.from <= d && d <= self.to
    }
}

/// Days since 1970-01-01 (proleptic Gregorian).
fn day_number((y, m, d): Date) -> i64 {
    let y = i64::from(y) - i64::from(m <= 2);
    let era = y.div_euclid(400);
    let yoe = y.rem_euclid(400);
    let mp = (i64::from(m) + 9) % 12;
    let doy = (153 * mp + 2) / 5 + i64::from(d) - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

fn date_of(z: i64) -> Date {
    let z = z + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    let y = (yoe + era * 400 + i64::from(m <= 2)) as i32;
    (y, m, d)
}

fn bar_class(tone: Tone) -> &'static str {
    match tone {
        Tone::Info => "bg-info/15 text-info hover:bg-info/25",
        Tone::Success => "bg-success/15 text-success hover:bg-success/25",
        Tone::Warning => "bg-warning/20 text-warning hover:bg-warning/30",
        Tone::Error => "bg-error/15 text-error hover:bg-error/25",
        Tone::Neutral => "bg-base-content/10 text-base-content hover:bg-base-content/15",
    }
}

fn dot_class(tone: Tone) -> &'static str {
    match tone {
        Tone::Info => "bg-info",
        Tone::Success => "bg-success",
        Tone::Warning => "bg-warning",
        Tone::Error => "bg-error",
        Tone::Neutral => "bg-base-content/50",
    }
}

/// Most bars shown per week row on larger screens; the rest is "+n more" on the day.
const MAX_LANES: usize = 3;

/// One bar in a week row: the entry, its first and last column (0–6) and its lane.
struct Bar {
    entry: usize,
    start: usize,
    end: usize,
    lane: usize,
    starts_here: bool,
    ends_here: bool,
}

/// A month calendar. `today` is marked. `week_starts_sunday` follows the user's preference.
/// Months outside `min..=max` cannot be chosen.
#[component]
pub fn Calendar(
    month: YearMonth,
    entries: Vec<CalendarEntry>,
    today: Date,
    min: YearMonth,
    max: YearMonth,
    on_month: EventHandler<YearMonth>,
    on_select: EventHandler<String>,
    #[props(default)] week_starts_sunday: bool,
    /// Shown in the phone list when the month has nothing.
    #[props(default)]
    empty_text: Option<String>,
) -> Element {
    let mut picked = use_signal(|| None::<Date>);

    let first = day_number((month.0, month.1, 1));
    let (ny, nm) = if month.1 == 12 { (month.0 + 1, 1) } else { (month.0, month.1 + 1) };
    let last = day_number((ny, nm, 1)) - 1;
    // 1970-01-01 was a Thursday: weekday 0 = Sunday.
    let weekday = |z: i64| (z + 4).rem_euclid(7);
    let lead = if week_starts_sunday { weekday(first) } else { (weekday(first) + 6) % 7 };
    let grid_start = first - lead;
    let weeks = ((last - grid_start) / 7 + 1) as usize;
    let names = ["Sunday", "Monday", "Tuesday", "Wednesday", "Thursday", "Friday", "Saturday"];
    let order: Vec<&str> = (0..7).map(|i| names[(i + if week_starts_sunday { 0 } else { 1 }) % 7]).collect();

    let mut sorted: Vec<usize> = (0..entries.len()).collect();
    sorted.sort_by_key(|i| (entries[*i].from, std::cmp::Reverse(entries[*i].to)));

    // Bars per week row, each in the first free lane.
    let rows: Vec<(i64, Vec<Bar>)> = (0..weeks)
        .map(|w| {
            let ws = grid_start + 7 * w as i64;
            let we = ws + 6;
            let mut lanes: Vec<usize> = Vec::new(); // last column used per lane
            let mut bars = Vec::new();
            for &i in &sorted {
                let (s, e) = (day_number(entries[i].from), day_number(entries[i].to));
                if e < ws || s > we {
                    continue;
                }
                let start = (s.max(ws) - ws) as usize;
                let end = (e.min(we) - ws) as usize;
                let lane = match lanes.iter().position(|used| *used < start) {
                    Some(l) => {
                        lanes[l] = end;
                        l
                    }
                    None => {
                        lanes.push(end);
                        lanes.len() - 1
                    }
                };
                bars.push(Bar { entry: i, start, end, lane, starts_here: s >= ws, ends_here: e <= we });
            }
            (ws, bars)
        })
        .collect();

    let in_month = move |z: i64| z >= first && z <= last;
    // Bars get a second line on extra-large screens when entries have details.
    let two_lines = entries.iter().any(|e| e.detail.is_some());
    // A tapped day counts only while it is on the shown grid (another month forgets it).
    let grid_end = grid_start + 7 * weeks as i64 - 1;
    let chosen = picked().filter(|d| (grid_start..=grid_end).contains(&day_number(*d)));
    let today_n = day_number(today);
    let month_list: Vec<usize> = sorted
        .iter()
        .copied()
        .filter(|i| day_number(entries[*i].to) >= first && day_number(entries[*i].from) <= last)
        .collect();
    let list: Vec<usize> = match chosen {
        Some(d) => sorted.iter().copied().filter(|i| entries[*i].covers(d)).collect(),
        None => month_list,
    };
    let list_title = match chosen {
        Some((y, m, d)) => format!("{d} {} {y}", crate::period::month_name(m)),
        None => format!("{} {}", crate::period::month_name(month.1), month.0),
    };

    rsx! {
        div { class: "flex flex-col gap-3",
            MonthPicker { value: month, min, max, today: (today.0, today.1), full_width: true, onchange: move |m| on_month.call(m) }

            // Larger screens: the month grid with bars.
            div { class: "hidden overflow-hidden rounded-box border border-base-300 md:block",
                div { class: "grid grid-cols-7 border-b border-base-300 bg-base-200/60",
                    for name in order.iter() {
                        div { key: "{name}", class: "px-2 py-1.5 text-xs font-medium text-base-content/60", "{&name[..3]}" }
                    }
                }
                for (w, (ws, bars)) in rows.iter().enumerate() {
                    {
                        let shown_lanes = bars.iter().map(|b| b.lane + 1).max().unwrap_or(0).min(MAX_LANES);
                        let template = format!("--lane: 1.625rem; grid-template-rows: 2rem repeat({shown_lanes}, var(--lane)) minmax(1.5rem, 1fr);");
                        let lane_size = if two_lines { "xl:[--lane:2.5rem]" } else { "" };
                        rsx! {
                            div { key: "{w}", class: "relative grid min-h-28 grid-cols-7 gap-y-1 border-b border-base-300 pb-1 last:border-b-0 {lane_size}", style: "{template}",
                                for c in 0..7usize {
                                    {
                                        let z = ws + c as i64;
                                        let (_, _, d) = date_of(z);
                                        let hidden = bars.iter().filter(|b| b.lane >= MAX_LANES && b.start <= c && c <= b.end).count();
                                        let is_today = z == today_n;
                                        rsx! {
                                            div {
                                                key: "d{c}",
                                                class: if c > 0 { "flex flex-col justify-between border-l border-base-300 px-1.5 pt-1" } else { "flex flex-col justify-between px-1.5 pt-1" },
                                                style: "grid-column: {c + 1}; grid-row: 1 / -1;",
                                                span {
                                                    class: if is_today { "grid size-6 place-items-center rounded-full bg-primary text-xs font-semibold text-primary-content" } else if in_month(z) { "grid size-6 place-items-center text-xs font-medium" } else { "grid size-6 place-items-center text-xs text-base-content/35" },
                                                    "{d}"
                                                }
                                                if hidden > 0 {
                                                    span { class: "pb-0.5 text-[11px] text-base-content/60", "+{hidden} more" }
                                                }
                                            }
                                        }
                                    }
                                }
                                for b in bars.iter().filter(|b| b.lane < MAX_LANES) {
                                    {
                                        let e = &entries[b.entry];
                                        let id = e.id.clone();
                                        let left = if b.starts_here { "ml-1 rounded-l-field" } else { "" };
                                        let right = if b.ends_here { "mr-1 rounded-r-field" } else { "" };
                                        rsx! {
                                            button {
                                                key: "b{b.entry}",
                                                r#type: "button",
                                                class: "z-10 flex min-w-0 flex-col justify-center px-2 text-left text-xs font-medium transition-colors {bar_class(e.tone)} {left} {right}",
                                                style: "grid-column: {b.start + 1} / {b.end + 2}; grid-row: {b.lane + 2};",
                                                title: "{e.title} · {e.subtitle}",
                                                onclick: move |_| on_select.call(id.clone()),
                                                span { class: "w-full truncate", "{e.title}" }
                                                if let Some(d) = &e.detail {
                                                    span { class: "hidden w-full truncate text-[11px] font-normal opacity-80 xl:block", "{d}" }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // Phones: a compact grid with dots, and the list below.
            div { class: "flex flex-col gap-3 md:hidden",
                div { class: "rounded-box border border-base-300 p-2",
                    div { class: "grid grid-cols-7 pb-1",
                        for name in order.iter() {
                            span { key: "{name}", class: "text-center text-[11px] font-medium text-base-content/60", "aria-label": "{name}", "{&name[..1]}" }
                        }
                    }
                    for (w, (ws, _)) in rows.iter().enumerate() {
                        div { key: "{w}", class: "grid grid-cols-7",
                            for c in 0..7usize {
                                {
                                    let z = ws + c as i64;
                                    let date = date_of(z);
                                    let here: Vec<Tone> = sorted.iter().filter(|i| entries[**i].covers(date)).map(|i| entries[*i].tone).take(3).collect();
                                    let selected = chosen == Some(date);
                                    let number = if z == today_n {
                                        "grid size-8 place-items-center rounded-full bg-primary text-sm font-semibold text-primary-content"
                                    } else if selected {
                                        "grid size-8 place-items-center rounded-full bg-base-content text-sm font-semibold text-base-100"
                                    } else if in_month(z) {
                                        "grid size-8 place-items-center rounded-full text-sm"
                                    } else {
                                        "grid size-8 place-items-center rounded-full text-sm text-base-content/35"
                                    };
                                    rsx! {
                                        button {
                                            key: "{c}",
                                            r#type: "button",
                                            class: "flex flex-col items-center gap-0.5 rounded-field py-1 transition-colors hover:bg-base-200",
                                            "aria-pressed": if selected { "true" } else { "false" },
                                            "aria-label": "{date.2} {crate::period::month_name(date.1)}, {here.len()} events",
                                            onclick: move |_| picked.set(if selected { None } else { Some(date) }),
                                            span { class: number, "{date.2}" }
                                            span { class: "flex h-1.5 gap-0.5",
                                                for (k, tone) in here.iter().enumerate() {
                                                    span { key: "{k}", class: "size-1.5 rounded-full {dot_class(*tone)}" }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                div { class: "flex items-center justify-between gap-2 px-1",
                    span { class: "text-sm font-semibold", "{list_title}" }
                    if chosen.is_some() {
                        button { r#type: "button", class: "text-sm font-medium text-primary", onclick: move |_| picked.set(None), "Whole month" }
                    }
                }
                if list.is_empty() {
                    p { class: "px-1 text-sm text-base-content/60", {empty_text.clone().unwrap_or_else(|| "Nothing on these days.".into())} }
                } else {
                    ul { class: "flex flex-col gap-2",
                        for i in list.iter().copied() {
                            {
                                let e = &entries[i];
                                let id = e.id.clone();
                                rsx! {
                                    li { key: "{i}",
                                        button {
                                            r#type: "button",
                                            class: "flex w-full items-center gap-3 rounded-box border border-base-300 p-3 text-left transition-colors hover:bg-base-200",
                                            onclick: move |_| on_select.call(id.clone()),
                                            span { class: "w-1 self-stretch rounded-full {dot_class(e.tone)}" }
                                            span { class: "text-base-content/60", IconView { icon: e.icon, size: IconSize::Sm } }
                                            span { class: "flex min-w-0 flex-1 flex-col",
                                                span { class: "truncate text-sm font-medium", "{e.title}" }
                                                span { class: "truncate text-xs text-base-content/60", "{e.subtitle}" }
                                            }
                                            IconView { icon: Icon::ArrowRight, size: IconSize::Xs }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn day_numbers_round_trip() {
        for d in [(1970, 1, 1), (2024, 2, 29), (2026, 11, 19), (2027, 12, 31)] {
            assert_eq!(date_of(day_number(d)), d);
        }
        assert_eq!(day_number((1970, 1, 2)), 1);
    }
}
