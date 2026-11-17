//! *Users › Field service groups*: the list, and the group page (new and edit).
//! Same patterns as *All users* (R-436): card list + filter bar, then a page with the form on the
//! left and related records on the right. Behaviour: `docs/model/m09-field-service-groups.md`.

use super::users::{save_export, ExportMenu};
use crate::export::{Format, Table};
use crate::people::{self, Directory, GroupDraft, GroupRole, Person};
use crate::prefs::Preferences;
use crate::state::use_app_state;
use crate::Route;
use dioxus::prelude::*;
use std::collections::BTreeSet;
use theo_ui::*;
use theo_ui::Action; // explicit: dioxus::prelude also exports an `Action`

fn role_label(r: GroupRole) -> &'static str {
    match r {
        GroupRole::Overseer => "Overseer",
        GroupRole::Assistant => "Assistant",
        GroupRole::Member => "Member",
    }
}

fn leaders_line(dir: &Directory, id: u32, prefs: &Preferences) -> String {
    let name = |n: Option<u32>| n.and_then(|n| dir.person(n)).map(|p| p.display_name(prefs));
    let Some(g) = dir.group(id) else { return String::new() };
    match (name(g.overseer), name(g.assistant)) {
        (Some(o), Some(a)) => format!("{o} · {a}"),
        (Some(o), None) => o,
        (None, Some(a)) => format!("Assistant: {a}"),
        (None, None) => "No overseer or assistant yet".into(),
    }
}

/// "Regular pioneer · Irregular" under a member's name.
fn member_subtitle(p: &Person) -> String {
    match p.activity() {
        Some(a @ (people::Activity::Irregular | people::Activity::Inactive)) => format!("{} · {}", p.publisher_type.label(), a.label()),
        _ => p.publisher_type.label().to_string(),
    }
}

/// Group, name, role, mobile phone, e-mail: one row per person, groups in name order.
fn export_table(dir: &Directory, groups: &[u32], prefs: &Preferences, congregation: &str) -> Table {
    let mut rows = Vec::new();
    for gid in groups {
        let Some(g) = dir.group(*gid) else { continue };
        for (n, role) in dir.group_members(*gid, prefs) {
            if let Some(p) = dir.person(n) {
                rows.push(vec![
                    g.name.clone(),
                    p.display_name(prefs),
                    role_label(role).to_string(),
                    p.mobile_phone.clone().unwrap_or_default(),
                    p.email.clone().unwrap_or_default(),
                ]);
            }
        }
    }
    Table {
        title: format!("{congregation} · Field service groups"),
        subtitle: format!("{} groups · {} people", groups.len(), rows.len()),
        columns: ["Group", "Name", "Role", "Mobile phone", "E-mail"].iter().map(|c| c.to_string()).collect(),
        widths: vec![1.2, 2.2, 1.1, 1.7, 2.6],
        rows,
    }
}

// ---------------------------------------------------------------------------------------------
// List
// ---------------------------------------------------------------------------------------------

