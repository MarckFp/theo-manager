use dioxus::prelude::*;
use dioxus_i18n::t;

use crate::components::ResponsiveModal;
use crate::database::{use_crypto, use_db};
use crate::models::user::{Appointment, Gender, User, UserData, UserType};

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
    name: String,
    gender: Option<Gender>,
    user_type: Option<UserType>,
    appointment: AppointmentFilter,
    family_head: Option<bool>,
}

// ── Form state ────────────────────────────────────────────────────────────────

#[derive(Clone, Default)]
struct UserFormState {
    first_name: String,
    last_name: String,
    gender: String,
    user_type: String,
    appointment: String,
    birthday: String,
    baptism_date: String,
    phone: String,
    address: String,
    email: String,
    password: String,
    family_head: bool,
    submitting: bool,
    error: Option<String>,
}

// ── User type key helpers ─────────────────────────────────────────────────────

fn key_to_user_type(s: &str) -> UserType {
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

// ── Main page ─────────────────────────────────────────────────────────────────

#[component]
pub fn AppUsers() -> Element {
    let db_signal = use_db();
    let crypto_signal = use_crypto();

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

    let mut restarted = use_signal(|| false);
    use_effect(move || {
        if *restarted.peek() { return; }
        restarted.set(true);
        users.restart();
    });

    let mut filters = use_signal(Filters::default);
    let mut display_limit = use_signal(|| PAGE_SIZE);
    let mut sheet_open = use_signal(|| false);

    let filtered = use_memo(move || {
        let all = users().unwrap_or_default();
        let f = filters();
        let norm = normalize(&f.name);

        let mut result: Vec<User> = all
            .into_iter()
            .filter(|p| {
                if !norm.is_empty() {
                    let full = normalize(&format!("{} {}", p.first_name, p.last_name));
                    if !full.contains(&norm) {
                        return false;
                    }
                }
                if let Some(g) = &f.gender {
                    if &p.gender != g {
                        return false;
                    }
                }
                if let Some(c) = &f.user_type {
                    if &p.user_type != c {
                        return false;
                    }
                }
                match &f.appointment {
                    AppointmentFilter::All => {}
                    AppointmentFilter::WithoutAppointment => {
                        if p.appointment.is_some() {
                            return false;
                        }
                    }
                    AppointmentFilter::Elder => {
                        if !matches!(p.appointment, Some(Appointment::Elder)) {
                            return false;
                        }
                    }
                    AppointmentFilter::MinisterialServant => {
                        if !matches!(p.appointment, Some(Appointment::MinisterialServant)) {
                            return false;
                        }
                    }
                }
                if let Some(fh) = f.family_head {
                    if p.family_head != fh {
                        return false;
                    }
                }
                true
            })
            .collect();

        result.sort_by(|a, b| {
            let ka = normalize(&format!("{} {}", a.last_name, a.first_name));
            let kb = normalize(&format!("{} {}", b.last_name, b.first_name));
            ka.cmp(&kb)
        });
        result
    });

    let is_loading = users.read().is_none();
    let total = filtered.read().len();
    let limit = *display_limit.read();
    let shown: Vec<User> = filtered.read()[..limit.min(total)].to_vec();
    let has_more = limit < total;

    rsx! {
        div { class: "relative w-full space-y-4 pb-24",
            // ── Filter card ───────────────────────────────────────────────
            FilterCard {
                filters,
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
                    for p in shown {
                        UserCard { user: p }
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
            }
        }
    }
}

// ── Filter card ───────────────────────────────────────────────────────────────

#[component]
fn FilterCard(filters: Signal<Filters>, on_filters_change: Callback<Filters>) -> Element {
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
        }
    }
}

// ── User card ────────────────────────────────────────────────────────────

#[component]
fn UserCard(user: User) -> Element {
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
        div { class: "bg-white rounded-xl border border-gray-200 px-4 py-3 flex items-center gap-3 hover:border-gray-300 transition-colors cursor-pointer",
            div { class: "w-10 h-10 rounded-full bg-primary-100 text-primary-700 flex items-center justify-center font-semibold text-sm shrink-0",
                "{initials}"
            }
            div { class: "flex-1 min-w-0",
                div { class: "flex items-center gap-1.5",
                    span { class: "text-sm font-medium text-gray-900 truncate",
                        "{user.first_name} {user.last_name}"
                    }
                    span { class: "text-gray-400 text-xs shrink-0", "{gender_icon}" }
                }
                div { class: "flex items-center gap-1.5 mt-0.5 flex-wrap",
                    span { class: "text-xs text-gray-500", "{category_label}" }
                    if let Some(a) = appt {
                        span { class: "inline-flex px-1.5 py-0.5 rounded text-xs font-medium bg-amber-100 text-amber-800",
                            "{a}"
                        }
                    }
                    if user.family_head {
                        span { class: "inline-flex px-1.5 py-0.5 rounded text-xs font-medium bg-green-100 text-green-700",
                            {t!("user-family-head-badge")}
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
fn UserFormBody(form: Signal<UserFormState>) -> Element {
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
                option { value: "student", {t!("user-type-student")} }
                option { value: "publisher", {t!("user-type-publisher")} }
                option { value: "baptized", {t!("user-type-baptized")} }
                option { value: "cont_aux", {t!("user-type-cont-aux-pioneer")} }
                option { value: "regular", {t!("user-type-regular-pioneer")} }
                option { value: "special", {t!("user-type-special-pioneer")} }
                option { value: "missionary", {t!("user-type-missionary")} }
            }
        }
        // Appointment (male only)
        if f.gender == "male" {
            div { class: "flex flex-col gap-1",
                label { class: "text-xs font-medium text-gray-700", {t!("user-form-appointment")} }
                select {
                    class: "w-full px-3 py-2 text-sm border border-gray-200 rounded-lg bg-white focus:outline-none focus:ring-2 focus:ring-primary-500",
                    onchange: move |e| form.write().appointment = e.value(),
                    option { value: "", {t!("user-appointment-none")} }
                    option { value: "elder", {t!("user-appointment-elder")} }
                    option { value: "ms", {t!("user-appointment-ms")} }
                }
            }
        }
        // Birthday + Baptism date
        div { class: "grid grid-cols-2 gap-3",
            div { class: "flex flex-col gap-1",
                label { class: "text-xs font-medium text-gray-700", {t!("user-form-birthday")} }
                input {
                    r#type: "date",
                    class: "w-full px-3 py-2 text-sm border border-gray-200 rounded-lg focus:outline-none focus:ring-2 focus:ring-primary-500",
                    value: f.birthday.clone(),
                    oninput: move |e| form.write().birthday = e.value(),
                }
            }
            div { class: "flex flex-col gap-1",
                label { class: "text-xs font-medium text-gray-700", {t!("user-form-baptism-date")} }
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
fn AddUserModal(open: Signal<bool>, on_close: Callback<()>, on_created: Callback<()>) -> Element {
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
            UserFormBody { form }
        }
    }
}
