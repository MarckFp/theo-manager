//! *Users › All users › Families*: every family with search and filters, and the family page (head, members,
//! how they are related). Same patterns as *Field service groups* (R-436). Rules:
//! `docs/model/m01-people.md`, "Family".

use crate::people::{self, Directory, Family, Person, RelationKind};
use crate::prefs::Preferences;
use crate::state::use_app_state;
use crate::Route;
use dioxus::prelude::*;
use std::collections::BTreeSet;
use theo_ui::*;
use theo_ui::Action; // explicit: dioxus::prelude also exports an `Action`

const F_STATUS: &str = "status";
const F_SIZE: &str = "size";
const F_GROUP: &str = "group";

fn family_name(dir: &Directory, f: &Family, prefs: &Preferences) -> String {
    dir.person(f.head).map(|h| format!("Family of {}", h.display_name(prefs))).unwrap_or_else(|| "Family".into())
}

/// Members in order: the head first, then by name.
fn members_sorted<'a>(dir: &'a Directory, f: &Family, prefs: &Preferences) -> Vec<&'a Person> {
    let mut v: Vec<&Person> = f.members.iter().filter_map(|m| dir.person(*m)).collect();
    v.sort_by_cached_key(|p| (p.id != f.head, people::fold(&p.display_name(prefs))));
    v
}

fn values(dir: &Directory, f: &Family, facet: &str) -> Vec<String> {
    match facet {
        F_STATUS => vec![if dir.family_moved_away(f) { "moved-away" } else { "current" }.into()],
        F_SIZE => vec![match f.members.len() {
            2 => "2",
            3 => "3",
            4 => "4",
            _ => "5+",
        }
        .into()],
        F_GROUP => {
            let groups: BTreeSet<String> =
                f.members.iter().flat_map(|m| dir.groups_of(*m).into_iter().map(|(g, _)| g.id.to_string())).collect();
            if groups.is_empty() { vec!["none".into()] } else { groups.into_iter().collect() }
        }
        _ => Vec::new(),
    }
}

/// The list starts with the families that live here.
fn default_filters() -> FilterSelection {
    let mut s = FilterSelection::new();
    s.insert(F_STATUS.into(), std::iter::once("current".to_string()).collect());
    s
}

