use std::collections::{HashMap, HashSet};

use dioxus::prelude::*;
use dioxus_i18n::t;
use surrealdb::types::RecordId;

use crate::components::ResponsiveModal;
use crate::database::{use_crypto, use_db};
use crate::models::congregation::{Congregation, NameFormat};
use crate::models::field_service_meeting::{FieldServiceMeeting, FieldServiceMeetingData};
use crate::models::privilege::UserPrivileges;
use crate::models::user::User;
use crate::pages::app::user::{effective_name_format, format_name};

// ── Platform helpers ──────────────────────────────────────────────────────────

#[cfg(target_arch = "wasm32")]
fn today_iso() -> String {
    let d = js_sys::Date::new_0();
    format!(
        "{:04}-{:02}-{:02}",
        d.get_full_year(),
        d.get_month() + 1,
        d.get_date()
    )
}

#[cfg(not(target_arch = "wasm32"))]
fn today_iso() -> String {
    "2026-06-11".to_string()
}

#[cfg(target_arch = "wasm32")]
fn current_year_month() -> (i32, u8) {
    let d = js_sys::Date::new_0();
    (d.get_full_year() as i32, (d.get_month() + 1) as u8)
}

#[cfg(not(target_arch = "wasm32"))]
fn current_year_month() -> (i32, u8) {
    (2026, 6)
}

// ── Calendar helpers ──────────────────────────────────────────────────────────

fn days_in_month(y: i32, m: u8) -> u8 {
    match m {
        4 | 6 | 9 | 11 => 30,
        2 => {
            if (y % 4 == 0 && y % 100 != 0) || y % 400 == 0 {
                29
            } else {
                28
            }
        }
        _ => 31,
    }
}

/// Tomohiko Sakamoto: 0=Sunday, 1=Monday, …, 6=Saturday.
fn day_of_week(y: i32, m: u8, d: u8) -> u8 {
    const T: [i32; 12] = [0, 3, 2, 5, 0, 3, 5, 1, 4, 6, 2, 4];
    let y = if (m as i32) < 3 { y - 1 } else { y };
    ((y + y / 4 - y / 100 + y / 400 + T[m as usize - 1] + d as i32) % 7) as u8
}

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

// ── Form state ────────────────────────────────────────────────────────────────

#[derive(Clone, Default)]
struct MeetingFormState {
    date: String,
    location: String,
    assignee_id: String,
    notes: String,
}

// ── Main page ─────────────────────────────────────────────────────────────────

