//! *Departments › Custom tasks*: any task the congregation needs that has no page of its own
//! (for example the literature desk), with the people who do it.

use crate::people::{self, Person};
use crate::state::use_app_state;
use dioxus::prelude::*;
use std::collections::BTreeSet;
use theo_ui::*;
use theo_ui::Action; // explicit: dioxus::prelude also exports an `Action`

/// `/app/departments/custom-tasks`
#[component]
pub fn CustomTasksPage() -> Element {
    let state = use_app_state();
    let prefs = state.preferences();
    let dir = state.directory();
    let congregation = state.active_congregation().map(|c| c.name).unwrap_or_default();
    let mut query = use_signal(String::new);
    let mut editing = use_signal(|| None::<Option<u32>>);
    let tokens: Vec<String> = people::fold(&query()).split_whitespace().map(str::to_string).collect();
    let mut shown: Vec<_> = dir
        .custom_tasks
        .iter()
        .filter(|t| {
            let names: Vec<String> = t.people.iter().filter_map(|p| dir.person(*p)).map(|p| format!("{} {}", p.first_name, p.last_name)).collect();
            let hay = people::fold(&format!("{} {} {}", t.name, t.description.clone().unwrap_or_default(), names.join(" ")));
            tokens.iter().all(|x| hay.contains(x.as_str()))
        })
        .collect();
    shown.sort_by_cached_key(|t| people::fold(&t.name));
    let total = dir.custom_tasks.len();
    rsx! {
        Page {
            title: "Custom tasks",
            subtitle: "{congregation} · Any other task, and who does it",
            actions: rsx! {
                ActionButton { action: Action::Add, label: "Add task", onclick: move |_| editing.set(Some(None)) }
            },
            StickyBar {
                FilterBar {
                    query: query(),
                    on_query: move |q| query.set(q),
                    search_placeholder: "Search by task or person",
                    facets: Vec::new(),
                    selected: FilterSelection::new(),
                    on_select: move |_| {},
                    summary: format!("{} of {total} tasks", shown.len()),
                }
            }
            if shown.is_empty() {
                EmptyState {
                    icon: Icon::Task,
                    title: if total == 0 { "No custom tasks yet" } else { "No tasks match" },
                    description: "For example the literature desk, accounts or the garden.",
                    action: rsx! { ActionButton { action: Action::Add, label: "Add task", onclick: move |_| editing.set(Some(None)) } },
                }
            } else {
                Grid { cols: 3, gap: Gap::Sm,
                    for t in shown.iter() {
                        {
                            let id = t.id;
                            let names: Vec<String> = t.people.iter().filter_map(|p| dir.person(*p)).map(|p| p.display_name(&prefs)).collect();
                            let count = names.len();
                            let tags = vec![Tag::new(if count == 1 { "1 person".to_string() } else { format!("{count} people") }, if count == 0 { Tone::Warning } else { Tone::Neutral }).with_icon(Icon::Users)];
                            rsx! {
                                RecordCard {
                                    key: "{id}",
                                    title: t.name.clone(),
                                    avatar: t.name.clone(),
                                    subtitle: t.description.clone().or_else(|| (!names.is_empty()).then(|| names.join(", "))),
                                    tags,
                                    onclick: move |_| editing.set(Some(Some(id))),
                                }
                            }
                        }
                    }
                }
            }
        }
        if let Some(id) = editing() {
            TaskDialog { key: "{id:?}", id, onclose: move |_| editing.set(None) }
        }
    }
}

#[component]
fn TaskDialog(id: Option<u32>, onclose: EventHandler<()>) -> Element {
    let mut state = use_app_state();
    let toaster = use_toaster();
    let prefs = state.preferences();
    let dir = state.directory();
    let existing = id.and_then(|i| dir.custom_task(i).cloned());
    let mut name = use_signal(|| existing.as_ref().map(|t| t.name.clone()).unwrap_or_default());
    let mut description = use_signal(|| existing.as_ref().and_then(|t| t.description.clone()).unwrap_or_default());
    let mut chosen = use_signal(|| existing.as_ref().map(|t| t.people.iter().map(|p| p.to_string()).collect::<BTreeSet<_>>()).unwrap_or_default());
    let mut query = use_signal(String::new);
    let mut error = use_signal(|| None::<String>);
    let mut confirm = use_signal(|| false);
    let tokens: Vec<String> = people::fold(&query()).split_whitespace().map(str::to_string).collect();
    let mut current: Vec<&Person> = dir
        .people
        .iter()
        .filter(|p| p.moved_away.is_none())
        .filter(|p| {
            let hay = people::fold(&format!("{} {}", p.first_name, p.last_name));
            tokens.iter().all(|t| hay.contains(t.as_str()))
        })
        .collect();
    current.sort_by_cached_key(|p| (!chosen().contains(&p.id.to_string()), people::fold(&p.display_name(&prefs))));
    let rows: Vec<Row> = current.iter().map(|p| Row::new(p.id.to_string(), p.display_name(&prefs)).with_subtitle(p.publisher_type.label())).collect();
    let save = move |_| {
        let ids: Vec<u32> = chosen().iter().filter_map(|s| s.parse().ok()).collect();
        match state.update_directory(|d| d.save_custom_task(id, &name(), &description(), ids)) {
            Ok(_) => {
                toaster.success("Task saved.");
                onclose.call(());
            }
            Err(e) => error.set(Some(e)),
        }
    };
    rsx! {
        Dialog {
            open: true,
            onclose: move |_| onclose.call(()),
            title: if existing.is_some() { "Custom task" } else { "Add task" },
            description: "A task with no page of its own, and the people who do it.",
            actions: rsx! {
                EditActions {
                    on_save: save,
                    on_cancel: move |_| onclose.call(()),
                    on_delete: existing.is_some().then_some(EventHandler::new(move |_| confirm.set(true))),
                }
            },
            Stack {
                Field { label: "Name", required: true, error: error(),
                    TextInput { value: name(), placeholder: "For example: Literature desk", oninput: move |v| { name.set(v); error.set(None); } }
                }
                Field { label: "Description",
                    TextArea { value: description(), rows: 2, oninput: move |v| description.set(v) }
                }
                Field { label: "Who does it", help: format!("{} chosen.", chosen().len()),
                    TextInput { kind: InputKind::Search, value: query(), icon: Icon::Search, placeholder: "Search by name", oninput: move |v| query.set(v) }
                }
                PickList {
                    rows,
                    selected: chosen(),
                    on_toggle: move |pid: String| {
                        let mut c = chosen.write();
                        if !c.remove(&pid) {
                            c.insert(pid);
                        }
                    },
                }
            }
        }
        ConfirmDialog {
            open: confirm(),
            title: "Delete this task?",
            message: "The task and its list of people are removed.",
            confirm: Action::Delete,
            oncancel: move |_| confirm.set(false),
            onconfirm: move |_| {
                if let Some(i) = id {
                    state.update_directory(|d| d.delete_custom_task(i));
                }
                toaster.success("Task deleted.");
                onclose.call(());
            },
        }
    }
}