/// The Families tab of `/app/users/all`.
#[component]
pub(super) fn FamiliesTab() -> Element {
    let state = use_app_state();
    let navigator = navigator();
    let prefs = state.preferences();
    let dir = state.directory();
    let congregation = state.active_congregation().map(|c| c.name).unwrap_or_default();
    let mut query = use_signal(String::new);
    let mut sel = use_signal(default_filters);
    let mut adding = use_signal(|| false);

    let tokens: Vec<String> = people::fold(&query()).split_whitespace().map(str::to_string).collect();
    let matches = |f: &Family, skip: Option<&str>| {
        let names: Vec<String> = f.members.iter().filter_map(|m| dir.person(*m)).map(|p| format!("{} {}", p.first_name, p.last_name)).collect();
        let hay = people::fold(&names.join(" "));
        tokens.iter().all(|t| hay.contains(t.as_str()))
            && sel().iter().all(|(id, chosen)| Some(id.as_str()) == skip || chosen.is_empty() || values(&dir, f, id).iter().any(|v| chosen.contains(v)))
    };
    let mut groups: Vec<(String, String)> = dir.groups_sorted().iter().map(|g| (g.id.to_string(), g.name.clone())).collect();
    groups.push(("none".into(), "No group".into()));
    let v = |a: &str, b: &str| (a.to_string(), b.to_string());
    let defs: Vec<people::FacetDef> = vec![
        (F_STATUS, "Status", Icon::User, vec![v("current", "Current"), v("moved-away", "Moved away")]),
        (F_SIZE, "Members", Icon::Users, vec![v("2", "2"), v("3", "3"), v("4", "4"), v("5+", "5 or more")]),
        (F_GROUP, "Group", Icon::Group, groups),
    ];
    let facets: Vec<Facet> = defs
        .into_iter()
        .map(|(id, label, icon, opts)| Facet {
            id: id.into(),
            label: label.into(),
            icon,
            options: opts
                .into_iter()
                .map(|(value, label)| {
                    let count = dir.families.iter().filter(|f| matches(f, Some(id)) && values(&dir, f, id).contains(&value)).count();
                    FacetOption { value, label, count }
                })
                .collect(),
        })
        .collect();
    let mut shown: Vec<&Family> = dir.families.iter().filter(|f| matches(f, None)).collect();
    shown.sort_by_cached_key(|f| people::fold(&family_name(&dir, f, &prefs)));
    // (id, title, head name for the avatar, member names, tags)
    type CardData = (u32, String, String, Option<String>, Vec<Tag>);
    let cards: Vec<CardData> = shown
        .iter()
        .map(|f| {
            let head = dir.person(f.head).map(|h| h.display_name(&prefs)).unwrap_or_default();
            let names: Vec<String> = members_sorted(&dir, f, &prefs).iter().map(|p| p.first_name.clone()).collect();
            let mut tags = vec![Tag::new(format!("{} members", f.members.len()), Tone::Neutral).with_icon(Icon::Users)];
            if dir.family_moved_away(f) {
                tags.push(Tag::new("Moved away", Tone::Neutral).with_icon(Icon::MovedAway));
            }
            (f.id, family_name(&dir, f, &prefs), head, Some(names.join(", ")), tags)
        })
        .collect();
    let total = dir.families.len();

    rsx! {
        Page {
            title: "All users",
            subtitle: "{congregation} · Families, their head and how members are related",
            actions: rsx! {
                ActionButton { action: Action::Add, label: "Add family", onclick: move |_| adding.set(true) }
            },
            if total == 0 {
                StickyBar { super::users::UsersTabs {} }
                EmptyState {
                    icon: Icon::Family,
                    title: "No families yet",
                    description: "Add a family, or add a relative on a person's page.",
                    action: rsx! { ActionButton { action: Action::Add, label: "Add family", onclick: move |_| adding.set(true) } },
                }
            } else {
                StickyBar {
                    super::users::UsersTabs {}
                    FilterBar {
                        query: query(),
                        on_query: move |q| query.set(q),
                        search_placeholder: "Search by member name",
                        facets,
                        selected: sel(),
                        on_select: move |s| sel.set(s),
                        summary: format!("{} of {total} families", cards.len()),
                        defaults: default_filters(),
                    }
                }
                if cards.is_empty() {
                    EmptyState { icon: Icon::Search, title: "No families match", description: "Try another search, or remove some filters." }
                } else {
                    Grid { cols: 3, gap: Gap::Sm,
                        for (id, title, head, subtitle, tags) in cards {
                            RecordCard {
                                key: "{id}",
                                title,
                                avatar: head,
                                subtitle,
                                tags,
                                onclick: move |_| { navigator.push(Route::FamilyDetail { id }); },
                            }
                        }
                    }
                }
            }
        }
        if adding() {
            AddFamilyDialog { onclose: move |_| adding.set(false) }
        }
    }
}

/// Back to the Families tab of All users.
fn to_families(navigator: dioxus::router::Navigator, state: crate::state::AppState) {
    let mut tab = state.users_tab;
    tab.set("families".into());
    navigator.push(Route::UserList {});
}

fn person_options(people: &[&Person], prefs: &Preferences, empty: &str) -> Vec<SelectOption> {
    let mut list: Vec<&&Person> = people.iter().collect();
    list.sort_by_cached_key(|p| people::fold(&p.display_name(prefs)));
    let mut out = vec![SelectOption::new("", empty)];
    out.extend(list.into_iter().map(|p| SelectOption::new(p.id.to_string(), p.display_name(prefs))));
    out
}

fn kind_options() -> Vec<SelectOption> {
    RelationKind::ALL.iter().map(|k| SelectOption::new(k.value(), k.label())).collect()
}

