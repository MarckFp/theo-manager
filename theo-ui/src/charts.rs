//! Small charts drawn as SVG/CSS: stat tiles, bar and line charts, sparklines.

use crate::icon::{Icon, IconSize, IconView};
use dioxus::prelude::*;

/// One bar of a [`BarChart`].
#[derive(Clone, Debug, PartialEq)]
pub struct ChartPoint {
    /// Label under the bar.
    pub label: String,
    /// Value of the bar.
    pub value: f64,
}

impl ChartPoint {
    /// Creates a point.
    pub fn new(label: impl Into<String>, value: f64) -> Self {
        Self { label: label.into(), value }
    }
}

/// A bar chart. The highlighted bar (usually the latest) uses the primary colour.
#[component]
pub fn BarChart(
    data: Vec<ChartPoint>,
    #[props(default)] highlight_last: bool,
    #[props(default)] unit: Option<String>,
) -> Element {
    // Bars are SVG rects in a 0..100 box (heights in percent), so they scale with any container
    // height without depending on CSS percentage heights.
    let max = data.iter().map(|d| d.value).fold(0.0_f64, f64::max).max(1.0);
    let n = data.len().max(1);
    let unit = unit.unwrap_or_default();
    let slot = 100.0 / n as f64;
    let bar_w = slot * 0.72;
    rsx! {
        div { class: "flex h-full min-h-16 flex-col gap-1.5", role: "img", "aria-label": "Bar chart",
            svg { view_box: "0 0 100 100", preserve_aspect_ratio: "none", class: "min-h-0 max-h-64 w-full flex-1 overflow-visible",
                for (i, d) in data.iter().enumerate() {
                    {
                        let h = (d.value / max * 100.0).max(2.0);
                        let x = i as f64 * slot + (slot - bar_w) / 2.0;
                        let last = highlight_last && i + 1 == data.len();
                        rsx! {
                            rect {
                                key: "{i}",
                                x: "{x:.2}",
                                y: "{100.0 - h:.2}",
                                width: "{bar_w:.2}",
                                height: "{h:.2}",
                                rx: "1.5",
                                class: if last { "theo-bar fill-primary animate-bar" } else { "theo-bar fill-primary/25 hover:fill-primary/45 animate-bar" },
                                style: "animation-delay: {i * 40}ms;",
                                "aria-label": "{d.label}: {d.value}{unit}",
                            }
                        }
                    }
                }
            }
            div { class: "grid text-center text-[10px] text-base-content/60", style: "grid-template-columns: repeat({n}, minmax(0, 1fr));",
                for (i, d) in data.iter().enumerate() {
                    span { key: "{i}", "{d.label}" }
                }
            }
        }
    }
}

/// Direction of a change.
#[allow(missing_docs)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Trend {
    Up,
    Down,
    #[default]
    Flat,
}

/// A key number with a label and an optional change.
#[component]
pub fn StatTile(
    label: String,
    value: String,
    icon: Icon,
    #[props(default)] delta: Option<String>,
    #[props(default)] trend: Trend,
) -> Element {
    let delta_class = match trend {
        Trend::Up => "text-success",
        Trend::Down => "text-error",
        Trend::Flat => "text-base-content/60",
    };
    rsx! {
        div { class: "flex h-full flex-col justify-between gap-2",
            div { class: "flex items-center justify-between text-base-content/60",
                span { class: "text-sm", "{label}" }
                IconView { icon, size: IconSize::Sm }
            }
            div { class: "flex flex-wrap items-end gap-x-2",
                span { class: "whitespace-nowrap text-2xl font-semibold tracking-tight sm:text-3xl", "{value}" }
                if let Some(d) = delta {
                    span { class: "pb-1 text-xs font-medium {delta_class}", "{d}" }
                }
            }
        }
    }
}

/// A tiny line chart without axes.
#[component]
pub fn Sparkline(values: Vec<f64>) -> Element {
    let max = values.iter().cloned().fold(f64::MIN, f64::max);
    let min = values.iter().cloned().fold(f64::MAX, f64::min);
    let span = (max - min).max(1.0);
    let n = values.len().max(2) - 1;
    let pts: Vec<String> = values
        .iter()
        .enumerate()
        .map(|(i, v)| format!("{:.1},{:.1}", i as f64 * 100.0 / n as f64, 36.0 - (v - min) / span * 32.0))
        .collect();
    let line = pts.join(" ");
    let area = format!("0,40 {line} 100,40");
    rsx! {
        svg { view_box: "0 0 100 40", preserve_aspect_ratio: "none", class: "h-16 w-full overflow-visible",
            polygon { points: "{area}", class: "fill-primary/10" }
            polyline { points: "{line}", class: "fill-none stroke-primary", stroke_width: "2", vector_effect: "non-scaling-stroke", stroke_linejoin: "round" }
        }
    }
}

