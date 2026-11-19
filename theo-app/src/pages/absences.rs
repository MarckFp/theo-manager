//! *Users › Absences*: who is away when, for one person or the whole congregation.
//! Rules: `docs/model/m01-people.md`, "Absences".

use crate::absences::{Absence, When};
use crate::people::{self, parse_date, Directory, Person};
use crate::prefs::Preferences;
use crate::state::use_app_state;
use dioxus::prelude::*;
use std::collections::BTreeSet;
use theo_ui::*;
use theo_ui::Action; // explicit: dioxus::prelude also exports an `Action`

const F_WHEN: &str = "when";
const F_WHO: &str = "who";
/// The people filters of All users that make sense here (same look and behaviour).
const PEOPLE_FACETS: [&str; 5] = [people::FACET_LABELS, people::FACET_TYPE, people::FACET_APPOINTMENT, people::FACET_GROUP, people::FACET_GENDER];
const EVERYONE: &str = "Whole congregation";

fn show(prefs: &Preferences, iso: &str) -> String {
    parse_date(iso).map(|d| prefs.format_date(d)).unwrap_or_else(|| iso.to_string())
}

/// The list starts with absences that are now or coming up.
fn default_filters() -> FilterSelection {
    let mut s = FilterSelection::new();
    s.insert(F_WHEN.into(), ["now".to_string(), "upcoming".to_string()].into_iter().collect::<BTreeSet<_>>());
    s
}

fn days(a: &Absence) -> i64 {
    parse_date(&a.to).zip(parse_date(&a.from)).map(|(t, f)| crate::prefs::days_from_ymd(t) - crate::prefs::days_from_ymd(f) + 1).unwrap_or(1)
}

/// The values an absence has for a filter. An absence of the whole congregation covers everyone,
/// so it has every value of the people filters.
fn values(dir: &Directory, defs: &[people::FacetDef], a: &Absence, today: &str, facet: &str) -> Vec<String> {
    match facet {
        F_WHEN => vec![a.when(today).value().into()],
        F_WHO => vec![if a.person.is_some() { "person" } else { "congregation" }.into()],
        _ => match a.person.and_then(|p| dir.person(p)) {
            Some(p) => people::person_values(dir, p, facet),
            None => defs.iter().find(|d| d.0 == facet).map(|d| d.3.iter().map(|(v, _)| v.clone()).collect()).unwrap_or_default(),
        },
    }
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

    let v = |a: &str, b: &str| (a.to_string(), b.to_string());
    let person_defs = people::standard_facets(&dir, &PEOPLE_FACETS);
    let mut defs: Vec<people::FacetDef> = vec![
        (F_WHEN, "When", Icon::Calendar, vec![v("now", "Away now"), v("upcoming", "Coming up"), v("past", "Past")]),
        (F_WHO, "Who", Icon::Users, vec![v("person", "One person"), v("congregation", EVERYONE)]),
    ];
    defs.extend(person_defs.iter().cloned());

    let tokens: Vec<String> = people::fold(&query()).split_whitespace().map(str::to_string).collect();
    let name_of = |a: &Absence| match a.person.and_then(|p| dir.person(p)) {
        Some(p) => format!("{} {}", p.first_name, p.last_name),
        None => EVERYONE.to_string(),
    };
    let matches = |a: &Absence, skip: Option<&str>| {
        let hay = people::fold(&name_of(a));
        tokens.iter().all(|t| hay.contains(t.as_str()))
            && sel().iter().all(|(facet, chosen)| {
                Some(facet.as_str()) == skip || chosen.is_empty() || values(&dir, &person_defs, a, &today, facet).iter().any(|x| chosen.contains(x))
            })
    };
    let facets: Vec<Facet> = defs
        .iter()
        .map(|(id, label, icon, opts)| Facet {
            id: id.to_string(),
            label: label.to_string(),
            icon: *icon,
            options: opts
                .iter()
                .map(|(value, label)| FacetOption {
                    value: value.clone(),
                    label: label.clone(),
                    count: dir.absences.iter().filter(|a| matches(a, Some(id)) && values(&dir, &person_defs, a, &today, id).contains(value)).count(),
                })
                .collect(),
        })
        .collect();
    let mut shown: Vec<&Absence> = dir.absences.iter().filter(|a| matches(a, None)).collect();
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
                            let name = match a.person.and_then(|p| dir.person(p)) {
                                Some(p) => p.display_name(&prefs),
                                None => EVERYONE.to_string(),
                            };
                            let n = days(a);
                            let mut tags = vec![
                                match a.when(&today) {
                                    When::Now => Tag::new("Away now", Tone::Warning).with_icon(Icon::Absence),
                                    When::Upcoming => Tag::new("Coming up", Tone::Info).with_icon(Icon::Calendar),
                                    When::Past => Tag::new("Past", Tone::Neutral),
                                },
                                Tag::new(if n == 1 { "1 day".to_string() } else { format!("{n} days") }, Tone::Neutral),
                            ];
                            if a.person.is_none() {
                                tags.push(Tag::new("Everyone", Tone::Neutral).with_icon(Icon::Congregation));
                            }
                            if a.event.is_some() {
                                tags.push(Tag::new("Event", Tone::Info).with_icon(Icon::Calendar));
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
            if let Some(event) = id.and_then(|i| dir.absences.iter().find(|a| a.id == i)).and_then(|a| a.event) {
                EventAbsenceDialog { key: "{id:?}", event, onclose: move |_| editing.set(None) }
            } else {
                AbsenceDialog { key: "{id:?}", id, onclose: move |_| editing.set(None) }
            }
        }
    }
}

