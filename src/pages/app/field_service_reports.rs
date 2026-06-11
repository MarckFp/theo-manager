use std::collections::HashMap;

use dioxus::prelude::*;
use dioxus_i18n::t;
use surrealdb::types::RecordId;

use crate::components::ResponsiveModal;
use crate::database::{use_crypto, use_db};
use crate::models::congregation::{Congregation, DateFormat, NameFormat};
use crate::models::field_service_report::{FieldServiceReport, FieldServiceReportData};
use crate::models::user::{Appointment, User, UserType};
use crate::pages::app::user::{effective_date_format, effective_name_format, format_name};
use crate::pages::app::user_detail::always_show_hours;

// ── Platform helpers ──────────────────────────────────────────────────────────

#[cfg(target_arch = "wasm32")]
fn current_year_month() -> (i32, u8) {
    let d = js_sys::Date::new_0();
    (d.get_full_year() as i32, (d.get_month() + 1) as u8)
}

#[cfg(not(target_arch = "wasm32"))]
fn current_year_month() -> (i32, u8) {
    (2026, 6)
}

fn month_name(m: u8) -> &'static str {
    match m {
        1 => "January",
        2 => "February",
        3 => "March",
        4 => "April",
        5 => "May",
        6 => "June",
        7 => "July",
        8 => "August",
        9 => "September",
        10 => "October",
        11 => "November",
        12 => "December",
        _ => "?",
    }
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

// ── Filter state ──────────────────────────────────────────────────────────────

#[derive(Clone, PartialEq, Default)]
enum AppointmentFilter {
    #[default]
    All,
    WithoutAppointment,
    Elder,
    MinisterialServant,
}

#[derive(Clone, PartialEq, Default)]
struct Filters {
    user_search: String,
    user_type: Option<UserType>,
    appointment: AppointmentFilter,
}

// ── Report form state (mirrors user_detail.rs) ────────────────────────────────

#[derive(Clone)]
struct ReportFormState {
    hours: String,
    credits: String,
    bible_studies: String,
    auxiliary_pioneer: bool,
    preached: bool,
    notes: String,
    submitting: bool,
    error: Option<String>,
}

impl Default for ReportFormState {
    fn default() -> Self {
        Self {
            preached: true,
            hours: String::new(),
            credits: String::new(),
            bible_studies: String::new(),
            auxiliary_pioneer: false,
            notes: String::new(),
            submitting: false,
            error: None,
        }
    }
}

impl ReportFormState {
    fn from_report(r: &FieldServiceReport) -> Self {
        Self {
            hours: r.hours.map(|v| v.to_string()).unwrap_or_default(),
            credits: r.credits.map(|v| v.to_string()).unwrap_or_default(),
            bible_studies: r.bible_studies.map(|v| v.to_string()).unwrap_or_default(),
            auxiliary_pioneer: r.auxiliary_pioneer,
            preached: r.preached,
            notes: r.notes.clone().unwrap_or_default(),
            ..Default::default()
        }
    }
}

// ── Editing state ─────────────────────────────────────────────────────────────

#[derive(Clone, PartialEq)]
struct EditTarget {
    publisher_id: RecordId,
    user_type: UserType,
    year: i32,
    month: u8,
    existing: Option<FieldServiceReport>,
}

// ── Main page ─────────────────────────────────────────────────────────────────

