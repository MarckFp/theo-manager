use std::collections::HashMap;

use dioxus::prelude::*;
use dioxus_i18n::t;
use surrealdb::types::RecordId;

use crate::components::ResponsiveModal;
use crate::database::{use_crypto, use_db};
use crate::models::congregation::Congregation;
use crate::models::field_service_group::{FieldServiceGroup, FieldServiceGroupData};
use crate::models::user::User;
use crate::pages::app::user::is_publisher_type;

// ── Helper ────────────────────────────────────────────────────────────────────

fn rid_str(id: &RecordId) -> String {
    format!(
        "{}:{}",
        id.table,
        match &id.key {
            surrealdb::types::RecordIdKey::String(k) => k.clone(),
            surrealdb::types::RecordIdKey::Number(n) => n.to_string(),
            _ => String::new(),
        }
    )
}

fn normalize(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            'á' | 'à' | 'ä' | 'â' | 'ã' | 'Á' | 'À' | 'Ä' | 'Â' | 'Ã' => 'a',
            'é' | 'è' | 'ë' | 'ê' | 'É' | 'È' | 'Ë' | 'Ê' => 'e',
            'í' | 'ì' | 'ï' | 'î' | 'Í' | 'Ì' | 'Ï' | 'Î' => 'i',
            'ó' | 'ò' | 'ö' | 'ô' | 'õ' | 'Ó' | 'Ò' | 'Ö' | 'Ô' | 'Õ' => 'o',
            'ú' | 'ù' | 'ü' | 'û' | 'Ú' | 'Ù' | 'Ü' | 'Û' => 'u',
            'ñ' | 'Ñ' => 'n',
            'ç' | 'Ç' => 'c',
            other => other.to_ascii_lowercase(),
        })
        .collect()
}

// ── Form state ────────────────────────────────────────────────────────────────

#[derive(Clone, Default)]
struct GroupFormState {
    name: String,
    overseer_str: String,     // "" or "user:KEY"
    assistant_str: String,    // "" or "user:KEY"
    member_strs: Vec<String>, // "user:KEY" strings of selected members
    submitting: bool,
    error: Option<String>,
}

// ── Main page ─────────────────────────────────────────────────────────────────

