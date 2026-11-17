//! Public (logged-out) page parts: top bar, hero, sections, feature and step cards, footer.

use crate::card::BrandMark;
use crate::icon::{Icon, IconSize, IconView};
use dioxus::prelude::*;

/// Layout of public pages: sticky top bar with actions, content, footer.
#[component]
pub fn PublicLayout(brand: String, actions: Element, footer: String, children: Element) -> Element {
    rsx! {
        div { class: "flex min-h-dvh flex-col bg-base-100",
            header { class: "sticky top-0 z-30 border-b border-base-300/70 bg-base-100/80 backdrop-blur",
                div { class: "mx-auto flex w-full max-w-6xl items-center gap-3 px-4 py-3 sm:px-6",
                    BrandMark {}
                    span { class: "hidden text-base font-semibold tracking-tight sm:inline", "{brand}" }
                    div { class: "ml-auto flex items-center gap-2", {actions} }
                }
            }
            main { class: "theo-page flex-1", {children} }
            footer { class: "border-t border-base-300",
                div { class: "mx-auto flex w-full max-w-6xl flex-col items-center justify-between gap-2 px-4 py-8 text-xs text-base-content/60 sm:flex-row sm:px-6",
                    span { "{footer}" }
                    span { class: "flex items-center gap-1.5",
                        IconView { icon: Icon::Shield, size: IconSize::Xs }
                        "End-to-end encrypted · No trackers"
                    }
                }
            }
        }
    }
}

/// The first block of the landing page.
#[component]
pub fn Hero(
    eyebrow: String,
    title: String,
    subtitle: String,
    actions: Element,
    #[props(default)] note: Option<String>,
    visual: Element,
) -> Element {
    rsx! {
        section { class: "relative overflow-hidden",
            div { class: "pointer-events-none absolute inset-0 -z-0", "aria-hidden": "true",
                div { class: "absolute left-1/2 top-[-12rem] size-[40rem] -translate-x-1/2 rounded-full bg-primary/10 blur-3xl" }
            }
            div { class: "relative mx-auto grid w-full max-w-6xl grid-cols-1 items-center gap-12 px-4 py-16 sm:px-6 lg:grid-cols-2 lg:py-24",
                div { class: "flex flex-col items-start gap-6",
                    span { class: "inline-flex items-center gap-2 rounded-full border border-base-300 bg-base-100 px-3 py-1 text-xs font-medium text-base-content/70",
                        span { class: "text-primary", IconView { icon: Icon::Sparkles, size: IconSize::Xs } }
                        "{eyebrow}"
                    }
                    h1 { class: "text-4xl font-bold tracking-tight text-balance sm:text-5xl lg:text-6xl", "{title}" }
                    p { class: "max-w-xl text-base leading-7 text-base-content/70 text-pretty sm:text-lg", "{subtitle}" }
                    div { class: "flex w-full flex-col gap-3 sm:w-auto sm:flex-row", {actions} }
                    if let Some(n) = note {
                        p { class: "text-xs text-base-content/50", "{n}" }
                    }
                }
                div { class: "animate-float", {visual} }
            }
        }
    }
}

/// A landing page section with a heading.
#[component]
pub fn Section(
    #[props(default)] eyebrow: Option<String>,
    title: String,
    #[props(default)] subtitle: Option<String>,
    #[props(default)] muted: bool,
    children: Element,
) -> Element {
    let bg = if muted { "bg-base-200/60 border-y border-base-300/70" } else { "" };
    rsx! {
        section { class: "{bg}",
            div { class: "mx-auto flex w-full max-w-6xl flex-col gap-10 px-4 py-16 sm:px-6 lg:py-20",
                div { class: "flex max-w-2xl flex-col gap-3",
                    if let Some(e) = eyebrow {
                        span { class: "text-xs font-semibold uppercase tracking-widest text-primary", "{e}" }
                    }
                    h2 { class: "text-2xl font-semibold tracking-tight text-balance sm:text-3xl", "{title}" }
                    if let Some(s) = subtitle {
                        p { class: "text-base text-base-content/70 text-pretty", "{s}" }
                    }
                }
                {children}
            }
        }
    }
}

/// A feature with an icon, a title and a short text.
#[component]
pub fn FeatureCard(icon: Icon, title: String, text: String) -> Element {
    rsx! {
        div { class: "group flex flex-col gap-3 rounded-box border border-base-300 bg-base-100 p-5 transition-all duration-300 hover:-translate-y-1 hover:shadow-xl hover:shadow-base-content/5",
            span { class: "grid size-10 place-items-center rounded-field bg-primary/10 text-primary transition-transform duration-300 group-hover:scale-110",
                IconView { icon, size: IconSize::Md }
            }
            h3 { class: "font-semibold", "{title}" }
            p { class: "text-sm leading-6 text-base-content/70", "{text}" }
        }
    }
}

/// A numbered step of a "how it works" list.
#[component]
pub fn StepCard(number: u8, title: String, text: String, icon: Icon) -> Element {
    rsx! {
        div { class: "relative flex flex-col gap-3 rounded-box border border-base-300 bg-base-100 p-6",
            div { class: "flex items-center justify-between",
                span { class: "grid size-9 place-items-center rounded-full bg-primary text-sm font-bold text-primary-content", "{number}" }
                span { class: "text-base-content/30", IconView { icon, size: IconSize::Lg } }
            }
            h3 { class: "font-semibold", "{title}" }
            p { class: "text-sm leading-6 text-base-content/70", "{text}" }
        }
    }
}

/// A side-by-side comparison card (for example Local vs Cloud).
#[component]
pub fn CompareCard(
    icon: Icon,
    title: String,
    tagline: String,
    points: Vec<String>,
    #[props(default)] highlighted: bool,
) -> Element {
    let border = if highlighted { "border-primary ring-4 ring-primary/10" } else { "border-base-300" };
    rsx! {
        div { class: "flex flex-col gap-4 rounded-box border-2 {border} bg-base-100 p-6",
            div { class: "flex items-center gap-3",
                span { class: "grid size-11 place-items-center rounded-field bg-base-200", IconView { icon, size: IconSize::Lg } }
                div { class: "flex flex-col",
                    h3 { class: "text-lg font-semibold", "{title}" }
                    span { class: "text-sm text-base-content/60", "{tagline}" }
                }
            }
            ul { class: "flex flex-col gap-2",
                for (i, p) in points.into_iter().enumerate() {
                    li { key: "{i}", class: "flex items-start gap-2 text-sm",
                        span { class: "mt-0.5 text-success", IconView { icon: Icon::Check, size: IconSize::Sm } }
                        "{p}"
                    }
                }
            }
        }
    }
}
