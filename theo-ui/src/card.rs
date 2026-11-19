//! Cards, avatars, lists and the brand mark.

use crate::feedback::{Badge, Tone};
use crate::icon::{Icon, IconSize, IconView};
use dioxus::prelude::*;

/// A surface with an optional header.
#[component]
pub fn Card(
    #[props(default)] title: Option<String>,
    #[props(default)] icon: Option<Icon>,
    #[props(default)] actions: Option<Element>,
    #[props(default)] elevated: bool,
    children: Element,
) -> Element {
    let shadow = if elevated { "shadow-xl shadow-base-content/5" } else { "shadow-xs" };
    rsx! {
        section { class: "card border border-base-300 bg-base-100 {shadow}",
            div { class: "card-body gap-4 p-5 sm:p-6",
                if title.is_some() || actions.is_some() {
                    div { class: "flex items-center gap-2",
                        if let Some(icon) = icon {
                            span { class: "text-base-content/60", IconView { icon, size: IconSize::Sm } }
                        }
                        if let Some(t) = title {
                            h2 { class: "card-title text-base", "{t}" }
                        }
                        div { class: "ml-auto flex items-center gap-1", {actions} }
                    }
                }
                {children}
            }
        }
    }
}

/// Initials in a circle. The colour is stable per name.
#[component]
pub fn Avatar(name: String, #[props(default)] large: bool) -> Element {
    let initials: String = name
        .split_whitespace()
        .filter_map(|w| w.chars().next())
        .take(2)
        .collect::<String>()
        .to_uppercase();
    let hash = name.bytes().fold(0u32, |a, b| a.wrapping_mul(31).wrapping_add(u32::from(b)));
    let color = match hash % 5 {
        0 => "bg-primary/15 text-primary",
        1 => "bg-accent/20 text-accent-content",
        2 => "bg-info/15 text-info",
        3 => "bg-warning/20 text-warning-content",
        _ => "bg-success/15 text-success",
    };
    let size = if large { "size-12 text-base" } else { "size-8 text-xs" };
    rsx! {
        span { class: "grid shrink-0 place-items-center rounded-full font-semibold {size} {color}", "aria-hidden": "true",
            "{initials}"
        }
    }
}

/// The Theo Manager logo mark.
#[component]
pub fn BrandMark(#[props(default)] large: bool) -> Element {
    let size = if large { "size-10" } else { "size-8" };
    rsx! {
        span { class: "grid {size} shrink-0 place-items-center rounded-field bg-gradient-to-br from-primary to-accent text-primary-content shadow-sm",
            svg { view_box: "0 0 24 24", class: "size-5", fill: "none", stroke: "currentColor", stroke_width: "2.2", stroke_linecap: "round", stroke_linejoin: "round",
                path { d: "M5 7h14" }
                path { d: "M12 7v12" }
                path { d: "M8 19h8" }
            }
        }
    }
}

/// One row of an [`ItemList`].
#[derive(Clone, Debug, PartialEq)]
pub struct ListEntry {
    /// Leading icon.
    pub icon: Icon,
    /// Main text.
    pub title: String,
    /// Secondary text.
    pub subtitle: String,
    /// Short trailing text (a date, a count).
    pub meta: String,
}

/// A compact list of entries (used inside cards and dashboard widgets).
#[component]
pub fn ItemList(
    items: Vec<ListEntry>,
    /// When set, each row is a button and sends its index (for example to open a record).
    #[props(default)]
    on_select: Option<EventHandler<usize>>,
) -> Element {
    rsx! {
        ul { class: "flex flex-col divide-y divide-base-300",
            for (i, it) in items.into_iter().enumerate() {
                if let Some(h) = on_select {
                    li { key: "{i}", class: "py-0.5 first:pt-0 last:pb-0",
                        button {
                            r#type: "button",
                            class: "group -mx-2 flex w-[calc(100%+1rem)] items-center gap-3 rounded-field px-2 py-2 text-left transition-colors hover:bg-base-200",
                            onclick: move |_| h.call(i),
                            ItemRow { entry: it }
                            span { class: "shrink-0 text-base-content/30 transition-transform group-hover:translate-x-0.5",
                                IconView { icon: Icon::ArrowRight, size: IconSize::Xs }
                            }
                        }
                    }
                } else {
                    li { key: "{i}", class: "flex items-center gap-3 py-2.5 first:pt-0 last:pb-0",
                        ItemRow { entry: it }
                    }
                }
            }
        }
    }
}

#[component]
fn ItemRow(entry: ListEntry) -> Element {
    rsx! {
        span { class: "grid size-8 shrink-0 place-items-center rounded-field bg-base-200 text-base-content/70",
            IconView { icon: entry.icon, size: IconSize::Sm }
        }
        span { class: "flex min-w-0 flex-1 flex-col",
            span { class: "truncate text-sm font-medium", "{entry.title}" }
            span { class: "truncate text-xs text-base-content/60", "{entry.subtitle}" }
        }
        span { class: "shrink-0 text-xs text-base-content/60", "{entry.meta}" }
    }
}

