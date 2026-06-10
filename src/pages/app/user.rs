use dioxus::prelude::*;
use dioxus_i18n::t;

use crate::components::ResponsiveModal;
use crate::database::{use_crypto, use_db};
use crate::models::congregation::{Congregation, DateFormat, NameFormat};
use crate::models::field_service_group::FieldServiceGroup;
use crate::models::field_service_report::FieldServiceReport;
use crate::models::user::{Appointment, Gender, User, UserData, UserType};
use crate::Route;

// ── Format helpers ────────────────────────────────────────────────────────────

/// Returns `"FirstLast"` or `"LastFirst"`.
pub fn effective_name_format(
    cong: Option<&Congregation>,
    user_prefs_name: &str,
) -> NameFormat {
    if !user_prefs_name.is_empty() {
        return if user_prefs_name == "LastFirst" { NameFormat::LastFirst } else { NameFormat::FirstLast };
    }
    cong.map(|c| c.name_format.clone()).unwrap_or_default()
}

/// Returns `"YMD"`, `"DMY"`, or `"MDY"`.
pub fn effective_date_format(
    cong: Option<&Congregation>,
    user_prefs_date: &str,
) -> DateFormat {
    match user_prefs_date {
        "DMY" => return DateFormat::DMY,
        "MDY" => return DateFormat::MDY,
        "YMD" => return DateFormat::YMD,
        _ => {}
    }
    cong.map(|c| c.date_format.clone()).unwrap_or_default()
}

pub fn format_name(first: &str, last: &str, fmt: &NameFormat) -> String {
    match fmt {
        NameFormat::LastFirst => format!("{last} {first}"),
        NameFormat::FirstLast => format!("{first} {last}"),
    }
}

/// Convert an ISO date string (`YYYY-MM-DD`) to the display format.
pub fn format_date(iso: &str, fmt: &DateFormat) -> String {
    if iso.len() != 10 { return iso.to_string(); }
    let parts: Vec<&str> = iso.splitn(3, '-').collect();
    if parts.len() != 3 { return iso.to_string(); }
    let (y, m, d) = (parts[0], parts[1], parts[2]);
    match fmt {
        DateFormat::YMD => format!("{y}-{m}-{d}"),
        DateFormat::DMY => format!("{d}/{m}/{y}"),
        DateFormat::MDY => format!("{m}/{d}/{y}"),
    }
}

/// Pattern hint shown in the date label.
pub fn date_format_hint(fmt: &DateFormat) -> &'static str {
    match fmt {
        DateFormat::YMD => "YYYY-MM-DD",
        DateFormat::DMY => "DD/MM/YYYY",
        DateFormat::MDY => "MM/DD/YYYY",
    }
}

#[cfg(target_arch = "wasm32")]
fn current_year_month() -> (i32, u8) {
    let d = js_sys::Date::new_0();
    (d.get_full_year() as i32, (d.get_month() + 1) as u8)
}

#[cfg(not(target_arch = "wasm32"))]
fn current_year_month() -> (i32, u8) { (2026, 6) }

/// Returns true if this user type should show an Active/Inactive badge.
pub fn is_publisher_type(t: &UserType) -> bool {
    !matches!(t, UserType::Student)
}

const PAGE_SIZE: usize = 20;

// ── Accent/case-insensitive normalisation ─────────────────────────────────────

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

