use dioxus::prelude::*;
use dioxus_i18n::t;

/// A responsive modal that renders as a bottom sheet on mobile (< lg)
/// and a centered dialog on desktop (≥ lg).
///
/// The `children` slot is placed once inside a unified panel that adapts its
/// shape with responsive Tailwind classes — no duplicate DOM needed.
#[component]
pub fn ResponsiveModal(
    /// Reactive flag controlling visibility and transition state.
    open: Signal<bool>,
    /// Called when the user dismisses via backdrop click, ✕ button, or cancel.
    on_close: Callback<()>,
    /// Modal header title.
    title: String,
    /// Modal header subtitle / description.
    description: String,
    /// When `true`, footer buttons are disabled and the save label shows a
    /// loading variant.
    submitting: bool,
    /// Called when the user clicks the save / confirm button.
    on_submit: Callback<Event<MouseData>>,
    /// Form body rendered inside the scrollable area.
    children: Element,
) -> Element {
    let is_open = *open.read();

    // The single overlay doubles as backdrop. `items-end` on mobile aligns the
    // panel to the bottom; `lg:items-center` centres it on desktop.
    let overlay_cls = if is_open {
        "fixed inset-0 z-50 flex items-end lg:items-center justify-center lg:p-4 \
         bg-black/40 transition-opacity duration-300"
    } else {
        "fixed inset-0 z-50 flex items-end lg:items-center justify-center lg:p-4 \
         bg-black/40 transition-opacity duration-300 opacity-0 pointer-events-none"
    };

    // Mobile: slide up/down via translate-y.
    // Desktop: scale in/out. `lg:translate-y-0` cancels the mobile translate.
    let panel_cls = if is_open {
        "relative bg-white w-full rounded-t-2xl lg:rounded-2xl shadow-2xl \
         flex flex-col max-h-[92vh] lg:max-h-[90vh] lg:max-w-lg overflow-hidden \
         transition-all duration-300 translate-y-0 lg:scale-100"
    } else {
        "relative bg-white w-full rounded-t-2xl lg:rounded-2xl shadow-2xl \
         flex flex-col max-h-[92vh] lg:max-h-[90vh] lg:max-w-lg overflow-hidden \
         transition-all duration-300 translate-y-full lg:translate-y-0 lg:scale-95 opacity-0"
    };

    rsx! {
        div { class: overlay_cls, onclick: move |_| on_close.call(()),

            div {
                class: panel_cls,
                // Stop clicks on the panel from bubbling to the backdrop.
                onclick: move |e| e.stop_propagation(),

                // ── Drag handle — mobile only ─────────────────────────────
                div { class: "lg:hidden shrink-0 flex justify-center pt-3 pb-2",
                    div { class: "w-10 h-1 bg-gray-300 rounded-full" }
                }

                // ── Header ────────────────────────────────────────────────
                div { class: "shrink-0 flex items-start justify-between \
                              px-4 lg:px-6 pb-3 lg:pt-5 lg:pb-4 \
                              border-b border-gray-100",
                    div {
                        h2 { class: "text-base lg:text-lg font-semibold text-gray-900",
                            "{title}"
                        }
                        p { class: "text-xs lg:text-sm text-gray-500 mt-0.5", "{description}" }
                    }
                    button {
                        class: "ml-4 p-1 lg:p-1.5 text-gray-400 hover:text-gray-600 \
                                rounded hover:bg-gray-100 transition-colors",
                        onclick: move |_| on_close.call(()),
                        "✕"
                    }
                }

                // ── Scrollable body ───────────────────────────────────────
                div { class: "flex-1 overflow-y-auto px-4 lg:px-6 py-4 lg:py-5 space-y-4",
                    {children}
                    div { class: "h-4" }
                }

                // ── Footer ────────────────────────────────────────────────
                div { class: "shrink-0 px-4 lg:px-6 py-3 lg:py-4 \
                              border-t border-gray-100 \
                              flex gap-2 lg:gap-3 lg:justify-end",
                    button {
                        class: "flex-1 lg:flex-none px-4 lg:px-5 py-2.5 lg:py-2 \
                                text-sm border border-gray-200 rounded-xl \
                                text-gray-700 hover:bg-gray-50 transition-colors",
                        disabled: submitting,
                        onclick: move |_| on_close.call(()),
                        {t!("btn-cancel")}
                    }
                    button {
                        class: "flex-1 lg:flex-none px-4 lg:px-5 py-2.5 lg:py-2 \
                                text-sm bg-primary-600 text-white rounded-xl \
                                hover:bg-primary-700 disabled:opacity-50 transition-colors font-medium",
                        disabled: submitting,
                        onclick: move |e| on_submit.call(e),
                        {if submitting { t!("btn-connecting") } else { t!("btn-save") }}
                    }
                }
            }
        }
    }
}
