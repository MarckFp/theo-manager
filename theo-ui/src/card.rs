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
    /// Smaller rows, so more fit (small dashboard widgets).
    #[props(default)]
    compact: bool,
) -> Element {
    let (button_pad, row_pad) = if compact { ("py-0.5", "py-1") } else { ("py-2", "py-2.5") };
    rsx! {
        ul { class: "flex flex-col divide-y divide-base-300",
            for (i, it) in items.into_iter().enumerate() {
                if let Some(h) = on_select {
                    li { key: "{i}", class: "py-0.5 first:pt-0 last:pb-0",
                        button {
                            r#type: "button",
                            class: "group -mx-2 flex w-[calc(100%+1rem)] items-center gap-3 rounded-field px-2 {button_pad} text-left transition-colors hover:bg-base-200",
                            onclick: move |_| h.call(i),
                            ItemRow { entry: it, compact }
                            span { class: "shrink-0 text-base-content/30 transition-transform group-hover:translate-x-0.5",
                                IconView { icon: Icon::ArrowRight, size: IconSize::Xs }
                            }
                        }
                    }
                } else {
                    li { key: "{i}", class: "flex items-center gap-3 {row_pad} first:pt-0 last:pb-0",
                        ItemRow { entry: it, compact }
                    }
                }
            }
        }
    }
}