fn rid_to_str(id: &surrealdb::types::RecordId) -> String {
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
enum ActivityFilter {
    #[default]
    All,
    Active,
    Inactive,
}

#[derive(Clone, PartialEq, Default)]
enum GroupFilter {
    #[default]
    All,
    NoGroup,
    InGroup(String), // group record id str, e.g. "field_service_group:KEY"
}

#[derive(Clone, PartialEq, Default)]
struct Filters {
    name: String,
    gender: Option<Gender>,
    user_type: Option<UserType>,
    appointment: AppointmentFilter,
    family_head: Option<bool>,
    activity: ActivityFilter,
    group_filter: GroupFilter,
}

// ── Form state ────────────────────────────────────────────────────────────────

#[derive(Clone, Default)]
pub struct UserFormState {
    pub first_name: String,
    pub last_name: String,
    pub gender: String,
    pub user_type: String,
    pub appointment: String,
    pub birthday: String,
    pub baptism_date: String,
    pub phone: String,
    pub address: String,
    pub email: String,
    pub password: String,
    pub family_head: bool,
    pub submitting: bool,
    pub error: Option<String>,
}

// ── User type key helpers ─────────────────────────────────────────────────────

pub fn key_to_user_type(s: &str) -> UserType {
    match s {
        "publisher" => UserType::Publisher,
        "baptized" => UserType::BaptizedPublisher,
        "cont_aux" => UserType::ContinuousAuxiliaryPioneer,
        "regular" => UserType::RegularPioneer,
        "special" => UserType::SpecialPioneer,
        "missionary" => UserType::Missionary,
        _ => UserType::Student,
    }
}

pub fn user_type_to_key(t: &UserType) -> &'static str {
    match t {
        UserType::Student => "student",
        UserType::Publisher => "publisher",
        UserType::BaptizedPublisher => "baptized",
        UserType::ContinuousAuxiliaryPioneer => "cont_aux",
        UserType::RegularPioneer => "regular",
        UserType::SpecialPioneer => "special",
        UserType::Missionary => "missionary",
    }
}

pub fn appointment_to_key(a: &Option<Appointment>) -> &'static str {
    match a {
        Some(Appointment::Elder) => "elder",
        Some(Appointment::MinisterialServant) => "ms",
        None => "",
    }
}

pub fn user_form_state_from(user: &User) -> UserFormState {
    UserFormState {
        first_name: user.first_name.clone(),
        last_name: user.last_name.clone(),
        gender: match user.gender {
            Gender::Male => "male",
            Gender::Female => "female",
        }
        .to_string(),
        user_type: user_type_to_key(&user.user_type).to_string(),
        appointment: appointment_to_key(&user.appointment).to_string(),
        birthday: user.birthday.clone().unwrap_or_default(),
        baptism_date: user.baptism_date.clone().unwrap_or_default(),
        phone: user.phone.clone().unwrap_or_default(),
        address: user.address.clone().unwrap_or_default(),
        email: user.email.clone().unwrap_or_default(),
        password: String::new(),
        family_head: user.family_head,
        submitting: false,
        error: None,
    }
}

// ── Main page ─────────────────────────────────────────────────────────────────