/// An absence an event created: it changes with the event (M-12), so it opens read-only. The
/// title of a private event shows only to the people who can see that event.
#[component]
fn EventAbsenceDialog(event: u32, onclose: EventHandler<()>) -> Element {
    let state = use_app_state();
    let navigator = navigator();
    let prefs = state.preferences();
    let dir = state.directory();
    let visible = dir.event(event).filter(|e| dir.can_see_event(e, state.me(), state.i_am_admin())).cloned();
    let text = match &visible {
        Some(e) => format!("It comes from the event \"{}\" ({}). It changes with the event: open Events to change or delete it.", e.title, super::events::period(&prefs, &e.from, &e.to)),
        None => "It comes from an event you cannot see. It changes with the event.".to_string(),
    };
    rsx! {
        Dialog {
            open: true,
            onclose: move |_| onclose.call(()),
            title: "Absence from an event",
            actions: rsx! {
                ActionButton { action: Action::Cancel, label: "Close", onclick: move |_| onclose.call(()) }
                if visible.is_some() {
                    ActionButton {
                        action: Action::View,
                        label: "Open events",
                        onclick: move |_| { navigator.push(crate::Route::EventsPage {}); },
                    }
                }
            },
            Alert { tone: Tone::Info, title: "Changed with its event", "{text}" }
        }
    }
}

/// The person field: `""` (not chosen), `"all"` (whole congregation) or a person id.
fn parse_target(v: &str) -> Option<Option<u32>> {
    match v {
        "all" => Some(None),
        _ => v.parse::<u32>().ok().map(Some),
    }
}

fn person_rows(dir: &Directory, prefs: &Preferences, ids: &[(String, u32)]) -> Vec<Row> {
    ids.iter()
        .filter_map(|(key, p)| dir.person(*p).map(|p| Row::new(key.clone(), p.display_name(prefs)).with_subtitle(p.publisher_type.label())))
        .collect()
}

