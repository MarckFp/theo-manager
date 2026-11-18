//! *Users › Absences*: who is away when. Rules: `docs/model/m01-people.md`, "Absences".

use crate::absences::{Absence, When};
use crate::people::{self, parse_date, Person};
use crate::prefs::Preferences;
use crate::state::use_app_state;
use dioxus::prelude::*;
use std::collections::BTreeSet;
use theo_ui::*;
use theo_ui::Action; // explicit: dioxus::prelude also exports an `Action`

fn show(prefs: &Preferences, iso: &str) -> String {
    parse_date(iso).map(|d| prefs.format_date(d)).unwrap_or_else(|| iso.to_string())
}

/// The list starts with absences that are now or coming up.
fn default_filters() -> FilterSelection {
    let mut s = FilterSelection::new();
    s.insert("when".into(), ["now".to_string(), "upcoming".to_string()].into_iter().collect::<BTreeSet<_>>());
    s
}

/// `/app/users/absences`
#[component]
pub fn AbsencesPage() -> Element {
    let state = use_app_state();
    let prefs = state.preferences();
    let dir = state.directory();
    let congregation = state.active_congregation().map(|c| c.name).unwrap_or_default();
    let mut query = use_signal(String::new);
    let mut sel = use_signal(default_filters);
    let mut editing = use_signal(|| None::<Option<u32>>);
    let today = people::today_iso();

    let tokens: Vec<String> = people::fold(&query()).split_whitespace().map(str::to_string).collect();
    let name_of = |a: &Absence| dir.person(a.person).map(|p| format!("{} {}", p.first_name, p.last_name)).unwrap_or_default();
    let matches = |a: &Absence, skip: bool| {
        let hay = people::fold(&name_of(a));
        tokens.iter().all(|t| hay.contains(t.as_str()))
            && (skip || sel().get("when").is_none_or(|w| w.is_empty() || w.contains(a.when(&today).value())))
    };
    let facets = vec![Facet {
        id: "when".into(),
        label: "When".into(),
        icon: Icon::Calendar,
        options: [(When::Now, "Away now"), (When::Upcoming, "Coming up"), (When::Past, "Past")]
            .into_iter()
            .map(|(w, label)| FacetOption {
                value: w.value().into(),
                label: label.into(),
                count: dir.absences.iter().filter(|a| matches(a, true) && a.when(&today) == w).count(),
            })
            .collect(),
    }];
    let mut shown: Vec<&Absence> = dir.absences.iter().filter(|a| matches(a, false)).collect();
    shown.sort_by(|a, b| a.from.cmp(&b.from));
    let total = dir.absences.len();

    rsx! {
        Page {
            title: "Absences",
            subtitle: "{congregation} · Who is away when. Schedules do not assign people while they are away.",
            actions: rsx! {
                ActionButton { action: Action::Add, label: "Add absence", onclick: move |_| editing.set(Some(None)) }
            },
            StickyBar {
                FilterBar {
                    query: query(),
                    on_query: move |q| query.set(q),
                    search_placeholder: "Search by name",
                    facets,
                    selected: sel(),
                    on_select: move |s| sel.set(s),
                    summary: format!("{} of {total} absences", shown.len()),
                    defaults: default_filters(),
                }
            }
            if shown.is_empty() {
                EmptyState { icon: Icon::Absence, title: "No absences here", description: "Add one, or change the filters." }
            } else {
                Grid { cols: 3, gap: Gap::Sm,
                    for a in shown.iter() {
                        {
                            let id = a.id;
                            let name = dir.person(a.person).map(|p| p.display_name(&prefs)).unwrap_or_default();
                            let days = parse_date(&a.to).zip(parse_date(&a.from)).map(|(t, f)| crate::prefs::days_from_ymd(t) - crate::prefs::days_from_ymd(f) + 1).unwrap_or(1);
                            let when = match a.when(&today) {
                                When::Now => Tag::new("Away now", Tone::Warning).with_icon(Icon::Absence),
                                When::Upcoming => Tag::new("Coming up", Tone::Info).with_icon(Icon::Calendar),
                                When::Past => Tag::new("Past", Tone::Neutral),
                            };
                            let mut tags = vec![when, Tag::new(if days == 1 { "1 day".to_string() } else { format!("{days} days") }, Tone::Neutral)];
                            if let Some(n) = &a.note {
                                tags.push(Tag::new(n.clone(), Tone::Neutral).with_icon(Icon::Note));
                            }
                            rsx! {
                                RecordCard {
                                    key: "{id}",
                                    title: name.clone(),
                                    avatar: name,
                                    subtitle: format!("{} – {}", show(&prefs, &a.from), show(&prefs, &a.to)),
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
            AbsenceDialog { key: "{id:?}", id, onclose: move |_| editing.set(None) }
        }
    }
}

#[component]
fn AbsenceDialog(id: Option<u32>, onclose: EventHandler<()>) -> Element {
    let mut state = use_app_state();
    let toaster = use_toaster();
    let prefs = state.preferences();
    let dir = state.directory();
    let existing = id.and_then(|i| dir.absences.iter().find(|a| a.id == i).cloned());
    let today = people::today_iso();
    let mut person = use_signal(|| existing.as_ref().map(|a| a.person.to_string()).unwrap_or_default());
    let mut from = use_signal(|| existing.as_ref().map(|a| a.from.clone()).unwrap_or_else(|| today.clone()));
    let mut to = use_signal(|| existing.as_ref().map(|a| a.to.clone()).unwrap_or_else(|| today.clone()));
    let mut note = use_signal(|| existing.as_ref().and_then(|a| a.note.clone()).unwrap_or_default());
    let mut errors = use_signal(Vec::<(&'static str, String)>::new);
    let mut confirm = use_signal(|| false);
    let err = move |f: &str| errors.read().iter().find(|(k, _)| *k == f).map(|(_, m)| m.clone());
    let mut current: Vec<&Person> = dir.people.iter().filter(|p| p.moved_away.is_none()).collect();
    current.sort_by_cached_key(|p| people::fold(&p.display_name(&prefs)));
    let mut options = vec![SelectOption::new("", "Choose a person…")];
    options.extend(current.iter().map(|p| SelectOption::new(p.id.to_string(), p.display_name(&prefs))));
    let save = move |_| {
        let p = person().parse::<u32>().unwrap_or(0);
        match state.update_directory(|d| d.save_absence(id, p, &from(), &to(), &note())) {
            Ok(_) => {
                toaster.success("Absence saved.");
                onclose.call(());
            }
            Err(e) => errors.set(e),
        }
    };
    rsx! {
        Dialog {
            open: true,
            onclose: move |_| onclose.call(()),
            title: if existing.is_some() { "Absence" } else { "Add absence" },
            description: "From the first to the last day the person is away (both included).",
            actions: rsx! {
                if existing.is_some() {
                    ActionButton { action: Action::Delete, variant: ButtonVariant::Ghost, onclick: move |_| confirm.set(true) }
                }
                ActionButton { action: Action::Cancel, onclick: move |_| onclose.call(()) }
                ActionButton { action: Action::Save, onclick: save }
            },
            Stack {
                Field { label: "Person", required: true, error: err("person"),
                    Select { value: person(), options, onchange: move |v| person.set(v) }
                }
                Grid { cols: 2,
                    Field { label: "First day", required: true, error: err("from"),
                        TextInput { kind: InputKind::Date, value: from(), oninput: move |v| from.set(v) }
                    }
                    Field { label: "Last day", required: true, error: err("to"),
                        TextInput { kind: InputKind::Date, value: to(), oninput: move |v| to.set(v) }
                    }
                }
                Field { label: "Reason", help: "Optional, for example \"Holiday\". Keep it short.",
                    TextInput { value: note(), oninput: move |v| note.set(v) }
                }
            }
        }
        ConfirmDialog {
            open: confirm(),
            title: "Delete this absence?",
            message: "Schedules can assign this person again on these days.",
            confirm: Action::Delete,
            oncancel: move |_| confirm.set(false),
            onconfirm: move |_| {
                if let Some(i) = id {
                    state.update_directory(|d| d.delete_absence(i));
                }
                toaster.success("Absence deleted.");
                onclose.call(());
            },
        }
    }
}