#[component]
pub fn AppFieldServiceGroups() -> Element {
    let db_signal = use_db();
    let crypto_signal = use_crypto();

    let mut users_res = use_resource(move || async move {
        let Some(db) = db_signal.read().db.clone() else {
            return vec![];
        };
        let crypto = crypto_signal.read().clone();
        User::all(&db, &crypto).await.unwrap_or_default()
    });

    let mut groups_res = use_resource(move || async move {
        let Some(db) = db_signal.read().db.clone() else {
            return vec![];
        };
        let crypto = crypto_signal.read().clone();
        FieldServiceGroup::all(&db, &crypto).await.unwrap_or_default()
    });

    let mut restarted = use_signal(|| false);
    use_effect(move || {
        if *restarted.peek() {
            return;
        }
        restarted.set(true);
        users_res.restart();
        groups_res.restart();
    });

    let mut sheet_open = use_signal(|| false);
    let mut editing_group: Signal<Option<FieldServiceGroup>> = use_signal(|| None);
    let mut delete_open = use_signal(|| false);
    let mut delete_id: Signal<Option<RecordId>> = use_signal(|| None);

    // Get congregation ID from the shared context resource (set by AppLayout)
    let congregation_res = use_context::<Resource<Option<Congregation>>>();

    let users: Vec<User> = users_res().unwrap_or_default();
    let groups: Vec<FieldServiceGroup> = groups_res().unwrap_or_default();

    // Lookup: record_id_str -> display_name for all users
    let user_map: HashMap<String, String> = users
        .iter()
        .filter_map(|u| {
            u.id.as_ref()
                .map(|id| (rid_str(id), format!("{} {}", u.first_name, u.last_name)))
        })
        .collect();

    // Non-student users eligible for group roles, sorted by last name
    let mut eligible_users: Vec<User> = users
        .iter()
        .filter(|u| is_publisher_type(&u.user_type))
        .cloned()
        .collect();
    eligible_users.sort_by(|a, b| {
        normalize(&format!("{} {}", a.last_name, a.first_name))
            .cmp(&normalize(&format!("{} {}", b.last_name, b.first_name)))
    });

    let is_loading = users_res.read().is_none() || groups_res.read().is_none();

    rsx! {
        div { class: "space-y-4 w-full pb-24",

            // ── Header ─────────────────────────────────────────────────────
            div { class: "flex items-center justify-between",
                h1 { class: "text-xl font-bold text-gray-900", {t!("page-field-service-groups")} }
            }

            if is_loading {
                div { class: "flex justify-center items-center py-20 text-gray-400",
                    span { class: "text-sm animate-pulse", {t!("group-loading")} }
                }
            } else if groups.is_empty() {
                div { class: "bg-white rounded-xl border border-gray-200",
                    div { class: "px-6 py-12 text-center text-gray-400",
                        p { class: "text-4xl mb-3", "🏘️" }
                        p { class: "font-medium text-gray-600", {t!("empty-groups-title")} }
                        p { class: "text-sm mt-1", {t!("empty-groups-desc")} }
                    }
                }
            } else {
                div { class: "space-y-2",
                    for group in groups.clone() {
                        {
                            let g_edit = group.clone();
                            let g_del_id = group.id.clone();
                            let um = user_map.clone();
                            rsx! {
                                GroupCard {
                                    group,
                                    user_map: um,
                                    on_edit: move |_| {
                                        editing_group.set(Some(g_edit.clone()));
                                        sheet_open.set(true);
                                    },
                                    on_delete: move |_| {
                                        delete_id.set(g_del_id.clone());
                                        delete_open.set(true);
                                    },
                                }
                            }
                        }
                    }
                }
            }

            // ── FAB ────────────────────────────────────────────────────────
            button {
                class: "fixed bottom-6 right-6 w-14 h-14 bg-primary-600 text-white rounded-full shadow-xl hover:bg-primary-700 active:scale-95 transition-all flex items-center justify-center text-2xl z-20 select-none",
                onclick: move |_| {
                    editing_group.set(None);
                    sheet_open.set(true);
                },
                "＋"
            }

            GroupFormModal {
                open: sheet_open,
                on_close: move |_| sheet_open.set(false),
                on_saved: move |_| {
                    groups_res.restart();
                    sheet_open.set(false);
                },
                existing: editing_group,
                eligible_users,
                congregation_res,
            }

            GroupDeleteModal {
                open: delete_open,
                on_close: move |_| delete_open.set(false),
                on_confirm: move |_| {
                    delete_open.set(false);
                    if let Some(gid) = delete_id.read().clone() {
                        spawn(async move {
                            let Some(db) = db_signal.read().db.clone() else {
                                return;
                            };
                            let _ = FieldServiceGroup::delete(&db, gid).await;
                            groups_res.restart();
                        });
                    }
                },
            }
        }
    }
}

// ── GroupCard ─────────────────────────────────────────────────────────────────

#[component]
fn GroupCard(
    group: FieldServiceGroup,
    user_map: HashMap<String, String>,
    on_edit: Callback<()>,
    on_delete: Callback<()>,
) -> Element {
    let overseer_name = group
        .overseer
        .as_ref()
        .and_then(|id| user_map.get(&rid_str(id)))
        .cloned()
        .unwrap_or_else(|| "—".to_string());
    let assistant_name = group
        .assistant
        .as_ref()
        .and_then(|id| user_map.get(&rid_str(id)))
        .cloned()
        .unwrap_or_else(|| "—".to_string());
    let member_count = group.members.len();

    rsx! {
        div { class: "bg-white rounded-xl border border-gray-200 p-4",
            div { class: "flex items-start justify-between gap-3",
                div { class: "flex-1 min-w-0",
                    h3 { class: "text-sm font-semibold text-gray-900 truncate", "{group.name}" }
                    div { class: "mt-2 space-y-1.5",
                        div { class: "flex items-center gap-2 text-xs",
                            span { class: "text-gray-400 w-24 shrink-0", {t!("group-role-overseer")} }
                            span { class: "text-gray-800 font-medium", "{overseer_name}" }
                        }
                        div { class: "flex items-center gap-2 text-xs",
                            span { class: "text-gray-400 w-24 shrink-0", {t!("group-role-assistant")} }
                            span { class: "text-gray-800 font-medium", "{assistant_name}" }
                        }
                        div { class: "flex items-center gap-2 text-xs",
                            span { class: "text-gray-400 w-24 shrink-0", {t!("group-members-count")} }
                            span { class: "text-gray-800 font-medium", "{member_count}" }
                        }
                    }
                }
                div { class: "flex gap-1.5 shrink-0",
                    button {
                        class: "px-2.5 py-1 text-xs border border-gray-200 rounded-lg text-gray-600 hover:bg-gray-50 transition-colors",
                        onclick: move |_| on_edit.call(()),
                        {t!("btn-edit")}
                    }
                    button {
                        class: "px-2.5 py-1 text-xs border border-red-200 rounded-lg text-red-600 hover:bg-red-50 transition-colors",
                        onclick: move |_| on_delete.call(()),
                        {t!("btn-delete")}
                    }
                }
            }
        }
    }
}

