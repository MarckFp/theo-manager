//! Small charts drawn as SVG/CSS: stat tiles, bar charts and sparklines.

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
        div { class: "flex h-full min-h-28 flex-col gap-1.5", role: "img", "aria-label": "Bar chart",
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