#[component]
pub fn AppUsers() -> Element {
    let db_signal = use_db();
    let crypto_signal = use_crypto();

    // Effective name/date formats (congregation default overridden by user prefs)
    let congregation_res = use_context::<Resource<Option<Congregation>>>();
    let db_state = use_db();
    let uid = db_state.read().congregation_uid.clone().unwrap_or_default();

    let name_fmt = use_signal(|| NameFormat::FirstLast);
    let date_fmt = use_signal(|| DateFormat::YMD);

    // Load user prefs and compute effective formats on mount / congregation change.
    {
        let uid = uid.clone();
        let mut name_fmt = name_fmt.clone();
        let mut date_fmt = date_fmt.clone();
        use_effect(move || {
            let uid = uid.clone();
            let cong_snap = congregation_res.read().clone();
            let db_opt = db_state.read().db.clone();
            spawn(async move {
                let prefs = crate::pages::app::user_settings::load_prefs(&uid, db_opt).await;
                let cong_ref = cong_snap.as_ref().and_then(|o| o.as_ref());
                name_fmt.set(effective_name_format(cong_ref, prefs.name_format.as_deref().unwrap_or("")));
                date_fmt.set(effective_date_format(cong_ref, prefs.date_format.as_deref().unwrap_or("")));
            });
        });
    }

    let mut users = use_resource(move || async move {
        let db_opt = db_signal.read().db.clone();
        let Some(db) = db_opt else { return vec![] };
        let crypto = crypto_signal.read().clone();
        match User::all(&db, &crypto).await {
            Ok(v) => v,
            Err(e) => {
                let err_str = e.to_string().replace("'", "\\'").replace("\n", " ");
                let js = format!("console.error('User::all err:', '{}');", err_str);
                let _ = document::eval(&js);
                vec![]
            }
        }
    });

    // Load active publisher IDs (reports in last 6 months, not flagged not_preached).
    let (cur_year, cur_month) = current_year_month();
    let (since_year, since_month) = if cur_month > 6 {
        (cur_year, cur_month - 6)
    } else {
        (cur_year - 1, cur_month + 6)
    };
    let mut active_ids_res = use_resource(move || async move {
        let Some(db) = db_signal.read().db.clone() else {
            return std::collections::HashSet::new();
        };
        FieldServiceReport::active_publisher_ids(&db, since_year, since_month)
            .await
            .unwrap_or_default()
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
        if *restarted.peek() { return; }
        restarted.set(true);
        users.restart();
        active_ids_res.restart();
        groups_res.restart();
    });

    let mut filters = use_signal(Filters::default);
    let mut display_limit = use_signal(|| PAGE_SIZE);
    let mut sheet_open = use_signal(|| false);

    let filtered = use_memo(move || {
        let all = users().unwrap_or_default();
        let active_ids = active_ids_res().unwrap_or_default();
        let groups = groups_res().unwrap_or_default();
        let f = filters();
        let norm = normalize(&f.name);

        // Build map: user_id_str -> (group_id_str, group_name)
        let mut user_group_map: std::collections::HashMap<String, (String, String)> =
            std::collections::HashMap::new();
        for group in &groups {
            let gid = group.id.as_ref().map(rid_to_str).unwrap_or_default();
            let gname = group.name.clone();
            for rid in &group.members {
                user_group_map
                    .entry(rid_to_str(rid))
                    .or_insert_with(|| (gid.clone(), gname.clone()));
            }
            if let Some(rid) = &group.overseer {
                user_group_map
                    .entry(rid_to_str(rid))
                    .or_insert_with(|| (gid.clone(), gname.clone()));
            }
            if let Some(rid) = &group.assistant {
                user_group_map
                    .entry(rid_to_str(rid))
                    .or_insert_with(|| (gid.clone(), gname.clone()));
            }
        }

        let mut result: Vec<(User, Option<bool>, Option<String>)> = all
            .into_iter()
            .filter(|p| {
                if !norm.is_empty() {
                    let full = normalize(&format!("{} {}", p.first_name, p.last_name));
                    if !full.contains(&norm) { return false; }
                }
                if let Some(g) = &f.gender {
                    if &p.gender != g { return false; }
                }
                if let Some(c) = &f.user_type {
                    if &p.user_type != c { return false; }
                }
                match &f.appointment {
                    AppointmentFilter::All => {}
                    AppointmentFilter::WithoutAppointment => {
                        if p.appointment.is_some() { return false; }
                    }
                    AppointmentFilter::Elder => {
                        if !matches!(p.appointment, Some(Appointment::Elder)) { return false; }
                    }
                    AppointmentFilter::MinisterialServant => {
                        if !matches!(p.appointment, Some(Appointment::MinisterialServant)) { return false; }
                    }
                }
                if let Some(fh) = f.family_head {
                    if p.family_head != fh { return false; }
                }
                // Activity filter only applies to publisher-type users.
                if is_publisher_type(&p.user_type) {
                    let uid_str = p.id.as_ref().map(rid_to_str).unwrap_or_default();
                    let is_act = active_ids.contains(&uid_str);
                    match f.activity {
                        ActivityFilter::Active => { if !is_act { return false; } }
                        ActivityFilter::Inactive => { if is_act { return false; } }
                        ActivityFilter::All => {}
                    }
                } else if !matches!(f.activity, ActivityFilter::All) {
                    return false;
                }
                // Group filter
                let uid_str = p.id.as_ref().map(rid_to_str).unwrap_or_default();
                match &f.group_filter {
                    GroupFilter::All => {}
                    GroupFilter::NoGroup => {
                        if user_group_map.contains_key(&uid_str) { return false; }
                    }
                    GroupFilter::InGroup(gid) => {
                        match user_group_map.get(&uid_str) {
                            Some((user_gid, _)) => {
                                if user_gid != gid { return false; }
                            }
                            None => return false,
                        }
                    }
                }
                true
            })
            .map(|p| {
                let uid_str = p.id.as_ref().map(rid_to_str).unwrap_or_default();
                let activity = if is_publisher_type(&p.user_type) {
                    Some(active_ids.contains(&uid_str))
                } else {
                    None
                };
                let group_name = user_group_map.get(&uid_str).map(|(_, n)| n.clone());
                (p, activity, group_name)
            })
            .collect();

        result.sort_by(|a, b| {
            let ka = normalize(&format!("{} {}", a.0.last_name, a.0.first_name));
            let kb = normalize(&format!("{} {}", b.0.last_name, b.0.first_name));
            ka.cmp(&kb)
        });
        result
    });

    let is_loading = users.read().is_none();
    let total = filtered.read().len();
    let limit = *display_limit.read();
    let shown: Vec<(User, Option<bool>, Option<String>)> = filtered.read()[..limit.min(total)].to_vec();
    let has_more = limit < total;

    // Build group options for the group filter dropdown: (id_str, name)
    let group_opts: Vec<(String, String)> = groups_res()
        .unwrap_or_default()
        .iter()
        .filter_map(|g| g.id.as_ref().map(|id| (rid_to_str(id), g.name.clone())))
        .collect();

    rsx! {
        div { class: "relative w-full space-y-4 pb-24",
            // ── Filter card ───────────────────────────────────────────────
            FilterCard {
                filters,
                group_options: group_opts,
                on_filters_change: move |f: Filters| {
                    filters.set(f);
                    display_limit.set(PAGE_SIZE);
                },
            }

            // ── User list ────────────────────────────────────────────
            if is_loading {
                div { class: "flex justify-center items-center py-20 text-gray-400",
                    span { class: "text-sm animate-pulse", {t!("user-loading")} }
                }
            } else if shown.is_empty() {
                EmptyUsers {}
            } else {
                div { class: "space-y-2",
                    for (p , is_active , group_name) in shown {
                        UserCard {
                            user: p,
                            name_fmt: name_fmt.read().clone(),
                            is_active,
                            group_name,
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
                                {t!("user-load-more")}
                            }
                        }
                    }
                }
            }

            // ── Floating add button ───────────────────────────────────────
            button {
                class: "fixed bottom-6 right-6 w-14 h-14 bg-primary-600 text-white rounded-full shadow-xl hover:bg-primary-700 active:scale-95 transition-all flex items-center justify-center text-2xl z-20 select-none",
                onclick: move |_| sheet_open.set(true),
                "＋"
            }

            // ── Add user modal ───────────────────────────────────────
            AddUserModal {
                open: sheet_open,
                on_close: move |_| sheet_open.set(false),
                on_created: move |_| {
                    users.restart();
                    sheet_open.set(false);
                },
                date_fmt: date_fmt.read().clone(),
            }
        }
    }
}

