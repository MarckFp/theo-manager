use dioxus::prelude::*;
use dioxus_i18n::t;

use crate::crypto::KeyStore;
use crate::database::{DatabaseMode, OnlineConfig, connect_offline, connect_online, signup_online, use_db, use_crypto, ls_set};
use crate::models::congregation::{Congregation, CongregationData};
use crate::models::user::{User, UserData};

#[component]
fn FormField(label: String, children: Element) -> Element {
    rsx! {
        div {
            label { class: "block text-sm font-medium text-gray-700 mb-1", "{label}" }
            {children}
        }
    }
}

fn password_strength(password: &str) -> f64 {
    if password.is_empty() {
        return 0.0;
    }
    let mut score: u32 = 0;
    if password.len() >= 8 { score += 20; }
    if password.len() >= 12 { score += 10; }
    if password.chars().any(|c| c.is_uppercase()) { score += 20; }
    if password.chars().any(|c| c.is_lowercase()) { score += 20; }
    if password.chars().any(|c| c.is_ascii_digit()) { score += 15; }
    if password.chars().any(|c| !c.is_alphanumeric()) { score += 15; }
    score.min(100) as f64
}

#[derive(Clone, Default)]
struct NewCongregationState {
    mode: Option<DatabaseMode>,
    name: String,
    address: String,
    circuit: String,
    language: String,
    email: String,
    password: String, // Online only
    enc_password: String,
    enc_confirm: String,
}

