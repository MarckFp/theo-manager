//! *Users › Permissions*: what each account may see and change in the app. Only people who joined
//! the app (a linked account) are listed. Rules: `docs/model/m02-users-and-access.md`.

use crate::access::{preset, Access, Level, ADMIN_ONLY_PAGES, PRESETS};
use crate::people::{self, AccountLink, Person};
use crate::state::{use_app_state, StorageMode};
use crate::{nav, Route};
use dioxus::prelude::*;
use theo_ui::*;
use theo_ui::Action; // explicit: dioxus::prelude also exports an `Action`

fn username(p: &Person) -> String {
    match &p.account {
        AccountLink::Linked { username } => format!("@{username}"),
        _ => String::new(),
    }
}

/// `/app/users/permissions`
#[component]
pub fn PermissionsPage() -> Element {
    let state = use_app_state();
    let navigator = navigator();
    let prefs = state.preferences();
    let dir = state.directory();
    let congregation = state.active_congregation();
    let local = congregation.as_ref().is_some_and(|c| c.mode == StorageMode::Local);
    let name = congregation.map(|c| c.name).unwrap_or_default();
    let mut query = use_signal(String::new);

    let tokens: Vec<String> = people::fold(&query()).split_whitespace().map(str::to_string).collect();
    let mut accounts: Vec<&Person> = dir.accounts();
    accounts.sort_by_cached_key(|p| (!dir.is_owner(p.id), people::fold(&p.display_name(&prefs))));
    let total = accounts.len();
    let shown: Vec<&Person> = accounts
        .into_iter()
        .filter(|p| {
            let hay = people::fold(&format!("{} {} {}", p.first_name, p.last_name, username(p)));
            tokens.iter().all(|t| hay.contains(t.as_str()))
        })
        .collect();
    let invited = dir.people.iter().filter(|p| matches!(p.account, AccountLink::Invited { .. })).count();
    let without = dir.people.iter().filter(|p| p.moved_away.is_none() && p.account == AccountLink::None).count();

    rsx! {
        Page {
            title: "Permissions",
            subtitle: "{name} · What each account may see and change in the app",
            if local {
                Alert { tone: Tone::Info, title: "Local congregation",
                    "Only you use this congregation, on this device, and you have every permission. Permissions are for Cloud congregations, where other people join with an invite. In the demo, open Hillcrest to try them."
                }
            } else {
                Text { tone: TextTone::Muted,
                    "Only people who joined the app are listed. Not listed: {invited} invited (permissions after they join) and {without} without an account."
                }
                StickyBar {
                    FilterBar {
                        query: query(),
                        on_query: move |q| query.set(q),
                        search_placeholder: "Search by name or username",
                        facets: Vec::new(),
                        selected: FilterSelection::new(),
                        on_select: move |_| {},
                        summary: format!("{} of {total} accounts", shown.len()),
                    }
                }
                if total == 0 {
                    EmptyState { icon: Icon::Invitation, title: "Nobody has joined yet", description: "Invite people from their page in Users › All users." }
                } else {
                    Grid { cols: 3, gap: Gap::Sm,
                        for p in shown.iter() {
                            {
                                let id = p.id;
                                let access = dir.access(id);
                                let mut tags = vec![Tag::new(access.summary(), if access.admin { Tone::Info } else { Tone::Neutral }).with_icon(Icon::Permission)];
                                if dir.is_owner(id) {
                                    tags.push(Tag::new("Created the congregation", Tone::Neutral));
                                }
                                let title = p.display_name(&prefs);
                                rsx! {
                                    RecordCard {
                                        key: "{id}",
                                        title: title.clone(),
                                        avatar: title,
                                        subtitle: username(p),
                                        tags,
                                        onclick: move |_| { navigator.push(Route::PermissionsDetail { id }); },
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

/// `/app/users/permissions/:id`
#[component]
pub fn PermissionsDetail(id: u32) -> Element {
    rsx! { PermissionsEditor { key: "{id}", id } }
}

#[component]
fn PermissionsEditor(id: u32) -> Element {
    let mut state = use_app_state();
    let navigator = navigator();
    let toaster = use_toaster();
    let prefs = state.preferences();
    let dir = state.directory();
    let mut draft = use_signal(|| dir.access(id));
    let mut chosen_preset = use_signal(String::new);
    let back = ("Permissions".to_string(), EventHandler::new(move |_| { navigator.push(Route::PermissionsPage {}); }));
    let Some(p) = dir.person(id).cloned() else {
        return rsx! { Page { title: "Not found", back, EmptyState { icon: Icon::Search, title: "This person does not exist" } } };
    };
    if !dir.has_account(id) {
        return rsx! {
            Page { title: p.display_name(&prefs), back,
                EmptyState {
                    icon: Icon::Permission,
                    title: "No account",
                    description: "Only people who joined the app can have permissions. Invite them from their page first.",
                }
            }
        };
    }
    let owner = dir.is_owner(id);
    let d = draft();
    let sections: Vec<NavEntry> = nav::sections().into_iter().filter(|s| !s.children.is_empty()).collect();
    let save = move |_| {
        let a = draft();
        let actor = state.account().map(|x| x.display_name).unwrap_or_default();
        match state.update_directory(|dir| dir.set_access(id, a, &actor)) {
            Ok(()) if state.manages_cloud() => {
                // Cloud: what the server lets them write, and the keys of what they now read.
                spawn(async move {
                    match state.sync_member(id).await {
                        Ok(()) => toaster.success("Permissions saved."),
                        Err(e) => toaster.error(format!("Permissions saved in the congregation, but not yet on the server: {e}")),
                    }
                });
            }
            Ok(()) => toaster.success("Permissions saved."),
            Err(e) => toaster.error(e),
        }
    };

    rsx! {
        Page {
            title: p.display_name(&prefs),
            subtitle: format!("{} · Permissions", username(&p)),
            back,
            Form {
                onsubmit: save,
                floating_save: true,
                actions: rsx! {
                    ActionButton { action: Action::Cancel, onclick: move |_| { navigator.push(Route::PermissionsPage {}); } }
                    ActionButton { action: Action::Save, kind: ButtonKind::Submit }
                },
                if !owner {
                    Card { title: "Start from a preset", icon: Icon::Magic,
                        Field {
                            label: "Preset",
                            help: preset(&chosen_preset()).map(|p| format!("{} The permissions below are filled in; review them and save.", p.description)).unwrap_or_else(|| "A ready set of permissions for a common responsibility. You can change them afterwards.".to_string()),
                            Select {
                                value: chosen_preset(),
                                options: std::iter::once(SelectOption::new("", "Choose a preset…")).chain(PRESETS.iter().map(|p| SelectOption::new(p.id, p.label))).collect(),
                                onchange: move |v: String| {
                                    if let Some(p) = preset(&v) {
                                        draft.set((p.access)());
                                    }
                                    chosen_preset.set(v);
                                },
                            }
                        }
                    }
                }
                Card { title: "Administrator", icon: Icon::Shield,
                    if owner {
                        Text { tone: TextTone::Muted, "Created the congregation, so always an administrator: every permission, now and for new pages." }
                    } else {
                        Switch {
                            checked: d.admin,
                            label: "Administrator",
                            description: "Every permission, now and for new pages. Only administrators can change permissions.",
                            onchange: move |v| draft.write().admin = v,
                        }
                    }
                }
                if !d.admin {
                    Card { title: "Field service groups and reports", icon: Icon::Group,
                        Switch {
                            checked: d.own_group_read,
                            label: "Read their own group",
                            description: "Contact details, field service reports and family of the members of their own field service group.",
                            onchange: move |v| draft.write().own_group_read = v,
                        }
                        Switch {
                            checked: d.all_groups_read,
                            label: "Read all groups",
                            description: "The same information for the members of every field service group.",
                            onchange: move |v| draft.write().all_groups_read = v,
                        }
                        Switch {
                            checked: d.own_group_reports,
                            label: "Write the reports of their own group",
                            description: "Enter and change the field service reports of their own group. To write all reports, give Write on Field service reports below.",
                            onchange: move |v| draft.write().own_group_reports = v,
                        }
                    }
                    for s in sections {
                        Card { key: "{s.id}", title: "{s.label}", icon: s.icon,
                            Grid { cols: 2, gap: Gap::Md,
                                for page in s.children.iter().filter(|c| !ADMIN_ONLY_PAGES.contains(&format!("{}/{}", s.id, c.id).as_str())) {
                                    {
                                        let key = format!("{}/{}", s.id, page.id);
                                        let level = d.level(&key);
                                        let k2 = key.clone();
                                        rsx! {
                                            Field { key: "{key}", label: page.label.clone(),
                                                Segmented {
                                                    value: level.value(),
                                                    options: vec![
                                                        Segment { value: "none".into(), label: "No access".into(), icon: None },
                                                        Segment { value: "read".into(), label: "Read".into(), icon: Some(Icon::Show) },
                                                        Segment { value: "write".into(), label: "Write".into(), icon: Some(Icon::Edit) },
                                                    ],
                                                    onchange: move |v: String| { draft.write().pages.insert(k2.clone(), Level::parse(&v)); },
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                    {
                        let level = d.level("settings");
                        rsx! {
                            Card { title: "Congregation", icon: Icon::Congregation,
                                Field { label: "Settings", help: "Details, meeting times, display defaults, backup and the activity log.",
                                    Segmented {
                                        value: level.value(),
                                        options: vec![
                                            Segment { value: "none".into(), label: "No access".into(), icon: None },
                                            Segment { value: "read".into(), label: "Read".into(), icon: Some(Icon::Show) },
                                            Segment { value: "write".into(), label: "Write".into(), icon: Some(Icon::Edit) },
                                        ],
                                        onchange: move |v: String| { draft.write().pages.insert("settings".into(), Level::parse(&v)); },
                                    }
                                }
                            }
                        }
                    }
                    Text { tone: TextTone::Small, "Write includes read. Users › Permissions is for administrators only." }
                }
            }
        }
    }
}

/// Summary used by the person page's account card.
pub(super) fn access_summary(access: &Access) -> String {
    access.summary()
}
