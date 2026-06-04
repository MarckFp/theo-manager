use dioxus::prelude::*;
use dioxus_i18n::t;

use crate::crypto::KeyStore;
use crate::database::{
    DatabaseMode, OnlineConfig, connect_offline, connect_online, signup_online, use_crypto, use_db,
};
use crate::models::congregation::{Congregation, CongregationData};
use crate::models::user::{User, UserData, UserType};

// ---------------------------------------------------------------------------
// localStorage helpers (JS interop via document::eval)
// ---------------------------------------------------------------------------

async fn ls_get(key: &str) -> Option<String> {
    let js = format!(
        "try {{ dioxus.send(localStorage.getItem({key:?})); }} catch(e) {{ dioxus.send(null); }}"
    );
    let mut eval = document::eval(&js);
    eval.recv::<serde_json::Value>().await.ok().and_then(|v| match v {
        serde_json::Value::String(s) => Some(s),
        _ => None,
    })
}

fn ls_set(key: &str, value: &str) {
    let js = format!("try {{ localStorage.setItem({key:?}, {value:?}); }} catch(e) {{}}");
    let _ = document::eval(&js);
}

fn ls_remove(key: &str) {
    let js = format!("try {{ localStorage.removeItem({key:?}); }} catch(e) {{}}");
    let _ = document::eval(&js);
}

// ---------------------------------------------------------------------------
// Password strength (0.0 – 100.0)
// ---------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------
// Step enum
// ---------------------------------------------------------------------------

#[derive(Clone, PartialEq)]
enum LandingStep {
    CheckingRestore,
    AccountChoice,
    Login,
    ForgotPassword,
    OnboardingMode,
    OnboardingUser,
    OnboardingCongregation,
    Connecting,
    ResumeSession { uid: String },
}

// ---------------------------------------------------------------------------
// Onboarding state
// ---------------------------------------------------------------------------

#[derive(Clone, Default)]
struct OnboardingState {
    mode: Option<DatabaseMode>,
    first_name: String,
    last_name: String,
    email: String,
    password: String,
    confirm_password: String,
    congregation_name: String,
    congregation_city: String,
    congregation_circuit: String,
    congregation_language: String,
}

// ---------------------------------------------------------------------------
// Root Landing
// ---------------------------------------------------------------------------

