//! The filter bar for lists (R-436): search, then one menu per filter on larger screens, or one
//! "Filters" sheet on phones (the button shows how many filters are changed). Inside a filter, values combine with OR; filters combine with AND.

use crate::action::{Action, ActionButton};
use crate::button::{Button, ButtonSize, ButtonVariant};
use crate::form::{ChipToggle, InputKind, TextInput};
use crate::icon::{Icon, IconSize, IconView};
use crate::menu::{Placement, Popover};
use crate::overlay::Dialog;
use dioxus::prelude::*;
use std::collections::{BTreeMap, BTreeSet};

/// Selected values per filter id.
pub type FilterSelection = BTreeMap<String, BTreeSet<String>>;

/// One value of a [`Facet`].
#[derive(Clone, Debug, PartialEq)]
pub struct FacetOption {
    /// Value stored in the selection.
    pub value: String,
    /// Text shown.
    pub label: String,
    /// How many results this value gives, with the other filters applied.
    pub count: usize,
}

/// One filter of a [`FilterBar`].
#[derive(Clone, Debug, PartialEq)]
pub struct Facet {
    /// Key in the [`FilterSelection`].
    pub id: String,
    /// Name of the filter.
    pub label: String,
    /// Icon of the filter.
    pub icon: Icon,
    /// The values to choose from.
    pub options: Vec<FacetOption>,
}

fn toggled(sel: &FilterSelection, facet: &str, value: &str) -> FilterSelection {
    let mut out = sel.clone();
    let set = out.entry(facet.to_string()).or_default();
    if !set.remove(value) {
        set.insert(value.to_string());
    }
    if set.is_empty() {
        out.remove(facet);
    }
    out
}

fn cleared(sel: &FilterSelection, facet: &str) -> FilterSelection {
    let mut out = sel.clone();
    out.remove(facet);
    out
}

/// How many chosen values differ from the defaults.
fn changed(sel: &FilterSelection, defaults: &FilterSelection) -> usize {
    let extra: usize = sel
        .iter()
        .map(|(k, vs)| vs.iter().filter(|v| !defaults.get(k).is_some_and(|d| d.contains(*v))).count())
        .sum();
    let missing: usize = defaults
        .iter()
        .map(|(k, vs)| vs.iter().filter(|v| !sel.get(k).is_some_and(|s| s.contains(*v))).count())
        .sum();
    extra + missing
}

