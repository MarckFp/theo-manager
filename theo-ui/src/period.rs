//! Choosing a month: previous and next buttons, and a popover to jump to any month and year.

use crate::icon::{Icon, IconSize, IconView};
use crate::menu::{Placement, Popover};
use dioxus::prelude::*;

const MONTHS: [&str; 12] = ["January", "February", "March", "April", "May", "June", "July", "August", "September", "October", "November", "December"];

/// A month of a year, `(year, 1..=12)`.
pub type YearMonth = (i32, u32);

fn shift((y, m): YearMonth, by: i32) -> YearMonth {
    let idx = y * 12 + m as i32 - 1 + by;
    (idx.div_euclid(12), idx.rem_euclid(12) as u32 + 1)
}

/// Full English month name (1..=12).
pub fn month_name(m: u32) -> &'static str {
    MONTHS[(m.clamp(1, 12) - 1) as usize]
}

/// `‹  October 2026 ▾  ›`. The label opens a year switcher and a grid of months.
/// Months outside `min..=max` cannot be chosen.
#[component]
pub fn MonthPicker(value: YearMonth, min: YearMonth, max: YearMonth, onchange: EventHandler<YearMonth>) -> Element {
    let mut open = use_signal(|| false);
    let mut year = use_signal(|| value.0);
    let prev = shift(value, -1);
    let next = shift(value, 1);
    let can_prev = prev >= min;
    let can_next = next <= max;
    let shown_year = year();
    let trigger = rsx! {
        button {
            r#type: "button",
            class: "flex h-10 min-w-44 items-center justify-center gap-1.5 rounded-field px-3 text-base font-semibold transition-colors hover:bg-base-200",
            "aria-haspopup": "dialog",
            "aria-expanded": if open() { "true" } else { "false" },
            onclick: move |_| {
                year.set(value.0);
                open.set(!open());
            },
            IconView { icon: Icon::Calendar, size: IconSize::Sm }
            "{month_name(value.1)} {value.0}"
            IconView { icon: Icon::ChevronDown, size: IconSize::Xs }
        }
    };
    rsx! {
        div { class: "inline-flex items-center gap-1 rounded-box border border-base-300 bg-base-100 p-1",
            button {
                r#type: "button",
                class: "btn btn-ghost btn-square btn-sm",
                "aria-label": "Previous month",
                disabled: !can_prev,
                onclick: move |_| onchange.call(prev),
                IconView { icon: Icon::ArrowLeft, size: IconSize::Sm }
            }
            Popover {
                open: open(),
                onclose: move |_| open.set(false),
                placement: Placement::Start,
                trigger,
                div { class: "flex items-center justify-between px-1 pb-2",
                    button {
                        r#type: "button",
                        class: "btn btn-ghost btn-square btn-sm",
                        "aria-label": "Previous year",
                        disabled: shown_year <= min.0,
                        onclick: move |_| year.set(shown_year - 1),
                        IconView { icon: Icon::ArrowLeft, size: IconSize::Sm }
                    }
                    span { class: "text-sm font-semibold tabular-nums", "{shown_year}" }
                    button {
                        r#type: "button",
                        class: "btn btn-ghost btn-square btn-sm",
                        "aria-label": "Next year",
                        disabled: shown_year >= max.0,
                        onclick: move |_| year.set(shown_year + 1),
                        IconView { icon: Icon::ArrowRight, size: IconSize::Sm }
                    }
                }
                div { class: "grid grid-cols-3 gap-1",
                    for m in 1..=12u32 {
                        {
                            let ym = (shown_year, m);
                            let selected = ym == value;
                            let enabled = ym >= min && ym <= max;
                            rsx! {
                                button {
                                    key: "{m}",
                                    r#type: "button",
                                    disabled: !enabled,
                                    class: if selected { "rounded-field bg-primary px-2 py-2 text-sm font-semibold text-primary-content" } else if enabled { "rounded-field px-2 py-2 text-sm transition-colors hover:bg-base-200" } else { "rounded-field px-2 py-2 text-sm text-base-content/30" },
                                    "aria-current": if selected { "date" } else { "false" },
                                    onclick: move |_| {
                                        open.set(false);
                                        onchange.call(ym);
                                    },
                                    "{&month_name(m)[..3]}"
                                }
                            }
                        }
                    }
                }
            }
            button {
                r#type: "button",
                class: "btn btn-ghost btn-square btn-sm",
                "aria-label": "Next month",
                disabled: !can_next,
                onclick: move |_| onchange.call(next),
                IconView { icon: Icon::ArrowRight, size: IconSize::Sm }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shift_crosses_years() {
        assert_eq!(shift((2026, 1), -1), (2025, 12));
        assert_eq!(shift((2025, 12), 1), (2026, 1));
        assert_eq!(shift((2026, 9), -12), (2025, 9));
    }
}