#[component]
fn AbsenceDialog(id: Option<u32>, onclose: EventHandler<()>) -> Element {
    let mut state = use_app_state();
    let toaster = use_toaster();
    let prefs = state.preferences();
    let dir = state.directory();
    let existing = id.and_then(|i| dir.absences.iter().find(|a| a.id == i).cloned());
    let today = people::today_iso();
    let mut person = use_signal(|| match &existing {
        Some(a) => a.person.map(|p| p.to_string()).unwrap_or_else(|| "all".into()),
        None => String::new(),
    });
    let mut from = use_signal(|| existing.as_ref().map(|a| a.from.clone()).unwrap_or_else(|| today.clone()));
    let mut to = use_signal(|| existing.as_ref().map(|a| a.to.clone()).unwrap_or_else(|| today.clone()));
    let mut notes = use_signal(|| existing.as_ref().and_then(|a| a.notes.clone()).unwrap_or_default());
    let mut errors = use_signal(Vec::<(&'static str, String)>::new);
    // Adding: the step that asks whether other family members are away too.
    let mut family_step = use_signal(|| false);
    let mut with = use_signal(BTreeSet::<String>::new);
    // Deleting: confirm, and choose the same absence of family members to delete too.
    let mut confirm = use_signal(|| false);
    let mut also = use_signal(BTreeSet::<String>::new);
    let err = move |f: &str| errors.read().iter().find(|(k, _)| *k == f).map(|(_, m)| m.clone());

    let mut current: Vec<&Person> = dir.people.iter().filter(|p| p.moved_away.is_none()).collect();
    current.sort_by_cached_key(|p| people::fold(&p.display_name(&prefs)));
    let mut options = vec![SelectOption::new("", "Choose…"), SelectOption::new("all", EVERYONE)];
    options.extend(current.iter().map(|p| SelectOption::new(p.id.to_string(), p.display_name(&prefs)).in_group("People")));

    let target = parse_target(&person());
    let who = match target {
        Some(Some(p)) => dir.person(p).map(|p| p.display_name(&prefs)).unwrap_or_default(),
        _ => String::new(),
    };
    let others: Vec<(String, u32)> = match target {
        Some(Some(p)) => dir.family_others(p).into_iter().map(|m| (m.to_string(), m)).collect(),
        _ => Vec::new(),
    };
    let family_rows = person_rows(&dir, &prefs, &others);
    let matches: Vec<(String, u32)> = id.map(|i| dir.family_matches(i).into_iter().map(|(a, p)| (a.to_string(), p)).collect()).unwrap_or_default();
    let match_rows = person_rows(&dir, &prefs, &matches);
    let has_matches = !matches.is_empty();

    // Saves the absence, and the same absence for the chosen family members (`with_family`).
    let mut save = move |with_family: bool| {
        let Some(target) = parse_target(&person()) else {
            errors.set(vec![("person", "Choose a person, or the whole congregation.".into())]);
            return;
        };
        let e = state.directory().absence_errors(target, &from(), &to());
        if !e.is_empty() {
            family_step.set(false);
            errors.set(e);
            return;
        }
        if id.is_none() && !family_step() && target.is_some_and(|p| !state.directory().family_others(p).is_empty()) {
            family_step.set(true);
            return;
        }
        let extra: Vec<u32> = if with_family { with().iter().filter_map(|s| s.parse().ok()).collect() } else { Vec::new() };
        let (f, t, n) = (from(), to(), notes());
        let result = state.update_directory(|d| {
            let r = d.save_absence(id, target, &f, &t, &n);
            if r.is_ok() {
                for m in &extra {
                    let _ = d.save_absence(None, Some(*m), &f, &t, &n);
                }
            }
            r
        });
        match result {
            Ok(_) => {
                toaster.success(match extra.len() {
                    0 => "Absence saved.".to_string(),
                    k => format!("Absence saved for {} people.", k + 1),
                });
                onclose.call(());
            }
            Err(e) => errors.set(e),
        }
    };
    let mut delete = move || {
        let ids: Vec<u32> = id.into_iter().chain(also().iter().filter_map(|s| s.parse().ok())).collect();
        state.update_directory(|d| ids.iter().for_each(|i| d.delete_absence(*i)));
        toaster.success(if ids.len() == 1 { "Absence deleted.".to_string() } else { format!("{} absences deleted.", ids.len()) });
        onclose.call(());
    };
    let period = format!("{} – {}", show(&prefs, &from()), show(&prefs, &to()));
    let chosen = with().len();

    rsx! {
        if family_step() {
            Dialog {
                open: true,
                onclose: move |_| onclose.call(()),
                title: "Is the family away too?",
                description: "{who} is away {period}. Choose the family members who are away too.",
                actions: rsx! {
                    ActionButton { action: Action::Back, onclick: move |_| family_step.set(false) }
                    ActionButton { action: Action::Save, variant: ButtonVariant::Secondary, label: "Only {who}", onclick: move |_| save(false) }
                    ActionButton {
                        action: Action::Save,
                        label: if chosen == 0 { "Continue".to_string() } else { format!("Save for {} people", chosen + 1) },
                        onclick: move |_| save(true),
                    }
                },
                PickList {
                    rows: family_rows,
                    selected: with(),
                    on_toggle: move |k: String| {
                        let mut w = with.write();
                        if !w.remove(&k) {
                            w.insert(k);
                        }
                    },
                }
            }
        } else {
            Dialog {
                open: true,
                onclose: move |_| onclose.call(()),
                title: if existing.is_some() { "Absence" } else { "Add absence" },
                description: "From the first to the last day (both included), for one person or the whole congregation.",
                actions: rsx! {
                    EditActions {
                        on_save: move |_| save(false),
                        on_cancel: move |_| onclose.call(()),
                        on_delete: existing.is_some().then_some(EventHandler::new(move |_| { also.set(BTreeSet::new()); confirm.set(true); })),
                    }
                },
                Stack {
                    Field { label: "Who", required: true, error: err("person"),
                        Select { value: person(), options, onchange: move |v| { person.set(v); errors.set(Vec::new()); } }
                    }
                    Grid { cols: 2,
                        Field { label: "First day", required: true, error: err("from"),
                            TextInput { kind: InputKind::Date, value: from(), oninput: move |v| from.set(v) }
                        }
                        Field { label: "Last day", required: true, error: err("to"),
                            TextInput { kind: InputKind::Date, value: to(), oninput: move |v| to.set(v) }
                        }
                    }
                    Field { label: "Notes", help: "Optional. Only shown here, not in the list.",
                        TextArea { value: notes(), rows: 3, oninput: move |v| notes.set(v) }
                    }
                }
            }
        }
        if has_matches {
            Dialog {
                open: confirm(),
                onclose: move |_| confirm.set(false),
                title: "Delete this absence?",
                description: "These family members have the same absence. Choose whose absence to delete too.",
                actions: rsx! {
                    ActionButton { action: Action::Cancel, onclick: move |_| confirm.set(false) }
                    ActionButton { action: Action::Delete, onclick: move |_| delete() }
                },
                PickList {
                    rows: match_rows,
                    selected: also(),
                    on_toggle: move |k: String| {
                        let mut a = also.write();
                        if !a.remove(&k) {
                            a.insert(k);
                        }
                    },
                }
            }
        } else {
            ConfirmDialog {
                open: confirm(),
                title: "Delete this absence?",
                message: "Schedules can assign them again on these days.",
                confirm: Action::Delete,
                oncancel: move |_| confirm.set(false),
                onconfirm: move |_| delete(),
            }
        }
    }
}