#[component]
pub fn AppFieldServiceReports() -> Element {
    let db_signal = use_db();
    let crypto_signal = use_crypto();
    let congregation_res = use_context::<Resource<Option<Congregation>>>();
    let uid = db_signal.read().congregation_uid.clone().unwrap_or_default();

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
                name_fmt.set(effective_name_format(
                    cong_ref,
                    prefs.name_format.as_deref().unwrap_or(""),
                ));
                date_fmt.set(effective_date_format(
                    cong_ref,
                    prefs.date_format.as_deref().unwrap_or(""),
                ));
            });
        });
    }

    let (cur_year, cur_month) = current_year_month();
    let mut sel_year = use_signal(|| cur_year);
    let mut sel_month = use_signal(|| cur_month);
    let mut show_picker = use_signal(|| false);

    let mut filters = use_signal(Filters::default);

    // ── Resources ─────────────────────────────────────────────────────────────
    let mut users_res = use_resource(move || async move {
        let Some(db) = db_signal.read().db.clone() else { return vec![] };
        let crypto = crypto_signal.read().clone();
        User::all(&db, &crypto).await.unwrap_or_default()
    });

    let mut reports_res = use_resource(move || {
        let y = sel_year();
        let m = sel_month();
        async move {
            let Some(db) = db_signal.read().db.clone() else { return vec![] };
            let crypto = crypto_signal.read().clone();
            FieldServiceReport::by_month(&db, &crypto, y, m)
                .await
                .unwrap_or_default()
        }
    });

    let mut restarted = use_signal(|| false);
    use_effect(move || {
        if *restarted.peek() { return; }
        restarted.set(true);
        users_res.restart();
        reports_res.restart();
    });

    // ── Modals ────────────────────────────────────────────────────────────────
    let mut view_open = use_signal(|| false);
    let mut viewing: Signal<Option<(FieldServiceReport, User)>> = use_signal(|| None);

    let mut edit_open = use_signal(|| false);
    let mut edit_target: Signal<Option<EditTarget>> = use_signal(|| None);

    let mut delete_open = use_signal(|| false);
    let mut delete_id: Signal<Option<RecordId>> = use_signal(|| None);

    // ── Computed: join + filter ───────────────────────────────────────────────
    let combined = use_memo(move || {
        let users = users_res().unwrap_or_default();
        let reports = reports_res().unwrap_or_default();
        let f = filters();
        let norm = normalize(&f.user_search);

        // Build report lookup: publisher_id_str -> report
        let report_map: HashMap<String, FieldServiceReport> = reports
            .into_iter()
            .filter_map(|r| {
                let key = rid_str(&r.publisher);
                Some((key, r))
            })
            .collect();

        let mut result: Vec<(User, Option<FieldServiceReport>)> = users
            .into_iter()
            .filter(|u| {
                // Exclude students from the list
                if matches!(u.user_type, UserType::Student) { return false; }

                if !norm.is_empty() {
                    let full = normalize(&format!("{} {}", u.first_name, u.last_name));
                    if !full.contains(&norm) { return false; }
                }
                if let Some(ref ut) = f.user_type {
                    if &u.user_type != ut { return false; }
                }
                match &f.appointment {
                    AppointmentFilter::All => {}
                    AppointmentFilter::WithoutAppointment => {
                        if u.appointment.is_some() { return false; }
                    }
                    AppointmentFilter::Elder => {
                        if !matches!(u.appointment, Some(Appointment::Elder)) { return false; }
                    }
                    AppointmentFilter::MinisterialServant => {
                        if !matches!(u.appointment, Some(Appointment::MinisterialServant)) {
                            return false;
                        }
                    }
                }
                true
            })
            .map(|u| {
                let uid_str = u.id.as_ref().map(rid_str).unwrap_or_default();
                let report = report_map.get(&uid_str).cloned();
                (u, report)
            })
            .collect();

        result.sort_by(|a, b| {
            let ka = normalize(&format!("{} {}", a.0.last_name, a.0.first_name));
            let kb = normalize(&format!("{} {}", b.0.last_name, b.0.first_name));
            ka.cmp(&kb)
        });
        result
    });

    let is_loading = users_res.read().is_none() || reports_res.read().is_none();
    let submitted_count = combined.read().iter().filter(|(_, r)| r.is_some()).count();
    let total_count = combined.read().len();

    rsx! {
        div { class: "space-y-5 w-full pb-10",

            // ── Header ────────────────────────────────────────────────────
            h1 { class: "text-2xl font-bold text-gray-900", {t!("page-field-service-reports")} }

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
                            reports_res.restart();
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
                            reports_res.restart();
                        },
                        "›"
                    }
                }

                // Month picker grid (shown when label clicked)
                if show_picker() {
                    div { class: "mt-3 space-y-3",
                        // Year stepper
                        div { class: "flex items-center justify-center gap-4",
                            button {
                                class: "px-3 py-1 text-sm border border-gray-200 rounded-lg text-gray-600 hover:bg-gray-50",
                                onclick: move |_| {
                                    sel_year.set(sel_year() - 1);
                                    reports_res.restart();
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
                                    reports_res.restart();
                                },
                                "＋"
                            }
                        }
                        // Month grid
                        div { class: "grid grid-cols-4 gap-1.5",
                            for m in 1u8..=12 {
                                {
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
                                                reports_res.restart();
                                            },
                                            {month_name(m).get(..3).unwrap_or(month_name(m))}
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // ── Filters ───────────────────────────────────────────────────
            div { class: "bg-white rounded-xl border border-gray-200 p-4 space-y-3",
                input {
                    r#type: "text",
                    class: "w-full px-3 py-2 text-sm border border-gray-200 rounded-lg focus:outline-none focus:ring-2 focus:ring-primary-500 placeholder-gray-400",
                    placeholder: t!("report-filter-user-placeholder"),
                    value: filters().user_search.clone(),
                    oninput: move |e| {
                        let mut f = filters();
                        f.user_search = e.value();
                        filters.set(f);
                    },
                }
                div { class: "grid grid-cols-2 gap-2",
                    div { class: "flex flex-col gap-1",
                        span { class: "text-xs font-medium text-gray-500", {t!("user-filter-type")} }
                        select {
                            class: "w-full px-2 py-1.5 text-sm border border-gray-200 rounded-lg bg-white focus:outline-none focus:ring-2 focus:ring-primary-500",
                            onchange: move |e| {
                                let mut f = filters();
                                f.user_type = match e.value().as_str() {
                                    "publisher" => Some(UserType::Publisher),
                                    "baptized" => Some(UserType::BaptizedPublisher),
                                    "cont_aux" => Some(UserType::ContinuousAuxiliaryPioneer),
                                    "regular" => Some(UserType::RegularPioneer),
                                    "special" => Some(UserType::SpecialPioneer),
                                    "missionary" => Some(UserType::Missionary),
                                    _ => None,
                                };
                                filters.set(f);
                            },
                            option { value: "", {t!("user-filter-all")} }
                            option { value: "publisher", {t!("user-type-publisher")} }
                            option { value: "baptized", {t!("user-type-baptized")} }
                            option { value: "cont_aux", {t!("user-type-cont-aux-pioneer")} }
                            option { value: "regular", {t!("user-type-regular-pioneer")} }
                            option { value: "special", {t!("user-type-special-pioneer")} }
                            option { value: "missionary", {t!("user-type-missionary")} }
                        }
                    }
                    div { class: "flex flex-col gap-1",
                        span { class: "text-xs font-medium text-gray-500",
                            {t!("user-filter-appointment")}
                        }
                        select {
                            class: "w-full px-2 py-1.5 text-sm border border-gray-200 rounded-lg bg-white focus:outline-none focus:ring-2 focus:ring-primary-500",
                            onchange: move |e| {
                                let mut f = filters();
                                f.appointment = match e.value().as_str() {
                                    "none" => AppointmentFilter::WithoutAppointment,
                                    "elder" => AppointmentFilter::Elder,
                                    "ms" => AppointmentFilter::MinisterialServant,
                                    _ => AppointmentFilter::All,
                                };
                                filters.set(f);
                            },
                            option { value: "", {t!("user-filter-all")} }
                            option { value: "none", {t!("user-appointment-none")} }
                            option { value: "elder", {t!("user-appointment-elder")} }
                            option { value: "ms", {t!("user-appointment-ms")} }
                        }
                    }
                }
            }

            // ── Summary bar ───────────────────────────────────────────────
            if !is_loading && total_count > 0 {
                div { class: "text-xs text-gray-500 px-1",
                    "{submitted_count} / {total_count} "
                    {t!("reports-submitted-of")}
                }
            }

            // ── Report list ───────────────────────────────────────────────
            if is_loading {
                div { class: "flex justify-center items-center py-20 text-gray-400",
                    span { class: "text-sm animate-pulse", {t!("reports-loading")} }
                }
            } else if combined.read().is_empty() {
                div { class: "bg-white rounded-xl border border-gray-200 px-6 py-16 text-center",
                    p { class: "text-4xl mb-3", "📊" }
                    p { class: "font-medium text-gray-600", {t!("reports-no-reports")} }
                }
            } else {
                div { class: "space-y-2",
                    for (user , report) in combined.read().clone() {
                        {
                            let u_view = user.clone();
                            let r_view = report.clone();
                            let u_edit = user.clone();
                            let r_edit = report.clone();
                            let r_del_id = report.as_ref().and_then(|r| r.id.clone());
                            let u_del = user.clone();
                            rsx! {
                                ReportCard {
                                    user: user.clone(),
                                    report: report.clone(),
                                    name_fmt: name_fmt.read().clone(),
                                    on_view: move |_| {
                                        if let Some(ref r) = r_view {
                                            viewing.set(Some((r.clone(), u_view.clone())));
                                            view_open.set(true);
                                        }
                                    },
                                    on_edit: move |_| {
                                        let publisher_id = u_edit
                                            .id
                                            .clone()
                                            .unwrap_or_else(|| { RecordId::parse_simple("user:unknown").unwrap() });
                                        edit_target
                                            .set(
                                                Some(EditTarget {
                                                    publisher_id,
                                                    user_type: u_edit.user_type.clone(),
                                                    year: sel_year(),
                                                    month: sel_month(),
                                                    existing: r_edit.clone(),
                                                }),
                                            );
                                        edit_open.set(true);
                                    },
                                    on_delete: move |_| {
                                        delete_id.set(r_del_id.clone());
                                        delete_open.set(true);
                                        // keep reference to user name for confirm dialog (unused here)
                                        let _ = u_del.first_name.clone();
                                    },
                                }
                            }
                        }
                    }
                }
            }
        }

        // ── View report detail modal ───────────────────────────────────────
        if let Some((report, user)) = viewing.read().clone() {
            ReportDetailModal {
                report,
                user,
                name_fmt: name_fmt.read().clone(),
                open: view_open,
                on_close: move |_| view_open.set(false),
            }
        }

        // ── Add / edit report modal ────────────────────────────────────────
        if let Some(target) = edit_target.read().clone() {
            ReportEditModal {
                target,
                open: edit_open,
                on_close: move |_| edit_open.set(false),
                on_saved: move |_| {
                    reports_res.restart();
                    edit_open.set(false);
                },
            }
        }

        // ── Delete confirmation ────────────────────────────────────────────
        {
            let is_open = *delete_open.read();
            let overlay_cls = if is_open {
                "fixed inset-0 z-50 flex items-center justify-center p-4 bg-black/40"
            } else {
                "fixed inset-0 z-50 flex items-center justify-center p-4 bg-black/40 opacity-0 pointer-events-none"
            };
            rsx! {
                div { class: overlay_cls, onclick: move |_| delete_open.set(false),
                    div {
                        class: "bg-white rounded-2xl shadow-2xl w-full max-w-sm p-6 space-y-4",
                        onclick: move |e| e.stop_propagation(),
                        h2 { class: "text-base font-semibold text-gray-900", {t!("report-delete-title")} }
                        p { class: "text-sm text-gray-600", {t!("report-delete-confirm")} }
                        div { class: "flex gap-2 pt-2",
                            button {
                                class: "flex-1 px-4 py-2 text-sm border border-gray-200 rounded-xl text-gray-700 hover:bg-gray-50 transition-colors",
                                onclick: move |_| delete_open.set(false),
                                {t!("btn-cancel")}
                            }
                            button {
                                class: "flex-1 px-4 py-2 text-sm bg-red-600 text-white rounded-xl hover:bg-red-700 transition-colors font-medium",
                                onclick: move |_| {
                                    if let Some(rid) = delete_id.read().clone() {
                                        spawn(async move {
                                            let Some(db) = db_signal.read().db.clone() else { return };
                                            let _ = FieldServiceReport::delete(&db, rid).await;
                                            reports_res.restart();
                                        });
                                    }
                                    delete_open.set(false);
                                },
                                {t!("btn-confirm")}
                            }
                        }
                    }
                }
            }
        }
    }
}

// ── ReportCard ────────────────────────────────────────────────────────────────

#[component]
fn ReportCard(
    user: User,
    report: Option<FieldServiceReport>,
    name_fmt: NameFormat,
    on_view: Callback<()>,
    on_edit: Callback<()>,
    on_delete: Callback<()>,
) -> Element {
    let display_name = format_name(&user.first_name, &user.last_name, &name_fmt);
    let type_label = match &user.user_type {
        UserType::Publisher => t!("user-type-publisher"),
        UserType::BaptizedPublisher => t!("user-type-baptized"),
        UserType::ContinuousAuxiliaryPioneer => t!("user-type-cont-aux-pioneer"),
        UserType::RegularPioneer => t!("user-type-regular-pioneer"),
        UserType::SpecialPioneer => t!("user-type-special-pioneer"),
        UserType::Missionary => t!("user-type-missionary"),
        UserType::Student => t!("user-type-student"),
    };
    let appointment_label = user.appointment.as_ref().map(|a| match a {
        Appointment::Elder => t!("user-appointment-elder"),
        Appointment::MinisterialServant => t!("user-appointment-ms"),
    });
    let has_report = report.is_some();

    rsx! {
        div { class: "bg-white rounded-xl border border-gray-200 p-4",
            div { class: "flex items-start justify-between gap-3",
                // Left: user info + report summary
                div { class: "flex-1 min-w-0",
                    div { class: "flex flex-wrap items-center gap-1.5 mb-1",
                        span { class: "text-sm font-semibold text-gray-900", "{display_name}" }
                        span { class: "inline-flex px-1.5 py-0.5 rounded-full text-xs bg-blue-100 text-blue-800",
                            "{type_label}"
                        }
                        if let Some(al) = &appointment_label {
                            span { class: "inline-flex px-1.5 py-0.5 rounded-full text-xs bg-amber-100 text-amber-800",
                                "{al}"
                            }
                        }
                    }

                    if let Some(ref r) = report {
                        if !r.preached {
                            span { class: "inline-flex px-1.5 py-0.5 rounded text-xs bg-gray-100 text-gray-500 italic",
                                {t!("report-form-not-preached")}
                            }
                        } else {
                            div { class: "flex flex-wrap gap-x-3 gap-y-0.5 text-xs text-gray-600",
                                if let Some(h) = r.hours {
                                    span {
                                        {t!("report-form-hours")}
                                        ": "
                                        span { class: "font-medium text-gray-800", "{h}" }
                                    }
                                }
                                if let Some(c) = r.credits {
                                    span {
                                        {t!("report-form-credits")}
                                        ": "
                                        span { class: "font-medium text-gray-800", "{c}" }
                                    }
                                }
                                if let Some(bs) = r.bible_studies {
                                    span {
                                        {t!("report-form-bible-studies")}
                                        ": "
                                        span { class: "font-medium text-gray-800", "{bs}" }
                                    }
                                }
                                if r.auxiliary_pioneer {
                                    span { class: "inline-flex px-1.5 py-0.5 rounded text-xs bg-indigo-100 text-indigo-700",
                                        {t!("report-form-aux-pioneer")}
                                    }
                                }
                            }
                        }
                    } else {
                        span { class: "text-xs text-gray-400 italic", {t!("report-not-submitted")} }
                    }
                }

                // Right: action buttons
                div { class: "flex gap-1.5 shrink-0",
                    if has_report {
                        button {
                            class: "px-2.5 py-1 text-xs border border-gray-200 rounded-lg text-gray-600 hover:bg-gray-50 transition-colors",
                            onclick: move |_| on_view.call(()),
                            {t!("report-btn-view")}
                        }
                    }
                    button {
                        class: "px-2.5 py-1 text-xs border border-gray-200 rounded-lg text-gray-600 hover:bg-gray-50 transition-colors",
                        onclick: move |_| on_edit.call(()),
                        if has_report {
                            {t!("btn-edit")}
                        } else {
                            {t!("report-add-btn")}
                        }
                    }
                    if has_report {
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
}

// ── ReportDetailModal ─────────────────────────────────────────────────────────

#[component]
fn ReportDetailModal(
    report: FieldServiceReport,
    user: User,
    name_fmt: NameFormat,
    open: Signal<bool>,
    on_close: Callback<()>,
) -> Element {
    let display_name = format_name(&user.first_name, &user.last_name, &name_fmt);
    let type_label = match &user.user_type {
        UserType::Publisher => t!("user-type-publisher"),
        UserType::BaptizedPublisher => t!("user-type-baptized"),
        UserType::ContinuousAuxiliaryPioneer => t!("user-type-cont-aux-pioneer"),
        UserType::RegularPioneer => t!("user-type-regular-pioneer"),
        UserType::SpecialPioneer => t!("user-type-special-pioneer"),
        UserType::Missionary => t!("user-type-missionary"),
        UserType::Student => t!("user-type-student"),
    };

    rsx! {
        ResponsiveModal {
            open,
            on_close,
            title: format!("{display_name} — {}", t!("report-detail-title")),
            description: format!("{} {}", month_name(report.month), report.year),
            submitting: false,
            on_submit: move |_| {},
            // Body
            div { class: "space-y-3",
                span { class: "inline-flex px-2 py-0.5 rounded-full text-xs bg-blue-100 text-blue-800",
                    "{type_label}"
                }
                if !report.preached {
                    div { class: "flex items-center gap-2 py-2",
                        span { class: "inline-flex px-2 py-1 rounded-lg text-sm bg-gray-100 text-gray-600",
                            {t!("report-form-not-preached")}
                        }
                    }
                } else {
                    div { class: "space-y-2",
                        if let Some(h) = report.hours {
                            DetailLine {
                                label: t!("report-form-hours"),
                                value: h.to_string(),
                            }
                        }
                        if let Some(c) = report.credits {
                            DetailLine {
                                label: t!("report-form-credits"),
                                value: c.to_string(),
                            }
                        }
                        if let Some(bs) = report.bible_studies {
                            DetailLine {
                                label: t!("report-form-bible-studies"),
                                value: bs.to_string(),
                            }
                        }
                        if report.auxiliary_pioneer {
                            div { class: "flex items-center gap-2",
                                span { class: "text-sm text-gray-500", {t!("report-form-aux-pioneer")} }
                                span { class: "inline-flex px-1.5 py-0.5 rounded text-xs bg-indigo-100 text-indigo-700",
                                    "✓"
                                }
                            }
                        }
                    }
                }
                if let Some(notes) = &report.notes {
                    if !notes.is_empty() {
                        div { class: "flex flex-col gap-1 pt-1 border-t border-gray-100",
                            span { class: "text-xs font-medium text-gray-500",
                                {t!("report-form-notes")}
                            }
                            p { class: "text-sm text-gray-800 whitespace-pre-wrap",
                                "{notes}"
                            }
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn DetailLine(label: String, value: String) -> Element {
    rsx! {
        div { class: "flex items-center justify-between text-sm",
            span { class: "text-gray-500", "{label}" }
            span { class: "font-medium text-gray-900", "{value}" }
        }
    }
}

// ── ReportEditModal ───────────────────────────────────────────────────────────

#[component]
fn ReportEditModal(
    target: EditTarget,
    open: Signal<bool>,
    on_close: Callback<()>,
    on_saved: Callback<()>,
) -> Element {
    let db_signal = use_db();
    let crypto_signal = use_crypto();
    let mut form = use_signal(ReportFormState::default);

    let target_for_effect = target.clone();
    use_effect(move || {
        if *open.read() {
            match &target_for_effect.existing {
                Some(r) => form.set(ReportFormState::from_report(r)),
                None => form.set(ReportFormState::default()),
            }
        }
    });

    let f = form.read().clone();
    let existing_id = target.existing.as_ref().and_then(|r| r.id.clone());
    let pub_id = target.publisher_id.clone();
    let year = target.year;
    let month = target.month;
    let user_type = target.user_type.clone();

    let is_pioneer = always_show_hours(&user_type);
    let show_hours = is_pioneer || f.auxiliary_pioneer;
    let show_credits = is_pioneer;

    let on_submit = use_callback(move |_: Event<MouseData>| {
        let fd = form.read().clone();
        let data = FieldServiceReportData {
            publisher: pub_id.clone(),
            year,
            month,
            hours: fd.hours.trim().parse().ok(),
            credits: fd.credits.trim().parse().ok(),
            bible_studies: fd.bible_studies.trim().parse().ok(),
            auxiliary_pioneer: fd.auxiliary_pioneer,
            preached: fd.preached,
            notes: (!fd.notes.trim().is_empty()).then(|| fd.notes.trim().to_string()),
        };
        let eid = existing_id.clone();
        form.write().submitting = true;
        form.write().error = None;
        spawn(async move {
            let Some(db) = db_signal.read().db.clone() else {
                form.write().submitting = false;
                return;
            };
            let crypto = crypto_signal.read().clone();
            let result = if let Some(rid) = eid {
                FieldServiceReport::update(&db, &crypto, rid, data).await.map(|_| ())
            } else {
                FieldServiceReport::create(&db, &crypto, data).await.map(|_| ())
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

    let title = format!("{} {}", t!("report-form-title"), month_name(month));

    rsx! {
        ResponsiveModal {
            open,
            on_close,
            title,
            description: String::new(),
            submitting: f.submitting,
            on_submit,
            // Form body inline
            div { class: "space-y-4",
                if let Some(err) = &f.error {
                    div { class: "bg-red-50 border border-red-200 text-red-700 text-sm px-3 py-2 rounded-lg",
                        "{err}"
                    }
                }
                // Auxiliary pioneer (only for publisher/baptized types)
                if !is_pioneer {
                    label { class: "flex items-center gap-3 cursor-pointer py-1",
                        input {
                            r#type: "checkbox",
                            class: "w-4 h-4 rounded border-gray-300 accent-primary-600",
                            checked: f.auxiliary_pioneer,
                            onchange: move |e| form.write().auxiliary_pioneer = e.checked(),
                        }
                        span { class: "text-sm text-gray-700", {t!("report-form-aux-pioneer")} }
                    }
                }
                // Hours + Credits (conditional)
                if show_hours {
                    div { class: "grid grid-cols-2 gap-3",
                        div { class: "flex flex-col gap-1",
                            label { class: "text-xs font-medium text-gray-700",
                                {t!("report-form-hours")}
                                span { class: "text-red-500 ml-0.5", " *" }
                            }
                            input {
                                r#type: "number",
                                min: "0",
                                step: "1",
                                class: "w-full px-3 py-2 text-sm border border-gray-200 rounded-lg focus:outline-none focus:ring-2 focus:ring-primary-500",
                                value: f.hours.clone(),
                                oninput: move |e| form.write().hours = e.value(),
                            }
                        }
                        if show_credits {
                            div { class: "flex flex-col gap-1",
                                label { class: "text-xs font-medium text-gray-700",
                                    {t!("report-form-credits")}
                                }
                                input {
                                    r#type: "number",
                                    min: "0",
                                    step: "1",
                                    class: "w-full px-3 py-2 text-sm border border-gray-200 rounded-lg focus:outline-none focus:ring-2 focus:ring-primary-500",
                                    value: f.credits.clone(),
                                    oninput: move |e| form.write().credits = e.value(),
                                }
                            }
                        }
                    }
                }
                // Bible studies
                div { class: "flex flex-col gap-1",
                    label { class: "text-xs font-medium text-gray-700",
                        {t!("report-form-bible-studies")}
                    }
                    input {
                        r#type: "number",
                        min: "0",
                        step: "1",
                        class: "w-full px-3 py-2 text-sm border border-gray-200 rounded-lg focus:outline-none focus:ring-2 focus:ring-primary-500",
                        value: f.bible_studies.clone(),
                        oninput: move |e| form.write().bible_studies = e.value(),
                    }
                }
                // Preached / not-preached toggle (hidden for pioneers/missionaries)
                if !is_pioneer {
                    div { class: "flex items-center gap-3 py-1",
                        button {
                            r#type: "button",
                            class: if f.preached { "relative inline-flex h-6 w-11 shrink-0 rounded-full bg-primary-600 transition-colors duration-200 cursor-pointer focus:outline-none focus:ring-2 focus:ring-primary-500 focus:ring-offset-1" } else { "relative inline-flex h-6 w-11 shrink-0 rounded-full bg-gray-300 transition-colors duration-200 cursor-pointer focus:outline-none focus:ring-2 focus:ring-primary-500 focus:ring-offset-1" },
                            onclick: move |_| {
                                let cur = form.read().preached;
                                form.write().preached = !cur;
                            },
                            span { class: if f.preached { "inline-block h-5 w-5 translate-x-5 transform rounded-full bg-white shadow transition duration-200 ease-in-out" } else { "inline-block h-5 w-5 translate-x-0.5 transform rounded-full bg-white shadow transition duration-200 ease-in-out" } }
                        }
                        span { class: if f.preached { "text-sm font-medium text-gray-900" } else { "text-sm text-gray-400" },
                            if f.preached {
                                {t!("report-preached")}
                            } else {
                                {t!("report-form-not-preached")}
                            }
                        }
                    }
                }
                // Notes
                div { class: "flex flex-col gap-1",
                    label { class: "text-xs font-medium text-gray-700", {t!("report-form-notes")} }
                    textarea {
                        class: "w-full px-3 py-2 text-sm border border-gray-200 rounded-lg focus:outline-none focus:ring-2 focus:ring-primary-500 resize-none",
                        rows: "3",
                        value: f.notes.clone(),
                        oninput: move |e| form.write().notes = e.value(),
                    }
                }
            }
        }
    }
}