#[component]
pub fn Landing() -> Element {
    let db_state = use_db();
    let nav = use_navigator();
    // All hooks must be declared before any conditional returns
    let mut step = use_signal(|| LandingStep::CheckingRestore);
    let onboarding = use_signal(OnboardingState::default);

    // Check localStorage on mount to restore an offline session
    use_effect(move || {
        spawn(async move {
            if let Some(uid) = ls_get("theo_offline_uid").await {
                step.set(LandingStep::ResumeSession { uid });
            } else {
                step.set(LandingStep::AccountChoice);
            }
        });
    });

    // Navigation guard (runs after hooks so hook count stays constant)
    if db_state.read().db.is_some() {
        nav.push(crate::Route::AppDashboard {});
        return rsx! {};
    }

    rsx! {
        div { class: "min-h-screen bg-gradient-to-br from-slate-50 to-blue-50 flex items-center justify-center p-4",
            div { class: "w-full max-w-md",
                div { class: "text-center mb-8",
                    h1 { class: "text-3xl font-bold text-gray-900", {t!("app-name")} }
                    p { class: "text-gray-500 mt-1", {t!("landing-subtitle")} }
                }
                div { class: "bg-white rounded-2xl shadow-lg p-6",
                    match step.read().clone() {
                        LandingStep::CheckingRestore => rsx! {
                            div { class: "flex justify-center py-8",
                                div { class: "w-8 h-8 border-4 border-blue-600 border-t-transparent rounded-full animate-spin" }
                            }
                        },
                        LandingStep::AccountChoice => rsx! {
                            AccountChoice { step }
                        },
                        LandingStep::Login => rsx! {
                            LoginScreen { step }
                        },
                        LandingStep::ForgotPassword => rsx! {
                            ForgotPasswordScreen { step }
                        },
                        LandingStep::OnboardingMode => rsx! {
                            OnboardingModeStep { step, onboarding }
                        },
                        LandingStep::OnboardingUser => rsx! {
                            OnboardingUserStep { step, onboarding }
                        },
                        LandingStep::OnboardingCongregation => rsx! {
                            OnboardingCongregationStep { step, onboarding }
                        },
                        LandingStep::Connecting => rsx! {
                            ConnectingStep { step, onboarding }
                        },
                        LandingStep::ResumeSession { uid } => rsx! {
                            ResumeSessionStep { step, uid }
                        },
                    }
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// AccountChoice
// ---------------------------------------------------------------------------

#[component]
fn AccountChoice(mut step: Signal<LandingStep>) -> Element {
    rsx! {
        div { class: "space-y-4",
            h2 { class: "text-xl font-semibold text-gray-800 text-center", {t!("landing-welcome")} }
            p { class: "text-gray-500 text-center text-sm", {t!("landing-welcome-desc")} }
            div { class: "pt-2 space-y-3",
                button {
                    class: "w-full py-3 px-4 bg-blue-600 text-white rounded-xl font-medium hover:bg-blue-700 transition-colors",
                    onclick: move |_| step.set(LandingStep::Login),
                    {t!("landing-have-account")}
                }
                button {
                    class: "w-full py-3 px-4 border-2 border-blue-600 text-blue-600 rounded-xl font-medium hover:bg-blue-50 transition-colors",
                    onclick: move |_| step.set(LandingStep::OnboardingMode),
                    {t!("landing-create-account")}
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// LoginScreen
// ---------------------------------------------------------------------------

#[component]
fn LoginScreen(mut step: Signal<LandingStep>) -> Element {
    let mut congregation_code = use_signal(String::new);
    let mut username = use_signal(String::new);
    let mut password = use_signal(String::new);
    let mut remember_me = use_signal(|| false);
    let mut loading = use_signal(|| false);
    let mut error: Signal<Option<String>> = use_signal(|| None);

    let mut db_state = use_db();
    let nav = use_navigator();

    // Pre-fill from localStorage if saved
    use_effect(move || {
        spawn(async move {
            if let Some(uid) = ls_get("theo_online_uid").await {
                congregation_code.set(uid);
                remember_me.set(true);
            }
            if let Some(user) = ls_get("theo_online_username").await {
                username.set(user);
            }
        });
    });

    rsx! {
        div { class: "space-y-4",
            h2 { class: "text-xl font-semibold text-gray-800", {t!("landing-login-title")} }
            p { class: "text-gray-500 text-sm", {t!("landing-login-desc")} }

            if let Some(err) = error.read().clone() {
                div { class: "bg-red-50 border border-red-200 rounded-lg p-3 text-red-700 text-sm",
                    "{err}"
                }
            }

            FormField { label: t!("onboarding-congregation-code"),
                input {
                    class: "w-full border border-gray-300 rounded-lg px-3 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-blue-500",
                    r#type: "text",
                    placeholder: "xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx",
                    value: congregation_code.read().clone(),
                    oninput: move |e| congregation_code.set(e.value()),
                }
            }
            FormField { label: t!("form-email"),
                input {
                    class: "w-full border border-gray-300 rounded-lg px-3 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-blue-500",
                    r#type: "email",
                    value: username.read().clone(),
                    oninput: move |e| username.set(e.value()),
                }
            }
            FormField { label: t!("form-password"),
                input {
                    class: "w-full border border-gray-300 rounded-lg px-3 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-blue-500",
                    r#type: "password",
                    value: password.read().clone(),
                    oninput: move |e| password.set(e.value()),
                }
            }

            div { class: "flex items-center justify-between",
                label { class: "flex items-center gap-2 text-sm text-gray-600 cursor-pointer",
                    input {
                        r#type: "checkbox",
                        class: "rounded border-gray-300 text-blue-600",
                        checked: *remember_me.read(),
                        oninput: move |e| remember_me.set(e.checked()),
                    }
                    {t!("form-remember-me")}
                }
                button {
                    class: "text-sm text-blue-600 hover:underline",
                    onclick: move |_| step.set(LandingStep::ForgotPassword),
                    {t!("landing-forgot-password")}
                }
            }

            button {
                class: "w-full py-3 bg-blue-600 text-white rounded-xl font-medium hover:bg-blue-700 transition-colors disabled:opacity-50",
                disabled: *loading.read(),
                onclick: move |_| {
                    let cid = congregation_code.read().clone();
                    let user = username.read().clone();
                    let pass = password.read().clone();
                    let save = *remember_me.read();
                    if cid.is_empty() || user.is_empty() || pass.is_empty() {
                        error.set(Some(t!("error-fields-required")));
                        return;
                    }
                    let config = OnlineConfig {
                        congregation_uid: cid.clone(),
                        username: user.clone(),
                    };
                    spawn(async move {
                        loading.set(true);
                        error.set(None);
                        match connect_online(&config, &pass).await {
                            Ok(db) => {
                                if save {
                                    ls_set("theo_online_uid", &cid);
                                    ls_set("theo_online_username", &user);
                                } else {
                                    ls_remove("theo_online_uid");
                                    ls_remove("theo_online_username");
                                }
                                let mut state = db_state.write();
                                state.db = Some(db);
                                state.mode = DatabaseMode::Online;
                                state.congregation_uid = Some(config.congregation_uid.clone());
                                state.config = Some(config);
                                nav.push(crate::Route::AppDashboard {});
                            }
                            Err(e) => {
                                error.set(Some(e.to_string()));
                                loading.set(false);
                            }
                        }
                    });
                },
                if *loading.read() {
                    {t!("btn-connecting")}
                } else {
                    {t!("btn-login")}
                }
            }

            button {
                class: "w-full py-2 text-gray-500 hover:text-gray-800 text-sm transition-colors",
                onclick: move |_| step.set(LandingStep::AccountChoice),
                {t!("btn-back")}
            }
        }
    }
}

// ---------------------------------------------------------------------------
// ForgotPasswordScreen
// ---------------------------------------------------------------------------

#[component]
fn ForgotPasswordScreen(mut step: Signal<LandingStep>) -> Element {
    rsx! {
        div { class: "space-y-4",
            h2 { class: "text-xl font-semibold text-gray-800", {t!("landing-forgot-password")} }
            p { class: "text-gray-500 text-sm", {t!("landing-forgot-password-desc")} }
            button {
                class: "w-full py-3 border border-gray-300 rounded-xl text-gray-700 font-medium hover:bg-gray-50 transition-colors",
                onclick: move |_| step.set(LandingStep::Login),
                {t!("btn-back")}
            }
        }
    }
}

// ---------------------------------------------------------------------------
// OnboardingModeStep
// ---------------------------------------------------------------------------

#[component]
fn OnboardingModeStep(
    mut step: Signal<LandingStep>,
    mut onboarding: Signal<OnboardingState>,
) -> Element {
    rsx! {
        div { class: "space-y-4",
            h2 { class: "text-xl font-semibold text-gray-800", {t!("onboarding-mode-title")} }
            p { class: "text-gray-500 text-sm", {t!("onboarding-mode-desc")} }

            div { class: "space-y-3 pt-1",
                button {
                    class: "w-full py-4 px-4 border-2 border-gray-200 rounded-xl text-left hover:border-blue-400 hover:bg-blue-50 transition-all",
                    onclick: move |_| {
                        onboarding.write().mode = Some(DatabaseMode::Offline);
                        step.set(LandingStep::OnboardingUser);
                    },
                    div { class: "font-medium text-gray-800", {t!("onboarding-mode-offline")} }
                    div { class: "text-sm text-gray-500 mt-0.5", {t!("onboarding-mode-offline-desc")} }
                }
                button {
                    class: "w-full py-4 px-4 border-2 border-gray-200 rounded-xl text-left hover:border-blue-400 hover:bg-blue-50 transition-all",
                    onclick: move |_| {
                        onboarding.write().mode = Some(DatabaseMode::Online);
                        step.set(LandingStep::OnboardingUser);
                    },
                    div { class: "font-medium text-gray-800", {t!("onboarding-mode-online")} }
                    div { class: "text-sm text-gray-500 mt-0.5", {t!("onboarding-mode-online-desc")} }
                }
            }

            button {
                class: "w-full py-2 text-gray-500 hover:text-gray-800 text-sm transition-colors",
                onclick: move |_| step.set(LandingStep::AccountChoice),
                {t!("btn-back")}
            }
        }
    }
}

// ---------------------------------------------------------------------------
// OnboardingUserStep
// ---------------------------------------------------------------------------

#[component]
fn OnboardingUserStep(
    mut step: Signal<LandingStep>,
    mut onboarding: Signal<OnboardingState>,
) -> Element {
    let mut error: Signal<Option<String>> = use_signal(|| None);

    let mut first_name = use_signal(|| onboarding.read().first_name.clone());
    let mut last_name = use_signal(|| onboarding.read().last_name.clone());
    let mut email = use_signal(|| onboarding.read().email.clone());
    let mut password = use_signal(|| onboarding.read().password.clone());
    let mut confirm_password = use_signal(|| onboarding.read().confirm_password.clone());

    let strength_pct = use_memo(move || password_strength(&password.read()));

    rsx! {
        div { class: "space-y-4",
            h2 { class: "text-xl font-semibold text-gray-800", {t!("onboarding-user-title")} }
            p { class: "text-gray-500 text-sm", {t!("onboarding-user-desc")} }

            if let Some(err) = error.read().clone() {
                div { class: "bg-red-50 border border-red-200 rounded-lg p-3 text-red-700 text-sm",
                    "{err}"
                }
            }

            div { class: "grid grid-cols-2 gap-3",
                FormField { label: t!("form-first-name"),
                    input {
                        class: "w-full border border-gray-300 rounded-lg px-3 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-blue-500",
                        r#type: "text",
                        value: first_name.read().clone(),
                        oninput: move |e| first_name.set(e.value()),
                    }
                }
                FormField { label: t!("form-last-name"),
                    input {
                        class: "w-full border border-gray-300 rounded-lg px-3 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-blue-500",
                        r#type: "text",
                        value: last_name.read().clone(),
                        oninput: move |e| last_name.set(e.value()),
                    }
                }
            }
            FormField { label: t!("form-email"),
                input {
                    class: "w-full border border-gray-300 rounded-lg px-3 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-blue-500",
                    r#type: "email",
                    value: email.read().clone(),
                    oninput: move |e| email.set(e.value()),
                }
            }
            FormField { label: t!("form-password"),
                input {
                    class: "w-full border border-gray-300 rounded-lg px-3 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-blue-500",
                    r#type: "password",
                    value: password.read().clone(),
                    oninput: move |e| password.set(e.value()),
                }
                // Password strength bar
                div { class: "mt-1.5",
                    div { class: "w-full bg-gray-200 rounded-full h-1.5",
                        div {
                            class: match strength_pct() as u32 {
                                1..=30 => "h-1.5 rounded-full transition-all duration-300 bg-red-500",
                                31..=60 => "h-1.5 rounded-full transition-all duration-300 bg-yellow-500",
                                61..=80 => "h-1.5 rounded-full transition-all duration-300 bg-blue-500",
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
                    class: "w-full border border-gray-300 rounded-lg px-3 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-blue-500",
                    r#type: "password",
                    value: confirm_password.read().clone(),
                    oninput: move |e| confirm_password.set(e.value()),
                }
            }

            div { class: "flex gap-3 pt-1",
                button {
                    class: "flex-1 py-3 border border-gray-300 rounded-xl text-gray-700 font-medium hover:bg-gray-50 transition-colors",
                    onclick: move |_| step.set(LandingStep::OnboardingMode),
                    {t!("btn-back")}
                }
                button {
                    class: "flex-1 py-3 bg-blue-600 text-white rounded-xl font-medium hover:bg-blue-700 transition-colors",
                    onclick: move |_| {
                        let fn_val = first_name.read().clone();
                        let ln_val = last_name.read().clone();
                        let em_val = email.read().clone();
                        let pw_val = password.read().clone();
                        let cp_val = confirm_password.read().clone();

                        if fn_val.is_empty() || ln_val.is_empty() || em_val.is_empty()
                            || pw_val.is_empty()
                        {
                            error.set(Some(t!("error-fields-required")));
                            return;
                        }
                        if pw_val != cp_val {
                            error.set(Some(t!("error-passwords-mismatch")));
                            return;
                        }
                        if !em_val.contains('@') || !em_val.contains('.') {
                            error.set(Some(t!("error-invalid-email")));
                            return;
                        }

                        let mut ob = onboarding.write();
                        ob.first_name = fn_val;
                        ob.last_name = ln_val;
                        ob.email = em_val;
                        ob.password = pw_val;
                        ob.confirm_password = cp_val;
                        drop(ob);

                        step.set(LandingStep::OnboardingCongregation);
                    },
                    {t!("btn-next")}
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// OnboardingCongregationStep
// ---------------------------------------------------------------------------

#[component]
fn OnboardingCongregationStep(
    mut step: Signal<LandingStep>,
    mut onboarding: Signal<OnboardingState>,
) -> Element {
    let mut error: Signal<Option<String>> = use_signal(|| None);

    let mut cong_name = use_signal(|| onboarding.read().congregation_name.clone());
    let mut cong_city = use_signal(|| onboarding.read().congregation_city.clone());
    let mut cong_circuit = use_signal(|| onboarding.read().congregation_circuit.clone());
    let mut cong_language = use_signal(|| onboarding.read().congregation_language.clone());

    rsx! {
        div { class: "space-y-4",
            h2 { class: "text-xl font-semibold text-gray-800", {t!("onboarding-congregation-title")} }
            p { class: "text-gray-500 text-sm", {t!("onboarding-congregation-desc")} }

            if let Some(err) = error.read().clone() {
                div { class: "bg-red-50 border border-red-200 rounded-lg p-3 text-red-700 text-sm",
                    "{err}"
                }
            }

            FormField { label: t!("onboarding-congregation-name"),
                input {
                    class: "w-full border border-gray-300 rounded-lg px-3 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-blue-500",
                    r#type: "text",
                    value: cong_name.read().clone(),
                    oninput: move |e| cong_name.set(e.value()),
                }
            }
            FormField { label: t!("onboarding-congregation-city"),
                input {
                    class: "w-full border border-gray-300 rounded-lg px-3 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-blue-500",
                    r#type: "text",
                    value: cong_city.read().clone(),
                    oninput: move |e| cong_city.set(e.value()),
                }
            }
            FormField { label: t!("onboarding-congregation-circuit"),
                input {
                    class: "w-full border border-gray-300 rounded-lg px-3 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-blue-500",
                    r#type: "text",
                    value: cong_circuit.read().clone(),
                    oninput: move |e| cong_circuit.set(e.value()),
                }
            }
            FormField { label: t!("onboarding-congregation-language"),
                input {
                    class: "w-full border border-gray-300 rounded-lg px-3 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-blue-500",
                    r#type: "text",
                    value: cong_language.read().clone(),
                    oninput: move |e| cong_language.set(e.value()),
                }
            }

            div { class: "flex gap-3 pt-1",
                button {
                    class: "flex-1 py-3 border border-gray-300 rounded-xl text-gray-700 font-medium hover:bg-gray-50 transition-colors",
                    onclick: move |_| step.set(LandingStep::OnboardingUser),
                    {t!("btn-back")}
                }
                button {
                    class: "flex-1 py-3 bg-blue-600 text-white rounded-xl font-medium hover:bg-blue-700 transition-colors",
                    onclick: move |_| {
                        let name = cong_name.read().clone();
                        let city = cong_city.read().clone();
                        let circuit = cong_circuit.read().clone();
                        let language = cong_language.read().clone();

                        if name.is_empty() || city.is_empty() || circuit.is_empty()
                            || language.is_empty()
                        {
                            error.set(Some(t!("error-fields-required")));
                            return;
                        }
                        let mut ob = onboarding.write();
                        ob.congregation_name = name;
                        ob.congregation_city = city;
                        ob.congregation_circuit = circuit;
                        ob.congregation_language = language;
                        drop(ob);
                        step.set(LandingStep::Connecting);
                    },
                    {t!("btn-finish")}
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// ConnectingStep
// ---------------------------------------------------------------------------

#[component]
fn ConnectingStep(mut step: Signal<LandingStep>, onboarding: Signal<OnboardingState>) -> Element {
    let mut db_state = use_db();
    let mut crypto_state = use_crypto();
    let nav = use_navigator();
    let mut error: Signal<Option<String>> = use_signal(|| None);

    use_effect(move || {
        let ob = onboarding.read().clone();
        spawn(async move {
            let uid = uuid::Uuid::new_v4().to_string();

            let db_result = match &ob.mode {
                Some(DatabaseMode::Online) => {
                    signup_online(&uid, &ob.email, &ob.email, &ob.password)
                        .await
                        .map_err(|e| e.to_string())
                }
                _ => connect_offline(&uid).await.map_err(|e| e.to_string()),
            };

            let db = match db_result {
                Ok(db) => db,
                Err(e) => {
                    error.set(Some(e));
                    return;
                }
            };

            // Run migrations (export/import helpers exist, but no schema init needed)
            // SurrealDB is schemaless — tables are created on first insert.

            // Initialise encryption
            let (keystore, sym_key) = match crate::crypto::KeyStore::create(&ob.password) {
                Ok(v) => v,
                Err(e) => {
                    error.set(Some(e.to_string()));
                    return;
                }
            };
            let keystore_json = match serde_json::to_value(&keystore) {
                Ok(v) => v,
                Err(e) => {
                    error.set(Some(e.to_string()));
                    return;
                }
            };
            let _: Option<serde_json::Value> =
                match db.create("_keystore").content(keystore_json).await {
                    Ok(v) => v,
                    Err(e) => {
                        error.set(Some(e.to_string()));
                        return;
                    }
                };

            crypto_state.write().set_key(sym_key);
            let crypto = crypto_state.read().clone();

            // Create congregation
            let cong_data = CongregationData {
                uid: uid.clone(),
                name: ob.congregation_name.clone(),
                city: ob.congregation_city.clone(),
                circuit: ob.congregation_circuit.clone(),
                language: ob.congregation_language.clone(),
            };
            let congregation = match Congregation::create(&db, &crypto, cong_data).await {
                Ok(Some(c)) => c,
                Ok(None) => {
                    error.set(Some(t!("error-congregation-create")));
                    return;
                }
                Err(e) => {
                    error.set(Some(e.to_string()));
                    return;
                }
            };

            // Create admin user
            let cong_id = match congregation.id.clone() {
                Some(id) => id,
                None => {
                    error.set(Some(t!("error-congregation-create")));
                    return;
                }
            };
            let user_data = UserData {
                first_name: ob.first_name.clone(),
                last_name: ob.last_name.clone(),
                birthday: None,
                baptism_date: None,
                phone: None,
                address: None,
                email: Some(ob.email.clone()),
                password: None,
                user_type: UserType::BaptizedPublisher,
                gender: crate::models::user::Gender::Male,
                appointment: None,
                family_head: false,
                active: true,
                congregations: vec![cong_id],
            };
            if let Err(e) = User::create(&db, &crypto, user_data).await {
                error.set(Some(e.to_string()));
                return;
            }

            // Persist connection state
            let mut state = db_state.write();
            state.congregation_uid = Some(uid.clone());
            match &ob.mode {
                Some(DatabaseMode::Online) => {
                    state.mode = DatabaseMode::Online;
                    state.config = Some(OnlineConfig {
                        congregation_uid: uid,
                        username: ob.email.clone(),
                    });
                    state.db = Some(db);
                }
                _ => {
                    // Save uid to localStorage so session can be restored on refresh
                    ls_set("theo_offline_uid", &uid);
                    state.mode = DatabaseMode::Offline;
                    state.db = Some(db);
                }
            }

            nav.push(crate::Route::AppDashboard {});
        });
    });

    rsx! {
        div { class: "space-y-6 py-4",
            if let Some(err) = error.read().clone() {
                div { class: "bg-red-50 border border-red-200 rounded-lg p-3 text-red-700 text-sm",
                    "{err}"
                }
                button {
                    class: "w-full py-2 text-gray-500 hover:text-gray-800 text-sm",
                    onclick: move |_| {
                        error.set(None);
                        step.set(LandingStep::OnboardingCongregation);
                    },
                    {t!("btn-back")}
                }
            } else {
                div { class: "flex flex-col items-center gap-4",
                    div { class: "w-10 h-10 border-4 border-blue-600 border-t-transparent rounded-full animate-spin" }
                    p { class: "text-gray-600 font-medium text-center",
                        {t!("onboarding-connecting")}
                    }
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// ResumeSessionStep
// ---------------------------------------------------------------------------

#[component]
fn ResumeSessionStep(mut step: Signal<LandingStep>, uid: String) -> Element {
    let mut password = use_signal(String::new);
    let mut loading = use_signal(|| false);
    let mut error: Signal<Option<String>> = use_signal(|| None);

    let mut db_state = use_db();
    let mut crypto_state = use_crypto();
    let nav = use_navigator();

    rsx! {
        div { class: "space-y-4",
            h2 { class: "text-xl font-semibold text-gray-800", {t!("landing-resume-title")} }
            p { class: "text-gray-500 text-sm", {t!("landing-resume-desc")} }

            if let Some(err) = error.read().clone() {
                div { class: "bg-red-50 border border-red-200 rounded-lg p-3 text-red-700 text-sm",
                    "{err}"
                }
            }

            FormField { label: t!("form-password"),
                input {
                    class: "w-full border border-gray-300 rounded-lg px-3 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-blue-500",
                    r#type: "password",
                    value: password.read().clone(),
                    oninput: move |e| password.set(e.value()),
                }
            }

            button {
                class: "w-full py-3 bg-blue-600 text-white rounded-xl font-medium hover:bg-blue-700 transition-colors disabled:opacity-50",
                disabled: *loading.read(),
                onclick: move |_| {
                    let pass = password.read().clone();
                    let uid_clone = uid.clone();
                    if pass.is_empty() {
                        error.set(Some(t!("error-fields-required")));
                        return;
                    }
                    spawn(async move {
                        loading.set(true);
                        error.set(None);

                        // Reconnect to IndexedDB
                        let db = match connect_offline(&uid_clone).await {
                            Ok(db) => db,
                            Err(e) => {
                                error.set(Some(e.to_string()));
                                loading.set(false);
                                return;
                            }
                        };

                        // Load keystore from DB (stored as JSON since KeyStore isn't SurrealValue)
                        let keystore_vals: Vec<serde_json::Value> = match db
                            .select("_keystore")
                            .await
                        {
                            Ok(ks) => ks,
                            Err(e) => {
                                error.set(Some(e.to_string()));
                                loading.set(false);
                                return;
                            }
                        };
                        let keystore = match keystore_vals.into_iter().next() {
                            Some(v) => {
                                match serde_json::from_value::<KeyStore>(v) {
                                    Ok(ks) => ks,
                                    Err(e) => {
                                        error.set(Some(e.to_string()));
                                        loading.set(false);
                                        return;
                                    }
                                }
                            }
                            None => {
                                error.set(Some(t!("error-congregation-create")));
                                loading.set(false);
                                return;
                            }
                        };
                        let sym_key = match keystore.unlock(&pass) {
                            Ok(k) => k,
                            Err(e) => {
                                error.set(Some(e.to_string()));
                                loading.set(false);
                                return;
                            }
                        };
                        crypto_state.write().set_key(sym_key);
                        let mut state = db_state.write();
                        state.db = Some(db);
                        state.mode = DatabaseMode::Offline;
                        state.congregation_uid = Some(uid_clone);
                        nav.push(crate::Route::AppDashboard {});
                    });
                },
                if *loading.read() {
                    {t!("btn-connecting")}
                } else {
                    {t!("btn-unlock")}
                }
            }

            button {
                class: "w-full py-2 text-sm text-gray-500 hover:text-gray-800 transition-colors",
                onclick: move |_| {
                    ls_remove("theo_offline_uid");
                    step.set(LandingStep::AccountChoice);
                },
                {t!("landing-resume-different-account")}
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Shared helpers
// ---------------------------------------------------------------------------

#[component]
fn FormField(label: String, children: Element) -> Element {
    rsx! {
        div {
            label { class: "block text-sm font-medium text-gray-700 mb-1", "{label}" }
            {children}
        }
    }
}