#[component]
pub fn AppFieldServiceMeetings() -> Element {
    let db_signal = use_db();
    let crypto_signal = use_crypto();
    let congregation_res = use_context::<Resource<Option<Congregation>>>();
    let uid = db_signal.read().congregation_uid.clone().unwrap_or_default();

    // Name format
    let mut name_fmt = use_signal(|| NameFormat::FirstLast);
    {
        let uid = uid.clone();
        use_effect(move || {
            let uid = uid.clone();
            let cong_snap = congregation_res.read().clone();
            let db_opt = db_signal.read().db.clone();
            spawn(async move {
                let prefs = crate::pages::app::user_settings::load_prefs(&uid, db_opt).await;
                let cong_ref = cong_snap.as_ref().and_then(|o| o.as_ref());
                name_fmt.set(effective_name_format(
                    cong_ref,
                    prefs.name_format.as_deref().unwrap_or(""),
                ));
            });
        });
    }

    // Month navigation
    let (cur_year, cur_month) = current_year_month();
    let mut sel_year = use_signal(|| cur_year);
    let mut sel_month = use_signal(|| cur_month);
    let mut show_picker = use_signal(|| false);

    // Resources
    let mut meetings_res = use_resource(move || async move {
        let Some(db) = db_signal.read().db.clone() else {
            return vec![];
        };
        let y = sel_year();
        let m = sel_month();
        FieldServiceMeeting::all_for_month(&db, y, m)
            .await
            .unwrap_or_default()
    });

    let mut users_res = use_resource(move || async move {
        let Some(db) = db_signal.read().db.clone() else {
            return vec![];
        };
        let crypto = crypto_signal.read().clone();
        User::all(&db, &crypto).await.unwrap_or_default()
    });

    let mut privs_res = use_resource(move || async move {
        let Some(db) = db_signal.read().db.clone() else {
            return vec![];
        };
        UserPrivileges::all(&db).await.unwrap_or_default()
    });

    let mut bootstrapped = use_signal(|| false);
    use_effect(move || {
        if *bootstrapped.peek() {
            return;
        }
        bootstrapped.set(true);
        meetings_res.restart();
        users_res.restart();
        privs_res.restart();
    });

    // Modal state
    let mut modal_open = use_signal(|| false);
    let mut edit_target: Signal<Option<FieldServiceMeeting>> = use_signal(|| None);
    let mut prefill_date = use_signal(String::new);
    let mut pending_delete: Signal<Option<RecordId>> = use_signal(|| None);

    // Derived
    let is_loading = meetings_res.read().is_none() || users_res.read().is_none();
    let today = today_iso();
    let meetings: Vec<FieldServiceMeeting> = meetings_res().unwrap_or_default();
    let users: Vec<User> = users_res().unwrap_or_default();
    let privs: Vec<UserPrivileges> = privs_res().unwrap_or_default();

    // date → meetings map for calendar pills
    let meetings_by_date: HashMap<String, Vec<FieldServiceMeeting>> = {
        let mut m: HashMap<String, Vec<FieldServiceMeeting>> = HashMap::new();
        for meeting in &meetings {
            m.entry(meeting.date.clone()).or_default().push(meeting.clone());
        }
        m
    };

    // user id → display name
    let user_map: HashMap<String, String> = users
        .iter()
        .filter_map(|u| {
            let id = u.id.as_ref().map(rid_str)?;
            let name = format_name(&u.first_name, &u.last_name, &name_fmt.read());
            Some((id, name))
        })
        .collect();

    // Calendar values
    let year = sel_year();
    let month = sel_month();
    let total_days = days_in_month(year, month) as u32;
    // Offset for Monday-first layout: (Sun=0 + 6) % 7 → 6; (Mon=1 + 6) % 7 → 0
    let offset = (day_of_week(year, month, 1) as u32 + 6) % 7;

    // Month picker abbreviations
    let month_picker_labels: Vec<String> = (1u8..=12)
        .map(|mi| {
            let full = match mi {
                1 => t!("month-1"),
                2 => t!("month-2"),
                3 => t!("month-3"),
                4 => t!("month-4"),
                5 => t!("month-5"),
                6 => t!("month-6"),
                7 => t!("month-7"),
                8 => t!("month-8"),
                9 => t!("month-9"),
                10 => t!("month-10"),
                11 => t!("month-11"),
                _ => t!("month-12"),
            };
            full.chars().take(3).collect::<String>()
        })
        .collect();

    let month_full = match month {
        1 => t!("month-1"),
        2 => t!("month-2"),
        3 => t!("month-3"),
        4 => t!("month-4"),
        5 => t!("month-5"),
        6 => t!("month-6"),
        7 => t!("month-7"),
        8 => t!("month-8"),
        9 => t!("month-9"),
        10 => t!("month-10"),
        11 => t!("month-11"),
        _ => t!("month-12"),
    };

    rsx! {
        div { class: "flex flex-col h-full gap-3",
            h1 { class: "text-2xl font-bold text-gray-900 shrink-0",
                {t!("page-field-service-meetings")}
            }

            // ── Month / year navigation ────────────────────────────────────
            div { class: "bg-white rounded-xl border border-gray-200 p-4",
                div { class: "flex items-center justify-between gap-3",
                    button {
                        class: "px-5 py-2.5 min-w-[56px] rounded-lg border border-gray-200 text-gray-600 hover:bg-gray-50 active:bg-gray-100 transition-colors select-none text-xl font-semibold",
                        onclick: move |_| {
                            let (y, m) = (sel_year(), sel_month());
                            if m == 1 {
                                sel_year.set(y - 1);
                                sel_month.set(12);
                            } else {
                                sel_month.set(m - 1);
                            }
                        },
                        "‹"
                    }

                    {
                        let month_abbr = month_full.chars().take(3).collect::<String>();
                        let year_str = year.to_string();
                        let btn_cls = if show_picker() {
                            "flex-1 inline-flex items-center justify-center gap-1 px-3 py-1.5 rounded-lg border border-primary-400 text-primary-700 font-semibold bg-primary-50 transition-colors"
                        } else {
                            "flex-1 inline-flex items-center justify-center gap-1 px-3 py-1.5 rounded-lg border border-dashed border-gray-300 text-gray-900 font-semibold hover:border-primary-400 hover:text-primary-600 hover:bg-primary-50 transition-colors"
                        };
                        rsx! {
                            button { class: btn_cls, onclick: move |_| show_picker.set(!show_picker()),
                                span { "{year_str}" }
                                span { class: "hidden sm:inline", " {month_full}" }
                                span { class: "sm:hidden", " {month_abbr}" }
                                span { class: "text-xs text-gray-400",
                                    if show_picker() {
                                        "▴"
                                    }
                                    }
                                        "▾"
                                    }
                                }
                            }
                        }
                    }

                    button {
                        class: "px-5 py-2.5 min-w-[56px] rounded-lg border border-gray-200 text-gray-600 hover:bg-gray-50 active:bg-gray-100 transition-colors select-none text-xl font-semibold",
                        onclick: move |_| {
                            let (y, m) = (sel_year(), sel_month());
                            if m == 12 {
                                sel_year.set(y + 1);
                                sel_month.set(1);
                            } else {
                                sel_month.set(m + 1);
                            }
                        },
                        "›"
                    }
                }

                // Month picker grid
                if show_picker() {
                    div { class: "mt-3 space-y-3",
                        div { class: "flex items-center justify-center gap-4",
                            button {
                                class: "px-3 py-1 text-sm border border-gray-200 rounded-lg text-gray-600 hover:bg-gray-50",
                                onclick: move |_| sel_year.set(sel_year() - 1),
                                "−"
                            }
                            span { class: "text-sm font-medium text-gray-800 w-12 text-center",
                                "{sel_year()}"
                            }
                            button {
                                class: "px-3 py-1 text-sm border border-gray-200 rounded-lg text-gray-600 hover:bg-gray-50",
                                onclick: move |_| sel_year.set(sel_year() + 1),
                                "＋"
                            }
                        }
                        div { class: "grid grid-cols-4 gap-1.5",
                            for (idx , abbr) in month_picker_labels.iter().enumerate() {
                                {
                                    let mi = (idx + 1) as u8;
                                    let abbr = abbr.clone();
                                    let is_sel = sel_month() == mi;
                                    let cls = if is_sel {
                                        "py-1.5 text-xs rounded-lg text-center bg-primary-600 text-white font-medium"
                                    } else {
                                        "py-1.5 text-xs rounded-lg text-center border border-gray-200 text-gray-700 hover:bg-gray-50 cursor-pointer"
                                    };
                                    rsx! {
                                        button {
                                            class: cls,
                                            onclick: move |_| {
                                                sel_month.set(mi);
                                                show_picker.set(false);
                                            },
                                            "{abbr}"
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // ── Calendar ───────────────────────────────────────────────────
            if is_loading {
                div { class: "flex-1 flex justify-center py-20 text-gray-400",
                    span { class: "text-sm animate-pulse", {t!("fsm-loading")} }
                }
            } else {
                div { class: "flex-1 min-h-0 bg-white rounded-xl border border-gray-200 overflow-hidden flex flex-col",
                    // Day headers Mon → Sun
                    div { class: "grid grid-cols-7 bg-gray-50 border-b border-gray-200",
                        span { class: "py-2 text-center text-xs font-semibold text-gray-500 uppercase tracking-wide",
                            {t!("fsm-day-mon")}
                        }
                        span { class: "py-2 text-center text-xs font-semibold text-gray-500 uppercase tracking-wide",
                            {t!("fsm-day-tue")}
                        }
                        span { class: "py-2 text-center text-xs font-semibold text-gray-500 uppercase tracking-wide",
                            {t!("fsm-day-wed")}
                        }
                        span { class: "py-2 text-center text-xs font-semibold text-gray-500 uppercase tracking-wide",
                            {t!("fsm-day-thu")}
                        }
                        span { class: "py-2 text-center text-xs font-semibold text-gray-500 uppercase tracking-wide",
                            {t!("fsm-day-fri")}
                        }
                        span { class: "py-2 text-center text-xs font-semibold text-gray-500 uppercase tracking-wide",
                            {t!("fsm-day-sat")}
                        }
                        span { class: "py-2 text-center text-xs font-semibold text-gray-500 uppercase tracking-wide",
                            {t!("fsm-day-sun")}
                        }
                    }
                    // Cells — gap-px on bg-gray-100 creates thin grid lines
                    div { class: "flex-1 grid grid-cols-7 gap-px bg-gray-100 auto-rows-fr",
                        // Empty offset cells
                        for _ in 0..offset {
                            div { class: "bg-gray-50 p-1" }
                        }
                        // Day cells
                        for day in 1..=total_days {
                            {
                                let date_str = format!("{:04}-{:02}-{:02}", year, month, day);
                                let is_past = date_str < today;
                                let is_today_day = date_str == today;
                                let day_meetings = meetings_by_date.get(&date_str).cloned().unwrap_or_default();
                                let date_for_click = date_str.clone();
                                let bg_cls = if is_today_day {
                                    "h-full bg-primary-50 hover:bg-primary-100 p-1.5 cursor-pointer transition-colors flex flex-col"
                                } else if is_past {
                                    "h-full bg-gray-50 hover:bg-gray-100 p-1.5 cursor-pointer transition-colors flex flex-col"
                                } else {
                                    "h-full bg-white hover:bg-primary-50 p-1.5 cursor-pointer transition-colors flex flex-col"
                                };
                                let num_cls = if is_today_day {
                                    "w-6 h-6 rounded-full bg-primary-600 text-white text-xs font-bold flex items-center justify-center shrink-0"
                                } else if is_past {
                                    "w-6 h-6 text-xs font-medium text-gray-400 flex items-center justify-center shrink-0"
                                } else {
                                    "w-6 h-6 text-xs font-medium text-gray-700 flex items-center justify-center shrink-0"
                                };
                                rsx! {
                                    div {
                                        class: bg_cls,
                                        onclick: move |_| {
                                            prefill_date.set(date_for_click.clone());
                                            edit_target.set(None);
                                            modal_open.set(true);
                                        },
                                        span { class: "{num_cls}", "{day}" }
                                        for meeting in day_meetings {
                                            {
                                                let m_id = meeting.id.clone();
                                                let m_for_edit = meeting.clone();
                                                let del_id = m_id.clone();
                                                let location_str = meeting.location.clone();
                                                let assignee_name = user_map
                                                    .get(&rid_str(&meeting.assignee))
                                                    .cloned()
                                                    .unwrap_or_else(|| "—".to_string());
                                                let is_confirming = pending_delete
                                                    .read()
                                                    .as_ref()
                                                    .and_then(|pd| m_id.as_ref().map(|id| rid_str(pd) == rid_str(id)))
                                                    .unwrap_or(false);
                                                rsx! {
                                                    if is_confirming {
                                                        div { class: "flex gap-0.5 mt-1", onclick: move |e| e.stop_propagation(),
                                                            button {
                                                                class: "flex-1 text-xs px-1.5 py-1 bg-red-600 text-white rounded-lg font-medium leading-tight",
                                                                onclick: move |e| {
                                                                    e.stop_propagation();
                                                                    let rid = pending_delete.read().clone();
                                                                    pending_delete.set(None);
                                                                    if let Some(rid) = rid {
                                                                        spawn(async move {
                                                                            if let Some(db) = db_signal.read().db.clone() {
                                                                                let _ = FieldServiceMeeting::delete(&db, rid).await;
                                                                            }
                                                                            meetings_res.restart();
                                                                        });
                                                                    }
                                                                },
                                                                "✓"
                                                            }
                                                            button {
                                                                class: "flex-1 text-xs px-1.5 py-1 bg-gray-200 text-gray-700 rounded-lg leading-tight",
                                                                onclick: move |e| {
                                                                    e.stop_propagation();
                                                                    pending_delete.set(None);
                                                                },
                                                                "✕"
                                                            }
                                                        }
                                                    } else {
                                                        div {
                                                            class: "group flex items-start gap-0.5 mt-1",
                                                            onclick: move |e| e.stop_propagation(),
                                                            button {
                                                                class: "flex-1 min-w-0 text-left text-[10px] sm:text-xs leading-tight px-1 sm:px-1.5 py-0.5 sm:py-1 bg-primary-100 text-primary-800 rounded-lg hover:bg-primary-200 transition-colors",
                                                                onclick: move |e| {
                                                                    e.stop_propagation();
                                                                    edit_target.set(Some(m_for_edit.clone()));
                                                                    prefill_date.set(m_for_edit.date.clone());
                                                                    modal_open.set(true);
                                                                },
                                                                div { class: "font-medium truncate", "{assignee_name}" }
                                                                if !location_str.is_empty() {
                                                                    div { class: "hidden sm:block text-[10px] text-primary-600 truncate mt-0.5",
                                                                        "📍 {location_str}"
                                                                    }
                                                                }
                                                            }
                                                            button {
                                                                class: "shrink-0 text-[10px] w-4 h-4 flex items-center justify-center text-gray-400 hover:text-red-500 hover:bg-red-50 rounded-full transition-colors opacity-0 group-hover:opacity-100",
                                                                onclick: move |e| {
                                                                    e.stop_propagation();
                                                                    pending_delete.set(del_id.clone());
                                                                },
                                                                "✕"
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
            }
        
        }

        // ── Modal ─────────────────────────────────────────────────────────
        if *modal_open.read() {
            MeetingFormModal {
                meeting: edit_target.read().clone(),
                prefill_date: prefill_date.read().clone(),
                users,
                privs,
                name_fmt: name_fmt.read().clone(),
                on_close: move |_| modal_open.set(false),
                on_saved: move |_| {
                    meetings_res.restart();
                    modal_open.set(false);
                },
            }
        }
    }
}

// ── MeetingFormModal ──────────────────────────────────────────────────────────

#[component]
fn MeetingFormModal(
    meeting: Option<FieldServiceMeeting>,
    prefill_date: String,
    users: Vec<User>,
    privs: Vec<UserPrivileges>,
    name_fmt: NameFormat,
    on_close: Callback<()>,
    on_saved: Callback<()>,
) -> Element {
    let db_signal = use_db();
    let mut open = use_signal(|| true);

    let mut form = use_signal(|| {
        if let Some(ref m) = meeting {
            MeetingFormState {
                date: m.date.clone(),
                location: m.location.clone(),
                assignee_id: rid_str(&m.assignee),
                notes: m.notes.clone().unwrap_or_default(),
            }
        } else {
            MeetingFormState {
                date: prefill_date.clone(),
                ..Default::default()
            }
        }
    });

    let mut assign_to_anyone = use_signal(|| false);
    let mut submitting = use_signal(|| false);
    let mut error: Signal<Option<String>> = use_signal(|| None);

    // Build privileged / other user lists
    let priv_ids: HashSet<String> = privs
        .iter()
        .filter(|p| p.field_service_meeting)
        .map(|p| rid_str(&p.publisher))
        .collect();

    let mut privileged: Vec<User> = users
        .iter()
        .filter(|u| {
            u.active && u.id.as_ref().map(|id| priv_ids.contains(&rid_str(id))).unwrap_or(false)
        })
        .cloned()
        .collect();

    let others: Vec<User> = users
        .iter()
        .filter(|u| {
            u.active && !u.id.as_ref().map(|id| priv_ids.contains(&rid_str(id))).unwrap_or(false)
        })
        .cloned()
        .collect();

    // If no privileged users exist yet, show everyone in the main list
    if privileged.is_empty() {
        privileged = users.iter().filter(|u| u.active).cloned().collect();
    }
    let others = if priv_ids.is_empty() { vec![] } else { others };

    let other_ids: HashSet<String> = others
        .iter()
        .filter_map(|u| u.id.as_ref().map(rid_str))
        .collect();

    let is_edit = meeting.is_some();
    let edit_id = meeting.as_ref().and_then(|m| m.id.clone());

    let on_submit = move |_: Event<MouseData>| {
        let f = form.read().clone();
        if f.date.is_empty() || f.location.trim().is_empty() || f.assignee_id.is_empty() {
            error.set(Some(t!("fsm-meeting-form-required-error")));
            return;
        }
        let assignee_rid = match surrealdb::types::RecordId::parse_simple(&f.assignee_id) {
            Ok(r) => r,
            Err(_) => {
                error.set(Some(t!("fsm-meeting-form-required-error")));
                return;
            }
        };
        submitting.set(true);
        error.set(None);
        let data = FieldServiceMeetingData {
            date: f.date,
            location: f.location,
            assignee: assignee_rid,
            notes: if f.notes.is_empty() { None } else { Some(f.notes) },
        };
        let eid = edit_id.clone();
        spawn(async move {
            let Some(db) = db_signal.read().db.clone() else {
                submitting.set(false);
                return;
            };
            let res = if let Some(id) = eid {
                FieldServiceMeeting::update(&db, id, data).await.map(|_| ())
            } else {
                FieldServiceMeeting::create(&db, data).await.map(|_| ())
            };
            submitting.set(false);
            match res {
                Ok(_) => on_saved.call(()),
                Err(e) => error.set(Some(e.to_string())),
            }
        });
    };

    rsx! {
        ResponsiveModal {
            open,
            on_close: move |_| {
                open.set(false);
                on_close.call(());
            },
            title: if is_edit { t!("fsm-meeting-form-title-edit") } else { t!("fsm-meeting-form-title-new") },
            description: String::new(),
            submitting: *submitting.read(),
            on_submit,

            div { class: "space-y-4",
                if let Some(err) = error.read().clone() {
                    div { class: "bg-red-50 border border-red-200 rounded-lg p-3 text-red-700 text-sm",
                        "{err}"
                    }
                }

                // Date
                div {
                    label { class: "block text-sm font-medium text-gray-700 mb-1",
                        {t!("fsm-meeting-form-date")}
                        span { class: "text-red-500 ml-0.5", "*" }
                    }
                    input {
                        r#type: "date",
                        class: "w-full border border-gray-300 rounded-lg px-3 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-primary-500",
                        value: form.read().date.clone(),
                        oninput: move |e| form.write().date = e.value(),
                    }
                }

                // Location
                div {
                    label { class: "block text-sm font-medium text-gray-700 mb-1",
                        {t!("fsm-meeting-form-location")}
                        span { class: "text-red-500 ml-0.5", "*" }
                    }
                    input {
                        r#type: "text",
                        class: "w-full border border-gray-300 rounded-lg px-3 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-primary-500",
                        placeholder: t!("fsm-meeting-form-location-placeholder"),
                        value: form.read().location.clone(),
                        oninput: move |e| form.write().location = e.value(),
                    }
                }

                // Assignee
                div {
                    label { class: "block text-sm font-medium text-gray-700 mb-1",
                        {t!("fsm-meeting-form-assignee")}
                        span { class: "text-red-500 ml-0.5", "*" }
                    }
                    select {
                        class: "w-full border border-gray-300 rounded-lg px-3 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-primary-500 bg-white",
                        value: form.read().assignee_id.clone(),
                        onchange: move |e| form.write().assignee_id = e.value(),
                        option {
                            value: "",
                            disabled: true,
                            selected: form.read().assignee_id.is_empty(),
                            {t!("fsm-meeting-form-assignee-select")}
                        }
                        for u in privileged.clone() {
                            {
                                let id = u.id.as_ref().map(rid_str).unwrap_or_default();
                                let name = format_name(&u.first_name, &u.last_name, &name_fmt);
                                let selected = form.read().assignee_id == id;
                                rsx! {
                                    option { value: "{id}", selected, "{name}" }
                                }
                            }
                        }
                        if *assign_to_anyone.read() && !others.is_empty() {
                            option { value: "", disabled: true, "────────" }
                            for u in others.clone() {
                                {
                                    let id = u.id.as_ref().map(rid_str).unwrap_or_default();
                                    let name = format_name(&u.first_name, &u.last_name, &name_fmt);
                                    let selected = form.read().assignee_id == id;
                                    rsx! {
                                        option { value: "{id}", selected, "{name}" }
                                    }
                                }
                            }
                        }
                    }

                    // "Assign to anyone" toggle — only shown when non-privileged users exist
                    if !others.is_empty() {
                        label { class: "flex items-center gap-2 mt-2 cursor-pointer select-none",
                            input {
                                r#type: "checkbox",
                                class: "w-4 h-4 rounded border-gray-300 accent-primary-600",
                                checked: *assign_to_anyone.read(),
                                onchange: {
                                    let other_ids = other_ids.clone();
                                    move |e| {
                                        let checked = e.checked();
                                        assign_to_anyone.set(checked);
                                        // If unchecking and current selection was an "other" user, reset
                                        if !checked {
                                            let cur = form.read().assignee_id.clone();
                                            if other_ids.contains(&cur) {
                                                form.write().assignee_id = String::new();
                                            }
                                        }
                                    }
                                },
                            }
                            span { class: "text-sm text-gray-600", {t!("fsm-assign-to-anyone")} }
                        }
                    }
                }

                // Notes
                div {
                    label { class: "block text-sm font-medium text-gray-700 mb-1",
                        {t!("fsm-meeting-form-notes")}
                    }
                    textarea {
                        class: "w-full border border-gray-300 rounded-lg px-3 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-primary-500 resize-none",
                        rows: "3",
                        placeholder: t!("fsm-meeting-form-notes-placeholder"),
                        value: form.read().notes.clone(),
                        oninput: move |e| form.write().notes = e.value(),
                    }
                }
            }
        }
    }
}