/// A new family: the head, a second member and how they are related.
#[component]
fn AddFamilyDialog(onclose: EventHandler<()>) -> Element {
    let mut state = use_app_state();
    let navigator = navigator();
    let toaster = use_toaster();
    let prefs = state.preferences();
    let dir = state.directory();
    let mut head = use_signal(String::new);
    let mut other = use_signal(String::new);
    let mut kind = use_signal(|| RelationKind::Spouse.value().to_string());
    let mut error = use_signal(|| None::<String>);
    let free = dir.without_family();
    let add = move |_| {
        let (Ok(h), Ok(o), Some(k)) = (head().parse::<u32>(), other().parse::<u32>(), RelationKind::parse(&kind())) else {
            error.set(Some("Choose the head, another person and how they are related.".into()));
            return;
        };
        let prefs = state.preferences();
        match state.update_directory(|d| d.add_relationship(h, o, k, &prefs).map(|_| d.make_head(h))) {
            Ok(()) => {
                let id = state.directory().family_of(h).map(|f| f.id);
                toaster.success("Family added.");
                onclose.call(());
                if let Some(id) = id {
                    navigator.push(Route::FamilyDetail { id });
                }
            }
            Err(e) => error.set(Some(e)),
        }
    };
    rsx! {
        Dialog {
            open: true,
            onclose: move |_| onclose.call(()),
            title: "Add family",
            description: "Choose two people who are not in a family yet. You can add more members on the family page.",
            actions: rsx! {
                ActionButton { action: Action::Cancel, onclick: move |_| onclose.call(()) }
                ActionButton { action: Action::Add, label: "Add family", onclick: add }
            },
            Stack {
                Field { label: "Family head", required: true, help: "The main contact of the family.",
                    Select { value: head(), options: person_options(&free, &prefs, "Choose a person…"), onchange: move |v| head.set(v) }
                }
                Field { label: "Another member", required: true, error: error(),
                    Select { value: other(), options: person_options(&free, &prefs, "Choose a person…"), onchange: move |v| other.set(v) }
                }
                Field { label: "This member is the head's",
                    Select { value: kind(), options: kind_options(), onchange: move |v| kind.set(v) }
                }
            }
        }
    }
}

/// `/app/users/families/:id`
#[component]
pub fn FamilyDetail(id: u32) -> Element {
    rsx! { FamilyEditor { key: "{id}", id } }
}