/// Search + filters + the number of results.
#[component]
pub fn FilterBar(
    query: String,
    on_query: EventHandler<String>,
    search_placeholder: String,
    facets: Vec<Facet>,
    selected: FilterSelection,
    on_select: EventHandler<FilterSelection>,
    /// For example "12 of 40 users".
    summary: String,
    /// The selection the list starts with. "Clear filters" goes back to it, and it does not
    /// count as an active filter (for example "Status: Current").
    #[props(default)]
    defaults: FilterSelection,
) -> Element {
    let mut sheet = use_signal(|| false);
    let n = changed(&selected, &defaults);
    let reset = defaults.clone();
    let reset_sheet = defaults.clone();

    rsx! {
        div { class: "flex flex-col gap-3",
            div { class: "flex items-center gap-2",
                div { class: "min-w-0 flex-1 md:max-w-md",
                    TextInput {
                        kind: InputKind::Search,
                        value: query,
                        icon: Icon::Search,
                        placeholder: search_placeholder,
                        oninput: move |v| on_query.call(v),
                    }
                }
                div { class: if facets.is_empty() { "hidden" } else { "md:hidden" },
                    Button {
                        variant: if n > 0 { ButtonVariant::Primary } else { ButtonVariant::Secondary },
                        icon: Icon::Filter,
                        onclick: move |_| sheet.set(true),
                        span { class: "sr-only", {Action::Filters.label()} }
                        if n > 0 {
                            span { class: "badge badge-sm border-0 bg-primary-content/20 text-primary-content", "{n}" }
                        }
                    }
                }
                span { class: "ml-auto hidden shrink-0 text-sm text-base-content/60 md:block", "{summary}" }
            }
            // Larger screens: one menu per filter.
            div { class: "hidden flex-wrap items-center gap-2 md:flex",
                for f in facets.iter().cloned() {
                    FacetMenu {
                        key: "{f.id}",
                        selected: selected.get(&f.id).cloned().unwrap_or_default(),
                        on_toggle: {
                            let s = selected.clone();
                            let id = f.id.clone();
                            move |v: String| on_select.call(toggled(&s, &id, &v))
                        },
                        on_clear: {
                            let s = selected.clone();
                            let id = f.id.clone();
                            move |_| on_select.call(cleared(&s, &id))
                        },
                        facet: f,
                    }
                }
                if n > 0 {
                    ActionButton { action: Action::ClearFilters, onclick: move |_| on_select.call(reset.clone()) }
                }
            }
            // Phones: only the result count. Filters are changed in the sheet.
            span { class: "text-xs text-base-content/60 md:hidden", "{summary}" }
        }
        Dialog {
            open: sheet(),
            onclose: move |_| sheet.set(false),
            title: Action::Filters.label(),
            description: "Choose one or more values per filter. Results must match every filter.",
            actions: rsx! {
                ActionButton { action: Action::ClearFilters, disabled: n == 0, onclick: move |_| on_select.call(reset_sheet.clone()) }
                Button { variant: ButtonVariant::Primary, onclick: move |_| sheet.set(false), "Show {summary}" }
            },
            div { class: "flex flex-col gap-5",
                for f in facets.iter().cloned() {
                    div { key: "{f.id}", class: "flex flex-col gap-2",
                        span { class: "flex items-center gap-1.5 text-sm font-medium",
                            span { class: "text-base-content/60", IconView { icon: f.icon, size: IconSize::Sm } }
                            "{f.label}"
                        }
                        div { class: "flex flex-wrap gap-2",
                            for o in f.options.iter().cloned() {
                                ChipToggle {
                                    key: "{o.value}",
                                    label: o.label.clone(),
                                    count: o.count,
                                    selected: selected.get(&f.id).is_some_and(|s| s.contains(&o.value)),
                                    onclick: {
                                        let s = selected.clone();
                                        let id = f.id.clone();
                                        let v = o.value.clone();
                                        move |_| on_select.call(toggled(&s, &id, &v))
                                    },
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

/// One filter as a pill that opens a checklist.
#[component]
fn FacetMenu(
    facet: Facet,
    selected: BTreeSet<String>,
    on_toggle: EventHandler<String>,
    on_clear: EventHandler<()>,
) -> Element {
    let mut open = use_signal(|| false);
    let chosen: Vec<&FacetOption> = facet.options.iter().filter(|o| selected.contains(&o.value)).collect();
    let summary = match chosen.as_slice() {
        [] => None,
        [one] => Some(one.label.clone()),
        many => Some(format!("{}", many.len())),
    };
    let active = summary.is_some();
    let trigger = rsx! {
        button {
            r#type: "button",
            class: if active {
                "inline-flex h-9 items-center gap-1.5 rounded-full border border-primary/40 bg-primary/10 px-3 text-sm font-medium text-primary transition-colors"
            } else {
                "inline-flex h-9 items-center gap-1.5 rounded-full border border-dashed border-base-content/25 px-3 text-sm text-base-content/80 transition-colors hover:bg-base-200"
            },
            "aria-haspopup": "true",
            "aria-expanded": if open() { "true" } else { "false" },
            onclick: move |_| open.set(!open()),
            IconView { icon: facet.icon, size: IconSize::Sm }
            "{facet.label}"
            if let Some(s) = summary {
                span { class: "h-4 w-px bg-primary/30" }
                span { class: "max-w-40 truncate", "{s}" }
            }
            IconView { icon: Icon::ChevronDown, size: IconSize::Xs }
        }
    };
    rsx! {
        Popover {
            open: open(),
            onclose: move |_| open.set(false),
            placement: Placement::Start,
            trigger,
            div { class: "px-2 pb-1 pt-1.5 text-xs font-medium text-base-content/60", "{facet.label}" }
            ul { class: "flex max-h-80 flex-col gap-0.5 overflow-y-auto overscroll-contain",
                for o in facet.options.iter().cloned() {
                    {
                        let checked = selected.contains(&o.value);
                        let v = o.value.clone();
                        rsx! {
                            li { key: "{o.value}",
                                label { class: "flex cursor-pointer items-center gap-3 rounded-field px-2 py-2 text-sm transition-colors hover:bg-base-200",
                                    input {
                                        r#type: "checkbox",
                                        class: "checkbox checkbox-primary checkbox-sm",
                                        checked,
                                        onchange: move |_| on_toggle.call(v.clone()),
                                    }
                                    span { class: "min-w-0 flex-1 truncate", "{o.label}" }
                                    span { class: "text-xs tabular-nums text-base-content/50", "{o.count}" }
                                }
                            }
                        }
                    }
                }
            }
            if active {
                div { class: "mt-1 border-t border-base-300 pt-1",
                    Button { variant: ButtonVariant::Ghost, size: ButtonSize::Sm, full_width: true, onclick: move |_| on_clear.call(()), "Clear {facet.label}" }
                }
            }
        }
    }
}