/// `/app/users/groups`
#[component]
pub fn GroupList() -> Element {
    let mut state = use_app_state();
    let navigator = navigator();
    let toaster = use_toaster();
    let prefs = state.preferences();
    let dir = state.directory();
    let congregation = state.active_congregation().map(|c| c.name).unwrap_or_default();
    let mut query = use_signal(String::new);

    let tokens: Vec<String> = people::fold(&query()).split_whitespace().map(str::to_string).collect();
    let all = dir.groups_sorted();
    let shown: Vec<u32> = all
        .iter()
        .filter(|g| {
            let names: Vec<String> = dir.group_members(g.id, &prefs).iter().map(|(n, _)| dir.person(*n).map(|p| p.display_name(&prefs)).unwrap_or_default()).collect();
            let hay = people::fold(&format!("{} {}", g.name, names.join(" ")));
            tokens.iter().all(|t| hay.contains(t.as_str()))
        })
        .map(|g| g.id)
        .collect();
    let cards: Vec<(u32, String, String, Vec<Tag>)> = shown
        .iter()
        .filter_map(|id| dir.group(*id))
        .map(|g| {
            let count = dir.group_members(g.id, &prefs).len();
            let mut tags = vec![Tag::new(format!("{count} {}", if count == 1 { "member" } else { "members" }), Tone::Neutral).with_icon(Icon::Users)];
            if g.overseer.is_none() {
                tags.push(Tag::new("No overseer", Tone::Warning).with_icon(Icon::Warning));
            }
            if g.assistant.is_none() {
                tags.push(Tag::new("No assistant", Tone::Warning).with_icon(Icon::Warning));
            }
            let a = dir.group_activity(g.id);
            if a.irregular + a.inactive > 0 {
                tags.push(Tag::new(format!("{} irregular or inactive", a.irregular + a.inactive), Tone::Warning));
            }
            (g.id, g.name.clone(), leaders_line(&dir, g.id, &prefs), tags)
        })
        .collect();
    let unassigned = dir.unassigned_publishers().len();
    let total = all.len();

    // The export always has every group, whatever the search (decided 2026-11-17).
    let on_export = {
        let (dir, prefs, congregation) = (dir.clone(), prefs.clone(), congregation.clone());
        let every: Vec<u32> = all.iter().map(|g| g.id).collect();
        move |id: String| {
            let Some(format) = Format::parse(&id) else { return };
            if every.is_empty() {
                toaster.error("There are no groups to export yet.");
                return;
            }
            save_export(&export_table(&dir, &every, &prefs, &congregation), format, &congregation, "groups", toaster);
        }
    };

    rsx! {
        Page {
            title: "Field service groups",
            subtitle: "{congregation} · Groups, overseers and assistants",
            actions: rsx! {
                ExportMenu { csv_description: "All groups, one row per person", onselect: on_export }
                ActionButton { action: Action::Add, label: "Add group", onclick: move |_| { navigator.push(Route::GroupNew {}); } }
            },
            if unassigned > 0 {
                Alert { tone: Tone::Warning, title: if unassigned == 1 { "1 publisher is not in a group".to_string() } else { format!("{unassigned} publishers are not in a group") },
                    Stack { gap: Gap::Sm,
                        Text { "Every publisher should belong to a field service group." }
                        Cluster {
                            ActionButton {
                                action: Action::View,
                                label: "Show them",
                                onclick: move |_| {
                                    let mut sel = people::default_filters();
                                    sel.insert(people::FACET_GROUP.into(), std::iter::once("none".to_string()).collect());
                                    sel.insert(people::FACET_TYPE.into(), people::PublisherType::ALL.iter().filter(|t| t.is_publisher()).map(|t| t.value().to_string()).collect());
                                    state.user_query.set(String::new());
                                    state.user_filters.set(sel);
                                    navigator.push(Route::UserList {});
                                },
                            }
                        }
                    }
                }
            }
            if total == 0 {
                EmptyState {
                    icon: Icon::Group,
                    title: "No field service groups yet",
                    description: "Create the groups of {congregation}, then add their publishers.",
                    action: rsx! { ActionButton { action: Action::Add, label: "Add group", onclick: move |_| { navigator.push(Route::GroupNew {}); } } },
                }
            } else {
                FilterBar {
                    query: query(),
                    on_query: move |v| query.set(v),
                    search_placeholder: "Search by group or member name",
                    facets: Vec::new(),
                    selected: FilterSelection::new(),
                    on_select: move |_| {},
                    summary: format!("{} of {total} groups", cards.len()),
                }
                if cards.is_empty() {
                    EmptyState { icon: Icon::Search, title: "No groups match", description: "Try another search." }
                } else {
                    Grid { cols: 3, gap: Gap::Sm,
                        for (id, name, subtitle, tags) in cards {
                            RecordCard {
                                key: "{id}",
                                title: name.clone(),
                                avatar: name,
                                subtitle,
                                tags,
                                onclick: move |_| { navigator.push(Route::GroupDetail { id }); },
                            }
                        }
                    }
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------------------------
// Group page
// ---------------------------------------------------------------------------------------------

/// `/app/users/groups/new`
#[component]
pub fn GroupNew() -> Element {
    rsx! { GroupEditor { id: None } }
}

/// `/app/users/groups/:id`
#[component]
pub fn GroupDetail(id: u32) -> Element {
    rsx! { GroupEditor { key: "{id}", id: Some(id) } }
}

#[component]
fn GroupEditor(id: Option<u32>) -> Element {
    let mut state = use_app_state();
    let navigator = navigator();
    let toaster = use_toaster();
    let prefs = state.preferences();
    let dir = state.directory();
    let existing = id.and_then(|n| dir.group(n).cloned());

    let mut draft = use_signal(|| existing.as_ref().map(GroupDraft::from_group).unwrap_or_default());
    let mut errors = use_signal(Vec::<(&'static str, String)>::new);
    let mut confirm_delete = use_signal(|| false);
    let mut adding = use_signal(|| false);

    let back = ("Field service groups".to_string(), EventHandler::new(move |_| { navigator.push(Route::GroupList {}); }));
    if id.is_some() && existing.is_none() {
        return rsx! {
            Page { title: "Group not found", back,
                EmptyState { icon: Icon::Search, title: "This group does not exist", description: "It may have been deleted." }
            }
        };
    }

    let d = draft();
    let err = move |f: &str| errors.read().iter().find(|(k, _)| *k == f).map(|(_, m)| m.clone());
    let warnings = dir.group_warnings(id, &d, &prefs);
    let members = id.map(|g| dir.group_members(g, &prefs)).unwrap_or_default();
    let title = existing.as_ref().map(|g| g.name.clone()).unwrap_or_else(|| "New group".into());
    let subtitle = if existing.is_some() {
        format!("{} {}", members.len(), if members.len() == 1 { "member" } else { "members" })
    } else {
        "A group of publishers with an overseer and an assistant.".to_string()
    };

    // Overseer and assistant: baptised brothers, in sections (elders, ministerial servants, others).
    let sections = dir.leader_candidates(&prefs);
    let leader_options = |none: &str| {
        let mut o = vec![SelectOption::new("", none)];
        for (heading, list) in &sections {
            o.extend(list.iter().map(|p| SelectOption::new(p.id.to_string(), p.display_name(&prefs)).in_group(*heading)));
        }
        o
    };
    let overseer_options = leader_options("No overseer");
    let assistant_options = leader_options("No assistant");

    let rows: Vec<Row> = members
        .iter()
        .filter_map(|(n, role)| {
            let p = dir.person(*n)?;
            let row = Row::new(n.to_string(), p.display_name(&prefs)).with_subtitle(member_subtitle(p));
            Some(match role {
                GroupRole::Overseer => row.with_tag(Tag::new("Overseer", Tone::Info)),
                GroupRole::Assistant => row.with_tag(Tag::new("Assistant", Tone::Info)),
                GroupRole::Member => row.with_action(),
            })
        })
        .collect();
    let activity = id.map(|g| dir.group_activity(g)).unwrap_or_default();
    let delete_name = title.clone();
    let delete_consequences = if members.is_empty() {
        Vec::new()
    } else {
        vec![format!("{} {} will have no field service group, unless they lead another group.", members.len(), if members.len() == 1 { "person" } else { "people" })]
    };

    let save = move |_| {
        let d = draft();
        match state.update_directory(|dir| dir.save_group(id, &d)) {
            Ok(g) => {
                errors.set(Vec::new());
                if id.is_none() {
                    toaster.success("Group added. Now add its members.");
                    navigator.replace(Route::GroupDetail { id: g });
                } else {
                    toaster.success("Changes saved.");
                    if let Some(g) = state.directory().group(g) {
                        draft.set(GroupDraft::from_group(g));
                    }
                }
            }
            Err(e) => {
                errors.set(e);
                toaster.error("Check the fields marked in red.");
            }
        }
    };

    rsx! {
        Page {
            title,
            subtitle,
            back,
            actions: rsx! {
                if id.is_some() {
                    ActionButton { action: Action::Delete, onclick: move |_| confirm_delete.set(true) }
                }
            },
            Split {
                main: rsx! {
                    Form {
                        onsubmit: save,
                        actions: rsx! {
                            ActionButton { action: Action::Cancel, onclick: move |_| { navigator.push(Route::GroupList {}); } }
                            ActionButton { action: Action::Save, kind: ButtonKind::Submit }
                        },
                        Card { title: "Group", icon: Icon::Group,
                            Field { label: "Name", required: true, error: err("name"), help: "For example the area or the place where the group meets.",
                                TextInput { value: d.name.clone(), autocomplete: "off", invalid: err("name").is_some(), oninput: move |v| draft.write().name = v }
                            }
                            Grid { cols: 2,
                                Field { label: "Group overseer", error: err("overseer"),
                                    Select { value: d.overseer.clone(), invalid: err("overseer").is_some(), options: overseer_options, onchange: move |v| draft.write().overseer = v }
                                }
                                Field { label: "Group assistant", error: err("assistant"),
                                    Select { value: d.assistant.clone(), invalid: err("assistant").is_some(), options: assistant_options, onchange: move |v| draft.write().assistant = v }
                                }
                            }
                            Text { tone: TextTone::Small, "Only baptised brothers. Elders and ministerial servants are listed first. The overseer and the assistant are always members of the group." }
                        }
                        if !warnings.is_empty() {
                            Alert { tone: Tone::Warning, title: "Please check",
                                Stack { gap: Gap::Xs,
                                    for (i, w) in warnings.iter().enumerate() {
                                        Text { key: "{i}", "{w}" }
                                    }
                                }
                            }
                        }
                    }
                },
                side: rsx! {
                    if let Some(gid) = id {
                        Card {
                            title: "Members",
                            icon: Icon::Users,
                            actions: rsx! { Badge { tone: Tone::Neutral, "{members.len()}" } },
                            if rows.is_empty() {
                                Text { tone: TextTone::Muted, "No members yet." }
                            } else {
                                RowList {
                                    rows,
                                    on_open: move |n: String| {
                                        if let Ok(n) = n.parse() {
                                            navigator.push(Route::UserDetail { id: n });
                                        }
                                    },
                                    action: (Icon::Close, "Remove from group".to_string()),
                                    on_action: move |n: String| {
                                        if let Ok(n) = n.parse::<u32>() {
                                            state.update_directory(|d| d.remove_member(gid, n));
                                            toaster.success("Removed from the group.");
                                        }
                                    },
                                }
                            }
                            ActionButton { action: Action::AddMembers, variant: ButtonVariant::Secondary, onclick: move |_| adding.set(true) }
                        }
                        Card { title: "Activity", icon: Icon::Report,
                            ItemList {
                                items: vec![
                                    ListEntry { icon: Icon::Success, title: "Active".into(), subtitle: "A report every month".into(), meta: activity.active.to_string() },
                                    ListEntry { icon: Icon::Warning, title: "Irregular".into(), subtitle: "Some months without a report".into(), meta: activity.irregular.to_string() },
                                    ListEntry { icon: Icon::Error, title: "Inactive".into(), subtitle: "No report in 6 months".into(), meta: activity.inactive.to_string() },
                                ],
                            }
                            Text { tone: TextTone::Small, "Field service reports of the last 6 complete months." }
                        }
                    } else {
                        Card { title: "After saving", icon: Icon::Info,
                            Text { tone: TextTone::Muted, "You can add the members of the group." }
                        }
                    }
                },
            }
        }
        if let Some(gid) = id {
            AddMembersDialog { open: adding(), group: gid, onclose: move |_| adding.set(false) }
        }
        ConfirmDialog {
            open: confirm_delete(),
            title: "Delete {delete_name}?",
            message: "The group is removed. Its members and their data stay.",
            confirm: Action::Delete,
            consequences: delete_consequences,
            oncancel: move |_| confirm_delete.set(false),
            onconfirm: move |_| {
                if let Some(gid) = id {
                    state.update_directory(|d| d.delete_group(gid));
                    toaster.success(format!("{delete_name} was deleted."));
                    navigator.replace(Route::GroupList {});
                }
            },
        }
    }
}

/// Choose publishers to add. People without a group come first; people in another group move.
#[component]
fn AddMembersDialog(open: bool, group: u32, onclose: EventHandler<()>) -> Element {
    let mut state = use_app_state();
    let toaster = use_toaster();
    let prefs = state.preferences();
    let dir = state.directory();
    let mut query = use_signal(String::new);
    let mut chosen = use_signal(BTreeSet::<String>::new);

    let tokens: Vec<String> = people::fold(&query()).split_whitespace().map(str::to_string).collect();
    let mut candidates: Vec<(&Person, Option<String>)> = dir
        .current_publishers()
        .into_iter()
        .filter(|p| dir.groups_of(p.id).is_empty())
        .filter(|p| {
            let hay = people::fold(&format!("{} {}", p.first_name, p.last_name));
            tokens.iter().all(|t| hay.contains(t.as_str()))
        })
        .map(|p| {
            // Where they are now: "in North", "leads North", or nothing.
            let now: Vec<String> = dir
                .groups_of(p.id)
                .into_iter()
                .map(|(g, r)| if r == GroupRole::Member { format!("now in {}", g.name) } else { format!("leads {}", g.name) })
                .collect();
            (p, (!now.is_empty()).then(|| now.join(", ")))
        })
        .collect();
    candidates.sort_by_cached_key(|(p, g)| (g.is_some(), people::fold(&p.display_name(&prefs))));
    let rows: Vec<Row> = candidates
        .iter()
        .map(|(p, g)| {
            Row::new(p.id.to_string(), p.display_name(&prefs)).with_subtitle(match g {
                Some(g) => format!("{} · {g}", p.publisher_type.label()),
                None => format!("{} · no group", p.publisher_type.label()),
            })
        })
        .collect();
    let count = chosen.read().len();
    // Signals are Copy: copies inside keep this closure `Fn`, so every handler can call it.
    let close = move || {
        let (mut c, mut q) = (chosen, query);
        c.set(BTreeSet::new());
        q.set(String::new());
        onclose.call(());
    };

    rsx! {
        Dialog {
            open,
            onclose: move |_| close(),
            title: "Add members",
            description: "Only publishers without a group are listed. To move someone, remove them from their group first.",
            actions: rsx! {
                ActionButton { action: Action::Cancel, onclick: move |_| close() }
                ActionButton {
                    action: Action::AddMembers,
                    label: if count == 0 { "Add members".to_string() } else if count == 1 { "Add 1 member".to_string() } else { format!("Add {count} members") },
                    disabled: count == 0,
                    onclick: move |_| {
                        let ids: Vec<u32> = chosen.read().iter().filter_map(|s| s.parse().ok()).collect();
                        state.update_directory(|d| d.add_members(group, &ids));
                        toaster.success(if ids.len() == 1 { "1 member added.".to_string() } else { format!("{} members added.", ids.len()) });
                        close();
                    },
                }
            },
            Stack {
                TextInput { kind: InputKind::Search, value: query(), icon: Icon::Search, placeholder: "Search by name", oninput: move |v| query.set(v) }
                if rows.is_empty() {
                    Text { tone: TextTone::Muted, if tokens.is_empty() { "Every publisher is already in a group." } else { "Nobody without a group matches this search." } }
                } else {
                    PickList {
                        rows,
                        selected: chosen(),
                        on_toggle: move |id: String| {
                            let mut c = chosen.write();
                            if !c.remove(&id) {
                                c.insert(id);
                            }
                        },
                    }
                }
            }
        }
    }
}
