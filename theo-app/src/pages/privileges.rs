//! *Users › Privileges*: who may take part in what. Two views of the same data: by person (cards,
//! each opens the privileges of that person) and by privilege (one privilege, tick who has it).
//! Rules: `docs/model/m01-people.md`, "Privileges".

use crate::people::{self, Directory, Person};
use crate::privileges::{Privilege, Section};
use crate::state::use_app_state;
use dioxus::prelude::*;
use std::collections::BTreeSet;
use theo_ui::*;
use theo_ui::Action; // explicit: dioxus::prelude also exports an `Action`

const F_PRIVILEGE: &str = "privilege";
const PEOPLE_FACETS: [&str; 5] = [people::FACET_LABELS, people::FACET_TYPE, people::FACET_APPOINTMENT, people::FACET_GENDER, people::FACET_GROUP];

/// The "Privilege" filter: people who have any of the chosen privileges (or none at all).
fn privilege_facet<'a>() -> people::ExtraFacet<'a> {
    let mut opts: Vec<(String, String)> = Privilege::all().map(|p| (p.value().to_string(), p.full_label())).collect();
    opts.push(("none".into(), "No privileges".into()));
    people::ExtraFacet {
        def: (F_PRIVILEGE, "Privilege", Icon::Privilege, opts),
        values: Box::new(|p: &Person| {
            if p.privileges.is_empty() { vec!["none".into()] } else { p.privileges.iter().map(|x| x.value().to_string()).collect() }
        }),
    }
}

/// `/app/users/privileges`
#[component]
pub fn PrivilegesPage() -> Element {
    let state = use_app_state();
    let navigator = navigator();
    let prefs = state.preferences();
    let dir = state.directory();
    let congregation = state.active_congregation().map(|c| c.name).unwrap_or_default();
    let mut view = use_signal(|| "person".to_string());
    let mut query = use_signal(String::new);
    let mut sel = use_signal(FilterSelection::new);
    let mut editing = use_signal(|| None::<u32>);
    let mut chosen = use_signal(|| Privilege::Prayer.value().to_string());

    let pool: Vec<&Person> = dir.people.iter().filter(|p| p.moved_away.is_none()).collect();
    let extra = [privilege_facet()];
    let facets = people::facets_for(&dir, &pool, &query(), &sel(), people::standard_facets(&dir, &PEOPLE_FACETS), &extra);
    let shown: Vec<&Person> = people::filter_pool(&dir, &pool, &query(), &sel(), &extra, &prefs);
    let summary = format!("{} of {} people", shown.len(), pool.len());

    rsx! {
        Page {
            title: "Privileges",
            subtitle: "{congregation} · What each person may take part in",
            StickyBar {
                Segmented {
                    value: view(),
                    options: vec![
                        Segment { value: "person".into(), label: "By person".into(), icon: Some(Icon::User) },
                        Segment { value: "privilege".into(), label: "By privilege".into(), icon: Some(Icon::Privilege) },
                    ],
                    onchange: move |v: String| view.set(v),
                }
                FilterBar {
                    query: query(),
                    on_query: move |q| query.set(q),
                    search_placeholder: "Search by name",
                    facets,
                    selected: sel(),
                    on_select: move |s| sel.set(s),
                    summary,
                }
            }
            if view() == "privilege" {
                ByPrivilege { chosen: chosen(), people: shown.iter().map(|p| p.id).collect::<Vec<u32>>(), on_choose: move |v| chosen.set(v) }
            } else if shown.is_empty() {
                EmptyState { icon: Icon::Search, title: "Nobody matches", description: "Try another search, or remove some filters." }
            } else {
                Grid { cols: 3, gap: Gap::Sm,
                    for p in shown.iter() {
                        {
                            let id = p.id;
                            let name = p.display_name(&prefs);
                            let subtitle = match p.appointment {
                                Some(a) => format!("{} · {}", p.publisher_type.label(), a.label()),
                                None => p.publisher_type.label().to_string(),
                            };
                            rsx! {
                                RecordCard {
                                    key: "{id}",
                                    title: name.clone(),
                                    avatar: name,
                                    subtitle,
                                    tags: privilege_tags(p),
                                    onclick: move |_| editing.set(Some(id)),
                                }
                            }
                        }
                    }
                }
            }
            if view() == "person" {
                Text { tone: TextTone::Small, "To open a person's page instead, use Users › All users." }
                Cluster {
                    ActionButton { action: Action::View, label: "All users", variant: ButtonVariant::Ghost, onclick: move |_| { navigator.push(crate::Route::UserList {}); } }
                }
            }
        }
        if let Some(person) = editing() {
            PrivilegesDialog { key: "{person}", person, onclose: move |_| editing.set(None) }
        }
    }
}

/// Up to 4 privileges as labels, then "+N".
fn privilege_tags(p: &Person) -> Vec<Tag> {
    if p.privileges.is_empty() {
        return vec![Tag::new("No privileges", Tone::Neutral)];
    }
    let mut tags: Vec<Tag> = p.privileges.iter().take(4).map(|x| Tag::new(x.full_label(), Tone::Neutral)).collect();
    if p.privileges.len() > 4 {
        tags.push(Tag::new(format!("+{}", p.privileges.len() - 4), Tone::Info));
    }
    tags
}

