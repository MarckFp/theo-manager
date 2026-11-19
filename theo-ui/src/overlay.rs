//! Dialogs. One component for every overlay: a bottom sheet on phones, a centred dialog on
//! larger screens (R-426).

use crate::action::{Action, ActionButton};
use crate::button::ButtonVariant;
use crate::icon::{Icon, IconSize, IconView};
use dioxus::prelude::*;

/// A dialog. `actions` go in the standard place: the bottom, full width on phones.
#[component]
pub fn Dialog(
    open: bool,
    onclose: EventHandler<()>,
    title: String,
    #[props(default)] description: Option<String>,
    #[props(default)] actions: Option<Element>,
    children: Element,
) -> Element {
    if !open {
        return rsx! {};
    }
    rsx! {
        div {
            class: "fixed inset-0 z-[55] flex items-end justify-center overscroll-contain sm:items-center sm:p-6",
            // While a dialog or sheet is open the page behind it does not scroll (theo-ui.css).
            "data-scroll-lock": "true",
            onkeydown: move |e| {
                if e.key() == Key::Escape {
                    onclose.call(())
                }
            },
            div { class: "absolute inset-0 bg-black/50 animate-fade-in", onclick: move |_| onclose.call(()) }
            div {
                role: "dialog",
                "aria-modal": "true",
                "aria-label": "{title}",
                class: "relative flex max-h-[88dvh] w-full flex-col rounded-t-box border border-base-300 bg-base-100 shadow-2xl animate-sheet-in sm:max-w-lg sm:rounded-box sm:animate-dialog-in pb-[env(safe-area-inset-bottom)]",
                div { class: "mx-auto mt-2 h-1.5 w-10 shrink-0 rounded-full bg-base-300 sm:hidden", "aria-hidden": "true" }
                div { class: "flex items-start gap-3 px-5 pb-2 pt-3 sm:pt-5",
                    div { class: "flex min-w-0 flex-1 flex-col gap-1",
                        h2 { class: "text-lg font-semibold tracking-tight", "{title}" }
                        if let Some(d) = description {
                            p { class: "text-sm text-base-content/60", "{d}" }
                        }
                    }
                    button {
                        r#type: "button",
                        class: "btn btn-ghost btn-sm btn-square -mr-2",
                        "aria-label": "Close",
                        onclick: move |_| onclose.call(()),
                        IconView { icon: Icon::Close, size: IconSize::Sm }
                    }
                }
                // `overscroll-contain`: reaching the end of the sheet does not scroll the page behind.
                div { class: "min-h-0 flex-1 overflow-y-auto overscroll-contain px-5 py-3", {children} }
                if let Some(a) = actions {
                    div { class: "flex flex-col-reverse gap-2 border-t border-base-300 px-5 py-4 sm:flex-row sm:justify-end [&>*]:w-full sm:[&>*]:w-auto",
                        {a}
                    }
                }
            }
        }
    }
}

/// Asks before an action that is hard to undo. `consequences` lists what else changes.
#[component]
pub fn ConfirmDialog(
    open: bool,
    title: String,
    message: String,
    confirm: Action,
    #[props(default)] confirm_label: Option<String>,
    #[props(default)] consequences: Vec<String>,
    onconfirm: EventHandler<()>,
    oncancel: EventHandler<()>,
) -> Element {
    rsx! {
        Dialog {
            open,
            onclose: move |_| oncancel.call(()),
            title,
            actions: rsx! {
                ActionButton { action: Action::Cancel, variant: ButtonVariant::Ghost, onclick: move |_| oncancel.call(()) }
                ActionButton { action: confirm, label: confirm_label, onclick: move |_| onconfirm.call(()) }
            },
            div { class: "flex flex-col gap-3 text-sm",
                p { class: "text-base-content/80", "{message}" }
                if !consequences.is_empty() {
                    ul { class: "flex flex-col gap-1.5 rounded-field bg-base-200 p-3",
                        for (i, c) in consequences.iter().enumerate() {
                            li { key: "{i}", class: "flex items-start gap-2",
                                span { class: "mt-0.5 text-warning", IconView { icon: Icon::Warning, size: IconSize::Xs } }
                                span { "{c}" }
                            }
                        }
                    }
                }
            }
        }
    }
}