/// One month of a [`MonthStrip`].
#[derive(Clone, Debug, PartialEq)]
pub struct MonthMark {
    /// Short label ("May").
    pub label: String,
    /// `Some(true)` done, `Some(false)` missing, `None` does not count.
    pub done: Option<bool>,
}

/// A row of months with a done / missing mark (for example field service reports).
#[component]
pub fn MonthStrip(months: Vec<MonthMark>) -> Element {
    rsx! {
        ul { class: "grid grid-cols-6 gap-1.5",
            for (i, m) in months.into_iter().enumerate() {
                li { key: "{i}", class: "flex flex-col items-center gap-1",
                    span {
                        class: match m.done {
                            Some(true) => "grid size-8 place-items-center rounded-full bg-success/15 text-success",
                            Some(false) => "grid size-8 place-items-center rounded-full bg-error/10 text-error",
                            None => "grid size-8 place-items-center rounded-full border border-dashed border-base-300 text-base-content/30",
                        },
                        title: match m.done { Some(true) => "Report submitted", Some(false) => "Report missing", None => "Not counted" },
                        IconView {
                            icon: match m.done { Some(true) => Icon::Check, Some(false) => Icon::Close, None => Icon::Clock },
                            size: IconSize::Xs,
                        }
                    }
                    span { class: "text-[11px] text-base-content/60", "{m.label}" }
                }
            }
        }
    }
}

/// Progress towards a goal: a label, the numbers, a bar, and an optional marker for where the
/// value should be by now (for example the hours expected so far in the service year).
#[component]
pub fn ProgressRow(
    label: String,
    /// Short text on the right, for example "312 / 600 h".
    detail: String,
    value: f64,
    max: f64,
    /// Expected value by now; drawn as a thin line on the bar.
    #[props(default)]
    marker: Option<f64>,
    /// Second line under the label, for example "On track" or "18 h behind".
    #[props(default)]
    note: Option<String>,
    #[props(default)] tone: crate::feedback::Tone,
) -> Element {
    use crate::feedback::Tone;
    let pct = if max > 0.0 { (value / max * 100.0).clamp(0.0, 100.0) } else { 0.0 };
    let marker_pct = marker.map(|m| if max > 0.0 { (m / max * 100.0).clamp(0.0, 100.0) } else { 0.0 });
    let bar = match tone {
        Tone::Success => "bg-success",
        Tone::Warning => "bg-warning",
        Tone::Error => "bg-error",
        Tone::Info => "bg-info",
        Tone::Neutral => "bg-primary",
    };
    let note_class = match tone {
        Tone::Success => "text-xs text-success",
        Tone::Warning => "text-xs text-warning",
        Tone::Error => "text-xs text-error",
        _ => "text-xs text-base-content/60",
    };
    rsx! {
        div { class: "flex flex-col gap-1.5",
            div { class: "flex items-baseline justify-between gap-3",
                span { class: "flex min-w-0 flex-col",
                    span { class: "truncate text-sm font-medium", "{label}" }
                    if let Some(n) = note {
                        span { class: note_class, "{n}" }
                    }
                }
                span { class: "shrink-0 text-xs tabular-nums text-base-content/70", "{detail}" }
            }
            div { class: "relative h-2 overflow-hidden rounded-full bg-base-300", role: "progressbar",
                "aria-valuenow": "{value}", "aria-valuemax": "{max}", "aria-label": "{label}",
                div { class: "h-full rounded-full {bar} transition-[width] duration-700 ease-[var(--ease-out-soft)]", style: "width: {pct:.1}%;" }
                if let Some(m) = marker_pct {
                    div { class: "absolute inset-y-0 w-0.5 bg-base-content/60", style: "left: calc({m:.1}% - 1px);", title: "Expected by now" }
                }
            }
        }
    }
}

/// One line of a [`LineChart`]: a name and one value per label (`None`: no value there).
#[derive(Clone, Debug, PartialEq)]
pub struct ChartSeries {
    /// Name in the legend and the tooltip.
    pub name: String,
    /// One value per label of the chart.
    pub values: Vec<Option<f64>>,
}

impl ChartSeries {
    /// Creates a series.
    pub fn new(name: impl Into<String>, values: Vec<Option<f64>>) -> Self {
        Self { name: name.into(), values }
    }
}

/// Series colours in fixed order (never by rank): `--chart-1`, `--chart-2` of the theme.
const SERIES_STROKE: [&str; 2] = ["stroke-chart-1", "stroke-chart-2"];
const SERIES_FILL: [&str; 2] = ["bg-chart-1", "bg-chart-2"];

