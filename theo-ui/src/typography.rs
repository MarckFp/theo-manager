//! Text. Screens choose a level or a tone, never a font size (R-433).

use dioxus::prelude::*;

/// Heading level. `Display` is for landing-page heroes only.
#[allow(missing_docs)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum HeadingLevel {
    Display,
    H1,
    #[default]
    H2,
    H3,
}

/// A heading.
#[component]
pub fn Heading(#[props(default)] level: HeadingLevel, children: Element) -> Element {
    match level {
        HeadingLevel::Display => rsx! {
            h1 { class: "text-4xl sm:text-5xl lg:text-6xl font-bold tracking-tight text-balance", {children} }
        },
        HeadingLevel::H1 => rsx! {
            h1 { class: "text-2xl sm:text-3xl font-semibold tracking-tight", {children} }
        },
        HeadingLevel::H2 => rsx! {
            h2 { class: "text-xl sm:text-2xl font-semibold tracking-tight", {children} }
        },
        HeadingLevel::H3 => rsx! {
            h3 { class: "text-base font-semibold", {children} }
        },
    }
}

/// Text style.
#[allow(missing_docs)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TextTone {
    #[default]
    Body,
    Lead,
    Muted,
    Small,
    Strong,
    Eyebrow,
}

/// A paragraph of text.
#[component]
pub fn Text(#[props(default)] tone: TextTone, children: Element) -> Element {
    let class = match tone {
        TextTone::Body => "text-sm leading-6",
        TextTone::Lead => "text-base sm:text-lg leading-7 text-base-content/70 text-pretty",
        TextTone::Muted => "text-sm leading-6 text-base-content/60",
        TextTone::Small => "text-xs leading-5 text-base-content/60",
        TextTone::Strong => "text-sm font-semibold",
        TextTone::Eyebrow => "text-xs font-semibold uppercase tracking-widest text-primary",
    };
    rsx! {
        p { class, {children} }
    }
}

/// Monospaced text, for passphrases and codes.
#[component]
pub fn Code(children: Element) -> Element {
    rsx! {
        code { class: "font-mono text-sm rounded-field bg-base-200 px-1.5 py-0.5 [overflow-wrap:anywhere]", {children} }
    }
}