// ── GroupFormModal ────────────────────────────────────────────────────────────

#[component]
fn GroupFormModal(
    open: Signal<bool>,
    on_close: Callback<()>,
    on_saved: Callback<()>,
    existing: Signal<Option<FieldServiceGroup>>,
    eligible_users: Vec<User>,
    congregation_res: Resource<Option<Congregation>>,
) -> Element {
    let db_signal = use_db();
    let crypto_signal = use_crypto();
    let mut form = use_signal(GroupFormState::default);

    // Pre-fill form from existing group whenever the modal opens.
    let existing_for_effect = existing.clone();
    use_effect(move || {
        if *open.read() {
            let fs = match existing_for_effect.read().as_ref() {
                Some(g) => GroupFormState {
                    name: g.name.clone(),
                    overseer_str: g.overseer.as_ref().map(rid_str).unwrap_or_default(),
                    assistant_str: g.assistant.as_ref().map(rid_str).unwrap_or_default(),
                    member_strs: g.members.iter().map(rid_str).collect(),
                    ..Default::default()
                },
                None => GroupFormState::default(),
            };
            form.set(fs);
        }
    });

    let f = form.read().clone();
    let cong_id_for_submit = congregation_res.read().clone().flatten().and_then(|c| c.id);
    let existing_for_submit = existing.clone();

    let on_submit = use_callback(move |_: Event<MouseData>| {
        let fd = form.read().clone();
        if fd.name.trim().is_empty() {
            form.write().error = Some(t!("group-form-required-error"));
            return;
        }
        let Some(cong_id) = cong_id_for_submit.clone() else {
            form.write().error = Some(t!("group-form-no-congregation"));
            return;
        };
        let overseer = if fd.overseer_str.is_empty() {
            None
        } else {
            RecordId::parse_simple(&fd.overseer_str).ok()
        };
        let assistant = if fd.assistant_str.is_empty() {
            None
        } else {
            RecordId::parse_simple(&fd.assistant_str).ok()
        };
        // Members: start from the checkbox selection then ensure overseer and
        // assistant are always included.
        let mut members: Vec<RecordId> = fd
            .member_strs
            .iter()
            .filter_map(|s| RecordId::parse_simple(s).ok())
            .collect();
        if let Some(ref rid) = overseer {
            if !members.iter().any(|m| m == rid) {
                members.push(rid.clone());
            }
        }
        if let Some(ref rid) = assistant {
            if !members.iter().any(|m| m == rid) {
                members.push(rid.clone());
            }
        }
        let data = FieldServiceGroupData {
            congregation: cong_id,
            name: fd.name.trim().to_string(),
            overseer,
            assistant,
            members,
        };
        let existing_id = existing_for_submit.read().as_ref().and_then(|g| g.id.clone());
        form.write().submitting = true;
        form.write().error = None;
        spawn(async move {
            let Some(db) = db_signal.read().db.clone() else {
                form.write().submitting = false;
                form.write().error = Some("No database connection.".to_string());
                return;
            };
            let crypto = crypto_signal.read().clone();
            let result = if let Some(id) = existing_id {
                FieldServiceGroup::update(&db, &crypto, id, data).await.map(|_| ())
            } else {
                FieldServiceGroup::create(&db, &crypto, data).await.map(|_| ())
            };
            match result {
                Ok(_) => on_saved.call(()),
                Err(e) => {
                    form.write().submitting = false;
                    form.write().error = Some(e.to_string());
                }
            }
        });
    });

    let is_editing = existing.read().is_some();
    let (modal_title, modal_desc) = if is_editing {
        (t!("group-form-title-edit"), t!("group-form-desc-edit"))
    } else {
        (t!("group-form-title-add"), t!("group-form-desc-add"))
    };

    rsx! {
        ResponsiveModal {
            open,
            on_close,
            title: modal_title,
            description: modal_desc,
            submitting: f.submitting,
            on_submit,
            GroupFormBody { form, eligible_users }
        }
    }
}

