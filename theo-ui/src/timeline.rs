//! A vertical timeline of dated items, grouped by month (for example "My assignments").

use crate::card::Tag;
use crate::feedback::{Badge, Tone};
use crate::icon::{Icon, IconSize, IconView};
use dioxus::prelude::*;

/// One item of a [`Timeline`].
#[derive(Clone, Debug, PartialEq)]
pub struct TimelineItem {
    /// Unique key.
    pub id: String,
    /// The group heading it belongs to ("December 2026"); items come grouped and in order.
    pub group: String,
    /// Big text in the date column ("2").
    pub day: String,
    /// Small text under it ("Wed").
    pub weekday: String,
    /// Main text ("Meeting for field service").
    pub title: String,
    /// Second line ("18:30 · Kingdom Hall").
    pub subtitle: String,
    /// Icon in the item.
    pub icon: Icon,
    /// A label after the title ("New", "Changed").
    pub tag: Option<Tag>,
    /// Shown faded (for example in the past).
    pub muted: bool,
    /// Marked as the next one.
    pub highlight: bool,
}

/// A timeline: month headings, then per item a date column, a line with a dot, and the item.
#[component]
pub fn Timeline(items: Vec<TimelineItem>) -> Element {
    let mut groups: Vec<(String, Vec<TimelineItem>)> = Vec::new();
    for item in items {
        match groups.last_mut() {
            Some((g, list)) if *g == item.group => list.push(item),
            _ => groups.push((item.group.clone(), vec![item])),
        }
    }
    rsx! {
        div { class: "flex flex-col gap-6",
            for (g, list) in groups {
                section { key: "{g}", class: "flex flex-col gap-1",
                    h3 { class: "pb-2 text-sm font-semibold text-base-content/70", "{g}" }
                    ol { class: "flex flex-col",
                        for (i, it) in list.iter().enumerate() {
                            {
                                let last = i + 1 == list.len();
                                let dot = if it.highlight { "bg-primary ring-4 ring-primary/20" } else if it.muted { "bg-base-300" } else { "bg-base-content/40" };
                                let card = if it.highlight { "border-primary/40 bg-primary/5" } else { "border-base-300 bg-base-100" };
                                let fade = if it.muted { "opacity-60" } else { "" };
                                rsx! {
                                    li { key: "{it.id}", class: "flex gap-3 {fade}",
                                        div { class: "flex w-11 shrink-0 flex-col items-center pt-2.5 text-center",
                                            span { class: "text-lg font-semibold leading-none tabular-nums", "{it.day}" }
                                            span { class: "pt-0.5 text-[11px] uppercase text-base-content/60", "{it.weekday}" }
                                        }
                                        div { class: "relative flex w-3 shrink-0 justify-center",
                                            span { class: if last { "absolute top-0 h-5 w-px bg-base-300" } else { "absolute inset-y-0 w-px bg-base-300" } }
                                            span { class: "relative mt-4 size-2.5 rounded-full {dot}" }
                                        }
                                        div { class: "mb-3 flex min-w-0 flex-1 items-center gap-3 rounded-box border p-3 {card}",
                                            span { class: "text-base-content/60", IconView { icon: it.icon, size: IconSize::Md } }
                                            div { class: "flex min-w-0 flex-1 flex-col",
                                                span { class: "flex min-w-0 flex-wrap items-center gap-2",
                                                    span { class: "truncate text-sm font-medium", "{it.title}" }
                                                    if let Some(t) = &it.tag {
                                                        Badge { tone: t.tone, icon: t.icon, "{t.label}" }
                                                    }
                                                    if it.highlight {
                                                        Badge { tone: Tone::Info, "Next" }
                                                    }
                                                }
                                                span { class: "truncate text-xs text-base-content/60", "{it.subtitle}" }
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
}