// ── Filter card ───────────────────────────────────────────────────────────────

#[component]
fn FilterCard(
    filters: Signal<Filters>,
    group_options: Vec<(String, String)>,
    on_filters_change: Callback<Filters>,
) -> Element {
    let f = filters();

    rsx! {
        div { class: "bg-white rounded-xl border border-gray-200 p-4 space-y-3",
            // Name search
            input {
                r#type: "text",
                class: "w-full px-3 py-2 text-sm border border-gray-200 rounded-lg focus:outline-none focus:ring-2 focus:ring-primary-500 placeholder-gray-400",
                placeholder: t!("user-search-placeholder"),
                value: f.name.clone(),
                oninput: move |e| {
                    let mut new = filters();
                    new.name = e.value();
                    on_filters_change.call(new);
                },
            }

            // Gender + Category
            div { class: "grid grid-cols-2 gap-2",
                div { class: "flex flex-col gap-1",
                    span { class: "text-xs font-medium text-gray-500", {t!("user-filter-gender")} }
                    select {
                        class: "w-full px-2 py-1.5 text-sm border border-gray-200 rounded-lg bg-white focus:outline-none focus:ring-2 focus:ring-primary-500",
                        onchange: move |e| {
                            let mut new = filters();
                            new.gender = match e.value().as_str() {
                                "male" => Some(Gender::Male),
                                "female" => Some(Gender::Female),
                                _ => None,
                            };
                            on_filters_change.call(new);
                        },
                        option { value: "", {t!("user-filter-all")} }
                        option { value: "male", {t!("user-gender-male")} }
                        option { value: "female", {t!("user-gender-female")} }
                    }
                }
                div { class: "flex flex-col gap-1",
                    span { class: "text-xs font-medium text-gray-500", {t!("user-filter-type")} }
                    select {
                        class: "w-full px-2 py-1.5 text-sm border border-gray-200 rounded-lg bg-white focus:outline-none focus:ring-2 focus:ring-primary-500",
                        onchange: move |e| {
                            let mut new = filters();
                            new.user_type = match e.value().as_str() {
                                "student" => Some(UserType::Student),
                                "publisher" => Some(UserType::Publisher),
                                "baptized" => Some(UserType::BaptizedPublisher),
                                "cont_aux" => Some(UserType::ContinuousAuxiliaryPioneer),
                                "regular" => Some(UserType::RegularPioneer),
                                "special" => Some(UserType::SpecialPioneer),
                                "missionary" => Some(UserType::Missionary),
                                _ => None,
                            };
                            on_filters_change.call(new);
                        },
                        option { value: "", {t!("user-filter-all")} }
                        option { value: "student", {t!("user-type-student")} }
                        option { value: "publisher", {t!("user-type-publisher")} }
                        option { value: "baptized", {t!("user-type-baptized")} }
                        option { value: "cont_aux", {t!("user-type-cont-aux-pioneer")} }
                        option { value: "regular", {t!("user-type-regular-pioneer")} }
                        option { value: "special", {t!("user-type-special-pioneer")} }
                        option { value: "missionary", {t!("user-type-missionary")} }
                    }
                }
            }

            // Appointment + Family head
            div { class: "grid grid-cols-2 gap-2",
                div { class: "flex flex-col gap-1",
                    span { class: "text-xs font-medium text-gray-500", {t!("user-filter-appointment")} }
                    select {
                        class: "w-full px-2 py-1.5 text-sm border border-gray-200 rounded-lg bg-white focus:outline-none focus:ring-2 focus:ring-primary-500",
                        onchange: move |e| {
                            let mut new = filters();
                            new.appointment = match e.value().as_str() {
                                "none" => AppointmentFilter::WithoutAppointment,
                                "elder" => AppointmentFilter::Elder,
                                "ms" => AppointmentFilter::MinisterialServant,
                                _ => AppointmentFilter::All,
                            };
                            on_filters_change.call(new);
                        },
                        option { value: "", {t!("user-filter-all")} }
                        option { value: "none", {t!("user-appointment-none")} }
                        option { value: "elder", {t!("user-appointment-elder")} }
                        option { value: "ms", {t!("user-appointment-ms")} }
                    }
                }
                div { class: "flex flex-col gap-1",
                    span { class: "text-xs font-medium text-gray-500", {t!("user-filter-family-head")} }
                    select {
                        class: "w-full px-2 py-1.5 text-sm border border-gray-200 rounded-lg bg-white focus:outline-none focus:ring-2 focus:ring-primary-500",
                        onchange: move |e| {
                            let mut new = filters();
                            new.family_head = match e.value().as_str() {
                                "yes" => Some(true),
                                "no" => Some(false),
                                _ => None,
                            };
                            on_filters_change.call(new);
                        },
                        option { value: "", {t!("user-filter-all")} }
                        option { value: "yes", {t!("user-yes")} }
                        option { value: "no", {t!("user-no")} }
                    }
                }
            }

            // Group filter
            div { class: "flex flex-col gap-1",
                span { class: "text-xs font-medium text-gray-500", {t!("user-filter-group")} }
                select {
                    class: "w-full px-2 py-1.5 text-sm border border-gray-200 rounded-lg bg-white focus:outline-none focus:ring-2 focus:ring-primary-500",
                    onchange: move |e| {
                        let mut new = filters();
                        new.group_filter = match e.value().as_str() {
                            "none" => GroupFilter::NoGroup,
                            v if !v.is_empty() => GroupFilter::InGroup(v.to_string()),
                            _ => GroupFilter::All,
                        };
                        on_filters_change.call(new);
                    },
                    option { value: "", {t!("user-filter-all")} }
                    option { value: "none", {t!("user-filter-no-group")} }
                    for (gid , gname) in group_options.iter() {
                        {
                            let gid = gid.clone();
                            let gname = gname.clone();
                            rsx! {
                                option { value: "{gid}", "{gname}" }
                            }
                        }
                    }
                }
            }
        }
    }
}