#[component]
fn FamilyEditor(id: u32) -> Element {
    let mut state = use_app_state();
    let navigator = navigator();
    let toaster = use_toaster();
    let prefs = state.preferences();
    let dir = state.directory();
    let family = dir.family(id).cloned();
    let mut head = use_signal(|| family.as_ref().map(|f| f.head.to_string()).unwrap_or_default());
    let mut confirm_delete = use_signal(|| false);
    let mut adding = use_signal(|| false);
    let back = ("Families".to_string(), EventHandler::new(move |_| { to_families(navigator, state); }));
    let Some(f) = family else {
        return rsx! {
            Page { title: "Family not found", back,
                EmptyState { icon: Icon::Search, title: "This family does not exist", description: "It may have been removed." }
            }
        };
    };

    let title = family_name(&dir, &f, &prefs);
    let members = members_sorted(&dir, &f, &prefs);
    let relatives = dir.relatives(f.head);
    let rows: Vec<Row> = members
        .iter()
        .map(|p| {
            let row = Row::new(p.id.to_string(), p.display_name(&prefs));
            if p.id == f.head {
                row.with_subtitle(p.publisher_type.label()).with_tag(Tag::new("Family head", Tone::Info).with_icon(Icon::FamilyHead))
            } else {
                let rel = relatives.iter().find(|r| r.id == p.id).and_then(|r| r.kind).map(|k| k.label()).unwrap_or("Family member");
                row.with_subtitle(format!("{rel} · {}", p.publisher_type.label())).with_action()
            }
        })
        .collect();
    let head_options: Vec<SelectOption> = members.iter().map(|p| SelectOption::new(p.id.to_string(), p.display_name(&prefs))).collect();
    let moved = dir.family_moved_away(&f);
    let count = f.members.len();

    let save = move |_| {
        if let Ok(h) = head().parse::<u32>() {
            state.update_directory(|d| d.make_head(h));
            toaster.success("Family head saved.");
        }
    };

    rsx! {
        Page {
            title: title.clone(),
            subtitle: format!("{count} members"),
            back,
            actions: rsx! {
                ActionButton { action: Action::Delete, label: "Delete family", onclick: move |_| confirm_delete.set(true) }
            },
            if moved {
                Alert { tone: Tone::Info, title: "This family moved away", "All its members moved away. They are kept for 1 year." }
            }
            Split {
                main: rsx! {
                    Form {
                        onsubmit: save,
                        floating_save: true,
                        actions: rsx! {
                            ActionButton { action: Action::Cancel, onclick: move |_| { to_families(navigator, state); } }
                            ActionButton { action: Action::Save, kind: ButtonKind::Submit }
                        },
                        Card { title: "Family", icon: Icon::Family,
                            Field { label: "Family head", help: "The main contact of the family. The family is named after the head.",
                                Select { value: head(), options: head_options, onchange: move |v| head.set(v) }
                            }
                            Text { tone: TextTone::Small, "Relationships are shown from the head's point of view. To change a relationship, open the person and use their Family card." }
                        }
                    }
                },
                side: rsx! {
                    Card { title: "Members", icon: Icon::Users, actions: rsx! { Badge { tone: Tone::Neutral, "{count}" } },
                        RowList {
                            rows,
                            on_open: move |n: String| {
                                if let Ok(n) = n.parse() {
                                    navigator.push(Route::UserDetail { id: n });
                                }
                            },
                            action: (Icon::Close, "Remove from family".to_string()),
                            on_action: move |n: String| {
                                if let Ok(n) = n.parse::<u32>() {
                                    state.update_directory(|d| d.remove_from_family(n));
                                    if state.directory().family(id).is_none() {
                                        toaster.success("Removed. Only one member was left, so the family was removed.");
                                        to_families(navigator, state);
                                    } else {
                                        toaster.success("Removed from the family.");
                                    }
                                }
                            },
                        }
                        ActionButton { action: Action::AddMembers, variant: ButtonVariant::Secondary, onclick: move |_| adding.set(true) }
                    }
                },
            }
        }
        if adding() {
            AddMemberDialog { family: id, onclose: move |_| adding.set(false) }
        }
        ConfirmDialog {
            open: confirm_delete(),
            title: "Delete {title}?",
            message: "The family is removed. Its members and their data stay; they just have no family.",
            confirm: Action::Delete,
            confirm_label: "Delete family",
            oncancel: move |_| confirm_delete.set(false),
            onconfirm: move |_| {
                state.update_directory(|d| d.delete_family(id));
                toaster.success("Family deleted.");
                to_families(navigator, state);
            },
        }
    }
}

/// Add someone without a family, with their relationship to the head.
#[component]
fn AddMemberDialog(family: u32, onclose: EventHandler<()>) -> Element {
    let mut state = use_app_state();
    let toaster = use_toaster();
    let prefs = state.preferences();
    let dir = state.directory();
    let mut person = use_signal(String::new);
    let mut kind = use_signal(|| RelationKind::Son.value().to_string());
    let mut error = use_signal(|| None::<String>);
    let head = dir.family(family).map(|f| f.head).unwrap_or(0);
    let head_name = dir.person(head).map(|p| p.display_name(&prefs)).unwrap_or_default();
    let free = dir.without_family();
    let add = move |_| {
        let (Ok(p), Some(k)) = (person().parse::<u32>(), RelationKind::parse(&kind())) else {
            error.set(Some("Choose a person.".into()));
            return;
        };
        let prefs = state.preferences();
        match state.update_directory(|d| d.add_relationship(head, p, k, &prefs)) {
            Ok(()) => {
                toaster.success("Member added.");
                onclose.call(());
            }
            Err(e) => error.set(Some(e)),
        }
    };
    rsx! {
        Dialog {
            open: true,
            onclose: move |_| onclose.call(()),
            title: "Add member",
            description: "Only people who are not in a family are listed.",
            actions: rsx! {
                ActionButton { action: Action::Cancel, onclick: move |_| onclose.call(()) }
                ActionButton { action: Action::AddMembers, label: "Add member", onclick: add }
            },
            Stack {
                Field { label: "Person", required: true, error: error(),
                    Select { value: person(), options: person_options(&free, &prefs, "Choose a person…"), onchange: move |v| person.set(v) }
                }
                Field { label: "This person is {head_name}'s",
                    Select { value: kind(), options: kind_options(), onchange: move |v| kind.set(v) }
                }
            }
        }
    }
}
