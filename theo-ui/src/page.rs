//! Page templates (R-421). Screens fill slots; they never build their own layout.

use crate::card::BrandMark;
use crate::icon::{Icon, IconSize, IconView};
use dioxus::prelude::*;

/// Standard page inside the app shell: header (title, subtitle, actions) + content.
#[component]
pub fn Page(
    title: String,
    #[props(default)] subtitle: Option<String>,
    #[props(default)] actions: Option<Element>,
    /// A "back" link above the title, for detail pages: (label, handler).
    #[props(default)]
    back: Option<(String, EventHandler<()>)>,
    children: Element,
) -> Element {
    rsx! {
        div { class: "flex flex-col gap-6",
            div { class: "flex flex-col gap-3 sm:flex-row sm:items-end sm:justify-between",
                div { class: "flex min-w-0 flex-col gap-1",
                    if let Some((label, onclick)) = back {
                        div { class: "mb-1",
                            TextLink { onclick: move |_| onclick.call(()), icon: Icon::ArrowLeft, "{label}" }
                        }
                    }
                    h1 { class: "text-2xl font-semibold tracking-tight sm:text-3xl", "{title}" }
                    if let Some(s) = subtitle {
                        p { class: "text-sm text-base-content/60", "{s}" }
                    }
                }
                if let Some(a) = actions {
                    div { class: "flex flex-wrap items-center gap-2", {a} }
                }
            }
            {children}
        }
    }
}

/// Centred card for log in, sign up and unlock.
#[component]
pub fn AuthLayout(
    title: String,
    #[props(default)] subtitle: Option<String>,
    /// Shown under the card (for example "No account? Sign up").
    #[props(default)]
    footer: Option<Element>,
    /// Shown above the title (for example a back link).
    #[props(default)]
    top: Option<Element>,
    children: Element,
) -> Element {
    rsx! {
        main { class: "relative flex min-h-dvh items-center justify-center overflow-hidden bg-base-200/60 px-4 py-10",
            Backdrop {}
            div { class: "theo-page relative z-10 flex w-full max-w-md flex-col gap-6",
                {top}
                div { class: "flex flex-col items-center gap-3 text-center",
                    BrandMark { large: true }
                    h1 { class: "text-2xl font-semibold tracking-tight", "{title}" }
                    if let Some(s) = subtitle {
                        p { class: "text-sm text-base-content/60 text-pretty", "{s}" }
                    }
                }
                div { class: "rounded-box border border-base-300 bg-base-100 p-6 shadow-xl shadow-base-content/5 sm:p-8",
                    {children}
                }
                if let Some(f) = footer {
                    div { class: "text-center text-sm text-base-content/70", {f} }
                }
            }
        }
    }
}

/// Multi-step flow (for example creating a congregation).
#[component]
pub fn WizardLayout(
    title: String,
    #[props(default)] subtitle: Option<String>,
    steps: Vec<String>,
    current: usize,
    #[props(default)] top: Option<Element>,
    children: Element,
) -> Element {
    let total = steps.len().max(1);
    let pct = ((current + 1) * 100) / total;
    rsx! {
        main { class: "relative flex min-h-dvh justify-center overflow-hidden bg-base-200/60 px-4 py-8 sm:py-14",
            Backdrop {}
            div { class: "theo-page relative z-10 flex w-full max-w-2xl flex-col gap-6",
                {top}
                div { class: "flex items-center gap-3",
                    BrandMark {}
                    div { class: "flex flex-col",
                        h1 { class: "text-xl font-semibold tracking-tight sm:text-2xl", "{title}" }
                        if let Some(s) = subtitle {
                            p { class: "text-sm text-base-content/60", "{s}" }
                        }
                    }
                }
                // Steps: labels on sm+, progress bar on phones.
                ul { class: "steps steps-horizontal hidden w-full sm:grid",
                    for (i, s) in steps.iter().enumerate() {
                        li {
                            key: "{i}",
                            class: if i <= current { "step step-primary text-xs" } else { "step text-xs text-base-content/50" },
                            "data-content": if i < current { "✓" } else { "" },
                            "{s}"
                        }
                    }
                }
                div { class: "flex flex-col gap-2 sm:hidden",
                    div { class: "flex justify-between text-xs text-base-content/60",
                        span { "Step {current + 1} of {total}" }
                        span { class: "font-medium text-base-content", {steps.get(current).cloned().unwrap_or_default()} }
                    }
                    div { class: "h-1.5 overflow-hidden rounded-full bg-base-300",
                        div { class: "h-full rounded-full bg-primary transition-all duration-500 ease-[var(--ease-out-soft)]", style: "width: {pct}%;" }
                    }
                }
                div { class: "rounded-box border border-base-300 bg-base-100 p-5 shadow-xl shadow-base-content/5 sm:p-8",
                    // Keyed by step: each step slides in.
                    for k in std::iter::once(current) {
                        div { key: "{k}", class: "theo-page", {children.clone()} }
                    }
                }
            }
        }
    }
}

/// Soft colour blobs behind auth and wizard pages.
#[component]
fn Backdrop() -> Element {
    rsx! {
        div { class: "pointer-events-none absolute inset-0", "aria-hidden": "true",
            div { class: "absolute -left-24 -top-24 size-96 rounded-full bg-primary/15 blur-3xl animate-float" }
            div { class: "absolute -bottom-32 -right-24 size-[28rem] rounded-full bg-accent/15 blur-3xl animate-float", style: "animation-delay: -3s;" }
        }
    }
}

/// A clickable text link with an icon, for "back" and similar navigation.
#[component]
pub fn TextLink(
    onclick: EventHandler<()>,
    #[props(default)] icon: Option<Icon>,
    children: Element,
) -> Element {
    rsx! {
        button {
            r#type: "button",
            class: "inline-flex items-center gap-1.5 text-sm font-medium text-primary underline-offset-4 hover:underline",
            onclick: move |_| onclick.call(()),
            if let Some(icon) = icon {
                IconView { icon, size: IconSize::Sm }
            }
            {children}
        }
    }
}
