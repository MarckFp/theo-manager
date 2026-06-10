use dioxus::prelude::*;
use dioxus_i18n::t;
use surrealdb::types::RecordId;

use crate::components::ResponsiveModal;
use crate::database::{use_crypto, use_db};
use crate::models::congregation::{Congregation, DateFormat, NameFormat};
use crate::models::emergency_contact::{EmergencyContact, EmergencyContactData};
use crate::models::field_service_group::FieldServiceGroup;
use crate::models::field_service_report::{FieldServiceReport, FieldServiceReportData};
use crate::models::user::{Appointment, Gender, User, UserData, UserType};
use crate::pages::app::user::{
    appointment_to_key, date_format_hint, effective_date_format, effective_name_format,
    format_date, format_name, is_publisher_type, key_to_user_type, user_form_state_from,
    user_type_to_key, UserFormBody, UserFormState,
};
use crate::Route;

// ── Date / calendar helpers ───────────────────────────────────────────────────

#[cfg(target_arch = "wasm32")]
fn current_year_month() -> (i32, u8) {
    let date = js_sys::Date::new_0();
    (date.get_full_year() as i32, date.get_month() as u8 + 1)
}

#[cfg(not(target_arch = "wasm32"))]
fn current_year_month() -> (i32, u8) {
    (2026, 6)
}

