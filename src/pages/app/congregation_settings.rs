use dioxus::prelude::*;
use dioxus_i18n::t;

use crate::components::ThemePreview;
use crate::crypto::{KeyStore, SessionCrypto};
use crate::database::{use_crypto, use_db};
use crate::models::congregation::{AccentColor, Congregation, CongregationData, DateFormat, NameFormat, Theme, TimeFormat};
use crate::models::user::{User, UserData};
use crate::models::absence::{Absence, AbsenceData};
use crate::models::emergency_contact::{EmergencyContact, EmergencyContactData};
use crate::models::field_service_group::{FieldServiceGroup, FieldServiceGroupData};
use crate::models::migrate;

#[component]
fn FormField(label: String, children: Element) -> Element {
    rsx! {
        div {
            label { class: "block text-sm font-medium text-gray-700 mb-1", "{label}" }
            {children}
        }
    }
}

#[component]
pub fn AppCongregationSettings() -> Element {
    let db_signal = use_db();
    let crypto_signal = use_crypto();

    // Shared congregation resource provided by AppLayout — restarting it
    // triggers the theme/accent use_effect in AppLayout to re-run.
    let mut layout_congregation = use_context::<Resource<Option<Congregation>>>();

    // ── Load Congregation ───────────────────────────────────────────
    let mut congregation_res = use_resource(move || async move {
        let db_opt = db_signal.read().db.clone();
        let Some(db) = db_opt else { return None };
        let crypto = crypto_signal.read().clone();
        match Congregation::all(&db, &crypto).await {
            Ok(all) => all.into_iter().next(),
            Err(e) => {
                let err_str = e.to_string().replace("'", "\\'").replace("\n", " ");
                let js = format!("console.error('Congregation::all err:', '{}');", err_str);
                let _ = document::eval(&js);
                None
            }
        }
    });

    let mut is_editing = use_signal(|| false);
    let mut save_loading = use_signal(|| false);
    let mut error = use_signal(|| Option::<String>::None);
    let mut success = use_signal(|| Option::<String>::None);

    let cong_opt = congregation_res.read_unchecked().clone().flatten();

    // Settings forms
    let mut cong_name = use_signal(|| String::new());
    let mut cong_address = use_signal(|| String::new());
    let mut cong_circuit = use_signal(|| String::new());
    let mut cong_language = use_signal(|| String::new());
    let mut time_format = use_signal(|| TimeFormat::default());
    let mut date_format = use_signal(|| DateFormat::default());
    let mut name_format = use_signal(|| NameFormat::default());
    let mut theme = use_signal(|| Theme::default());
    let mut accent_color = use_signal(|| crate::models::congregation::AccentColor::default());

    use_effect(move || {
        if let Some(c) = congregation_res.read().as_ref().cloned().flatten() {
            if !*is_editing.peek() {
                cong_name.set(c.name);
                cong_address.set(c.address.unwrap_or_default());
                cong_circuit.set(c.circuit.unwrap_or_default());
                cong_language.set(c.language);
                time_format.set(c.time_format);
                date_format.set(c.date_format);
                name_format.set(c.name_format);
                theme.set(c.theme);
                accent_color.set(c.accent_color);
            }
        }
    });

    // ── Password change state ───────────────────────────────────────────
    let mut old_password = use_signal(|| String::new());
    let mut new_password = use_signal(|| String::new());
    let mut confirm_password = use_signal(|| String::new());
    let mut password_loading = use_signal(|| false);
    let mut password_error = use_signal(|| Option::<String>::None);
    let mut password_success = use_signal(|| Option::<String>::None);

    // ── Export / Import ───────────────────────────────────────────
    let mut io_loading = use_signal(|| false);
    let mut io_error = use_signal(|| Option::<String>::None);

    rsx! {
        div { class: "max-w-4xl mx-auto space-y-6 w-full pb-24",
            h1 { class: "text-2xl font-bold text-gray-900", {t!("page-congregation-settings")} }

            // Loading state
            if congregation_res.read().is_none() {
                div { class: "flex justify-center items-center py-20 text-gray-400",
                    span { class: "text-sm animate-pulse", {t!("user-loading")} }
                }
            } else if cong_opt.is_none() {
                div { class: "bg-white rounded-xl border border-gray-200 px-6 py-12 text-center text-gray-400",
                    p { class: "text-4xl mb-3", "⚙️" }
                    p { class: "font-medium text-gray-600", {t!("empty-cong-settings-title")} }
                    p { class: "text-sm mt-1", {t!("empty-cong-settings-desc")} }
                }
            } else {
                // ── Congregation Details Form ───────────────────────────────────────────
                div { class: "bg-white rounded-xl border border-gray-200 overflow-hidden",
                    div { class: "px-6 py-4 border-b border-gray-200 bg-gray-50 flex justify-between items-center",
                        h2 { class: "text-lg font-semibold text-gray-800",
                            {t!("congregation-details")}
                        }
                        button {
                            class: "text-primary-600 hover:underline text-sm font-medium",
                            onclick: move |_| {
                                error.set(None);
                                success.set(None);
                                let current = *is_editing.peek();
                                is_editing.set(!current);
                            },
                            if *is_editing.read() {
                                {t!("btn-cancel")}
                            } else {
                                {t!("btn-edit")}
                            }
                        }
                    }
                    div { class: "p-6 space-y-4",
                        if let Some(err) = error.read().clone() {
                            div { class: "bg-red-50 border border-red-200 rounded-lg p-3 text-red-700 text-sm",
                                "{err}"
                            }
                        }
                        if let Some(msg) = success.read().clone() {
                            div { class: "bg-green-50 border border-green-200 rounded-lg p-3 text-green-700 text-sm",
                                "{msg}"
                            }
                        }

                        div { class: "grid grid-cols-1 md:grid-cols-2 gap-4",
                            FormField { label: t!("onboarding-congregation-name"),
                                input {
                                    class: "w-full border border-gray-300 rounded-lg px-3 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-primary-500",
                                    disabled: !*is_editing.read(),
                                    value: cong_name.read().clone(),
                                    oninput: move |e| cong_name.set(e.value()),
                                }
                            }
                            FormField { label: t!("onboarding-congregation-address"),
                                input {
                                    class: "w-full border border-gray-300 rounded-lg px-3 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-primary-500",
                                    disabled: !*is_editing.read(),
                                    value: cong_address.read().clone(),
                                    oninput: move |e| cong_address.set(e.value()),
                                }
                            }
                            FormField { label: t!("onboarding-congregation-circuit"),
                                input {
                                    class: "w-full border border-gray-300 rounded-lg px-3 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-primary-500",
                                    disabled: !*is_editing.read(),
                                    value: cong_circuit.read().clone(),
                                    oninput: move |e| cong_circuit.set(e.value()),
                                }
                            }
                            FormField { label: t!("onboarding-congregation-language"),
                                select {
                                    class: "w-full border border-gray-300 rounded-lg px-3 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-primary-500 bg-white",
                                    disabled: !*is_editing.read(),
                                    value: cong_language.read().clone(),
                                    onchange: move |e| cong_language.set(e.value()),
                                    option { value: "en-US", "\u{1f1fa}\u{1f1f8} English" }
                                    option { value: "es-ES", "\u{1f1ea}\u{1f1f8} Español" }
                                }
                            }
                            FormField { label: t!("onboarding-congregation-time-format"),
                                select {
                                    class: "w-full border border-gray-300 rounded-lg px-3 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-primary-500 bg-white",
                                    disabled: !*is_editing.read(),
                                    value: match *time_format.read() {
                                        TimeFormat::H12 => "12h",
                                        TimeFormat::H24 => "24h",
                                    },
                                    onchange: move |e| {
                                        time_format
                                            .set(
                                                match e.value().as_str() {
                                                    "12h" => TimeFormat::H12,
                                                    _ => TimeFormat::H24,
                                                },
                                            );
                                    },
                                    option { value: "12h", "12h (AM/PM)" }
                                    option { value: "24h", "24h" }
                                }
                            }
                            FormField { label: t!("onboarding-congregation-date-format"),
                                select {
                                    class: "w-full border border-gray-300 rounded-lg px-3 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-primary-500 bg-white",
                                    disabled: !*is_editing.read(),
                                    value: match *date_format.read() {
                                        DateFormat::YMD => "YMD",
                                        DateFormat::DMY => "DMY",
                                        DateFormat::MDY => "MDY",
                                    },
                                    onchange: move |e| {
                                        date_format
                                            .set(
                                                match e.value().as_str() {
                                                    "DMY" => DateFormat::DMY,
                                                    "MDY" => DateFormat::MDY,
                                                    _ => DateFormat::YMD,
                                                },
                                            );
                                    },
                                    option { value: "YMD", "YYYY-MM-DD" }
                                    option { value: "DMY", "DD-MM-YYYY" }
                                    option { value: "MDY", "MM-DD-YYYY" }
                                }
                            }
                            FormField { label: t!("onboarding-congregation-name-format"),
                                select {
                                    class: "w-full border border-gray-300 rounded-lg px-3 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-primary-500 bg-white",
                                    disabled: !*is_editing.read(),
                                    value: match *name_format.read() {
                                        NameFormat::FirstLast => "FirstLast",
                                        NameFormat::LastFirst => "LastFirst",
                                    },
                                    onchange: move |e| {
                                        name_format
                                            .set(
                                                match e.value().as_str() {
                                                    "LastFirst" => NameFormat::LastFirst,
                                                    _ => NameFormat::FirstLast,
                                                },
                                            );
                                    },
                                    option { value: "FirstLast", {t!("format-first-last")} }
                                    option { value: "LastFirst", {t!("format-last-first")} }
                                }
                            }
                            FormField { label: t!("onboarding-congregation-theme"),
                                select {
                                    class: "w-full border border-gray-300 rounded-lg px-3 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-primary-500 bg-white",
                                    disabled: !*is_editing.read(),
                                    value: match *theme.read() {
                                        Theme::Light => "Light",
                                        Theme::Dark => "Dark",
                                    },
                                    onchange: move |e| {
                                        theme
                                            .set(
                                                match e.value().as_str() {
                                                    "Dark" => Theme::Dark,
                                                    _ => Theme::Light,
                                                },
                                            );
                                    },
                                    option { value: "Light", {t!("theme-light")} }
                                    option { value: "Dark", {t!("theme-dark")} }
                                }
                            }
                            FormField { label: t!("onboarding-congregation-accent-color"),
                                select {
                                    class: "w-full border border-gray-300 rounded-lg px-3 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-primary-500 bg-white",
                                    disabled: !*is_editing.read(),
                                    value: match *accent_color.read() {
                                        crate::models::congregation::AccentColor::Blue => "Blue",
                                        crate::models::congregation::AccentColor::Green => "Green",
                                        crate::models::congregation::AccentColor::Purple => "Purple",
                                        crate::models::congregation::AccentColor::Rose => "Rose",
                                        crate::models::congregation::AccentColor::Amber => "Amber",
                                    },
                                    onchange: move |e| {
                                        accent_color
                                            .set(
                                                match e.value().as_str() {
                                                    "Green" => crate::models::congregation::AccentColor::Green,
                                                    "Purple" => crate::models::congregation::AccentColor::Purple,
                                                    "Rose" => crate::models::congregation::AccentColor::Rose,
                                                    "Amber" => crate::models::congregation::AccentColor::Amber,
                                                    _ => crate::models::congregation::AccentColor::Blue,
                                                },
                                            );
                                    },
                                    option { value: "Blue", {t!("accent-blue")} }
                                    option { value: "Green", {t!("accent-green")} }
                                    option { value: "Purple", {t!("accent-purple")} }
                                    option { value: "Rose", {t!("accent-rose")} }
                                    option { value: "Amber", {t!("accent-amber")} }
                                }
                            }

                            // ── Theme preview ──────────────────────────
                            ThemePreview {
                                theme: match *theme.read() {
                                    Theme::Dark => "dark".to_string(),
                                    _ => "light".to_string(),
                                },
                                accent: match *accent_color.read() {
                                    AccentColor::Green => "Green".to_string(),
                                    AccentColor::Purple => "Purple".to_string(),
                                    AccentColor::Rose => "Rose".to_string(),
                                    AccentColor::Amber => "Amber".to_string(),
                                    _ => "Blue".to_string(),
                                },
                            }
                        
                        } // end grid

                        if *is_editing.read() {
                            div { class: "pt-4 flex justify-end",
                                button {
                                    class: "px-6 py-2 bg-primary-600 text-white rounded-lg font-medium hover:bg-primary-700 transition-colors disabled:opacity-50",
                                    disabled: *save_loading.read(),
                                    onclick: move |_| {
                                        if *save_loading.peek() {
                                            return;
                                        }
                                        let name = cong_name.read().clone();
                                        if name.is_empty() {
                                            error.set(Some(t!("error-fields-required")));
                                            return;
                                        }
                                        save_loading.set(true);
                                        error.set(None);
                                        success.set(None);
                                        let db_opt = db_signal.read().db.clone();
                                        let crypto = crypto_signal.read().clone();
                                        let c_id = cong_opt.as_ref().unwrap().id.clone().unwrap();
                                        let c_uid = cong_opt.as_ref().unwrap().uid.clone();
                                        let wk_uid = c_uid.clone();
                                        let data = CongregationData {
                                            uid: c_uid,
                                            name,
                                            address: (!cong_address.read().is_empty())
                                                .then(|| cong_address.read().clone()),
                                            circuit: (!cong_circuit.read().is_empty())
                                                .then(|| cong_circuit.read().clone()),
                                            language: cong_language.read().clone(),
                                            time_format: time_format.read().clone(),
                                            date_format: date_format.read().clone(),
                                            name_format: name_format.read().clone(),
                                            theme: theme.read().clone(),
                                            accent_color: accent_color.read().clone(),
                                        };
                                        let theme_val = theme.peek().clone();
                                        let accent_val = accent_color.peek().clone();
                                        spawn(async move {
                                            if let Some(db) = db_opt {
                                                match Congregation::update(&db, &crypto, c_id, data).await {
                                                    Ok(_) => {
                                                        success.set(Some(t!("success-congregation-updated")));
                                                        is_editing.set(false);
                                                        congregation_res.restart();
                                                        layout_congregation.restart();
                                                        let theme_str = match theme_val {
                                                            Theme::Dark => "dark",
                                                            _ => "light",
                                                        };
                                                        let accent_str = match accent_val {
                                                            AccentColor::Green => "Green",
                                                            AccentColor::Purple => "Purple",
                                                            AccentColor::Rose => "Rose",
                                                            AccentColor::Amber => "Amber",
                                                            _ => "Blue",
                                                        };
                                                        let _ = document::eval(
                                                            &format!(
                                                                "document.body.setAttribute('data-theme', '{}'); document.body.setAttribute('data-accent', '{}')",
                                                                theme_str,
                                                                accent_str,
                                                            ),
                                                        );
                                                        // Update workspace theme/accent in localStorage
                                                        let mut wks = crate::database::get_workspaces().await;
                                                        if let Some(wk) = wks.iter_mut().find(|w| w.uid == wk_uid) {
                                                            wk.theme = theme_str.to_string();
                                                            wk.accent_color = accent_str.to_string();
                                                        }
                                                        if let Ok(json) = serde_json::to_string(&wks) {
                                                            crate::database::ls_set("theo_workspaces", &json);
                                                        }
                                                    }
                                                    Err(e) => {
                                                        error.set(Some(e.to_string()));
                                                    }
                                                }
                                            }
                                            save_loading.set(false);
                                        });
                                    },
                                    if *save_loading.read() {
                                        {t!("btn-saving")}
                                    } else {
                                        {t!("btn-save")}
                                    }
                                }
                            }
                        }
                    }
                }

                // ── Danger Zone ───────────────────────────────────────────
                div { class: "bg-red-50 rounded-xl border border-red-200 overflow-hidden mt-8",
                    div { class: "px-6 py-4 border-b border-red-200 bg-red-100/50",
                        h2 { class: "text-lg font-bold text-red-800", {t!("danger-zone")} }
                    }
                    div { class: "p-6 space-y-8",

                        // Encryption Password Change
                        div { class: "space-y-4",
                            h3 { class: "text-md font-semibold text-red-900",
                                {t!("danger-change-password-title")}
                            }
                            p { class: "text-sm text-red-700", {t!("danger-change-password-desc")} }

                            if let Some(err) = password_error.read().clone() {
                                div { class: "bg-white/80 border border-red-300 rounded-lg p-3 text-red-700 text-sm font-medium",
                                    "{err}"
                                }
                            }
                            if let Some(msg) = password_success.read().clone() {
                                div { class: "bg-white/80 border border-green-300 rounded-lg p-3 text-green-700 text-sm font-medium",
                                    "{msg}"
                                }
                            }

                            div { class: "grid grid-cols-1 md:grid-cols-3 gap-4",
                                FormField { label: t!("danger-old-password"),
                                    input {
                                        class: "w-full border border-red-300 rounded-lg px-3 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-red-500",
                                        r#type: "password",
                                        value: old_password.read().clone(),
                                        oninput: move |e| old_password.set(e.value()),
                                    }
                                }
                                FormField { label: t!("danger-new-password"),
                                    input {
                                        class: "w-full border border-red-300 rounded-lg px-3 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-red-500",
                                        r#type: "password",
                                        value: new_password.read().clone(),
                                        oninput: move |e| new_password.set(e.value()),
                                    }
                                }
                                FormField { label: t!("danger-confirm-password"),
                                    input {
                                        class: "w-full border border-red-300 rounded-lg px-3 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-red-500",
                                        r#type: "password",
                                        value: confirm_password.read().clone(),
                                        oninput: move |e| confirm_password.set(e.value()),
                                    }
                                }
                            }

                            button {
                                class: "px-6 py-2 bg-red-600 text-white rounded-lg font-medium hover:bg-red-700 transition-colors disabled:opacity-50",
                                disabled: *password_loading.read(),
                                onclick: move |_| {
                                    if *password_loading.peek() {
                                        return;
                                    }
                                    let old_p = old_password.read().clone();
                                    let new_p = new_password.read().clone();
                                    let conf_p = confirm_password.read().clone();
                                    if old_p.is_empty() || new_p.is_empty() || conf_p.is_empty() {
                                        password_error.set(Some(t!("error-fields-required")));
                                        return;
                                    }
                                    if new_p != conf_p {
                                        password_error.set(Some(t!("error-passwords-mismatch")));
                                        return;
                                    }
                                    password_loading.set(true);
                                    password_error.set(None);
                                    password_success.set(None);
                                    let db_opt = db_signal.read().db.clone();
                                    let mut crypto_state_mut = crypto_signal.clone();
                                    spawn(async move {
                                        let Some(db) = db_opt else {
                                            password_loading.set(false);
                                            return;
                                        };
                                        let old_crypto = crypto_state_mut.read().clone();
                                        let keystore_vals: Vec<serde_json::Value> = match db
                                            .select("_keystore")
                                            .await

                                        // 3. Create new keystore & new crypto
                                        {
                                            Ok(v) => v,
                                            Err(e) => {
                                                password_error.set(Some(e.to_string()));
                                                password_loading.set(false);
                                                return;
                                            }
                                        };
                                        let ks: KeyStore = match keystore_vals.into_iter().next() {
                                            Some(v) => serde_json::from_value(v).unwrap(),
                                            None => {
                                                password_error.set(Some("Keystore not found".into()));
                                                password_loading.set(false);
                                                return;
                                            }
                                        };
                                        if ks.unlock(&old_p).is_err() {
                                            password_error.set(Some(t!("error-incorrect-password")));
                                            password_loading.set(false);
                                            return;
                                        }
                                        let congregations = Congregation::all(&db, &old_crypto)
                                            .await
                                            .unwrap_or_default();
                                        let users = User::all(&db, &old_crypto).await.unwrap_or_default();
                                        let ecs = EmergencyContact::all(&db, &old_crypto).await.unwrap_or_default();
                                        let fsgs = FieldServiceGroup::all(&db, &old_crypto)
                                            .await
                                            .unwrap_or_default();
                                        let absences = Absence::all(&db, &old_crypto).await.unwrap_or_default();
                                        let (new_ks, new_sym) = match KeyStore::create(&new_p) { // Update state variable for rendering
                                            Ok(v) => v,
                                            Err(e) => {
                                                password_error.set(Some(e.to_string()));
                                                password_loading.set(false);
                                                return;
                                            }
                                        };
                                        let ks_json = serde_json::to_value(&new_ks).unwrap();
                                        let _: Vec<serde_json::Value> = db
                                            .update("_keystore")
                                            .content(ks_json)
                                            .await
                                            .unwrap();
                                        let mut new_crypto = SessionCrypto::default();
                                        new_crypto.set_key(new_sym.clone());
                                        for c in congregations {
                                            let data = CongregationData {
                                                uid: c.uid,
                                                name: c.name,
                                                address: c.address,
                                                circuit: c.circuit,
                                                language: c.language,
                                                time_format: c.time_format,
                                                date_format: c.date_format,
                                                name_format: c.name_format,
                                                theme: c.theme,
                                                accent_color: c.accent_color,
                                            };
                                            let _ = Congregation::update(&db, &new_crypto, c.id.unwrap(), data)
                                                .await;
                                        }
                                        for u in users {
                                            let data = UserData {
                                                first_name: u.first_name,
                                                last_name: u.last_name,
                                                birthday: u.birthday,
                                                baptism_date: u.baptism_date,
                                                phone: u.phone,
                                                address: u.address,
                                                email: u.email,
                                                password: u.password,
                                                user_type: u.user_type,
                                                gender: u.gender,
                                                appointment: u.appointment,
                                                family_head: u.family_head,
                                                congregations: u.congregations,
                                                active: u.active,
                                            };
                                            let _ = User::update(&db, &new_crypto, u.id.unwrap(), data).await;
                                        }
                                        for ec in ecs {
                                            let data = EmergencyContactData {
                                                publisher: ec.publisher,
                                                first_name: ec.first_name,
                                                last_name: ec.last_name,
                                                relationship: ec.relationship,
                                                phone: ec.phone,
                                                email: ec.email,
                                                address: ec.address,
                                            };
                                            let _ = EmergencyContact::update(&db, &new_crypto, ec.id.unwrap(), data)
                                                .await;
                                        }
                                        for fsg in fsgs {
                                            let data = FieldServiceGroupData {
                                                congregation: fsg.congregation,
                                                name: fsg.name,
                                                overseer: fsg.overseer,
                                                assistant: fsg.assistant,
                                                members: fsg.members,
                                            };
                                            let _ = FieldServiceGroup::update(
                                                    &db,
                                                    &new_crypto,
                                                    fsg.id.unwrap(),
                                                    data,
                                                )
                                                .await;
                                        }
                                        for a in absences {
                                            let data = AbsenceData {
                                                publisher: a.publisher,
                                                start_date: a.start_date,
                                                end_date: a.end_date,
                                                reason: a.reason,
                                            };
                                            let _ = Absence::update(&db, &new_crypto, a.id.unwrap(), data).await;
                                        }
                                        crypto_state_mut.write().set_key(new_sym);
                                        old_password.set(String::new());
                                        new_password.set(String::new());
                                        confirm_password.set(String::new());
                                        password_success.set(Some(t!("success-password-changed")));
                                        password_loading.set(false);
                                    });
                                },
                                if *password_loading.read() {
                                    {t!("btn-saving")}
                                } else {
                                    {t!("danger-change-password-btn")}
                                }
                            }
                        }

                        // Divider
                        div { class: "h-px w-full bg-red-200" }

                        // Export / Import
                        div { class: "space-y-4",
                            h3 { class: "text-md font-semibold text-red-900",
                                {t!("danger-data-title")}
                            }
                            p { class: "text-sm text-red-700", {t!("danger-data-desc")} }

                            if let Some(err) = io_error.read().clone() {
                                div { class: "bg-white/80 border border-red-300 rounded-lg p-3 text-red-700 text-sm font-medium",
                                    "{err}"
                                }
                            }

                            div { class: "flex gap-4 flex-wrap",
                                button {
                                    class: "px-6 py-2 bg-white text-red-700 border border-red-300 rounded-lg font-medium hover:bg-red-50 transition-colors disabled:opacity-50",
                                    disabled: *io_loading.read(),
                                    onclick: move |_| {
                                        if *io_loading.peek() {
                                            return;
                                        }
                                        io_loading.set(true);
                                        io_error.set(None);
                                        let db_opt = db_signal.read().db.clone();
                                        spawn(async move {
                                            if let Some(db) = db_opt {
                                                match migrate::export(&db).await {
                                                    Ok(json) => {
                                                        let json_str = serde_json::to_string(&json).unwrap();
                                                        let mut eval = document::eval(
                                                            "
                                                                                                                                                                                                                                                                                                                            let data = await dioxus.recv();
                                                                                                                                                                                                                                                                                                                            const blob = new Blob([data], { type: 'application/json' });
                                                                                                                                                                                                                                                                                                                            const url = URL.createObjectURL(blob);
                                                                                                                                                                                                                                                                                                                            const a = document.createElement('a');
                                                                                                                                                                                                                                                                                                                            a.href = url;
                                                                                                                                                                                                                                                                                                                            a.download = 'theo-manager-export.json';
                                                                                                                                                                                                                                                                                                                            a.click();
                                                                                                                                                                                                                                                                                                                            URL.revokeObjectURL(url);
                                                                                                                                                                                                                                                                                                                        ",
                                                        );
                                                        eval.send(json_str).unwrap();
                                                    }
                                                    Err(e) => {
                                                        io_error.set(Some(e.to_string()));
                                                    }
                                                }
                                            }
                                            io_loading.set(false);
                                        });
                                    },
                                    "📥 "
                                    {t!("danger-export-btn")}
                                }

                                div { class: "relative",
                                    input {
                                        r#type: "file",
                                        accept: ".json",
                                        class: "absolute inset-0 opacity-0 w-full h-full cursor-pointer",
                                        onchange: move |e| {
                                            if *io_loading.peek() {
                                                return;
                                            }
                                            let files = e.files();
                                            if let Some(file_data) = files.into_iter().next() {
                                                io_loading.set(true);
                                                io_error.set(None);
                                                let db_opt = db_signal.read().db.clone();
                                                spawn(async move {
                                                    if let Ok(str_data) = file_data.read_string().await {
                                                        if let Ok(json_val) = serde_json::from_str::<
                                                            serde_json::Value,
                                                        >(&str_data) {
                                                            if let Some(db) = db_opt {
                                                                if let Err(err) = migrate::import(&db, json_val).await {
                                                                    io_error.set(Some(err.to_string()));
                                                                } else {
                                                                    let _ = document::eval("window.location.reload();");
                                                                }
                                                            }
                                                        } else {
                                                            io_error.set(Some("Invalid JSON file".into()));
                                                        }
                                                    } else {
                                                        io_error.set(Some("Cannot read file".into()));
                                                    }
                                                    io_loading.set(false);
                                                });
                                            }
                                        },
                                    }
                                    button {
                                        class: "px-6 py-2 bg-white text-red-700 border border-red-300 rounded-lg font-medium hover:bg-red-50 transition-colors disabled:opacity-50",
                                        disabled: *io_loading.read(),
                                        "📤 "
                                        {t!("danger-import-btn")}
                                    }
                                }

                                button {
                                    class: "px-6 py-2 bg-red-600 text-white rounded-lg font-medium hover:bg-red-700 transition-colors disabled:opacity-50",
                                    disabled: *io_loading.read(),
                                    onclick: move |_| {
                                        if *io_loading.peek() {
                                            return;
                                        }
                                        let confirmed = document::eval(
                                            "
                                                                                                                                                                                                                                                                                                            dioxus.send(confirm('Are you sure you want to delete all data? This cannot be undone.'));
                                                                                                                                                                                                                                                                                                        ",
                                        );
                                        let db_opt = db_signal.read().db.clone();
                                        spawn(async move {
                                            let mut confirmed = confirmed;
                                            if let Ok(serde_json::Value::Bool(true)) = confirmed.recv().await {
                                                io_loading.set(true);
                                                if let Some(db) = db_opt {
                                                    if let Err(e) = migrate::wipe(&db).await {
                                                        io_error.set(Some(e.to_string()));
                                                    } else {
                                                        let _ = document::eval("window.location.reload();");
                                                    }
                                                }
                                                io_loading.set(false);
                                            }
                                        });
                                    },
                                    "🗑️ "
                                    {t!("danger-wipe-btn")}
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