/// One privilege, and everyone (within the filters) who may have it, with a checkbox each.
#[component]
fn ByPrivilege(chosen: String, people: Vec<u32>, on_choose: EventHandler<String>) -> Element {
    let mut state = use_app_state();
    let prefs = state.preferences();
    let dir = state.directory();
    let Some(privilege) = Privilege::parse(&chosen) else { return rsx! {} };
    let options: Vec<SelectOption> = Privilege::all().map(|p| SelectOption::new(p.value(), p.label()).in_group(p.section().label())).collect();
    let eligible: Vec<&Person> = people.iter().filter_map(|n| dir.person(*n)).filter(|p| privilege.allowed_for(p.publisher_type, p.gender)).collect();
    let holders: BTreeSet<String> = eligible.iter().filter(|p| p.privileges.contains(&privilege)).map(|p| p.id.to_string()).collect();
    let rows: Vec<Row> = eligible
        .iter()
        .map(|p| {
            let sub = match p.appointment {
                Some(a) => format!("{} · {}", p.publisher_type.label(), a.label()),
                None => p.publisher_type.label().to_string(),
            };
            Row::new(p.id.to_string(), p.display_name(&prefs)).with_subtitle(sub)
        })
        .collect();
    let hidden = people.len() - eligible.len();
    rsx! {
        Card { title: "Who has this privilege", icon: Icon::Privilege,
            Grid { cols: 2,
                Field { label: "Privilege",
                    Select { value: chosen.clone(), options, onchange: move |v| on_choose.call(v) }
                }
            }
            Text { tone: TextTone::Muted,
                "Who may have it: {privilege.rule().label()}. {holders.len()} of {eligible.len()} have it."
            }
            if rows.is_empty() {
                EmptyState { icon: Icon::Users, title: "Nobody here may have this privilege", description: "Change the filters, or choose another privilege." }
            } else {
                PickList {
                    rows,
                    selected: holders.clone(),
                    on_toggle: move |id: String| {
                        if let Ok(n) = id.parse::<u32>() {
                            let on = !holders.contains(&id);
                            state.update_directory(|d| d.set_privilege(n, privilege, on));
                        }
                    },
                }
            }
            if hidden > 0 {
                Text { tone: TextTone::Small, "{hidden} people in the list may not have this privilege, so they are not shown." }
            }
        }
    }
}

/// All privileges of one person, by section; only the ones they may have. Changes apply at once.
#[component]
pub(super) fn PrivilegesDialog(person: u32, onclose: EventHandler<()>) -> Element {
    let mut state = use_app_state();
    let prefs = state.preferences();
    let dir = state.directory();
    let Some(p) = dir.person(person).cloned() else { return rsx! {} };
    let sections: Vec<(Section, Vec<Privilege>)> = Section::ALL
        .iter()
        .map(|s| (*s, Privilege::all().filter(|x| x.section() == *s && x.allowed_for(p.publisher_type, p.gender)).collect::<Vec<_>>()))
        .filter(|(_, l)| !l.is_empty())
        .collect();
    let not_allowed = Privilege::all().filter(|x| !x.allowed_for(p.publisher_type, p.gender)).count();
    let note = if not_allowed == 0 {
        String::new()
    } else {
        format!("{not_allowed} privileges are not shown: {} may not have them.", p.publisher_type.label().to_lowercase())
    };
    rsx! {
        Dialog {
            open: true,
            onclose: move |_| onclose.call(()),
            title: "Privileges · {p.display_name(&prefs)}",
            description: "Tap to give or take a privilege. Changes are saved at once.",
            actions: rsx! { ActionButton { action: Action::Done, onclick: move |_| onclose.call(()) } },
            Stack { gap: Gap::Lg,
                if sections.is_empty() {
                    Text { tone: TextTone::Muted, "Visitors have no privileges." }
                }
                for (section, list) in sections {
                    Stack { key: "{section.label()}", gap: Gap::Sm,
                        Text { tone: TextTone::Strong, "{section.label()}" }
                        Cluster { gap: Gap::Sm,
                            for x in list {
                                ChipToggle {
                                    key: "{x.value()}",
                                    label: x.label(),
                                    selected: p.privileges.contains(&x),
                                    onclick: move |_| {
                                        let on = !state.directory().person(person).is_some_and(|q| q.privileges.contains(&x));
                                        state.update_directory(|d| d.set_privilege(person, x, on));
                                    },
                                }
                            }
                        }
                    }
                }
                if !note.is_empty() {
                    Text { tone: TextTone::Small, "{note}" }
                }
            }
        }
    }
}

/// The privileges card of the person page.
#[component]
pub(super) fn PersonPrivilegesCard(person: u32) -> Element {
    let state = use_app_state();
    let dir: Directory = state.directory();
    let mut editing = use_signal(|| false);
    let Some(p) = dir.person(person).cloned() else { return rsx! {} };
    let count = p.privileges.len();
    rsx! {
        Card { title: "Privileges", icon: Icon::Privilege, actions: rsx! { Badge { tone: Tone::Neutral, "{count}" } },
            if count == 0 {
                Text { tone: TextTone::Muted, "No privileges yet." }
            } else {
                Cluster { gap: Gap::Xs,
                    for x in p.privileges.iter() {
                        Badge { key: "{x.value()}", tone: Tone::Neutral, "{x.full_label()}" }
                    }
                }
            }
            ActionButton { action: Action::Edit, variant: ButtonVariant::Secondary, label: "Edit privileges", onclick: move |_| editing.set(true) }
        }
        if editing() {
            PrivilegesDialog { person, onclose: move |_| editing.set(false) }
        }
    }
}
