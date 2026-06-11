use dioxus::prelude::*;
use dioxus_i18n::t;
use surrealdb::types::RecordId;

use crate::components::ResponsiveModal;
use crate::database::{use_crypto, use_db};
use crate::models::absence::{Absence, AbsenceData};
use crate::models::congregation::{Congregation, DateFormat, NameFormat};
use crate::models::user::User;
use crate::pages::app::user::{
    effective_date_format, effective_name_format, format_date, format_name,
};

// ── Platform date helpers ─────────────────────────────────────────────────────

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
    "2026-06-10".to_string()
}

/// Returns yesterday's ISO date string — used as the expiry threshold.
fn yesterday_iso() -> String {
    let today = today_iso();
    if today.len() != 10 {
        return today;
    }
    let parts: Vec<&str> = today.splitn(3, '-').collect();
    if parts.len() != 3 {
        return today;
    }
    let y: i32 = parts[0].parse().unwrap_or(2026);
    let m: u32 = parts[1].parse().unwrap_or(1);
    let d: u32 = parts[2].parse().unwrap_or(1);
    if d > 1 {
        return format!("{:04}-{:02}-{:02}", y, m, d - 1);
    }
    let (py, pm) = if m == 1 { (y - 1, 12u32) } else { (y, m - 1) };
    let last_day = match pm {
        4 | 6 | 9 | 11 => 30,
        2 => if (py % 4 == 0 && py % 100 != 0) || py % 400 == 0 { 29 } else { 28 },
        _ => 31,
    };
    format!("{:04}-{:02}-{:02}", py, pm, last_day)
}

fn iso_month(iso: &str) -> Option<u8> { iso.get(5..7)?.parse().ok() }
fn iso_year(iso: &str) -> Option<i32> { iso.get(0..4)?.parse().ok() }

fn current_year_month() -> (i32, u8) {
    let today = today_iso();
    let y = iso_year(&today).unwrap_or(2026);
    let m = iso_month(&today).unwrap_or(6);
    (y, m)
}

fn normalize(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            'á'|'à'|'ä'|'â'|'ã'|'Á'|'À'|'Ä'|'Â'|'Ã' => 'a',
            'é'|'è'|'ë'|'ê'|'É'|'È'|'Ë'|'Ê' => 'e',
            'í'|'ì'|'ï'|'î'|'Í'|'Ì'|'Ï'|'Î' => 'i',
            'ó'|'ò'|'ö'|'ô'|'õ'|'Ó'|'Ò'|'Ö'|'Ô'|'Õ' => 'o',
            'ú'|'ù'|'ü'|'û'|'Ú'|'Ù'|'Ü'|'Û' => 'u',
            'ñ'|'Ñ' => 'n', 'ç'|'Ç' => 'c',
            other => other.to_ascii_lowercase(),
        })
        .collect()
}