/// A list of short statements with check marks (landing page, explanations).
#[component]
pub fn CheckList(items: Vec<String>) -> Element {
    rsx! {
        ul { class: "flex flex-col gap-2.5",
            for (i, it) in items.into_iter().enumerate() {
                li { key: "{i}", class: "flex items-start gap-2.5 text-sm",
                    span { class: "mt-0.5 text-success", IconView { icon: Icon::Success, size: IconSize::Sm } }
                    span { "{it}" }
                }
            }
        }
    }
}

/// A label on a [`RecordCard`].
#[derive(Clone, Debug, PartialEq)]
pub struct Tag {
    /// Text shown.
    pub label: String,
    /// Colour by meaning.
    pub tone: Tone,
    /// Optional icon before the text.
    pub icon: Option<Icon>,
}

impl Tag {
    /// Creates a tag.
    pub fn new(label: impl Into<String>, tone: Tone) -> Self {
        Self { label: label.into(), tone, icon: None }
    }

    /// Adds an icon.
    pub fn with_icon(mut self, icon: Icon) -> Self {
        self.icon = Some(icon);
        self
    }
}

/// One record in a list of cards (people, groups, territories…). Lists use cards, not tables,
/// so they work on every screen size (R-436).
#[component]
pub fn RecordCard(
    title: String,
    /// Name used for the avatar initials.
    avatar: String,
    #[props(default)] subtitle: Option<String>,
    /// Short text in the top-right corner (for example "#12").
    #[props(default)]
    meta: Option<String>,
    #[props(default)] tags: Vec<Tag>,
    onclick: EventHandler<()>,
) -> Element {
    rsx! {
        button {
            r#type: "button",
            class: "group flex w-full flex-col gap-3 rounded-box border border-base-300 bg-base-100 p-4 text-left shadow-xs transition-all duration-200 hover:-translate-y-0.5 hover:border-base-content/20 hover:shadow-md focus-visible:outline-2 focus-visible:outline-primary active:translate-y-0",
            onclick: move |_| onclick.call(()),
            div { class: "flex items-center gap-3",
                Avatar { name: avatar }
                div { class: "flex min-w-0 flex-1 flex-col",
                    span { class: "truncate text-sm font-semibold", "{title}" }
                    if let Some(s) = subtitle {
                        span { class: "truncate text-xs text-base-content/60", "{s}" }
                    }
                }
                if let Some(m) = meta {
                    span { class: "shrink-0 self-start text-xs tabular-nums text-base-content/50", "{m}" }
                }
                span { class: "shrink-0 text-base-content/30 transition-transform duration-200 group-hover:translate-x-0.5",
                    IconView { icon: Icon::ArrowRight, size: IconSize::Sm }
                }
            }
            if !tags.is_empty() {
                div { class: "flex flex-wrap gap-1.5",
                    for (i, t) in tags.into_iter().enumerate() {
                        Badge { key: "{i}", tone: t.tone, icon: t.icon, "{t.label}" }
                    }
                }
            }
        }
    }
}

/// A colour that tells kinds of things apart (for example kinds of assignment), in fixed order.
/// The same validated colours as the chart series.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Accent {
    /// The first colour (the theme's `--chart-1`).
    #[default]
    One,
    /// The second colour (`--chart-2`).
    Two,
}

impl Accent {
    pub(crate) fn tile(self) -> &'static str {
        match self {
            Accent::One => "bg-chart-1/15 text-chart-1",
            Accent::Two => "bg-chart-2/15 text-chart-2",
        }
    }

    pub(crate) fn text(self) -> &'static str {
        match self {
            Accent::One => "text-chart-1",
            Accent::Two => "text-chart-2",
        }
    }
}

/// One row of an [`AccentList`].
#[derive(Clone, Debug, PartialEq)]
pub struct AccentItem {
    /// Icon of the kind, in its accent.
    pub icon: Icon,
    /// Colour of the kind.
    pub accent: Accent,
    /// The kind, in its accent ("Public witnessing").
    pub kind: String,
    /// What and when ("25-11-2026 · 18:30 · Kingdom Hall").
    pub text: String,
    /// A label on the right ("New", "Cancelled").
    pub badge: Option<Tag>,
}

/// A compact list where each row shows its kind in an accent colour (for example the kinds of
/// assignment in "My assignments").
#[component]
pub fn AccentList(items: Vec<AccentItem>) -> Element {
    rsx! {
        ul { class: "flex flex-col divide-y divide-base-300",
            for (i, it) in items.into_iter().enumerate() {
                li { key: "{i}", class: "flex items-center gap-3 py-2.5 first:pt-0 last:pb-0",
                    span { class: "grid size-8 shrink-0 place-items-center rounded-field {it.accent.tile()}",
                        IconView { icon: it.icon, size: IconSize::Sm }
                    }
                    span { class: "flex min-w-0 flex-1 flex-col gap-0.5",
                        span { class: "flex min-w-0 items-center gap-2",
                            span { class: "truncate text-xs font-semibold {it.accent.text()}", "{it.kind}" }
                            if let Some(t) = it.badge {
                                Badge { tone: t.tone, icon: t.icon, "{t.label}" }
                            }
                        }
                        span { class: "truncate text-sm", "{it.text}" }
                    }
                }
            }
        }
    }
}