// ── GroupFormBody ─────────────────────────────────────────────────────────────

#[component]
fn GroupFormBody(form: Signal<GroupFormState>, eligible_users: Vec<User>) -> Element {
    let f = form.read().clone();

    rsx! {
        if let Some(err) = &f.error {
            div { class: "bg-red-50 border border-red-200 text-red-700 text-sm px-3 py-2 rounded-lg",
                "{err}"
            }
        }

        // Group name
        div { class: "flex flex-col gap-1",
            label { class: "text-xs font-medium text-gray-700",
                {t!("group-form-name")}
                span { class: "text-red-500 ml-0.5", " *" }
            }
            input {
                r#type: "text",
                class: "w-full px-3 py-2 text-sm border border-gray-200 rounded-lg focus:outline-none focus:ring-2 focus:ring-primary-500",
                value: f.name.clone(),
                oninput: move |e| form.write().name = e.value(),
            }
        }

        // Overseer
        div { class: "flex flex-col gap-1",
            label { class: "text-xs font-medium text-gray-700", {t!("group-form-overseer")} }
            select {
                class: "w-full px-3 py-2 text-sm border border-gray-200 rounded-lg bg-white focus:outline-none focus:ring-2 focus:ring-primary-500",
                onchange: move |e| {
                    let uid = e.value();
                    let mut fw = form.write();
                    // Remove old overseer from forced-member list only if it's
                    // not also the assistant (edge case).
                    let old = fw.overseer_str.clone();
                    if !old.is_empty() && old != fw.assistant_str {
                        fw.member_strs.retain(|s| s != &old);
                    }
                    fw.overseer_str = uid.clone();
                    // Auto-add new overseer to members.
                    if !uid.is_empty() && !fw.member_strs.contains(&uid) {
                        fw.member_strs.push(uid);
                    }
                },
                option { value: "", selected: f.overseer_str.is_empty(), {t!("group-form-none")} }
                for user in eligible_users.iter() {
                    {
                        let uid = user.id.as_ref().map(rid_str).unwrap_or_default();
                        let name = format!("{} {}", user.first_name, user.last_name);
                        let selected = f.overseer_str == uid;
                        rsx! {
                            option { value: "{uid}", selected, "{name}" }
                        }
                    }
                }
            }
        }

        // Assistant
        div { class: "flex flex-col gap-1",
            label { class: "text-xs font-medium text-gray-700", {t!("group-form-assistant")} }
            select {
                class: "w-full px-3 py-2 text-sm border border-gray-200 rounded-lg bg-white focus:outline-none focus:ring-2 focus:ring-primary-500",
                onchange: move |e| {
                    let uid = e.value();
                    let mut fw = form.write();
                    let old = fw.assistant_str.clone();
                    if !old.is_empty() && old != fw.overseer_str {
                        fw.member_strs.retain(|s| s != &old);
                    }
                    fw.assistant_str = uid.clone();
                    if !uid.is_empty() && !fw.member_strs.contains(&uid) {
                        fw.member_strs.push(uid);
                    }
                },
                option { value: "", selected: f.assistant_str.is_empty(), {t!("group-form-none")} }
                for user in eligible_users.iter() {
                    {
                        let uid = user.id.as_ref().map(rid_str).unwrap_or_default();
                        let name = format!("{} {}", user.first_name, user.last_name);
                        let selected = f.assistant_str == uid;
                        rsx! {
                            option { value: "{uid}", selected, "{name}" }
                        }
                    }
                }
            }
        }

        // Members (checkboxes)
        div { class: "flex flex-col gap-1",
            label { class: "text-xs font-medium text-gray-700", {t!("group-form-members")} }
            if eligible_users.is_empty() {
                p { class: "text-xs text-gray-400 italic py-2", {t!("group-form-no-eligible")} }
            } else {
                div { class: "max-h-52 overflow-y-auto border border-gray-200 rounded-lg divide-y divide-gray-100",
                    for user in eligible_users.iter() {
                        {
                            let uid = user.id.as_ref().map(rid_str).unwrap_or_default();
                            let name = format!("{} {}", user.first_name, user.last_name);
                            let is_locked = uid == f.overseer_str || uid == f.assistant_str;
                            let is_checked = is_locked || f.member_strs.contains(&uid);
                            let uid_toggle = uid.clone();
                            let row_cls = if is_locked {
                                "flex items-center gap-2.5 px-3 py-2.5 bg-gray-50 select-none"
                            } else {
                                "flex items-center gap-2.5 px-3 py-2.5 hover:bg-gray-50 cursor-pointer select-none"
                            };
                            let label_cls = if is_locked {
                                "text-sm text-gray-500"
                            } else {
                                "text-sm text-gray-800"
                            };
                            rsx! {
                                label { class: row_cls,
                                    input {
                                        r#type: "checkbox",
                                        class: "rounded border-gray-300 text-primary-600 focus:ring-primary-500",
                                        checked: is_checked,
                                        disabled: is_locked,
                                        onchange: move |e| {
                                            if is_locked {
                                                return;
                                            }
                                            let mut fw = form.write();
                                            if e.checked() {
                                                if !fw.member_strs.contains(&uid_toggle) {
                                                    fw.member_strs.push(uid_toggle.clone());
                                                }
                                            } else {
                                                fw.member_strs.retain(|s| s != &uid_toggle);
                                            }
                                        },
                                    }
                                    span { class: label_cls, "{name}" }
                                    if is_locked {
                                        span { class: "ml-auto text-xs text-gray-400 italic",
                                            if uid == f.overseer_str && uid == f.assistant_str {
                                                {t!("group-form-role-both")}
                                            } else if uid == f.overseer_str {
                                                {t!("group-form-role-overseer")}
                                            } else {
                                                {t!("group-form-role-assistant")}
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
}

// ── Delete confirmation modal ─────────────────────────────────────────────────

#[component]
fn GroupDeleteModal(
    open: Signal<bool>,
    on_close: Callback<()>,
    on_confirm: Callback<()>,
) -> Element {
    let is_open = *open.read();
    let overlay_cls = if is_open {
        "fixed inset-0 z-50 flex items-center justify-center p-4 bg-black/40"
    } else {
        "fixed inset-0 z-50 flex items-center justify-center p-4 bg-black/40 opacity-0 pointer-events-none"
    };

    rsx! {
        div { class: overlay_cls, onclick: move |_| on_close.call(()),
            div {
                class: "bg-white rounded-2xl shadow-2xl w-full max-w-sm p-6 space-y-4",
                onclick: move |e| e.stop_propagation(),
                h2 { class: "text-base font-semibold text-gray-900", {t!("group-delete-title")} }
                p { class: "text-sm text-gray-600", {t!("group-delete-confirm")} }
                div { class: "flex gap-2 pt-2",
                    button {
                        class: "flex-1 px-4 py-2 text-sm border border-gray-200 rounded-xl text-gray-700 hover:bg-gray-50 transition-colors",
                        onclick: move |_| on_close.call(()),
                        {t!("btn-cancel")}
                    }
                    button {
                        class: "flex-1 px-4 py-2 text-sm bg-red-600 text-white rounded-xl hover:bg-red-700 transition-colors font-medium",
                        onclick: move |_| on_confirm.call(()),
                        {t!("btn-confirm")}
                    }
                }
            }
        }
    }
}
