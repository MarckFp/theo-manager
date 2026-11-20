//! Page templates (R-421). Screens fill slots; they never build their own layout.

use crate::card::BrandMark;
use crate::icon::{Icon, IconSize, IconView};
use dioxus::prelude::*;

/// Set by a [`Page`] with a back link: on phones its title sits in a sticky bar, and a
/// [`crate::StickyBar`] on that page sticks just below it instead of covering it.
#[derive(Clone, Copy, PartialEq)]
pub(crate) struct PhoneTitleBar(pub bool);

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
    let detail = back.is_some();
    let has_actions = actions.is_some();
    let mut bar = use_context_provider(|| Signal::new(PhoneTitleBar(detail)));
    if bar.peek().0 != detail {
        bar.set(PhoneTitleBar(detail));
    }
    // Detail pages on phones: the title moves into a sticky bar with a back arrow, so you always
    // see what you are editing, and the arrow is the way to leave without saving.
    let title_class = if detail { "hidden min-w-0 flex-col gap-1 md:flex" } else { "flex min-w-0 flex-col gap-1" };
    rsx! {
        div { class: "flex flex-col gap-6",
            if let Some((label, onclick)) = back.clone() {
                div { class: "sticky z-20 -mx-4 -mt-6 flex h-14 items-center gap-2 border-b border-base-300 bg-base-100/90 px-2 backdrop-blur sm:-mx-6 sm:px-4 md:hidden",
                    // Just under the phone top bar: 61 px + its top padding (which grows with the notch).
                    style: "top: calc(3.8125rem + max(0.625rem, env(safe-area-inset-top)));",
                    button {
                        r#type: "button",
                        class: "btn btn-ghost btn-square btn-sm",
                        "aria-label": "{label}",
                        title: "{label}",
                        onclick: move |_| onclick.call(()),
                        IconView { icon: Icon::ArrowLeft, size: IconSize::Md }
                    }
                    div { class: "flex min-w-0 flex-col",
                        span { class: "truncate text-base font-semibold leading-tight", "{title}" }
                        if let Some(s) = subtitle.clone() {
                            span { class: "truncate text-xs text-base-content/60", "{s}" }
                        }
                    }
                }
            }
            // On phones a detail page shows its title in the sticky bar; the header only holds the
            // actions, so without actions it is not shown at all (no empty gap).
            div { class: match (detail, has_actions) {
                    (true, false) => "hidden md:flex md:flex-row md:items-end md:justify-between md:gap-3",
                    _ => "flex flex-col gap-3 sm:flex-row sm:items-end sm:justify-between",
                },
                div { class: title_class,
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

/// A link to a page outside the app (opens in a new tab or the browser).
#[component]
pub fn ExternalLink(href: String, children: Element) -> Element {
    rsx! {
        a { class: "link link-primary inline-flex items-center gap-1 break-all", href: "{href}", target: "_blank", rel: "noopener noreferrer",
            {children}
            IconView { icon: Icon::Link, size: IconSize::Xs }
        }
    }
}

/// A button that opens an address outside the app (for example directions in the map app).
#[component]
pub fn LinkButton(href: String, label: String, icon: Icon) -> Element {
    let external = href.starts_with("http");
    rsx! {
        a {
            class: "btn btn-soft gap-2 border-base-300 bg-base-100 font-medium hover:bg-base-200",
            href: "{href}",
            target: if external { "_blank" } else { "_self" },
            rel: "noopener noreferrer",
            IconView { icon, size: IconSize::Sm }
            "{label}"
        }
    }
}