/// Last 12 calendar months (inclusive of current), newest → oldest.
fn last_12_months(now_year: i32, now_month: u8) -> Vec<(i32, u8)> {
    (0..12u32)
        .map(|i| {
            let total = (now_year as u32 * 12 + now_month as u32 - 1).saturating_sub(i);
            let y = (total / 12) as i32;
            let m = (total % 12 + 1) as u8;
            (y, m)
        })
        .collect()
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

fn record_id_key(id: &RecordId) -> String {
    match &id.key {
        surrealdb::types::RecordIdKey::String(k) => k.clone(),
        surrealdb::types::RecordIdKey::Number(n) => n.to_string(),
        _ => String::new(),
    }
}

// ── Emergency contact form state ──────────────────────────────────────────────

#[derive(Clone, Default)]
struct ContactFormState {
    first_name: String,
    last_name: String,
    phone: String,
    email: String,
    address: String,
    relationship: String,
    submitting: bool,
    error: Option<String>,
}

impl ContactFormState {
    fn from_contact(c: &EmergencyContact) -> Self {
        Self {
            first_name: c.first_name.clone(),
            last_name: c.last_name.clone(),
            phone: c.phone.clone().unwrap_or_default(),
            email: c.email.clone().unwrap_or_default(),
            address: c.address.clone().unwrap_or_default(),
            relationship: c.relationship.clone().unwrap_or_default(),
            ..Default::default()
        }
    }
}

// ── Field service report form state ──────────────────────────────────────────

#[derive(Clone, Default)]
struct ReportFormState {
    placements: String,
    videos: String,
    return_visits: String,
    bible_studies: String,
    hours: String,
    auxiliary_pioneer: bool,
    not_preached: bool,
    notes: String,
    submitting: bool,
    error: Option<String>,
}

impl ReportFormState {
    fn from_report(r: &FieldServiceReport) -> Self {
        Self {
            placements: r.placements.map(|v| v.to_string()).unwrap_or_default(),
            videos: r.videos.map(|v| v.to_string()).unwrap_or_default(),
            return_visits: r.return_visits.map(|v| v.to_string()).unwrap_or_default(),
            bible_studies: r.bible_studies.map(|v| v.to_string()).unwrap_or_default(),
            hours: r.hours.map(|v| v.to_string()).unwrap_or_default(),
            auxiliary_pioneer: r.auxiliary_pioneer,
            not_preached: r.not_preached,
            notes: r.notes.clone().unwrap_or_default(),
            ..Default::default()
        }
    }
}

// ── Main page ─────────────────────────────────────────────────────────────────

#[component]
pub fn AppUserDetail(id: String) -> Element {
    let db_signal = use_db();
    let crypto_signal = use_crypto();
    let nav = use_navigator();
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

    // Parse record ID from route param (the raw key, e.g. a ULID)
    let Ok(record_id) = RecordId::parse_simple(&format!("user:{}", id)) else {
        return rsx! {
            div { class: "text-center py-20 text-gray-500", "Invalid user ID" }
        };
    };

    // ── Resources ────────────────────────────────────────────────────────────
    let mut user_res = {
        let rid = record_id.clone();
        use_resource(move || {
            let rid = rid.clone();
            async move {
                let Some(db) = db_signal.read().db.clone() else { return None };
                let crypto = crypto_signal.read().clone();
                User::get(&db, &crypto, rid).await.ok().flatten()
            }
        })
    };

    let mut contacts_res = {
        let rid = record_id.clone();
        use_resource(move || {
            let rid = rid.clone();
            async move {
                let Some(db) = db_signal.read().db.clone() else { return vec![] };
                let crypto = crypto_signal.read().clone();
                EmergencyContact::by_publisher(&db, &crypto, rid)
                    .await
                    .unwrap_or_default()
            }
        })
    };

    let mut reports_res = {
        let rid = record_id.clone();
        use_resource(move || {
            let rid = rid.clone();
            async move {
                let Some(db) = db_signal.read().db.clone() else { return vec![] };
                let crypto = crypto_signal.read().clone();
                FieldServiceReport::by_publisher(&db, &crypto, rid)
                    .await
                    .unwrap_or_default()
            }
        })
    };

    // Group this user belongs to (as overseer, assistant, or member)
    let group_res = {
        let rid = record_id.clone();
        use_resource(move || {
            let rid = rid.clone();
            async move {
                let Some(db) = db_signal.read().db.clone() else { return None };
                let crypto = crypto_signal.read().clone();
                FieldServiceGroup::of_user(&db, &crypto, rid).await.ok().flatten()
            }
        })
    };

    // 12-month grid slots
    let (cur_year, cur_month) = current_year_month();
    let months = last_12_months(cur_year, cur_month);

    // ── Modal signals ─────────────────────────────────────────────────────────
    let mut edit_open = use_signal(|| false);
    let mut delete_open = use_signal(|| false);

    let mut contact_modal_open = use_signal(|| false);
    let mut editing_contact: Signal<Option<EmergencyContact>> = use_signal(|| None);
    let mut delete_contact_open = use_signal(|| false);
    let mut delete_contact_id: Signal<Option<RecordId>> = use_signal(|| None);

    let mut report_modal_open = use_signal(|| false);
    let mut editing_report: Signal<Option<(i32, u8, Option<FieldServiceReport>)>> =
        use_signal(|| None);
    let mut delete_report_open = use_signal(|| false);
    let mut delete_report_id: Signal<Option<RecordId>> = use_signal(|| None);

    // ── Loading / not-found guard ─────────────────────────────────────────────
    match user_res() {
        None => {
            return rsx! {
                div { class: "flex justify-center items-center py-20 text-gray-400",
                    span { class: "text-sm animate-pulse", {t!("user-loading")} }
                }
            };
        }
        Some(None) => {
            return rsx! {
                div { class: "bg-white rounded-xl border border-gray-200 px-6 py-16 text-center",
                    p { class: "text-5xl mb-3", "❓" }
                    p { class: "font-medium text-gray-600", {t!("user-not-found")} }
                    button {
                        class: "mt-4 px-4 py-2 text-sm text-primary-600 border border-primary-200 rounded-lg hover:bg-primary-50",
                        onclick: move |_| {
                            let _ = nav.push(Route::AppUsers {});
                        },
                        "← "
                        {t!("user-back-to-list")}
                    }
                }
            };
        }
        Some(Some(_)) => {}
    }

    // SAFETY: we checked Some(Some(_)) above, so unwrap is safe.
    let user = user_res().unwrap().unwrap();

    let display_name = format_name(&user.first_name, &user.last_name, &name_fmt.read());
    let initials = format!(
        "{}{}",
        user.first_name
            .chars()
            .next()
            .unwrap_or('?')
            .to_ascii_uppercase(),
        user.last_name
            .chars()
            .next()
            .unwrap_or('?')
            .to_ascii_uppercase(),
    );

    let category_label = match &user.user_type {
        UserType::Student => t!("user-type-student"),
        UserType::Publisher => t!("user-type-publisher"),
        UserType::BaptizedPublisher => t!("user-type-baptized"),
        UserType::ContinuousAuxiliaryPioneer => t!("user-type-cont-aux-pioneer"),
        UserType::RegularPioneer => t!("user-type-regular-pioneer"),
        UserType::SpecialPioneer => t!("user-type-special-pioneer"),
        UserType::Missionary => t!("user-type-missionary"),
    };
    let appointment_label = user.appointment.as_ref().map(|a| match a {
        Appointment::Elder => t!("user-appointment-elder"),
        Appointment::MinisterialServant => t!("user-appointment-ms"),
    });
    let gender_label = match &user.gender {
        Gender::Male => t!("user-gender-male"),
        Gender::Female => t!("user-gender-female"),
    };
    let birthday_str = user
        .birthday
        .as_deref()
        .map(|d| format_date(d, &date_fmt.read()))
        .unwrap_or_else(|| "—".to_string());
    let baptism_str = user
        .baptism_date
        .as_deref()
        .map(|d| format_date(d, &date_fmt.read()))
        .unwrap_or_else(|| "—".to_string());

    let contacts: Vec<EmergencyContact> = contacts_res().unwrap_or_default();
    let reports: Vec<FieldServiceReport> = reports_res().unwrap_or_default();

    // Compute active/inactive from reports for publisher-type users.
    let (cy, cm) = current_year_month();
    let (since_year, since_month) = if cm > 6 {
        (cy, cm - 6)
    } else {
        (cy - 1, cm + 6)
    };
    let computed_active: Option<bool> = if is_publisher_type(&user.user_type) {
        let active = reports.iter().any(|r| {
            !r.not_preached
                && (r.year > since_year || (r.year == since_year && r.month >= since_month))
        });
        Some(active)
    } else {
        None
    };

    let user_rid = record_id.clone();

    rsx! {
        div { class: "space-y-5 w-full pb-10",

            // ── Back link ─────────────────────────────────────────────────
            button {
                class: "inline-flex items-center gap-1.5 text-sm text-primary-600 hover:text-primary-700 font-medium transition-colors",
                onclick: move |_| {
                    let _ = nav.push(Route::AppUsers {});
                },
                "← "
                {t!("user-back-to-list")}
            }

            // ── Header card ───────────────────────────────────────────────
            div { class: "bg-white rounded-xl border border-gray-200 p-5",
                div { class: "flex flex-col sm:flex-row sm:items-start gap-4",
                    // Avatar
                    div { class: "w-16 h-16 rounded-full bg-primary-100 text-primary-700 flex items-center justify-center font-bold text-xl shrink-0",
                        "{initials}"
                    }
                    // Name + badges
                    div { class: "flex-1 min-w-0",
                        h1 { class: "text-xl font-bold text-gray-900 break-words",
                            "{display_name}"
                        }
                        div { class: "flex flex-wrap gap-1.5 mt-2",
                            span { class: "inline-flex px-2 py-0.5 rounded-full text-xs font-medium bg-blue-100 text-blue-800",
                                "{category_label}"
                            }
                            if let Some(a) = &appointment_label {
                                span { class: "inline-flex px-2 py-0.5 rounded-full text-xs font-medium bg-amber-100 text-amber-800",
                                    "{a}"
                                }
                            }
                            if let Some(active) = computed_active {
                                if active {
                                    span { class: "inline-flex px-2 py-0.5 rounded-full text-xs font-medium bg-emerald-600 text-white",
                                        {t!("user-badge-active")}
                                    }
                                } else {
                                    span { class: "inline-flex px-2 py-0.5 rounded-full text-xs font-medium bg-gray-400 text-white",
                                        {t!("user-badge-inactive")}
                                    }
                                }
                            }
                            if user.family_head {
                                span { class: "inline-flex px-2 py-0.5 rounded-full text-xs font-medium bg-purple-100 text-purple-700",
                                    {t!("user-family-head-badge")}
                                }
                            }
                        }
                    }
                    // Action buttons
                    div { class: "flex gap-2 shrink-0",
                        button {
                            class: "px-3 py-1.5 text-sm border border-gray-200 rounded-lg text-gray-700 hover:bg-gray-50 transition-colors",
                            onclick: move |_| edit_open.set(true),
                            {t!("btn-edit")}
                        }
                        button {
                            class: "px-3 py-1.5 text-sm bg-red-50 border border-red-200 rounded-lg text-red-700 hover:bg-red-100 transition-colors",
                            onclick: move |_| delete_open.set(true),
                            {t!("btn-delete")}
                        }
                    }
                }
            }

            // ── Info grid ─────────────────────────────────────────────────
            div { class: "grid grid-cols-1 md:grid-cols-2 gap-4",
                // Personal info
                div { class: "bg-white rounded-xl border border-gray-200 p-5 space-y-3",
                    h2 { class: "text-xs font-semibold text-gray-500 uppercase tracking-wider pb-1",
                        {t!("user-detail-personal")}
                    }
                    DetailRow {
                        label: t!("user-form-gender"),
                        value: gender_label.to_string(),
                    }
                    DetailRow { label: t!("user-form-birthday"), value: birthday_str }
                    DetailRow {
                        label: t!("user-form-baptism-date"),
                        value: baptism_str,
                    }
                    DetailRow {
                        label: t!("user-form-family-head"),
                        value: if user.family_head { t!("user-yes") } else { t!("user-no") },
                    }
                    {
                        let group_name = group_res()
                            .flatten()
                            .map(|g| g.name.clone())
                            .unwrap_or_else(|| t!("user-detail-no-group"));
                        rsx! {
                            DetailRow { label: t!("user-detail-group"), value: group_name }
                        }
                    }
                }
                // Contact info
                div { class: "bg-white rounded-xl border border-gray-200 p-5 space-y-3",
                    h2 { class: "text-xs font-semibold text-gray-500 uppercase tracking-wider pb-1",
                        {t!("user-detail-contact")}
                    }
                    DetailRow {
                        label: t!("user-form-phone"),
                        value: user.phone.clone().unwrap_or_else(|| "—".to_string()),
                    }
                    DetailRow {
                        label: t!("user-form-email"),
                        value: user.email.clone().unwrap_or_else(|| "—".to_string()),
                    }
                    DetailRow {
                        label: t!("user-form-address"),
                        value: user.address.clone().unwrap_or_else(|| "—".to_string()),
                    }
                }
            }

            // ── Emergency contacts ────────────────────────────────────────
            div { class: "bg-white rounded-xl border border-gray-200 overflow-hidden",
                div { class: "flex items-center justify-between px-5 py-4 border-b border-gray-100",
                    h2 { class: "text-xs font-semibold text-gray-500 uppercase tracking-wider",
                        {t!("user-detail-emergency-contacts")}
                    }
                    button {
                        class: "px-3 py-1.5 text-sm bg-primary-600 text-white rounded-lg hover:bg-primary-700 transition-colors",
                        onclick: move |_| {
                            editing_contact.set(None);
                            contact_modal_open.set(true);
                        },
                        "＋ "
                        {t!("user-detail-add-contact")}
                    }
                }
                if contacts.is_empty() {
                    div { class: "px-5 py-8 text-center text-gray-400 text-sm",
                        {t!("user-detail-no-contacts")}
                    }
                } else {
                    div { class: "divide-y divide-gray-100",
                        for contact in contacts.clone() {
                            {
                                let c_edit = contact.clone();
                                let c_del_id = contact.id.clone();
                                rsx! {
                                    ContactCard {
                                        contact: contact.clone(),
                                        on_edit: move |_| {
                                            editing_contact.set(Some(c_edit.clone()));
                                            contact_modal_open.set(true);
                                        },
                                        on_delete: move |_| {
                                            delete_contact_id.set(c_del_id.clone());
                                            delete_contact_open.set(true);
                                        },
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // ── Field service reports ─────────────────────────────────────
            div { class: "bg-white rounded-xl border border-gray-200 overflow-hidden",
                div { class: "px-5 py-4 border-b border-gray-100",
                    h2 { class: "text-xs font-semibold text-gray-500 uppercase tracking-wider",
                        {t!("user-detail-reports-title")}
                    }
                }
                div { class: "grid grid-cols-1 sm:grid-cols-2 xl:grid-cols-3 divide-y divide-gray-100",
                    for (year , month) in months.iter().copied() {
                        {
                            let existing = reports
                                .iter()
                                .find(|r| r.year == year && r.month == month)
                                .cloned();
                            let ex_edit = existing.clone();
                            let ex_del_id = existing.as_ref().and_then(|r| r.id.clone());
                            rsx! {
                                ReportMonthCard {
                                    year,
                                    month,
                                    report: existing,
                                    on_edit: move |_| {
                                        editing_report.set(Some((year, month, ex_edit.clone())));
                                        report_modal_open.set(true);
                                    },
                                    on_delete: move |_| {
                                        delete_report_id.set(ex_del_id.clone());
                                        delete_report_open.set(true);
                                    },
                                }
                            }
                        }
                    }
                }
            }
        }

        // ── Edit user modal ────────────────────────────────────────────────
        EditUserModal {
            user: user.clone(),
            open: edit_open,
            on_close: move |_| edit_open.set(false),
            on_updated: move |_| {
                user_res.restart();
                edit_open.set(false);
            },
            date_fmt: date_fmt.read().clone(),
        }

        // ── Delete user confirmation ───────────────────────────────────────
        ConfirmModal {
            open: delete_open,
            title: t!("user-delete-title"),
            message: t!("user-delete-confirm"),
            destructive: true,
            on_close: move |_| delete_open.set(false),
            on_confirm: {
                let rid = user_rid.clone();
                move |_| {
                    let rid = rid.clone();
                    delete_open.set(false);
                    spawn(async move {
                        let Some(db) = db_signal.read().db.clone() else { return };
                        let _ = EmergencyContact::delete_by_publisher(&db, rid.clone()).await;
                        let _ = FieldServiceReport::delete_by_publisher(&db, rid.clone()).await;
                        let _ = User::delete(&db, rid).await;
                        nav.push(Route::AppUsers {});
                    });
                }
            },
        }

        // ── Add / edit emergency contact ───────────────────────────────────
        ContactFormModal {
            publisher_id: record_id.clone(),
            open: contact_modal_open,
            existing: editing_contact,
            on_close: move |_| contact_modal_open.set(false),
            on_saved: move |_| {
                contacts_res.restart();
                contact_modal_open.set(false);
            },
        }

        // ── Delete contact confirmation ────────────────────────────────────
        ConfirmModal {
            open: delete_contact_open,
            title: t!("contact-delete-title"),
            message: t!("contact-delete-confirm"),
            destructive: true,
            on_close: move |_| delete_contact_open.set(false),
            on_confirm: move |_| {
                if let Some(cid) = delete_contact_id.read().clone() {
                    spawn(async move {
                        let Some(db) = db_signal.read().db.clone() else { return };
                        let _ = EmergencyContact::delete(&db, cid).await;
                        contacts_res.restart();
                    });
                }
                delete_contact_open.set(false);
            },
        }

        // ── Add / edit report ──────────────────────────────────────────────
        if let Some((year, month, existing)) = editing_report.read().clone() {
            ReportFormModal {
                publisher_id: record_id.clone(),
                year,
                month,
                open: report_modal_open,
                existing,
                on_close: move |_| report_modal_open.set(false),
                on_saved: move |_| {
                    reports_res.restart();
                    report_modal_open.set(false);
                },
            }
        }

        // ── Delete report confirmation ─────────────────────────────────────
        ConfirmModal {
            open: delete_report_open,
            title: t!("report-delete-title"),
            message: t!("report-delete-confirm"),
            destructive: true,
            on_close: move |_| delete_report_open.set(false),
            on_confirm: move |_| {
                if let Some(rid) = delete_report_id.read().clone() {
                    spawn(async move {
                        let Some(db) = db_signal.read().db.clone() else { return };
                        let _ = FieldServiceReport::delete(&db, rid).await;
                        reports_res.restart();
                    });
                }
                delete_report_open.set(false);
            },
        }
    }
}

// ── DetailRow ─────────────────────────────────────────────────────────────────

#[component]
fn DetailRow(label: String, value: String) -> Element {
    rsx! {
        div { class: "flex items-start justify-between gap-4 text-sm",
            span { class: "text-gray-500 shrink-0", "{label}" }
            span { class: "text-gray-900 text-right break-words min-w-0", "{value}" }
        }
    }
}

// ── ContactCard ───────────────────────────────────────────────────────────────

#[component]
fn ContactCard(
    contact: EmergencyContact,
    on_edit: Callback<()>,
    on_delete: Callback<()>,
) -> Element {
    rsx! {
        div { class: "px-5 py-3.5 flex items-start justify-between gap-3",
            div { class: "flex-1 min-w-0",
                div { class: "flex flex-wrap items-center gap-x-2 gap-y-0.5",
                    span { class: "text-sm font-medium text-gray-900",
                        "{contact.first_name} {contact.last_name}"
                    }
                    if let Some(r) = &contact.relationship {
                        span { class: "text-xs text-gray-400", "· {r}" }
                    }
                }
                div { class: "flex flex-wrap gap-x-3 gap-y-0.5 mt-0.5",
                    if let Some(p) = &contact.phone {
                        span { class: "text-xs text-gray-500", "📞 {p}" }
                    }
                    if let Some(e) = &contact.email {
                        span { class: "text-xs text-gray-500", "✉ {e}" }
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

// ── ReportMonthCard ───────────────────────────────────────────────────────────

#[component]
fn ReportMonthCard(
    year: i32,
    month: u8,
    report: Option<FieldServiceReport>,
    on_edit: Callback<()>,
    on_delete: Callback<()>,
) -> Element {
    let has_report = report.is_some();

    rsx! {
        div { class: "p-4 hover:bg-gray-50 transition-colors",
            div { class: "flex items-center justify-between gap-2 mb-2",
                h3 { class: "text-sm font-semibold text-gray-800", "{month_name(month)} {year}" }
                div { class: "flex gap-1.5",
                    if has_report {
                        button {
                            class: "text-xs px-2 py-0.5 border border-gray-200 rounded text-gray-600 hover:bg-gray-100 transition-colors",
                            onclick: move |_| on_edit.call(()),
                            {t!("btn-edit")}
                        }
                        button {
                            class: "text-xs px-2 py-0.5 border border-red-200 rounded text-red-500 hover:bg-red-50 transition-colors",
                            onclick: move |_| on_delete.call(()),
                            {t!("btn-delete")}
                        }
                    } else {
                        button {
                            class: "text-xs px-2 py-0.5 bg-primary-600 text-white rounded hover:bg-primary-700 transition-colors",
                            onclick: move |_| on_edit.call(()),
                            {t!("report-add-btn")}
                        }
                    }
                }
            }
            if let Some(r) = report {
                div { class: "grid grid-cols-2 gap-x-3 gap-y-1",
                    ReportStat {
                        label: t!("report-form-placements"),
                        value: r.placements,
                    }
                    ReportStat { label: t!("report-form-videos"), value: r.videos }
                    ReportStat {
                        label: t!("report-form-return-visits"),
                        value: r.return_visits,
                    }
                    ReportStat {
                        label: t!("report-form-bible-studies"),
                        value: r.bible_studies,
                    }
                    if let Some(h) = r.hours {
                        div { class: "col-span-2 flex items-center justify-between text-xs",
                            span { class: "text-gray-500", {t!("report-form-hours")} }
                            span { class: "font-medium text-gray-800", "{h}" }
                        }
                    }
                    if r.auxiliary_pioneer {
                        div { class: "col-span-2 mt-0.5",
                            span { class: "inline-flex px-1.5 py-0.5 rounded text-xs bg-indigo-100 text-indigo-700",
                                {t!("report-form-aux-pioneer")}
                            }
                        }
                    }
                }
            } else {
                p { class: "text-xs text-gray-400 italic", {t!("report-not-submitted")} }
            }
        }
    }
}

#[component]
fn ReportStat(label: String, value: Option<u32>) -> Element {
    let display = value.map(|v| v.to_string()).unwrap_or_else(|| "—".to_string());
    rsx! {
        div { class: "flex items-center justify-between text-xs",
            span { class: "text-gray-500", "{label}" }
            span { class: "font-medium text-gray-800", "{display}" }
        }
    }
}

// ── Generic confirm modal ─────────────────────────────────────────────────────

#[component]
fn ConfirmModal(
    open: Signal<bool>,
    title: String,
    message: String,
    destructive: bool,
    on_close: Callback<()>,
    on_confirm: Callback<()>,
) -> Element {
    let is_open = *open.read();
    let overlay_cls = if is_open {
        "fixed inset-0 z-50 flex items-center justify-center p-4 bg-black/40"
    } else {
        "fixed inset-0 z-50 flex items-center justify-center p-4 bg-black/40 opacity-0 pointer-events-none"
    };
    let confirm_cls = if destructive {
        "flex-1 lg:flex-none px-4 py-2 text-sm bg-red-600 text-white rounded-xl hover:bg-red-700 transition-colors font-medium"
    } else {
        "flex-1 lg:flex-none px-4 py-2 text-sm bg-primary-600 text-white rounded-xl hover:bg-primary-700 transition-colors font-medium"
    };

    rsx! {
        div { class: overlay_cls, onclick: move |_| on_close.call(()),
            div {
                class: "bg-white rounded-2xl shadow-2xl w-full max-w-sm p-6 space-y-4",
                onclick: move |e| e.stop_propagation(),
                h2 { class: "text-base font-semibold text-gray-900", "{title}" }
                p { class: "text-sm text-gray-600", "{message}" }
                div { class: "flex gap-2 pt-2",
                    button {
                        class: "flex-1 px-4 py-2 text-sm border border-gray-200 rounded-xl text-gray-700 hover:bg-gray-50 transition-colors",
                        onclick: move |_| on_close.call(()),
                        {t!("btn-cancel")}
                    }
                    button {
                        class: confirm_cls,
                        onclick: move |_| on_confirm.call(()),
                        {t!("btn-confirm")}
                    }
                }
            }
        }
    }
}

// ── Edit user modal ───────────────────────────────────────────────────────────

#[component]
fn EditUserModal(
    user: User,
    open: Signal<bool>,
    on_close: Callback<()>,
    on_updated: Callback<()>,
    date_fmt: DateFormat,
) -> Element {
    let db_signal = use_db();
    let crypto_signal = use_crypto();
    let mut form = use_signal(UserFormState::default);

    // Pre-fill form from user whenever the modal opens.
    let user_for_effect = user.clone();
    use_effect(move || {
        if *open.read() {
            form.set(user_form_state_from(&user_for_effect));
        }
    });

    let f = form.read().clone();
    let user_id = user.id.clone();
    let user_congregations = user.congregations.clone();

    let on_submit = use_callback(move |_: Event<MouseData>| {
        let fd = form.read().clone();
        if fd.first_name.trim().is_empty()
            || fd.last_name.trim().is_empty()
            || fd.gender.is_empty()
        {
            form.write().error = Some(t!("user-form-required-error"));
            return;
        }
        let gender = if fd.gender == "male" { Gender::Male } else { Gender::Female };
        let appointment = if gender == Gender::Male {
            match fd.appointment.as_str() {
                "elder" => Some(Appointment::Elder),
                "ms" => Some(Appointment::MinisterialServant),
                _ => None,
            }
        } else {
            None
        };
        let data = UserData {
            first_name: fd.first_name.trim().to_string(),
            last_name: fd.last_name.trim().to_string(),
            birthday: (!fd.birthday.is_empty()).then(|| fd.birthday.clone()),
            baptism_date: (!fd.baptism_date.is_empty()).then(|| fd.baptism_date.clone()),
            phone: (!fd.phone.trim().is_empty()).then(|| fd.phone.trim().to_string()),
            address: (!fd.address.trim().is_empty()).then(|| fd.address.trim().to_string()),
            email: (!fd.email.trim().is_empty()).then(|| fd.email.trim().to_string()),
            password: (!fd.password.is_empty()).then(|| fd.password.clone()),
            user_type: key_to_user_type(&fd.user_type),
            gender,
            appointment,
            family_head: fd.family_head,
            congregations: user_congregations.clone(),
            active: true,
        };
        let Some(rid) = user_id.clone() else { return };
        form.write().submitting = true;
        form.write().error = None;
        spawn(async move {
            let Some(db) = db_signal.read().db.clone() else {
                form.write().submitting = false;
                form.write().error = Some("No database connection.".to_string());
                return;
            };
            let crypto = crypto_signal.read().clone();
            match User::update(&db, &crypto, rid, data).await {
                Ok(_) => on_updated.call(()),
                Err(e) => {
                    form.write().submitting = false;
                    form.write().error = Some(e.to_string());
                }
            }
        });
    });

    rsx! {
        ResponsiveModal {
            open,
            on_close,
            title: t!("user-edit-title"),
            description: t!("user-edit-desc"),
            submitting: f.submitting,
            on_submit,
            UserFormBody { form, date_fmt }
        }
    }
}

// ── Contact form body ─────────────────────────────────────────────────────────

#[component]
fn ContactFormBody(form: Signal<ContactFormState>) -> Element {
    let f = form.read().clone();
    rsx! {
        if let Some(err) = &f.error {
            div { class: "bg-red-50 border border-red-200 text-red-700 text-sm px-3 py-2 rounded-lg",
                "{err}"
            }
        }
        div { class: "grid grid-cols-2 gap-3",
            div { class: "flex flex-col gap-1",
                label { class: "text-xs font-medium text-gray-700",
                    {t!("contact-form-firstname")}
                    span { class: "text-red-500 ml-0.5", " *" }
                }
                input {
                    r#type: "text",
                    class: "w-full px-3 py-2 text-sm border border-gray-200 rounded-lg focus:outline-none focus:ring-2 focus:ring-primary-500",
                    value: f.first_name.clone(),
                    oninput: move |e| form.write().first_name = e.value(),
                }
            }
            div { class: "flex flex-col gap-1",
                label { class: "text-xs font-medium text-gray-700",
                    {t!("contact-form-lastname")}
                    span { class: "text-red-500 ml-0.5", " *" }
                }
                input {
                    r#type: "text",
                    class: "w-full px-3 py-2 text-sm border border-gray-200 rounded-lg focus:outline-none focus:ring-2 focus:ring-primary-500",
                    value: f.last_name.clone(),
                    oninput: move |e| form.write().last_name = e.value(),
                }
            }
        }
        div { class: "flex flex-col gap-1",
            label { class: "text-xs font-medium text-gray-700", {t!("contact-form-relationship")} }
            input {
                r#type: "text",
                class: "w-full px-3 py-2 text-sm border border-gray-200 rounded-lg focus:outline-none focus:ring-2 focus:ring-primary-500",
                value: f.relationship.clone(),
                oninput: move |e| form.write().relationship = e.value(),
            }
        }
        div { class: "grid grid-cols-2 gap-3",
            div { class: "flex flex-col gap-1",
                label { class: "text-xs font-medium text-gray-700", {t!("contact-form-phone")} }
                input {
                    r#type: "tel",
                    class: "w-full px-3 py-2 text-sm border border-gray-200 rounded-lg focus:outline-none focus:ring-2 focus:ring-primary-500",
                    value: f.phone.clone(),
                    oninput: move |e| form.write().phone = e.value(),
                }
            }
            div { class: "flex flex-col gap-1",
                label { class: "text-xs font-medium text-gray-700", {t!("contact-form-email")} }
                input {
                    r#type: "email",
                    class: "w-full px-3 py-2 text-sm border border-gray-200 rounded-lg focus:outline-none focus:ring-2 focus:ring-primary-500",
                    value: f.email.clone(),
                    oninput: move |e| form.write().email = e.value(),
                }
            }
        }
        div { class: "flex flex-col gap-1",
            label { class: "text-xs font-medium text-gray-700", {t!("contact-form-address")} }
            input {
                r#type: "text",
                class: "w-full px-3 py-2 text-sm border border-gray-200 rounded-lg focus:outline-none focus:ring-2 focus:ring-primary-500",
                value: f.address.clone(),
                oninput: move |e| form.write().address = e.value(),
            }
        }
    }
}

// ── Contact form modal ────────────────────────────────────────────────────────

#[component]
fn ContactFormModal(
    publisher_id: RecordId,
    open: Signal<bool>,
    existing: Signal<Option<EmergencyContact>>,
    on_close: Callback<()>,
    on_saved: Callback<()>,
) -> Element {
    let db_signal = use_db();
    let crypto_signal = use_crypto();
    let mut form = use_signal(ContactFormState::default);

    use_effect(move || {
        if *open.read() {
            match existing.read().as_ref() {
                Some(c) => form.set(ContactFormState::from_contact(c)),
                None => form.set(ContactFormState::default()),
            }
        }
    });

    let f = form.read().clone();
    let is_edit = existing.read().is_some();
    let existing_id = existing.read().as_ref().and_then(|c| c.id.clone());
    let pub_id = publisher_id.clone();

    let on_submit = use_callback(move |_: Event<MouseData>| {
        let fd = form.read().clone();
        if fd.first_name.trim().is_empty() || fd.last_name.trim().is_empty() {
            form.write().error = Some(t!("user-form-required-error"));
            return;
        }
        let data = EmergencyContactData {
            publisher: pub_id.clone(),
            first_name: fd.first_name.trim().to_string(),
            last_name: fd.last_name.trim().to_string(),
            phone: (!fd.phone.trim().is_empty()).then(|| fd.phone.trim().to_string()),
            email: (!fd.email.trim().is_empty()).then(|| fd.email.trim().to_string()),
            address: (!fd.address.trim().is_empty()).then(|| fd.address.trim().to_string()),
            relationship: (!fd.relationship.trim().is_empty())
                .then(|| fd.relationship.trim().to_string()),
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
                EmergencyContact::update(&db, &crypto, rid, data).await.map(|_| ())
            } else {
                EmergencyContact::create(&db, &crypto, data).await.map(|_| ())
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

    let title = if is_edit {
        t!("contact-form-title-edit")
    } else {
        t!("contact-form-title-add")
    };

    rsx! {
        ResponsiveModal {
            open,
            on_close,
            title,
            description: String::new(),
            submitting: f.submitting,
            on_submit,
            ContactFormBody { form }
        }
    }
}

// ── Report form body ──────────────────────────────────────────────────────────

#[component]
fn ReportFormBody(form: Signal<ReportFormState>) -> Element {
    let f = form.read().clone();
    rsx! {
        if let Some(err) = &f.error {
            div { class: "bg-red-50 border border-red-200 text-red-700 text-sm px-3 py-2 rounded-lg",
                "{err}"
            }
        }
        // Placements + Videos
        div { class: "grid grid-cols-2 gap-3",
            div { class: "flex flex-col gap-1",
                label { class: "text-xs font-medium text-gray-700", {t!("report-form-placements")} }
                input {
                    r#type: "number",
                    min: "0",
                    class: "w-full px-3 py-2 text-sm border border-gray-200 rounded-lg focus:outline-none focus:ring-2 focus:ring-primary-500",
                    value: f.placements.clone(),
                    oninput: move |e| form.write().placements = e.value(),
                }
            }
            div { class: "flex flex-col gap-1",
                label { class: "text-xs font-medium text-gray-700", {t!("report-form-videos")} }
                input {
                    r#type: "number",
                    min: "0",
                    class: "w-full px-3 py-2 text-sm border border-gray-200 rounded-lg focus:outline-none focus:ring-2 focus:ring-primary-500",
                    value: f.videos.clone(),
                    oninput: move |e| form.write().videos = e.value(),
                }
            }
        }
        // Return visits + Bible studies
        div { class: "grid grid-cols-2 gap-3",
            div { class: "flex flex-col gap-1",
                label { class: "text-xs font-medium text-gray-700", {t!("report-form-return-visits")} }
                input {
                    r#type: "number",
                    min: "0",
                    class: "w-full px-3 py-2 text-sm border border-gray-200 rounded-lg focus:outline-none focus:ring-2 focus:ring-primary-500",
                    value: f.return_visits.clone(),
                    oninput: move |e| form.write().return_visits = e.value(),
                }
            }
            div { class: "flex flex-col gap-1",
                label { class: "text-xs font-medium text-gray-700", {t!("report-form-bible-studies")} }
                input {
                    r#type: "number",
                    min: "0",
                    class: "w-full px-3 py-2 text-sm border border-gray-200 rounded-lg focus:outline-none focus:ring-2 focus:ring-primary-500",
                    value: f.bible_studies.clone(),
                    oninput: move |e| form.write().bible_studies = e.value(),
                }
            }
        }
        // Hours
        div { class: "flex flex-col gap-1",
            label { class: "text-xs font-medium text-gray-700", {t!("report-form-hours")} }
            input {
                r#type: "number",
                min: "0",
                step: "0.5",
                class: "w-full px-3 py-2 text-sm border border-gray-200 rounded-lg focus:outline-none focus:ring-2 focus:ring-primary-500",
                value: f.hours.clone(),
                oninput: move |e| form.write().hours = e.value(),
            }
        }
        // Auxiliary pioneer
        label { class: "flex items-center gap-3 cursor-pointer py-1",
            input {
                r#type: "checkbox",
                class: "w-4 h-4 rounded border-gray-300 accent-primary-600",
                checked: f.auxiliary_pioneer,
                onchange: move |e| form.write().auxiliary_pioneer = e.checked(),
            }
            span { class: "text-sm text-gray-700", {t!("report-form-aux-pioneer")} }
        }
        // Did not preach
        label { class: "flex items-center gap-3 cursor-pointer py-1",
            input {
                r#type: "checkbox",
                class: "w-4 h-4 rounded border-gray-300 accent-primary-600",
                checked: f.not_preached,
                onchange: move |e| form.write().not_preached = e.checked(),
            }
            span { class: "text-sm text-gray-700", {t!("report-form-not-preached")} }
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

// ── Report form modal ─────────────────────────────────────────────────────────

#[component]
fn ReportFormModal(
    publisher_id: RecordId,
    year: i32,
    month: u8,
    open: Signal<bool>,
    existing: Option<FieldServiceReport>,
    on_close: Callback<()>,
    on_saved: Callback<()>,
) -> Element {
    let db_signal = use_db();
    let crypto_signal = use_crypto();
    let mut form = use_signal(ReportFormState::default);

    let existing_for_effect = existing.clone();
    use_effect(move || {
        if *open.read() {
            match &existing_for_effect {
                Some(r) => form.set(ReportFormState::from_report(r)),
                None => form.set(ReportFormState::default()),
            }
        }
    });

    let f = form.read().clone();
    let existing_id = existing.as_ref().and_then(|r| r.id.clone());
    let pub_id = publisher_id.clone();

    let on_submit = use_callback(move |_: Event<MouseData>| {
        let fd = form.read().clone();
        let data = FieldServiceReportData {
            publisher: pub_id.clone(),
            year,
            month,
            placements: fd.placements.trim().parse().ok(),
            videos: fd.videos.trim().parse().ok(),
            return_visits: fd.return_visits.trim().parse().ok(),
            bible_studies: fd.bible_studies.trim().parse().ok(),
            hours: fd.hours.trim().parse().ok(),
            auxiliary_pioneer: fd.auxiliary_pioneer,
            not_preached: fd.not_preached,
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

    let title = format!("{} {} — {}", t!("report-form-title"), month_name(month), year);

    rsx! {
        ResponsiveModal {
            open,
            on_close,
            title,
            description: String::new(),
            submitting: f.submitting,
            on_submit,
            ReportFormBody { form }
        }
    }
}
