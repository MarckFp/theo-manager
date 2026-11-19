//! Lists of people (or other records) inside a card or a dialog: one row per record, with an
//! avatar. `RowList` opens a row or runs an action on it; `PickList` chooses several rows.

use crate::card::{Avatar, Tag};
use crate::feedback::Badge;
use crate::icon::{Icon, IconSize, IconView};
use dioxus::prelude::*;
use std::collections::BTreeSet;

/// One row of a [`RowList`] or [`PickList`].
#[derive(Clone, Debug, PartialEq)]
pub struct Row {
    /// Identifier sent to the handlers.
    pub id: String,
    /// Main text (usually a name). Also used for the avatar.
    pub title: String,
    /// Second line.
    pub subtitle: Option<String>,
    /// A label after the title (for example "Overseer").
    pub tag: Option<Tag>,
    /// Shows the row action (for example "Remove") on this row.
    pub action: bool,
}

impl Row {
    /// Creates a row with no subtitle, tag or action.
    pub fn new(id: impl Into<String>, title: impl Into<String>) -> Self {
        Self { id: id.into(), title: title.into(), subtitle: None, tag: None, action: false }
    }
    /// Adds a second line.
    pub fn with_subtitle(mut self, s: impl Into<String>) -> Self {
        self.subtitle = Some(s.into());
        self
    }
    /// Adds a label.
    pub fn with_tag(mut self, t: Tag) -> Self {
        self.tag = Some(t);
        self
    }
    /// Shows the row action.
    pub fn with_action(mut self) -> Self {
        self.action = true;
        self
    }
}

#[component]
fn RowBody(row: Row) -> Element {
    rsx! {
        Avatar { name: row.title.clone() }
        span { class: "flex min-w-0 flex-1 flex-col",
            span { class: "flex min-w-0 items-center gap-2",
                span { class: "truncate text-sm font-medium", "{row.title}" }
                if let Some(t) = row.tag {
                    Badge { tone: t.tone, icon: t.icon, "{t.label}" }
                }
            }
            if let Some(s) = row.subtitle {
                span { class: "truncate text-xs text-base-content/60", "{s}" }
            }
        }
    }
}

/// Rows that open on click, with an optional action button per row (for example "Remove").
#[component]
pub fn RowList(
    rows: Vec<Row>,
    on_open: EventHandler<String>,
    /// Icon and accessible label of the row action.
    #[props(default)]
    action: Option<(Icon, String)>,
    #[props(default)] on_action: Option<EventHandler<String>>,
) -> Element {
    rsx! {
        ul { class: "flex flex-col divide-y divide-base-300",
            for row in rows {
                {
                    let open_id = row.id.clone();
                    let act_id = row.id.clone();
                    let show_action = row.action;
                    rsx! {
                        li { key: "{row.id}", class: "flex items-center gap-1 py-1 first:pt-0 last:pb-0",
                            button {
                                r#type: "button",
                                class: "-mx-2 flex min-w-0 flex-1 items-center gap-3 rounded-field px-2 py-2 text-left transition-colors hover:bg-base-200",
                                onclick: move |_| on_open.call(open_id.clone()),
                                RowBody { row: row.clone() }
                            }
                            if let (true, Some((icon, label)), Some(h)) = (show_action, action.clone(), on_action) {
                                button {
                                    r#type: "button",
                                    class: "btn btn-ghost btn-square btn-sm text-base-content/60 hover:text-error",
                                    "aria-label": "{label} {row.title}",
                                    title: "{label}",
                                    onclick: move |_| h.call(act_id.clone()),
                                    IconView { icon, size: IconSize::Sm }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

/// Rows with a checkbox, to choose several (for example members to add).
#[component]
pub fn PickList(rows: Vec<Row>, selected: BTreeSet<String>, on_toggle: EventHandler<String>) -> Element {
    rsx! {
        ul { class: "flex flex-col gap-0.5",
            for row in rows {
                {
                    let id = row.id.clone();
                    let checked = selected.contains(&row.id);
                    rsx! {
                        li { key: "{row.id}",
                            label { class: if checked { "flex cursor-pointer items-center gap-3 rounded-field bg-primary/5 px-2 py-2 transition-colors" } else { "flex cursor-pointer items-center gap-3 rounded-field px-2 py-2 transition-colors hover:bg-base-200" },
                                input {
                                    r#type: "checkbox",
                                    class: "checkbox checkbox-primary checkbox-sm",
                                    checked,
                                    onchange: move |_| on_toggle.call(id.clone()),
                                }
                                RowBody { row: row.clone() }
                            }
                        }
                    }
                }
            }
        }
    }
}

/// A group of rows in a [`ChoiceList`], with a heading.
#[derive(Clone, Debug, PartialEq)]
pub struct RowGroup {
    /// Heading above the rows ("Meeting for field service").
    pub title: String,
    /// The rows.
    pub rows: Vec<Row>,
}

/// Rows with a radio button, to choose one (or none), in groups with a heading. Each row can
/// carry a tag (for example whether the person is available).
#[component]
pub fn ChoiceList(
    groups: Vec<RowGroup>,
    selected: Option<String>,
    on_choose: EventHandler<Option<String>>,
    /// A first row that chooses nothing ("Nobody yet").
    #[props(default)]
    none_label: Option<String>,
) -> Element {
    let radio = |checked: bool| if checked { "flex cursor-pointer items-center gap-3 rounded-field bg-primary/5 px-2 py-2 transition-colors" } else { "flex cursor-pointer items-center gap-3 rounded-field px-2 py-2 transition-colors hover:bg-base-200" };
    rsx! {
        div { class: "flex flex-col gap-3", role: "radiogroup",
            if let Some(label) = none_label {
                label { class: radio(selected.is_none()),
                    input { r#type: "radio", class: "radio radio-primary radio-sm", checked: selected.is_none(), onchange: move |_| on_choose.call(None) }
                    span { class: "text-sm text-base-content/70", "{label}" }
                }
            }
            for group in groups.into_iter().filter(|g| !g.rows.is_empty()) {
                div { key: "{group.title}", class: "flex flex-col gap-0.5",
                    span { class: "px-2 pb-1 text-xs font-semibold uppercase tracking-wide text-base-content/50", "{group.title}" }
                    ul { class: "flex flex-col gap-0.5",
                        for row in group.rows {
                            {
                                let id = row.id.clone();
                                let checked = selected.as_deref() == Some(row.id.as_str());
                                rsx! {
                                    li { key: "{row.id}",
                                        label { class: radio(checked),
                                            input {
                                                r#type: "radio",
                                                class: "radio radio-primary radio-sm",
                                                checked,
                                                onchange: move |_| on_choose.call(Some(id.clone())),
                                            }
                                            RowBody { row: row.clone() }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
