//! Feedback: alerts, badges, spinners, skeletons, empty states and toasts (R-426).

use crate::icon::{Icon, IconSize, IconView};
use dioxus::prelude::*;
use std::time::Duration;

/// Meaning of a message.
#[allow(missing_docs)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Tone {
    #[default]
    Info,
    Success,
    Warning,
    Error,
    Neutral,
}

impl Tone {
    fn icon(self) -> Icon {
        match self {
            Tone::Info | Tone::Neutral => Icon::Info,
            Tone::Success => Icon::Success,
            Tone::Warning => Icon::Warning,
            Tone::Error => Icon::Error,
        }
    }
}

/// A page-level message.
#[component]
pub fn Alert(
    #[props(default)] tone: Tone,
    #[props(default)] title: Option<String>,
    children: Element,
) -> Element {
    let class = match tone {
        Tone::Info => "alert-info",
        Tone::Success => "alert-success",
        Tone::Warning => "alert-warning",
        Tone::Error => "alert-error",
        Tone::Neutral => "",
    };
    rsx! {
        div { role: "alert", class: "alert alert-soft {class} items-start text-sm",
            IconView { icon: tone.icon(), size: IconSize::Sm }
            div { class: "flex flex-col gap-1",
                if let Some(t) = title {
                    span { class: "font-semibold", "{t}" }
                }
                div { {children} }
            }
        }
    }
}

/// A small label.
#[component]
pub fn Badge(
    #[props(default)] tone: Tone,
    #[props(default)] icon: Option<Icon>,
    children: Element,
) -> Element {
    let class = match tone {
        Tone::Info => "badge-info",
        Tone::Success => "badge-success",
        Tone::Warning => "badge-warning",
        Tone::Error => "badge-error",
        Tone::Neutral => "badge-ghost",
    };
    rsx! {
        span { class: "badge badge-soft badge-sm {class} gap-1 font-medium",
            if let Some(icon) = icon {
                IconView { icon, size: IconSize::Xs }
            }
            {children}
        }
    }
}

/// A loading spinner.
#[component]
pub fn Spinner(#[props(default)] small: bool) -> Element {
    let size = if small { "loading-xs" } else { "loading-md" };
    rsx! {
        span { class: "loading loading-spinner {size}", "aria-label": "Loading" }
    }
}

/// Placeholder while data loads (R-426).
#[component]
pub fn Skeleton(#[props(default = 3)] lines: u8) -> Element {
    rsx! {
        div { class: "flex flex-col gap-3", "aria-hidden": "true",
            for i in 0..lines {
                div { key: "{i}", class: if i % 3 == 2 { "skeleton h-4 w-2/3" } else { "skeleton h-4 w-full" } }
            }
        }
    }
}

/// Shown when a list or page has nothing yet (R-426).
#[component]
pub fn EmptyState(
    icon: Icon,
    title: String,
    #[props(default)] description: Option<String>,
    #[props(default)] action: Option<Element>,
) -> Element {
    rsx! {
        div { class: "flex flex-col items-center justify-center gap-3 rounded-box border border-dashed border-base-300 px-6 py-14 text-center",
            div { class: "grid size-12 place-items-center rounded-full bg-base-200 text-base-content/70",
                IconView { icon, size: IconSize::Lg }
            }
            h3 { class: "text-base font-semibold", "{title}" }
            if let Some(d) = description {
                p { class: "max-w-sm text-sm text-base-content/60", "{d}" }
            }
            {action}
        }
    }
}

// ---------------------------------------------------------------------------------------------
// Toasts
// ---------------------------------------------------------------------------------------------

#[derive(Clone, PartialEq)]
struct ToastItem {
    id: u64,
    tone: Tone,
    text: String,
}

/// Handle to show toasts. Get it with [`use_toaster`].
#[derive(Clone, Copy)]
pub struct Toaster {
    items: Signal<Vec<ToastItem>>,
    next: Signal<u64>,
}

impl Toaster {
    /// Shows a toast that closes itself: after 4 seconds, or 7 seconds for errors (more to read).
    pub fn show(&self, tone: Tone, text: impl Into<String>) {
        let mut items = self.items;
        let mut next = self.next;
        let id = next();
        next.set(id + 1);
        items.write().push(ToastItem { id, tone, text: text.into() });
        let ms = if tone == Tone::Error { 7000 } else { 4000 };
        // `spawn_forever` runs at the app root: the toast closes even if the page that showed it
        // is gone (for example after "Save" navigates away).
        dioxus::dioxus_core::spawn_forever(async move {
            futures_timer::Delay::new(Duration::from_millis(ms)).await;
            items.write().retain(|t| t.id != id);
        });
    }

    /// Shows a success toast.
    pub fn success(&self, text: impl Into<String>) {
        self.show(Tone::Success, text);
    }

    /// Shows an error toast.
    pub fn error(&self, text: impl Into<String>) {
        self.show(Tone::Error, text);
    }
}

/// Returns the [`Toaster`] provided by [`crate::ThemeRoot`].
pub fn use_toaster() -> Toaster {
    use_context::<Toaster>()
}

pub(crate) fn provide_toaster() -> Toaster {
    use_context_provider(|| Toaster { items: Signal::new(Vec::new()), next: Signal::new(1) })
}

/// Renders the toasts. Always in the same corner (R-426).
#[component]
pub(crate) fn ToastHost() -> Element {
    let toaster = use_toaster();
    let mut items = toaster.items;
    rsx! {
        div { class: "toast toast-top toast-center sm:toast-end z-[60] pointer-events-none", "aria-live": "polite",
            for t in items.read().iter().cloned() {
                div {
                    key: "{t.id}",
                    class: match t.tone {
                        Tone::Success => "alert alert-success shadow-lg pointer-events-auto animate-toast-in",
                        Tone::Error => "alert alert-error shadow-lg pointer-events-auto animate-toast-in",
                        Tone::Warning => "alert alert-warning shadow-lg pointer-events-auto animate-toast-in",
                        _ => "alert shadow-lg pointer-events-auto animate-toast-in",
                    },
                    IconView { icon: t.tone.icon(), size: IconSize::Sm }
                    span { class: "text-sm", "{t.text}" }
                    button {
                        r#type: "button",
                        class: "btn btn-ghost btn-xs btn-square",
                        "aria-label": "Close",
                        onclick: move |_| items.write().retain(|x| x.id != t.id),
                        IconView { icon: Icon::Close, size: IconSize::Xs }
                    }
                }
            }
        }
    }
}