fn record_id_str(id: &RecordId) -> String {
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

const PAGE_SIZE: usize = 20;

// ── Filter state ──────────────────────────────────────────────────────────────

#[derive(Clone, Default, PartialEq)]
struct Filters {
    user_search: String,
    ongoing_only: bool,
}

// ── Form state ────────────────────────────────────────────────────────────────

#[derive(Clone, Default)]
struct AbsenceFormState {
    user_id: String,
    start_date: String,
    end_date: String,
    reason: String,
    whole_congregation: bool,
    submitting: bool,
    error: Option<String>,
}

// ── Main page ─────────────────────────────────────────────────────────────────

#[component]
pub fn AppAbsences() -> Element {
    let db_signal = use_db();
    let crypto_signal = use_crypto();
    let congregation_res = use_context::<Resource<Option<Congregation>>>();
    let uid = db_signal.read().congregation_uid.clone().unwrap_or_default();

    // Effective display formats
    let mut name_fmt = use_signal(|| NameFormat::FirstLast);
    let mut date_fmt = use_signal(|| DateFormat::YMD);
    {
        let uid = uid.clone();
        use_effect(move || {
            let uid = uid.clone();
            let cong_snap = congregation_res.read().clone();
            let db_opt = db_signal.read().db.clone();
            spawn(async move {
                let prefs = crate::pages::app::user_settings::load_prefs(&uid, db_opt).await;
                let cong_ref = cong_snap.as_ref().and_then(|o| o.as_ref());
                name_fmt.set(effective_name_format(cong_ref, prefs.name_format.as_deref().unwrap_or("")));
                date_fmt.set(effective_date_format(cong_ref, prefs.date_format.as_deref().unwrap_or("")));
            });
        });
    }

    // Resources
    let mut users_res = use_resource(move || async move {
        let Some(db) = db_signal.read().db.clone() else { return vec![] };
        let crypto = crypto_signal.read().clone();
        User::all(&db, &crypto).await.unwrap_or_default()
    });

    let mut absences_res = use_resource(move || async move {
        let Some(db) = db_signal.read().db.clone() else { return vec![] };
        let crypto = crypto_signal.read().clone();
        // Auto-purge absences whose end_date < yesterday
        let _ = Absence::delete_expired(&db, &crypto, &yesterday_iso()).await;
        let mut rows = Absence::all(&db, &crypto).await.unwrap_or_default();
        rows.sort_by(|a, b| b.start_date.cmp(&a.start_date));
        rows
    });

    let mut bootstrapped = use_signal(|| false);
    use_effect(move || {
        if *bootstrapped.peek() { return; }
        bootstrapped.set(true);
        users_res.restart();
        absences_res.restart();
    });

    // Modal signals
    let mut modal_open = use_signal(|| false);
    let mut editing: Signal<Option<Absence>> = use_signal(|| None);
    let mut delete_open = use_signal(|| false);
    let mut delete_id: Signal<Option<RecordId>> = use_signal(|| None);

    // Month/year navigation
    let (cur_year, cur_month) = current_year_month();
    let mut sel_year = use_signal(|| cur_year);
    let mut sel_month = use_signal(|| cur_month);
    let mut show_picker = use_signal(|| false);

    // Filters + pagination
    let mut filters = use_signal(Filters::default);
    let mut display_limit = use_signal(|| PAGE_SIZE);

    let is_loading = absences_res.read().is_none() || users_res.read().is_none();
    let users: Vec<User> = users_res().unwrap_or_default();
    // Years present in data (for year filter) — reactive read
    let years: Vec<i32> = {
        let abs = absences_res().unwrap_or_default();
        let mut ys: Vec<i32> = abs
            .iter()
            .filter_map(|a| iso_year(&a.start_date))
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .collect();
        ys.sort_unstable();
        ys
    };
    let _ = years; // no longer used in dropdowns

    // Pre-compute localized month abbreviations for the picker grid
    let month_picker_labels: Vec<String> = vec![
        t!("month-1"), t!("month-2"), t!("month-3"),
        t!("month-4"), t!("month-5"), t!("month-6"),
        t!("month-7"), t!("month-8"), t!("month-9"),
        t!("month-10"), t!("month-11"), t!("month-12"),
    ]
    .into_iter()
    .map(|s| s.chars().take(3).collect::<String>())
    .collect();

    // Filtered + sorted rows, returning pre-computed display strings so user_map
    // and absences_res are read reactively INSIDE the memo.
    type FilteredRow = (Absence, String, String, Option<String>); // absence, user_name, start, end
    let filtered = use_memo(move || -> Vec<FilteredRow> {
        // Reactive reads — these make the memo re-run when resources update.
        let absences: Vec<Absence> = absences_res().unwrap_or_default();
        let users_snap: Vec<User> = users_res().unwrap_or_default();
        let f = filters();
        let norm = normalize(&f.user_search);
        let name_fmt_snap = name_fmt.read().clone();
        let date_fmt_snap = date_fmt.read().clone();
        let sel_m = sel_month();
        let sel_y = sel_year();

        let um: std::collections::HashMap<String, (String, String)> = users_snap
            .iter()
            .filter_map(|u| {
                let id = u.id.as_ref().map(record_id_str)?;
                Some((id, (u.first_name.clone(), u.last_name.clone())))
            })
            .collect();

        let mut result: Vec<FilteredRow> = absences
            .iter()
            .filter(|a| {
                // Always filter by selected month/year
                if iso_month(&a.start_date) != Some(sel_m) { return false; }
                if iso_year(&a.start_date) != Some(sel_y) { return false; }
                if !norm.is_empty() {
                    let uid_str = record_id_str(&a.user);
                    let full = um
                        .get(&uid_str)
                        .map(|(first, last)| normalize(&format!("{} {}", first, last)))
                        .unwrap_or_default();
                    if !full.contains(&norm) { return false; }
                }
                if f.ongoing_only && a.end_date.is_some() { return false; }
                true
            })
            .map(|a| {
                let uid_str = record_id_str(&a.user);
                let (first, last) = um.get(&uid_str).cloned().unwrap_or_default();
                let user_name = format_name(&first, &last, &name_fmt_snap);
                let start = format_date(&a.start_date, &date_fmt_snap);
                let end = a.end_date.as_deref().map(|d| format_date(d, &date_fmt_snap));
                (a.clone(), user_name, start, end)
            })
            .collect();

        result.sort_by(|a, b| {
            b.0.start_date.cmp(&a.0.start_date).then_with(|| {
                normalize(&a.1).cmp(&normalize(&b.1))
            })
        });
        result
    });

    let total = filtered.read().len();
    let limit = *display_limit.read();
    let shown: Vec<(Absence, String, String, Option<String>)> = filtered.read()[..limit.min(total)].to_vec();
    let has_more = limit < total;

    rsx! {
        div { class: "relative space-y-4 w-full pb-24",

            // ── Month / year navigation ───────────────────────────────────
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
                            display_limit.set(PAGE_SIZE);
                        },
                        "‹"
                    }

                    // Clickable month/year label — opens picker
                    {
                        let month_full = match sel_month() {
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
                            12 => t!("month-12"),
                            _ => t!("month-1"),
                        };
                        let month_abbr = month_full.chars().take(3).collect::<String>();
                        let year_str = sel_year().to_string();
                        let is_ymd = matches!(date_fmt(), DateFormat::YMD);
                        let btn_cls = if show_picker() {
                            "flex-1 inline-flex items-center justify-center gap-1 px-3 py-1.5 rounded-lg border border-primary-400 text-primary-700 font-semibold bg-primary-50 transition-colors"
                        } else {
                            "flex-1 inline-flex items-center justify-center gap-1 px-3 py-1.5 rounded-lg border border-dashed border-gray-300 text-gray-900 font-semibold hover:border-primary-400 hover:text-primary-600 hover:bg-primary-50 transition-colors"
                        };
                        rsx! {
                            button { class: btn_cls, onclick: move |_| show_picker.set(!show_picker()),
                                if is_ymd {
                                    span { "{year_str}" }
                                    span { class: "hidden sm:inline", " {month_full}" }
                                    span { class: "sm:hidden", " {month_abbr}" }
                                } else {
                                    span { class: "hidden sm:inline", "{month_full}" }
                                    span { class: "sm:hidden", "{month_abbr}" }
                                    span { " {year_str}" }
                                }
                                span { class: "text-xs text-gray-400",
                                    if show_picker() {
                                        "▴"
                                    } else {
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
                            display_limit.set(PAGE_SIZE);
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
                                onclick: move |_| {
                                    sel_year.set(sel_year() - 1);
                                    display_limit.set(PAGE_SIZE);
                                },
                                "−"
                            }
                            span { class: "text-sm font-medium text-gray-800 w-12 text-center",
                                "{sel_year()}"
                            }
                            button {
                                class: "px-3 py-1 text-sm border border-gray-200 rounded-lg text-gray-600 hover:bg-gray-50",
                                onclick: move |_| {
                                    sel_year.set(sel_year() + 1);
                                    display_limit.set(PAGE_SIZE);
                                },
                                "＋"
                            }
                        }
                        div { class: "grid grid-cols-4 gap-1.5",
                            for (idx , abbr) in month_picker_labels.iter().enumerate() {
                                {
                                    let m = (idx + 1) as u8;
                                    let abbr = abbr.clone();
                                    let is_sel = sel_month() == m;
                                    let cls = if is_sel {
                                        "py-1.5 text-xs rounded-lg text-center bg-primary-600 text-white font-medium"
                                    } else {
                                        "py-1.5 text-xs rounded-lg text-center border border-gray-200 text-gray-700 hover:bg-gray-50 cursor-pointer"
                                    };
                                    rsx! {
                                        button {
                                            class: cls,
                                            onclick: move |_| {
                                                sel_month.set(m);
                                                show_picker.set(false);
                                                display_limit.set(PAGE_SIZE);
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

            // ── Filter card ────────────────────────────────────────────────
            div { class: "bg-white rounded-xl border border-gray-200 p-4 space-y-3",
                input {
                    r#type: "text",
                    class: "w-full px-3 py-2 text-sm border border-gray-200 rounded-lg focus:outline-none focus:ring-2 focus:ring-primary-500 placeholder-gray-400",
                    placeholder: t!("absence-filter-search-placeholder"),
                    value: filters.read().user_search.clone(),
                    oninput: move |e| {
                        filters.write().user_search = e.value();
                        display_limit.set(PAGE_SIZE);
                    },
                }
                label { class: "flex items-center gap-2 px-1 cursor-pointer",
                    input {
                        r#type: "checkbox",
                        class: "w-4 h-4 rounded border-gray-300 accent-primary-600",
                        checked: filters.read().ongoing_only,
                        onchange: move |e| {
                            filters.write().ongoing_only = e.checked();
                            display_limit.set(PAGE_SIZE);
                        },
                    }
                    span { class: "text-sm text-gray-700", {t!("absence-filter-ongoing")} }
                }
            }

            // ── List ──────────────────────────────────────────────────────
            if is_loading {
                div { class: "flex justify-center items-center py-20 text-gray-400",
                    span { class: "text-sm animate-pulse", {t!("absence-loading")} }
                }
            } else if shown.is_empty() {
                div { class: "bg-white rounded-xl border border-gray-200",
                    div { class: "px-6 py-16 text-center",
                        p { class: "text-5xl mb-3", "📅" }
                        p { class: "font-medium text-gray-600", {t!("empty-absences-title")} }
                        p { class: "text-sm mt-1 text-gray-400", {t!("empty-absences-desc")} }
                    }
                }
            } else {
                div { class: "space-y-2",
                    for (absence , user_name , start , end) in shown {
                        {
                            let abs_edit = absence.clone();
                            let abs_del_id = absence.id.clone();
                            rsx! {
                                AbsenceCard {
                                    user_name,
                                    start_date: start,
                                    end_date: end,
                                    reason: absence.reason.clone(),
                                    on_edit: move |_| {
                                        editing.set(Some(abs_edit.clone()));
                                        modal_open.set(true);
                                    },
                                    on_delete: move |_| {
                                        delete_id.set(abs_del_id.clone());
                                        delete_open.set(true);
                                    },
                                }
                            }
                        }
                    }
                    if has_more {
                        div { class: "flex justify-center pt-2",
                            button {
                                class: "px-5 py-2 text-sm text-primary-600 border border-primary-200 rounded-full hover:bg-primary-50 transition-colors",
                                onclick: move |_| {
                                    let cur = *display_limit.read();
                                    display_limit.set(cur + PAGE_SIZE);
                                },
                                {t!("absence-load-more")}
                            }
                        }
                    }
                }
            }

            // ── FAB ───────────────────────────────────────────────────────
            button {
                class: "fixed bottom-20 right-6 md:bottom-6 w-14 h-14 bg-primary-600 text-white rounded-full shadow-xl hover:bg-primary-700 active:scale-95 transition-all flex items-center justify-center text-2xl z-20 select-none",
                onclick: move |_| {
                    editing.set(None);
                    modal_open.set(true);
                },
                "＋"
            }
        }

        // ── Add / edit modal ───────────────────────────────────────────────
        AbsenceFormModal {
            open: modal_open,
            existing: editing,
            users: users_res().unwrap_or_default(),
            on_close: move |_| modal_open.set(false),
            on_saved: move |_| {
                absences_res.restart();
                modal_open.set(false);
            },
            date_fmt: date_fmt.read().clone(),
            name_fmt: name_fmt.read().clone(),
        }

        // ── Delete confirmation ────────────────────────────────────────────
        ConfirmDeleteModal {
            open: delete_open,
            on_close: move |_| delete_open.set(false),
            on_confirm: move |_| {
                if let Some(id) = delete_id.read().clone() {
                    spawn(async move {
                        let Some(db) = db_signal.read().db.clone() else { return };
                        let _ = Absence::delete(&db, id).await;
                        absences_res.restart();
                    });
                }
                delete_open.set(false);
            },
        }
    }
}

// ── AbsenceCard ───────────────────────────────────────────────────────────────

#[component]
fn AbsenceCard(
    user_name: String,
    start_date: String,
    end_date: Option<String>,
    reason: Option<String>,
    on_edit: Callback<()>,
    on_delete: Callback<()>,
) -> Element {
    let initials: String = user_name
        .split_whitespace()
        .take(2)
        .filter_map(|w| w.chars().next())
        .map(|c| c.to_ascii_uppercase())
        .collect();

    let date_range = match &end_date {
        Some(end) => format!("{} → {}", start_date, end),
        None => format!("{} → …", start_date),
    };
    let is_ongoing = end_date.is_none();

    rsx! {
        div {
            class: "bg-white rounded-xl border border-gray-200 px-4 py-3 flex items-center gap-3 hover:border-primary-200 hover:shadow-sm transition-all cursor-pointer",
            onclick: move |_| on_edit.call(()),
            div { class: "w-10 h-10 rounded-full bg-primary-100 text-primary-700 flex items-center justify-center font-semibold text-sm shrink-0",
                "{initials}"
            }
            div { class: "flex-1 min-w-0",
                div { class: "text-sm font-medium text-gray-900 truncate", "{user_name}" }
                div { class: "flex items-center gap-2 mt-0.5 flex-wrap",
                    span { class: "text-xs text-gray-500", "{date_range}" }
                    if is_ongoing {
                        span { class: "inline-flex px-1.5 py-0.5 rounded text-xs font-medium bg-amber-500 text-white",
                            {t!("absence-badge-ongoing")}
                        }
                    }
                    if let Some(r) = &reason {
                        if !r.is_empty() {
                            span { class: "text-xs text-gray-400 italic truncate", "· {r}" }
                        }
                    }
                }
            }
            button {
                class: "shrink-0 p-1.5 text-gray-300 hover:text-red-500 hover:bg-red-50 rounded-lg transition-colors",
                onclick: move |e| {
                    e.stop_propagation();
                    on_delete.call(());
                },
                svg {
                    xmlns: "http://www.w3.org/2000/svg",
                    fill: "none",
                    view_box: "0 0 24 24",
                    stroke_width: "1.5",
                    stroke: "currentColor",
                    class: "w-4 h-4",
                    path {
                        stroke_linecap: "round",
                        stroke_linejoin: "round",
                        d: "M14.74 9l-.346 9m-4.788 0L9.26 9m9.968-3.21c.342.052.682.107 1.022.166m-1.022-.165L18.16 19.673a2.25 2.25 0 01-2.244 2.077H8.084a2.25 2.25 0 01-2.244-2.077L4.772 5.79m14.456 0a48.108 48.108 0 00-3.478-.397m-12 .562c.34-.059.68-.114 1.022-.165m0 0a48.11 48.11 0 013.478-.397m7.5 0v-.916c0-1.18-.91-2.164-2.09-2.201a51.964 51.964 0 00-3.32 0c-1.18.037-2.09 1.022-2.09 2.201v.916m7.5 0a48.667 48.667 0 00-7.5 0",
                    }
                }
            }
        }
    }
}

// ── Absence form modal ────────────────────────────────────────────────────────

#[component]
fn AbsenceFormModal(
    open: Signal<bool>,
    existing: Signal<Option<Absence>>,
    users: Vec<User>,
    on_close: Callback<()>,
    on_saved: Callback<()>,
    date_fmt: DateFormat,
    name_fmt: NameFormat,
) -> Element {
    let db_signal = use_db();
    let crypto_signal = use_crypto();
    let mut form = use_signal(AbsenceFormState::default);

    use_effect(move || {
        if *open.read() {
            match existing.read().as_ref() {
                Some(a) => form.set(AbsenceFormState {
                    user_id: record_id_str(&a.user),
                    start_date: a.start_date.clone(),
                    end_date: a.end_date.clone().unwrap_or_default(),
                    reason: a.reason.clone().unwrap_or_default(),
                    whole_congregation: false,
                    ..Default::default()
                }),
                None => form.set(AbsenceFormState::default()),
            }
        }
    });

    let f = form.read().clone();
    let is_edit = existing.read().is_some();
    let existing_id = existing.read().as_ref().and_then(|a| a.id.clone());
    let all_users = users.clone();

    let on_submit = use_callback(move |_: Event<MouseData>| {
        let fd = form.read().clone();
        if fd.start_date.is_empty() || (!fd.whole_congregation && fd.user_id.is_empty()) {
            form.write().error = Some(t!("absence-form-required-error"));
            return;
        }
        if !fd.end_date.is_empty() && fd.end_date < fd.start_date {
            form.write().error = Some(t!("absence-form-date-order-error"));
            return;
        }

        let users_to_save: Vec<String> = if fd.whole_congregation {
            all_users.iter().filter_map(|u| u.id.as_ref().map(record_id_str)).collect()
        } else {
            vec![fd.user_id.clone()]
        };

        let start = fd.start_date.clone();
        let end_opt = (!fd.end_date.is_empty()).then(|| fd.end_date.clone());
        let reason_opt = (!fd.reason.trim().is_empty()).then(|| fd.reason.trim().to_string());
        let eid = existing_id.clone();

        form.write().submitting = true;
        form.write().error = None;

        spawn(async move {
            let Some(db) = db_signal.read().db.clone() else { form.write().submitting = false; return; };
            let crypto = crypto_signal.read().clone();

            if let Some(rid) = eid {
                // Update single existing absence
                let uid_str = users_to_save.first().cloned().unwrap_or_default();
                match RecordId::parse_simple(&uid_str) {
                    Ok(user_rid) => {
                        let data = AbsenceData { user: user_rid, start_date: start, end_date: end_opt, reason: reason_opt };
                        match Absence::update(&db, &crypto, rid, data).await {
                            Ok(_) => on_saved.call(()),
                            Err(e) => { form.write().submitting = false; form.write().error = Some(e.to_string()); }
                        }
                    }
                    Err(_) => { form.write().submitting = false; form.write().error = Some("Invalid user ID".to_string()); }
                }
            } else {
                // Create one absence per user in the list
                let mut last_err: Option<String> = None;
                for uid_str in &users_to_save {
                    if let Ok(user_rid) = RecordId::parse_simple(uid_str) {
                        let data = AbsenceData {
                            user: user_rid,
                            start_date: start.clone(),
                            end_date: end_opt.clone(),
                            reason: reason_opt.clone(),
                        };
                        if let Err(e) = Absence::create(&db, &crypto, data).await {
                            last_err = Some(e.to_string());
                        }
                    }
                }
                match last_err {
                    Some(err) => { form.write().submitting = false; form.write().error = Some(err); }
                    None => on_saved.call(()),
                }
            }
        });
    });

    let date_hint = crate::pages::app::user::date_format_hint(&date_fmt);
    let title = if is_edit { t!("absence-form-title-edit") } else { t!("absence-form-title-add") };

    rsx! {
        ResponsiveModal {
            open,
            on_close,
            title,
            description: String::new(),
            submitting: f.submitting,
            on_submit,

            if let Some(err) = &f.error {
                div { class: "bg-red-50 border border-red-200 text-red-700 text-sm px-3 py-2 rounded-lg",
                    "{err}"
                }
            }

            // Whole-congregation toggle (add only)
            if !is_edit {
                label { class: "flex items-center gap-3 py-1 cursor-pointer",
                    input {
                        r#type: "checkbox",
                        class: "w-4 h-4 rounded border-gray-300 accent-primary-600",
                        checked: f.whole_congregation,
                        onchange: move |e| form.write().whole_congregation = e.checked(),
                    }
                    span { class: "text-sm font-medium text-gray-700",
                        {t!("absence-form-whole-congregation")}
                    }
                }
            }

            // User selector (hidden when whole_congregation)
            if !f.whole_congregation {
                div { class: "flex flex-col gap-1",
                    label { class: "text-xs font-medium text-gray-700",
                        {t!("absence-form-user")}
                        span { class: "text-red-500 ml-0.5", " *" }
                    }
                    select {
                        class: "w-full px-3 py-2 text-sm border border-gray-200 rounded-lg bg-white focus:outline-none focus:ring-2 focus:ring-primary-500",
                        disabled: is_edit,
                        onchange: move |e| form.write().user_id = e.value(),
                        option { value: "", {t!("absence-form-select-user")} }
                        for u in users.iter() {
                            {
                                let uid = u.id.as_ref().map(record_id_str).unwrap_or_default();
                                let uname = format_name(&u.first_name, &u.last_name, &name_fmt);
                                let selected = f.user_id == uid;
                                rsx! {
                                    option { value: "{uid}", selected, "{uname}" }
                                }
                            }
                        }
                    }
                }
            }

            // Start + end dates
            div { class: "grid grid-cols-2 gap-3",
                div { class: "flex flex-col gap-1",
                    label { class: "text-xs font-medium text-gray-700",
                        {t!("absence-form-start-date")}
                        span { class: "text-red-500 ml-0.5", " *" }
                        span { class: "text-xs text-gray-400 ml-1", "({date_hint})" }
                    }
                    input {
                        r#type: "date",
                        class: "w-full px-3 py-2 text-sm border border-gray-200 rounded-lg focus:outline-none focus:ring-2 focus:ring-primary-500",
                        value: f.start_date.clone(),
                        oninput: move |e| form.write().start_date = e.value(),
                    }
                }
                div { class: "flex flex-col gap-1",
                    label { class: "text-xs font-medium text-gray-700",
                        {t!("absence-form-end-date")}
                        span { class: "text-xs text-gray-400 ml-1", "({date_hint})" }
                    }
                    input {
                        r#type: "date",
                        class: "w-full px-3 py-2 text-sm border border-gray-200 rounded-lg focus:outline-none focus:ring-2 focus:ring-primary-500",
                        value: f.end_date.clone(),
                        oninput: move |e| form.write().end_date = e.value(),
                    }
                }
            }

            // Reason
            div { class: "flex flex-col gap-1",
                label { class: "text-xs font-medium text-gray-700", {t!("absence-form-reason")} }
                textarea {
                    class: "w-full px-3 py-2 text-sm border border-gray-200 rounded-lg focus:outline-none focus:ring-2 focus:ring-primary-500 resize-none",
                    rows: "3",
                    placeholder: t!("absence-form-reason-placeholder"),
                    value: f.reason.clone(),
                    oninput: move |e| form.write().reason = e.value(),
                }
            }
        }
    }
}

// ── Confirm delete modal ──────────────────────────────────────────────────────

#[component]
fn ConfirmDeleteModal(
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
                h2 { class: "text-base font-semibold text-gray-900", {t!("absence-delete-title")} }
                p { class: "text-sm text-gray-600", {t!("absence-delete-confirm")} }
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
