//! Layout primitives. Screens never set spacing themselves (R-433): they pick a [`Gap`].

use dioxus::prelude::*;

/// Spacing between children, from the design tokens.
#[allow(missing_docs)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Gap {
    None,
    Xs,
    Sm,
    #[default]
    Md,
    Lg,
    Xl,
}

impl Gap {
    fn class(self) -> &'static str {
        match self {
            Gap::None => "gap-0",
            Gap::Xs => "gap-1",
            Gap::Sm => "gap-2",
            Gap::Md => "gap-4",
            Gap::Lg => "gap-6",
            Gap::Xl => "gap-10",
        }
    }
}

/// Cross-axis alignment.
#[allow(missing_docs)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Align {
    #[default]
    Stretch,
    Start,
    Center,
    End,
}

impl Align {
    fn class(self) -> &'static str {
        match self {
            Align::Stretch => "items-stretch",
            Align::Start => "items-start",
            Align::Center => "items-center",
            Align::End => "items-end",
        }
    }
}

/// Main-axis distribution.
#[allow(missing_docs)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Justify {
    #[default]
    Start,
    Center,
    End,
    Between,
}

impl Justify {
    fn class(self) -> &'static str {
        match self {
            Justify::Start => "justify-start",
            Justify::Center => "justify-center",
            Justify::End => "justify-end",
            Justify::Between => "justify-between",
        }
    }
}

/// Vertical stack.
#[component]
pub fn Stack(
    #[props(default)] gap: Gap,
    #[props(default)] align: Align,
    children: Element,
) -> Element {
    rsx! {
        div { class: "flex flex-col min-w-0 {gap.class()} {align.class()}", {children} }
    }
}

/// Horizontal group that wraps on small screens.
#[component]
pub fn Cluster(
    #[props(default)] gap: Gap,
    #[props(default = Align::Center)] align: Align,
    #[props(default)] justify: Justify,
    children: Element,
) -> Element {
    rsx! {
        div { class: "flex flex-wrap min-w-0 {gap.class()} {align.class()} {justify.class()}", {children} }
    }
}

/// Responsive grid: 1 column on phones, up to `cols` on large screens.
#[component]
pub fn Grid(
    #[props(default = 3)] cols: u8,
    #[props(default)] gap: Gap,
    /// Two columns already on phones (for small tiles such as statistics).
    #[props(default)]
    compact: bool,
    children: Element,
) -> Element {
    let cols_class = match (cols, compact) {
        (1, _) => "grid-cols-1",
        (_, true) => "grid-cols-2 lg:grid-cols-4",
        (2, _) => "grid-cols-1 md:grid-cols-2",
        (3, _) => "grid-cols-1 md:grid-cols-2 lg:grid-cols-3",
        _ => "grid-cols-1 sm:grid-cols-2 lg:grid-cols-4",
    };
    rsx! {
        div { class: "grid {cols_class} {gap.class()}", {children} }
    }
}

/// Centred content with a maximum width.
#[component]
pub fn Container(#[props(default)] narrow: bool, children: Element) -> Element {
    let width = if narrow { "max-w-md" } else { "max-w-6xl" };
    rsx! {
        div { class: "mx-auto w-full {width} px-4 sm:px-6", {children} }
    }
}

/// Takes the remaining space in a [`Cluster`].
#[component]
pub fn Spacer() -> Element {
    rsx! {
        div { class: "flex-1" }
    }
}

/// A thin separator line, optionally with a label.
#[component]
pub fn Divider(#[props(default)] label: Option<String>) -> Element {
    rsx! {
        div { class: "divider my-1 text-xs text-base-content/60", {label} }
    }
}

/// Main content with a narrower side column on large screens; stacked on small screens
/// (side first on phones when `side_first`, for summaries).
#[component]
pub fn Split(main: Element, side: Element, #[props(default)] side_first: bool) -> Element {
    let order = if side_first { "order-first lg:order-none" } else { "" };
    rsx! {
        div { class: "grid grid-cols-1 items-start gap-6 lg:grid-cols-[minmax(0,1fr)_20rem]",
            div { class: "flex min-w-0 flex-col gap-6", {main} }
            div { class: "flex min-w-0 flex-col gap-6 {order}", {side} }
        }
    }
}

/// Controls that stay in view while the page scrolls (tabs, a month picker…). Spans the full
/// width of the content. On phones it sits under the top bar.
#[component]
pub fn StickyBar(children: Element) -> Element {
    rsx! {
        div { class: "sticky z-20 -mx-4 flex flex-col gap-3 border-b border-base-300 bg-base-100/90 px-4 py-3 backdrop-blur sm:-mx-6 sm:px-6 lg:-mx-10 lg:px-10 max-md:top-[calc(3.8125rem+max(0.625rem,env(safe-area-inset-top)))] md:top-0",
            {children}
        }
    }
}