#[component]
fn ItemRow(entry: ListEntry, compact: bool) -> Element {
    rsx! {
        span { class: if compact { "grid size-6 shrink-0 place-items-center rounded-field bg-base-200 text-base-content/70" } else { "grid size-8 shrink-0 place-items-center rounded-field bg-base-200 text-base-content/70" },
            IconView { icon: entry.icon, size: if compact { IconSize::Xs } else { IconSize::Sm } }
        }
        span { class: "flex min-w-0 flex-1 flex-col",
            span { class: "truncate text-sm font-medium leading-tight", "{entry.title}" }
            if !entry.subtitle.is_empty() {
                span { class: "truncate text-xs leading-tight text-base-content/60", "{entry.subtitle}" }
            }
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
    /// The third colour (`--chart-3`).
    Three,
    /// The fourth colour (`--chart-4`).
    Four,
    /// The fifth colour (`--chart-5`).
    Five,
    /// The sixth colour (`--chart-6`).
    Six,
}

impl Accent {
    pub(crate) fn tile(self) -> &'static str {
        match self {
            Accent::One => "bg-chart-1/15 text-chart-1",
            Accent::Two => "bg-chart-2/15 text-chart-2",
            Accent::Three => "bg-chart-3/15 text-chart-3",
            Accent::Four => "bg-chart-4/15 text-chart-4",
            Accent::Five => "bg-chart-5/15 text-chart-5",
            Accent::Six => "bg-chart-6/15 text-chart-6",
        }
    }

    pub(crate) fn text(self) -> &'static str {
        match self {
            Accent::One => "text-chart-1",
            Accent::Two => "text-chart-2",
            Accent::Three => "text-chart-3",
            Accent::Four => "text-chart-4",
            Accent::Five => "text-chart-5",
            Accent::Six => "text-chart-6",
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
pub fn AccentList(
    items: Vec<AccentItem>,
    /// Bigger icons and text (for example in a wide widget).
    #[props(default)]
    large: bool,
) -> Element {
    let (tile, kind_text, main_text, row) = if large {
        ("size-11", "text-sm", "text-base font-medium", "py-3")
    } else {
        ("size-8", "text-xs", "text-sm", "py-2.5")
    };
    rsx! {
        ul { class: "flex flex-col divide-y divide-base-300",
            for (i, it) in items.into_iter().enumerate() {
                li { key: "{i}", class: "flex items-center gap-3 {row} first:pt-0 last:pb-0",
                    span { class: "grid {tile} shrink-0 place-items-center rounded-field {it.accent.tile()}",
                        IconView { icon: it.icon, size: if large { IconSize::Md } else { IconSize::Sm } }
                    }
                    span { class: "flex min-w-0 flex-1 flex-col gap-0.5",
                        span { class: "flex min-w-0 items-center gap-2",
                            span { class: "truncate {kind_text} font-semibold {it.accent.text()}", "{it.kind}" }
                            if let Some(t) = it.badge {
                                Badge { tone: t.tone, icon: t.icon, "{t.label}" }
                            }
                        }
                        span { class: if large { "{main_text}" } else { "truncate {main_text}" }, "{it.text}" }
                    }
                }
            }
        }
    }
}

/// One place of an [`AssignmentRow`] ("Student: Anna Lee").
#[derive(Clone, Debug, PartialEq)]
pub struct SlotView {
    /// What the place is ("Student", "Assistant", "Auxiliary classroom").
    pub label: String,
    /// Who has it (None: nobody yet).
    pub person: Option<String>,
    /// A short warning (for example "Away").
    pub warning: Option<String>,
}

/// A part of a meeting: number, title, minutes, a short description and its places. With
/// `on_slot` each place is a button that opens the choice of a person.
#[component]
pub fn AssignmentRow(
    /// "3." or "".
    number: String,
    title: String,
    /// "4 min".
    #[props(default)]
    minutes: String,
    #[props(default)] detail: String,
    slots: Vec<SlotView>,
    #[props(default)] on_slot: Option<EventHandler<usize>>,
    #[props(default)] on_edit: Option<EventHandler<()>>,
    /// Move the part up or down (shown next to the number).
    #[props(default)]
    on_up: Option<EventHandler<()>>,
    #[props(default)] on_down: Option<EventHandler<()>>,
    /// An icon instead of the number (for example a song).
    #[props(default)]
    icon: Option<Icon>,
) -> Element {
    let mover = |h: Option<EventHandler<()>>, icon: Icon, label: &'static str| match h {
        Some(h) => rsx! {
            button { r#type: "button", class: "grid size-5 place-items-center rounded text-base-content/40 hover:bg-base-200 hover:text-base-content", "aria-label": label, title: label, onclick: move |_| h.call(()),
                IconView { icon, size: IconSize::Xs }
            }
        },
        None => rsx! { span { class: "size-5" } },
    };
    let movable = on_up.is_some() || on_down.is_some();
    rsx! {
        div { class: "flex flex-col gap-2 py-3 first:pt-0 last:pb-0 md:flex-row md:items-start md:gap-4",
            div { class: "flex min-w-0 flex-1 items-start gap-2",
                if movable {
                    span { class: "flex shrink-0 flex-col",
                        {mover(on_up, Icon::MoveUp, "Move up")}
                        {mover(on_down, Icon::MoveDown, "Move down")}
                    }
                }
                if let Some(i) = icon {
                    span { class: "mt-0.5 grid size-6 shrink-0 place-items-center rounded-full bg-base-200 text-base-content/70",
                        IconView { icon: i, size: IconSize::Xs }
                    }
                } else if !number.is_empty() {
                    span { class: "mt-0.5 grid size-6 shrink-0 place-items-center rounded-full bg-base-200 text-xs font-semibold", "{number}" }
                }
                div { class: "flex min-w-0 flex-1 flex-col gap-0.5",
                    div { class: "flex min-w-0 items-center gap-2",
                        span { class: "min-w-0 font-medium", "{title}" }
                        if !minutes.is_empty() {
                            span { class: "shrink-0 rounded-full bg-base-200 px-2 py-0.5 text-xs text-base-content/70", "{minutes}" }
                        }
                        if let Some(h) = on_edit {
                            button { r#type: "button", class: "btn btn-ghost btn-square btn-xs text-base-content/50", "aria-label": "Edit {title}", title: "Edit", onclick: move |_| h.call(()),
                                IconView { icon: Icon::Edit, size: IconSize::Xs }
                            }
                        }
                    }
                    if !detail.is_empty() {
                        span { class: "line-clamp-2 text-xs text-base-content/60", "{detail}" }
                    }
                }
            }
            div { class: "flex flex-wrap gap-2 md:max-w-[55%] md:justify-end",
                for (i, s) in slots.iter().enumerate() {
                    {
                        let filled = s.person.is_some();
                        let name = s.person.clone().unwrap_or_else(|| "Choose".to_string());
                        let class = match (filled, s.warning.is_some()) {
                            (_, true) => "border-warning/60 bg-warning/10",
                            (true, false) => "border-base-300 bg-base-100",
                            (false, false) => "border-dashed border-primary/50 text-primary",
                        };
                        let inner = rsx! {
                            span { class: "text-[10px] font-medium uppercase tracking-wide text-base-content/60", "{s.label}" }
                            span { class: "truncate text-sm font-medium", "{name}" }
                            if let Some(w) = &s.warning {
                                span { class: "truncate text-[10px] text-warning", "{w}" }
                            }
                        };
                        match on_slot {
                            Some(h) => rsx! {
                                button { key: "{i}", r#type: "button", class: "flex min-w-36 max-w-full flex-col items-start rounded-field border px-3 py-1.5 text-left transition-colors hover:bg-base-200 {class}", onclick: move |_| h.call(i), {inner} }
                            },
                            None => rsx! {
                                div { key: "{i}", class: "flex min-w-36 max-w-full flex-col items-start rounded-field border px-3 py-1.5 {class}", {inner} }
                            },
                        }
                    }
                }
            }
        }
    }
}

/// The colour of a section of a meeting, as in the printed workbook and on wol.jw.org.
#[allow(missing_docs)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SectionTone {
    /// Opening and closing.
    #[default]
    Neutral,
    /// Treasures from God's Word (teal).
    Teal,
    /// Apply yourself to the field ministry (gold).
    Gold,
    /// Living as Christians (maroon).
    Maroon,
}

impl SectionTone {
    /// (accent colour, light background).
    fn colors(self) -> (&'static str, &'static str) {
        match self {
            SectionTone::Neutral => ("", ""),
            SectionTone::Teal => ("#3c7f8b", "rgba(60, 127, 139, 0.10)"),
            SectionTone::Gold => ("#c18626", "rgba(193, 134, 38, 0.10)"),
            SectionTone::Maroon => ("#942926", "rgba(148, 41, 38, 0.10)"),
        }
    }
}

/// A section of a meeting: a header band in the section's colour (title, minutes, actions) and
/// its parts.
#[component]
pub fn MeetingSection(
    title: String,
    #[props(default)] subtitle: String,
    #[props(default)] tone: SectionTone,
    #[props(default)] icon: Option<Icon>,
    #[props(default)] actions: Option<Element>,
    children: Element,
) -> Element {
    let (accent, bg) = tone.colors();
    let header_style = if accent.is_empty() { String::new() } else { format!("background: {bg}; border-left: 4px solid {accent};") };
    let title_style = if accent.is_empty() { String::new() } else { format!("color: {accent};") };
    rsx! {
        section { class: "overflow-hidden rounded-box border border-base-300 bg-base-100 shadow-xs",
            div { class: "flex flex-wrap items-center gap-2 px-5 py-3", style: "{header_style}",
                if let Some(i) = icon {
                    span { style: "{title_style}", IconView { icon: i, size: IconSize::Sm } }
                }
                h3 { class: "min-w-0 flex-1 text-sm font-bold uppercase tracking-wide", style: "{title_style}", "{title}" }
                if !subtitle.is_empty() {
                    span { class: "text-xs text-base-content/60", "{subtitle}" }
                }
                if let Some(a) = actions {
                    div { class: "flex items-center gap-1", {a} }
                }
            }
            div { class: "flex flex-col divide-y divide-base-300 px-5 py-3", {children} }
        }
    }
}