// ── User card ────────────────────────────────────────────────────────────

#[component]
fn UserCard(user: User, name_fmt: NameFormat, is_active: Option<bool>, group_name: Option<String>) -> Element {
    let nav = use_navigator();
    let id_str = user
        .id
        .as_ref()
        .map(|id| match &id.key {
            surrealdb::types::RecordIdKey::String(k) => k.clone(),
            surrealdb::types::RecordIdKey::Number(n) => n.to_string(),
            _ => String::new(),
        })
        .unwrap_or_default();
    let display_name = format_name(&user.first_name, &user.last_name, &name_fmt);
    let initials = format!(
        "{}{}",
        user.first_name.chars().next().unwrap_or('?').to_ascii_uppercase(),
        user.last_name.chars().next().unwrap_or('?').to_ascii_uppercase(),
    );

    let gender_icon = match user.gender {
        Gender::Male => "♂",
        Gender::Female => "♀",
    };

    let category_label = match &user.user_type {
        UserType::Student => t!("user-type-student"),
        UserType::Publisher => t!("user-type-publisher"),
        UserType::BaptizedPublisher => t!("user-type-baptized"),
        UserType::ContinuousAuxiliaryPioneer => t!("user-type-cont-aux-pioneer"),
        UserType::RegularPioneer => t!("user-type-regular-pioneer"),
        UserType::SpecialPioneer => t!("user-type-special-pioneer"),
        UserType::Missionary => t!("user-type-missionary"),
    };

    let appt = user.appointment.as_ref().map(|a| match a {
        Appointment::Elder => t!("user-appointment-elder"),
        Appointment::MinisterialServant => t!("user-appointment-ms"),
    });

    rsx! {
        div {
            class: "bg-white rounded-xl border border-gray-200 px-4 py-3 flex items-center gap-3 hover:border-primary-200 hover:shadow-sm transition-all cursor-pointer",
            onclick: move |_| {
                let _ = nav
                    .push(Route::AppUserDetail {
                        id: id_str.clone(),
                    });
            },
            div { class: "w-10 h-10 rounded-full bg-primary-100 text-primary-700 flex items-center justify-center font-semibold text-sm shrink-0",
                "{initials}"
            }
            div { class: "flex-1 min-w-0",
                div { class: "flex items-center gap-1.5",
                    span { class: "text-sm font-medium text-gray-900 truncate", "{display_name}" }
                    span { class: "text-gray-400 text-xs shrink-0", "{gender_icon}" }
                }
                div { class: "flex items-center gap-1.5 mt-0.5 flex-wrap",
                    span { class: "inline-flex px-1.5 py-0.5 rounded text-xs font-medium bg-gray-100 text-gray-700",
                        "{category_label}"
                    }
                    if let Some(a) = appt {
                        span { class: "inline-flex px-1.5 py-0.5 rounded text-xs font-medium bg-amber-100 text-amber-800",
                            "{a}"
                        }
                    }
                    if user.family_head {
                        span { class: "inline-flex px-1.5 py-0.5 rounded text-xs font-medium bg-purple-600 text-white",
                            {t!("user-family-head-badge")}
                        }
                    }
                    if let Some(active) = is_active {
                        if active {
                            span { class: "inline-flex px-1.5 py-0.5 rounded text-xs font-medium bg-emerald-600 text-white",
                                {t!("user-badge-active")}
                            }
                        } else {
                            span { class: "inline-flex px-1.5 py-0.5 rounded text-xs font-medium bg-gray-400 text-white",
                                {t!("user-badge-inactive")}
                            }
                        }
                    }
                    if let Some(gname) = &group_name {
                        span { class: "inline-flex px-1.5 py-0.5 rounded text-xs font-medium bg-teal-100 text-teal-800",
                            "{gname}"
                        }
                    }
                }
            }
            span { class: "text-gray-300 text-sm shrink-0", "›" }
        }
    }
}