#[component]
pub fn AppNewCongregation() -> Element {
    let db_state = use_db();
    let crypto_state = use_crypto();
    let nav = use_navigator();
    let mut error: Signal<Option<String>> = use_signal(|| None);
    let mut loading = use_signal(|| false);

    let mut state = use_signal(|| NewCongregationState {
        language: String::new(),
        ..Default::default()
    });

    use_effect(move || {
        if state.read().language.is_empty() {
            spawn(async move {
                let mut eval = document::eval("dioxus.send(navigator.language || 'en-US');");
                let locale = eval
                    .recv::<String>()
                    .await
                    .unwrap_or_else(|_| "en-US".to_string());
                let lang = if locale.starts_with("es") { "es-ES" } else { "en-US" };
                state.write().language = lang.to_string();
            });
        }
    });

    let strength_pct = use_memo(move || password_strength(&state.read().enc_password));

    rsx! {
        div { class: "max-w-2xl mx-auto space-y-6 w-full pb-24",
            h1 { class: "text-2xl font-bold text-gray-900", {t!("sidebar-congregation-new")} }

            div { class: "bg-white rounded-xl border border-gray-200 p-6 space-y-6",
                if let Some(err) = error.read().clone() {
                    div { class: "bg-red-50 border border-red-200 rounded-lg p-3 text-red-700 text-sm",
                        "{err}"
                    }
                }

                div { class: "space-y-4",
                    h2 { class: "text-lg font-semibold text-gray-800", {t!("onboarding-mode-title")} }
                    div { class: "grid grid-cols-2 gap-3",
                        button {
                            class: format!(
                                "py-3 px-4 border-2 rounded-xl text-left transition-all {}",
                                if state.read().mode == Some(DatabaseMode::Offline) {
                                    "border-primary-500 bg-primary-50"
                                } else {
                                    "border-gray-200 hover:border-primary-400"
                                },
                            ),
                            onclick: move |_| {
                                error.set(None);
                                state.write().mode = Some(DatabaseMode::Offline);
                            },
                            div { class: "font-medium text-gray-800", {t!("onboarding-mode-offline")} }
                        }
                        button {
                            class: format!(
                                "py-3 px-4 border-2 rounded-xl text-left transition-all {}",
                                if state.read().mode == Some(DatabaseMode::Online) {
                                    "border-primary-500 bg-primary-50"
                                } else {
                                    "border-gray-200 hover:border-primary-400"
                                },
                            ),
                            onclick: move |_| {
                                error.set(None);
                                state.write().mode = Some(DatabaseMode::Online);
                            },
                            div { class: "font-medium text-gray-800", {t!("onboarding-mode-online")} }
                        }
                    }
                }

                if state.read().mode.is_some() {
                    div { class: "space-y-4 pt-4 border-t border-gray-100",
                        h2 { class: "text-lg font-semibold text-gray-800",
                            {t!("onboarding-congregation-title")}
                        }

                        FormField { label: t!("onboarding-congregation-name"),
                            input {
                                class: "w-full border border-gray-300 rounded-lg px-3 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-primary-500",
                                r#type: "text",
                                value: state.read().name.clone(),
                                oninput: move |e| state.write().name = e.value(),
                            }
                        }
                        FormField { label: t!("onboarding-congregation-address"),
                            input {
                                class: "w-full border border-gray-300 rounded-lg px-3 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-primary-500",
                                r#type: "text",
                                value: state.read().address.clone(),
                                oninput: move |e| state.write().address = e.value(),
                            }
                        }
                        FormField { label: t!("onboarding-congregation-circuit"),
                            input {
                                class: "w-full border border-gray-300 rounded-lg px-3 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-primary-500",
                                r#type: "text",
                                value: state.read().circuit.clone(),
                                oninput: move |e| state.write().circuit = e.value(),
                            }
                        }
                        FormField { label: t!("onboarding-congregation-language"),
                            select {
                                class: "w-full border border-gray-300 rounded-lg px-3 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-primary-500 bg-white",
                                value: state.read().language.clone(),
                                onchange: move |e| state.write().language = e.value(),
                                option { value: "en-US", "\u{1f1fa}\u{1f1f8} English" }
                                option { value: "es-ES", "\u{1f1ea}\u{1f1f8} Español" }
                            }
                        }
                    }

                    if state.read().mode == Some(DatabaseMode::Online) {
                        div { class: "space-y-4 pt-4 border-t border-gray-100",
                            h2 { class: "text-lg font-semibold text-gray-800",
                                {t!("onboarding-user-title")}
                            }
                            p { class: "text-sm text-gray-600 mb-2",
                                "Credentials for syncing to the cloud."
                            }

                            FormField { label: t!("form-email"),
                                input {
                                    class: "w-full border border-gray-300 rounded-lg px-3 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-primary-500",
                                    r#type: "email",
                                    value: state.read().email.clone(),
                                    oninput: move |e| state.write().email = e.value(),
                                }
                            }
                            FormField { label: t!("form-password"),
                                input {
                                    class: "w-full border border-gray-300 rounded-lg px-3 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-primary-500",
                                    r#type: "password",
                                    value: state.read().password.clone(),
                                    oninput: move |e| state.write().password = e.value(),
                                }
                            }
                        }
                    }

                    div { class: "space-y-4 pt-4 border-t border-gray-100",
                        h2 { class: "text-lg font-semibold text-gray-800",
                            {t!("onboarding-encryption-title")}
                        }
                        p { class: "text-sm text-gray-600 mb-2", {t!("onboarding-encryption-desc")} }

                        FormField { label: t!("form-password"),
                            input {
                                class: "w-full border border-gray-300 rounded-lg px-3 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-primary-500",
                                r#type: "password",
                                value: state.read().enc_password.clone(),
                                oninput: move |e| state.write().enc_password = e.value(),
                            }
                            div { class: "mt-1.5",
                                div { class: "w-full bg-gray-200 rounded-full h-1.5",
                                    div {
                                        class: match strength_pct() as u32 {
                                            1..=30 => "h-1.5 rounded-full transition-all duration-300 bg-red-500",
                                            31..=60 => "h-1.5 rounded-full transition-all duration-300 bg-yellow-500",
                                            61..=80 => "h-1.5 rounded-full transition-all duration-300 bg-primary-500",
                                            _ => "h-1.5 rounded-full transition-all duration-300 bg-green-500",
                                        },
                                        style: format!("width: {}%", strength_pct()),
                                    }
                                }
                                p { class: "text-xs text-gray-400 mt-0.5",
                                    {
                                        match strength_pct() as u32 {
                                            0 => String::new(),
                                            1..=30 => t!("password-weak"),
                                            31..=60 => t!("password-fair"),
                                            61..=80 => t!("password-strong"),
                                            _ => t!("password-very-strong"),
                                        }
                                    }
                                }
                            }
                        }
                        FormField { label: t!("form-confirm-password"),
                            input {
                                class: "w-full border border-gray-300 rounded-lg px-3 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-primary-500",
                                r#type: "password",
                                value: state.read().enc_confirm.clone(),
                                oninput: move |e| state.write().enc_confirm = e.value(),
                            }
                        }
                    }

                    div { class: "pt-4",
                        button {
                            class: "w-full py-3 bg-primary-600 text-white rounded-xl font-medium hover:bg-primary-700 transition-colors disabled:opacity-50",
                            disabled: *loading.read(),
                            onclick: move |_| {
                                if *loading.peek() {
                                    return;
                                }
                                let s = state.peek().clone();
                                if s.name.is_empty() || s.language.is_empty() {
                                    error.set(Some(t!("error-fields-required")));
                                    return;
                                }
                                if s.enc_password.is_empty() {
                                    error.set(Some(t!("error-fields-required")));
                                    return;
                                }
                                if s.enc_password != s.enc_confirm {
                                    error.set(Some(t!("error-passwords-mismatch")));
                                    return;
                                }
                                if s.mode == Some(DatabaseMode::Online)
                                    && (s.email.is_empty() || s.password.is_empty())
                                {
                                    error.set(Some(t!("error-fields-required")));
                                    return;
                                }
                                loading.set(true);
                                error.set(None);
                                let old_db = db_state.read().db.clone();
                                let old_crypto = crypto_state.read().clone();
                                let mut db_state_mut = db_state.clone();
                                let mut crypto_state_mut = crypto_state.clone();
                                spawn(async move {
                                    let Some(old_db) = old_db else {
                                        error.set(Some("Database error.".to_string()));
                                        loading.set(false);
                                        return;
                                    };
                                    // Fetch current user data (from localStorage, then fallback to first user)
                                    let mut current_user_data: Option<UserData> = None;
                                    let mut eval = document::eval(
                                        "
                                                                                                                                        try { dioxus.send(localStorage.getItem('theo_my_user_id')); } 
                                                                                                                                        catch(e) { dioxus.send(null); }
                                                                                                                                    ",
                                    );
                                    let user_id_str = eval
                                        .recv::<serde_json::Value>()
                                        .await
                                        .ok()
                                        .and_then(|v| {
                                            match v {
                                                serde_json::Value::String(val) => Some(val),
                                                _ => None,
                                            }
                                        });
                                    if let Some(id_str) = user_id_str {
                                        if let Ok(record_id) = surrealdb::types::RecordId::parse_simple(
                                            &id_str,
                                        ) {
                                            if let Ok(Some(user)) = User::get(&old_db, &old_crypto, record_id)
                                                .await
                                            {
                                                current_user_data = Some(UserData {
                                                    first_name: user.first_name,
                                                    last_name: user.last_name,
                                                    birthday: user.birthday,
                                                    baptism_date: user.baptism_date,
                                                    phone: user.phone,
                                                    address: user.address,
                                                    email: user.email,
                                                    password: user.password,
                                                    user_type: user.user_type,
                                                    gender: user.gender,
                                                    appointment: user.appointment,
                                                    family_head: user.family_head,
                                                    congregations: vec![],
                                                    active: user.active,
                                                });
                                            }
                                        }
                                    }
                                    // Fallback: if not found, grab first admin user
                                    if current_user_data.is_none() {
                                        if let Ok(users) = User::all(&old_db, &old_crypto).await {
                                            if let Some(user) = users.into_iter().next() {
                                                current_user_data = Some(UserData {
                                                    first_name: user.first_name,
                                                    last_name: user.last_name,
                                                    birthday: user.birthday,
                                                    baptism_date: user.baptism_date,
                                                    phone: user.phone,
                                                    address: user.address,
                                                    email: user.email,
                                                    password: user.password,
                                                    user_type: user.user_type,
                                                    gender: user.gender,
                                                    appointment: user.appointment,
                                                    family_head: user.family_head,
                                                    congregations: vec![],
                                                    active: user.active,
                                                });
                                            }
                                        }
                                    }
                                    let Some(current_user_data) = current_user_data else {
                                        error
                                            .set(
                                                Some(
                                                    "Could not fetch your active user profile to copy to the new congregation."
                                                        .into(),
                                                ),
                                            );
                                        loading.set(false);
                                        return;
                                    };
                                    let new_uid = uuid::Uuid::new_v4().to_string();
                                    let db_result = match s.mode {
                                        Some(DatabaseMode::Online) => {
                                            signup_online(&new_uid, &s.email, &s.email, &s.password)
                                                .await
                                                .map_err(|e| e.to_string())
                                        }
                                        _ => connect_offline(&new_uid).await.map_err(|e| e.to_string()),
                                    };
                                    let new_db = match db_result {
                                        Ok(db) => db,
                                        Err(e) => {
                                            error.set(Some(e));
                                            loading.set(false);
                                            return;
                                        }
                                    };
                                    let (keystore, sym_key) = match KeyStore::create(&s.enc_password) {
                                        Ok(v) => v,
                                        Err(e) => {
                                            error.set(Some(e.to_string()));
                                            loading.set(false);
                                            return;
                                        }
                                    };
                                    let keystore_json = serde_json::to_value(&keystore).unwrap();
                                    let _: Option<serde_json::Value> = match new_db
                                        .create("_keystore")
                                        .content(keystore_json)
                                        .await
                                    {
                                        Ok(v) => v,
                                        Err(e) => {
                                            error.set(Some(e.to_string()));
                                            loading.set(false);
                                            return;
                                        }
                                    };
                                    let mut new_crypto = crate::crypto::SessionCrypto::default();
                                    new_crypto.set_key(sym_key.clone());
                                    let cong_data = CongregationData {
                                        uid: new_uid.clone(),
                                        name: s.name.clone(),
                                        address: (!s.address.is_empty()).then(|| s.address),
                                        circuit: (!s.circuit.is_empty()).then(|| s.circuit),
                                        language: s.language,
                                        time_format: Default::default(),
                                        date_format: Default::default(),
                                        name_format: Default::default(),
                                        theme: Default::default(),
                                        accent_color: Default::default(),
                                    };
                                    let new_cong = match Congregation::create(&new_db, &new_crypto, cong_data)
                                        .await
                                    {
                                        Ok(Some(c)) => c,
                                        _ => {
                                            error.set(Some(t!("error-congregation-create")));
                                            loading.set(false);
                                            return;
                                        }
                                    };
                                    let mut ported_user = current_user_data;
                                    ported_user.congregations = vec![new_cong.id.clone().unwrap()];
                                    let user_created = match User::create(&new_db, &new_crypto, ported_user)
                                        .await
                                    {
                                        Ok(Some(u)) => u,
                                        _ => {
                                            error
                                                .set(
                                                    Some(
                                                        "Failed to create admin user in the new connection.".into(),
                                                    ),
                                                );
                                            loading.set(false);
                                            return;
                                        }
                                    };
                                    if let Some(ref new_id) = user_created.id {
                                        let key_str = match &new_id.key {
                                            surrealdb::types::RecordIdKey::String(ky) => ky.clone(),
                                            surrealdb::types::RecordIdKey::Number(ky) => ky.to_string(),
                                            _ => "unknown".to_string(),
                                        };
                                        ls_set(
                                            "theo_my_user_id",
                                            &format!("{}:{}", new_id.table.as_str(), key_str),
                                        );
                                    }
                                    let workspace = crate::database::Workspace {
                                        uid: new_uid.clone(),
                                        name: s.name.clone(),
                                        mode: s.mode.clone().unwrap(),
                                        username: Some(s.email.clone()),
                                        theme: "light".to_string(),
                                        accent_color: "Blue".to_string(),
                                    };
                                    crate::database::add_workspace(workspace).await;
                                    ls_set("theo_active_uid", &new_uid);
                                    let mut state = db_state_mut.write();
                                    state.congregation_uid = Some(new_uid.clone());
                                    state.active_congregation_id = new_cong.id.clone();
                                    if let Some(old) = state.db.take() {
                                        state.leaked_dbs.push(old);
                                    }
                                    state.db = Some(new_db);
                                    if s.mode == Some(DatabaseMode::Online) {
                                        state.mode = DatabaseMode::Online;
                                        state.config = Some(OnlineConfig {
                                            congregation_uid: new_uid,
                                            username: s.email,
                                        });
                                    } else {
                                        state.mode = DatabaseMode::Offline;
                                    }
                                    crypto_state_mut.write().set_key(sym_key.clone());
                                    nav.push(crate::Route::AppDashboard {});
                                });
                            },
                            if *loading.read() {
                                {t!("btn-saving")}
                            } else {
                                {t!("btn-save")}
                            }
                        }
                    }
                }
            }
        }
    }
}