/// A round top for the y axis and its step: 4 or 5 ticks of 1, 2 or 5 × 10ⁿ.
fn nice_scale(top: f64) -> (f64, f64) {
    let top = top.max(1.0);
    let raw = top / 4.0;
    let mag = 10f64.powf(raw.log10().floor());
    let step = [1.0, 2.0, 5.0, 10.0].iter().map(|f| f * mag).find(|s| *s >= raw).unwrap_or(10.0 * mag);
    ((top / step).ceil() * step, step)
}

fn chart_number(v: f64) -> String {
    if (v - v.round()).abs() < 0.05 { format!("{v:.0}") } else { format!("{v:.1}") }
}

/// A line chart of one or two series over the same labels (for example months), from a zero
/// baseline. Two series get a legend and their last value at the end of each line (left out when
/// the two would collide). Hover or focus a label for a tooltip with every series.
#[component]
pub fn LineChart(labels: Vec<String>, series: Vec<ChartSeries>, #[props(default)] unit: Option<String>) -> Element {
    let series: Vec<ChartSeries> = series.into_iter().take(SERIES_STROKE.len()).collect();
    let n = labels.len().max(1);
    let unit = unit.unwrap_or_default();
    let top = series.iter().flat_map(|s| s.values.iter().flatten()).cloned().fold(0.0, f64::max);
    let (max, step) = nice_scale(top);
    let ticks: Vec<f64> = (0..=((max / step).round() as usize)).map(|i| i as f64 * step).collect();
    let x = move |i: usize| (i as f64 + 0.5) / n as f64 * 100.0;
    let y = move |v: f64| 100.0 - v / max * 100.0;
    // Lines break where a value is missing.
    let segments: Vec<Vec<String>> = series
        .iter()
        .map(|s| {
            let mut out = Vec::new();
            let mut cur: Vec<String> = Vec::new();
            for (i, v) in s.values.iter().enumerate() {
                match v {
                    Some(v) => cur.push(format!("{:.2},{:.2}", x(i), y(*v))),
                    None if !cur.is_empty() => out.push(std::mem::take(&mut cur).join(" ")),
                    None => {}
                }
            }
            if !cur.is_empty() {
                out.push(cur.join(" "));
            }
            out
        })
        .collect();
    let ends: Vec<Option<(usize, f64)>> =
        series.iter().map(|s| s.values.iter().enumerate().rev().find_map(|(i, v)| v.map(|v| (i, v)))).collect();
    let show_ends = match (ends.first().copied().flatten(), ends.get(1).copied().flatten()) {
        (Some(a), Some(b)) => (y(a.1) - y(b.1)).abs() >= 9.0,
        _ => true,
    };
    let columns = format!("grid-template-columns: repeat({n}, minmax(0, 1fr));");
    rsx! {
        div { class: "flex flex-col gap-3",
            if series.len() > 1 {
                div { class: "flex flex-wrap gap-x-4 gap-y-1 text-xs text-base-content/70",
                    for (k, s) in series.iter().enumerate() {
                        span { key: "{k}", class: "inline-flex items-center gap-1.5",
                            span { class: "h-0.5 w-4 rounded-full {SERIES_FILL[k]}" }
                            "{s.name}"
                        }
                    }
                }
            }
            div { class: "flex pt-2",
                div { class: "relative w-8 shrink-0 text-right text-[10px] tabular-nums text-base-content/50", "aria-hidden": "true",
                    for (t_i, t) in ticks.iter().enumerate() {
                        span { key: "{t_i}", class: "absolute right-2 -translate-y-1/2", style: "top: {y(*t):.2}%;", "{chart_number(*t)}" }
                    }
                }
                div { class: "relative mr-8 h-56 min-w-0 flex-1",
                    svg { view_box: "0 0 100 100", preserve_aspect_ratio: "none", class: "absolute inset-0 h-full w-full overflow-visible", "aria-hidden": "true",
                        for (t_i, t) in ticks.iter().enumerate() {
                            line { key: "{t_i}", x1: "0", x2: "100", y1: "{y(*t):.2}", y2: "{y(*t):.2}", class: "stroke-base-300", stroke_width: "1", vector_effect: "non-scaling-stroke" }
                        }
                        for (k, segs) in segments.iter().enumerate() {
                            for (j, pts) in segs.iter().enumerate() {
                                polyline {
                                    key: "{k}-{j}",
                                    points: "{pts}",
                                    class: "fill-none {SERIES_STROKE[k]}",
                                    stroke_width: "2",
                                    stroke_linejoin: "round",
                                    stroke_linecap: "round",
                                    vector_effect: "non-scaling-stroke",
                                }
                            }
                        }
                    }
                    for (k, s) in series.iter().enumerate() {
                        for (i, v) in s.values.iter().enumerate() {
                            if let Some(v) = v {
                                span {
                                    key: "{k}-{i}",
                                    class: "pointer-events-none absolute size-2 -translate-x-1/2 -translate-y-1/2 rounded-full ring-2 ring-base-100 {SERIES_FILL[k]}",
                                    style: "left: {x(i):.2}%; top: {y(*v):.2}%;",
                                }
                            }
                        }
                    }
                    if show_ends {
                        for (k, end) in ends.iter().enumerate() {
                            if let Some((i, v)) = end {
                                span {
                                    key: "end-{k}",
                                    class: "pointer-events-none absolute translate-x-2 -translate-y-1/2 text-xs font-semibold tabular-nums text-base-content",
                                    style: "left: {x(*i):.2}%; top: {y(*v):.2}%;",
                                    "{chart_number(*v)}"
                                }
                            }
                        }
                    }
                    div { class: "absolute inset-0 grid", style: "{columns}",
                        for (i, label) in labels.iter().enumerate() {
                            {
                                let place = if i == 0 { "left-0" } else if i + 1 == n { "right-0" } else { "left-1/2 -translate-x-1/2" };
                                let read: Vec<String> = series
                                    .iter()
                                    .map(|s| format!("{} {}", s.name, s.values.get(i).copied().flatten().map(|v| format!("{}{unit}", chart_number(v))).unwrap_or_else(|| "no value".into())))
                                    .collect();
                                rsx! {
                                    div { key: "{i}", tabindex: "0", class: "group relative outline-none", "aria-label": "{label}: {read.join(\", \")}",
                                        div { class: "absolute inset-y-0 left-1/2 w-px bg-base-content/25 opacity-0 transition-opacity group-hover:opacity-100 group-focus-visible:opacity-100" }
                                        div { class: "pointer-events-none absolute -top-2 z-10 -translate-y-full whitespace-nowrap rounded-field border border-base-300 bg-base-100 px-2.5 py-1.5 text-xs opacity-0 shadow-lg transition-opacity group-hover:opacity-100 group-focus-visible:opacity-100 {place}",
                                            div { class: "mb-1 text-base-content/60", "{label}" }
                                            for (k, s) in series.iter().enumerate() {
                                                div { key: "{k}", class: "flex items-center gap-2",
                                                    span { class: "h-0.5 w-3 rounded-full {SERIES_FILL[k]}" }
                                                    span { class: "font-semibold tabular-nums text-base-content",
                                                        {s.values.get(i).copied().flatten().map(|v| format!("{}{unit}", chart_number(v))).unwrap_or_else(|| "–".into())}
                                                    }
                                                    span { class: "text-base-content/60", "{s.name}" }
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
            div { class: "ml-8 mr-8 grid text-center text-[10px] text-base-content/60", style: "{columns}", "aria-hidden": "true",
                for (i, l) in labels.iter().enumerate() {
                    span { key: "{i}", class: "truncate", "{l}" }
                }
            }
        }
    }
}

/// One line of a [`StatList`].
#[derive(Clone, Debug, PartialEq)]
pub struct StatItem {
    /// What is counted ("Elders").
    pub label: String,
    /// The number ("4").
    pub value: String,
    /// Its colour (neutral for most).
    pub tone: crate::feedback::Tone,
}

impl StatItem {
    /// A neutral line.
    pub fn new(label: impl Into<String>, value: impl Into<String>) -> Self {
        StatItem { label: label.into(), value: value.into(), tone: crate::feedback::Tone::Neutral }
    }

    /// The same line in a colour.
    pub fn with_tone(mut self, tone: crate::feedback::Tone) -> Self {
        self.tone = tone;
        self
    }
}

/// Small numbers with their labels, in one or two columns (for example the publishers of the
/// congregation by kind in a widget).
#[component]
pub fn StatList(items: Vec<StatItem>, #[props(default = 1)] columns: u8) -> Element {
    use crate::feedback::Tone;
    let grid = if columns >= 2 { "grid grid-cols-2 gap-x-6" } else { "grid grid-cols-1" };
    rsx! {
        dl { class: "{grid}",
            for (i, it) in items.iter().enumerate() {
                div { key: "{i}", class: "flex items-baseline justify-between gap-3 border-b border-base-300/60 py-1.5",
                    dt { class: "truncate text-sm text-base-content/70", "{it.label}" }
                    dd {
                        class: match it.tone {
                            Tone::Error => "text-sm font-semibold tabular-nums text-error",
                            Tone::Warning => "text-sm font-semibold tabular-nums text-warning",
                            Tone::Success => "text-sm font-semibold tabular-nums text-success",
                            Tone::Info => "text-sm font-semibold tabular-nums text-info",
                            Tone::Neutral => "text-sm font-semibold tabular-nums",
                        },
                        "{it.value}"
                    }
                }
            }
        }
    }
}