// ── Empty state ───────────────────────────────────────────────────────────────

#[component]
fn EmptyUsers() -> Element {
    rsx! {
        div { class: "bg-white rounded-xl border border-gray-200",
            div { class: "px-6 py-16 text-center",
                p { class: "text-5xl mb-3", "👤" }
                p { class: "font-medium text-gray-600 text-base", {t!("empty-users-title")} }
                p { class: "text-sm mt-1 text-gray-400", {t!("empty-users-desc")} }
            }
        }
    }
}

// ── User form body (shared between mobile sheet and desktop dialog) ──────

#[component]
pub fn UserFormBody(form: Signal<UserFormState>, date_fmt: DateFormat) -> Element {
    let date_hint = date_format_hint(&date_fmt);
    let f = form.read().clone();
    rsx! {
        // Error banner
        if let Some(err) = &f.error {
            div { class: "bg-red-50 border border-red-200 text-red-700 text-sm px-3 py-2 rounded-lg",
                "{err}"
            }
        }
        // First + Last name
        div { class: "grid grid-cols-2 gap-3",
            div { class: "flex flex-col gap-1",
                label { class: "text-xs font-medium text-gray-700",
                    {t!("user-form-firstname")}
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
                    {t!("user-form-lastname")}
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
        // Gender
        div { class: "flex flex-col gap-2",
            label { class: "text-xs font-medium text-gray-700",
                {t!("user-form-gender")}
                span { class: "text-red-500 ml-0.5", " *" }
            }
            div { class: "flex gap-4",
                label { class: "flex items-center gap-2 cursor-pointer",
                    input {
                        r#type: "radio",
                        name: "user-gender",
                        value: "male",
                        checked: f.gender == "male",
                        onchange: move |_| form.write().gender = "male".to_string(),
                    }
                    span { class: "text-sm text-gray-700", {t!("user-gender-male")} }
                }
                label { class: "flex items-center gap-2 cursor-pointer",
                    input {
                        r#type: "radio",
                        name: "user-gender",
                        value: "female",
                        checked: f.gender == "female",
                        onchange: move |_| form.write().gender = "female".to_string(),
                    }
                    span { class: "text-sm text-gray-700", {t!("user-gender-female")} }
                }
            }
        }
        // Type
        div { class: "flex flex-col gap-1",
            label { class: "text-xs font-medium text-gray-700", {t!("user-form-type")} }
            select {
                class: "w-full px-3 py-2 text-sm border border-gray-200 rounded-lg bg-white focus:outline-none focus:ring-2 focus:ring-primary-500",
                onchange: move |e| form.write().user_type = e.value(),
                option {
                    value: "student",
                    selected: f.user_type == "student" || f.user_type.is_empty(),
                    {t!("user-type-student")}
                }
                option { value: "publisher", selected: f.user_type == "publisher",
                    {t!("user-type-publisher")}
                }
                option { value: "baptized", selected: f.user_type == "baptized",
                    {t!("user-type-baptized")}
                }
                option { value: "cont_aux", selected: f.user_type == "cont_aux",
                    {t!("user-type-cont-aux-pioneer")}
                }
                option { value: "regular", selected: f.user_type == "regular",
                    {t!("user-type-regular-pioneer")}
                }
                option { value: "special", selected: f.user_type == "special",
                    {t!("user-type-special-pioneer")}
                }
                option {
                    value: "missionary",
                    selected: f.user_type == "missionary",
                    {t!("user-type-missionary")}
                }
            }
        }
        // Appointment (male only)
        if f.gender == "male" {
            div { class: "flex flex-col gap-1",
                label { class: "text-xs font-medium text-gray-700", {t!("user-form-appointment")} }
                select {
                    class: "w-full px-3 py-2 text-sm border border-gray-200 rounded-lg bg-white focus:outline-none focus:ring-2 focus:ring-primary-500",
                    onchange: move |e| form.write().appointment = e.value(),
                    option { value: "", selected: f.appointment.is_empty(),
                        {t!("user-appointment-none")}
                    }
                    option { value: "elder", selected: f.appointment == "elder",
                        {t!("user-appointment-elder")}
                    }
                    option { value: "ms", selected: f.appointment == "ms", {t!("user-appointment-ms")} }
                }
            }
        }
        // Birthday + Baptism date
        div { class: "grid grid-cols-2 gap-3",
            div { class: "flex flex-col gap-1",
                label { class: "text-xs font-medium text-gray-700",
                    {t!("user-form-birthday")}
                    span { class: "text-xs text-gray-400 ml-1", "({date_hint})" }
                }
                input {
                    r#type: "date",
                    class: "w-full px-3 py-2 text-sm border border-gray-200 rounded-lg focus:outline-none focus:ring-2 focus:ring-primary-500",
                    value: f.birthday.clone(),
                    oninput: move |e| form.write().birthday = e.value(),
                }
            }
            div { class: "flex flex-col gap-1",
                label { class: "text-xs font-medium text-gray-700",
                    {t!("user-form-baptism-date")}
                    span { class: "text-xs text-gray-400 ml-1", "({date_hint})" }
                }
                input {
                    r#type: "date",
                    class: "w-full px-3 py-2 text-sm border border-gray-200 rounded-lg focus:outline-none focus:ring-2 focus:ring-primary-500",
                    value: f.baptism_date.clone(),
                    oninput: move |e| form.write().baptism_date = e.value(),
                }
            }
        }
        // Phone + Email
        div { class: "grid grid-cols-2 gap-3",
            div { class: "flex flex-col gap-1",
                label { class: "text-xs font-medium text-gray-700", {t!("user-form-phone")} }
                input {
                    r#type: "tel",
                    class: "w-full px-3 py-2 text-sm border border-gray-200 rounded-lg focus:outline-none focus:ring-2 focus:ring-primary-500",
                    value: f.phone.clone(),
                    oninput: move |e| form.write().phone = e.value(),
                }
            }
            div { class: "flex flex-col gap-1",
                label { class: "text-xs font-medium text-gray-700", {t!("user-form-email")} }
                input {
                    r#type: "email",
                    class: "w-full px-3 py-2 text-sm border border-gray-200 rounded-lg focus:outline-none focus:ring-2 focus:ring-primary-500",
                    value: f.email.clone(),
                    oninput: move |e| form.write().email = e.value(),
                }
            }
        }
        // Address
        div { class: "flex flex-col gap-1",
            label { class: "text-xs font-medium text-gray-700", {t!("user-form-address")} }
            input {
                r#type: "text",
                class: "w-full px-3 py-2 text-sm border border-gray-200 rounded-lg focus:outline-none focus:ring-2 focus:ring-primary-500",
                value: f.address.clone(),
                oninput: move |e| form.write().address = e.value(),
            }
        }
        // Password
        div { class: "flex flex-col gap-1",
            label { class: "text-xs font-medium text-gray-700", {t!("user-form-password")} }
            input {
                r#type: "password",
                class: "w-full px-3 py-2 text-sm border border-gray-200 rounded-lg focus:outline-none focus:ring-2 focus:ring-primary-500",
                value: f.password.clone(),
                oninput: move |e| form.write().password = e.value(),
            }
        }
        // Family head
        label { class: "flex items-center gap-3 cursor-pointer py-1",
            input {
                r#type: "checkbox",
                class: "w-4 h-4 rounded border-gray-300 accent-primary-600",
                checked: f.family_head,
                onchange: move |e| form.write().family_head = e.checked(),
            }
            span { class: "text-sm text-gray-700", {t!("user-form-family-head")} }
        }
    }
}

// ── Add user modal ───────────────────────────────────────────────────────

#[component]
fn AddUserModal(open: Signal<bool>, on_close: Callback<()>, on_created: Callback<()>, date_fmt: DateFormat) -> Element {
    let db_signal = use_db();
    let crypto_signal = use_crypto();

    let mut form = use_signal(UserFormState::default);

    // Reset form each time the modal opens.
    use_effect(move || {
        if *open.read() {
            form.set(UserFormState::default());
        }
    });

    let f = form.read().clone();

    let on_submit = use_callback(move |_: Event<MouseData>| {
        let fd = form.read().clone();
        if fd.first_name.trim().is_empty() || fd.last_name.trim().is_empty() || fd.gender.is_empty()
        {
            form.write().error = Some(t!("user-form-required-error"));
            return;
        }

        let gender = if fd.gender == "male" {
            Gender::Male
        } else {
            Gender::Female
        };
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
            congregations: vec![], // TODO: link to active congregation from context
            active: true,
        };

        form.write().submitting = true;
        form.write().error = None;

        spawn(async move {
            let db_opt = db_signal.read().db.clone();
            let Some(db) = db_opt else {
                form.write().submitting = false;
                form.write().error = Some("No database connection.".to_string());
                return;
            };
            // Surreal<Any> is Arc-wrapped — clones share the same connection and
            // session (ns/db already set by connect_offline). Do NOT call use_ns/
            // use_db here: it would corrupt the shared session for all other
            // concurrent queries (e.g. User::all) and write to the wrong namespace.
            let crypto = crypto_signal.read().clone();
            match User::create(&db, &crypto, data).await {
                Ok(_) => on_created.call(()),
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
            title: t!("user-add-title"),
            description: t!("user-add-desc"),
            submitting: f.submitting,
            on_submit,
            UserFormBody { form, date_fmt }
        }
    }
}
