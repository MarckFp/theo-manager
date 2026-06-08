use dioxus::prelude::*;
use dioxus_i18n::t;

use crate::components::ThemePreview;
use crate::crypto::KeyStore;
use crate::database::{
    DatabaseMode, OnlineConfig, connect_offline, connect_online, signup_online, use_crypto, use_db, ls_get, ls_set, ls_remove
};
use crate::models::congregation::{AccentColor, Congregation, CongregationData, DateFormat, NameFormat, Theme, TimeFormat};
use crate::models::user::{User, UserData, UserType};

// ---------------------------------------------------------------------------
// localStorage helpers (JS interop via document::eval)
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
    OnboardingEncryption,
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
    enc_password: String,
    enc_confirm_password: String,
    congregation_name: String,
    congregation_address: String,
    congregation_circuit: String,
    congregation_language: String,
    time_format: TimeFormat,
    date_format: DateFormat,
    name_format: NameFormat,
    theme: Theme,
    accent_color: AccentColor,
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
    let mut restore_checked = use_signal(|| false);
    use_effect(move || {
        if *restore_checked.peek() { return; }
        restore_checked.set(true);

        spawn(async move {
            let active = ls_get("theo_active_uid").await;
            if let Some(uid) = ls_get("theo_active_uid").await {
                step.set(LandingStep::ResumeSession { uid });
            } else if let Some(uid) = ls_get("theo_offline_uid").await { // legacy fallback
                step.set(LandingStep::ResumeSession { uid });
            } else {
                step.set(LandingStep::AccountChoice);
            }
        });
    });

    // Navigation guard (runs after hooks so hook count stays constant).
    // Require BOTH a db connection AND an unlocked crypto key: the resume flow
    // stores the db before decrypting so the connection is never dropped on a
    // wrong-password attempt (which would trigger a wasm32 panic via shutdown).
    let crypto_state_nav = use_crypto();
    if db_state.read().db.is_some() && crypto_state_nav.read().is_unlocked() {
        nav.push(crate::Route::AppDashboard {});
        return rsx! {};
    }

    rsx! {
        div { class: "min-h-screen bg-gradient-to-br from-slate-50 to-primary-50 flex items-center justify-center p-4",
            div { class: "w-full max-w-md",
                div { class: "text-center mb-8",
                    h1 { class: "text-3xl font-bold text-gray-900", {t!("app-name")} }
                    p { class: "text-gray-500 mt-1", {t!("landing-subtitle")} }
                }
                div { class: "bg-white rounded-2xl shadow-lg p-6",
                    match step.read().clone() {
                        LandingStep::CheckingRestore => rsx! {
                            div { class: "flex justify-center py-8",
                                div { class: "w-8 h-8 border-4 border-primary-600 border-t-transparent rounded-full animate-spin" }
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
                        LandingStep::OnboardingEncryption => rsx! {
                            OnboardingEncryptionStep { step, onboarding }
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
                    class: "w-full py-3 px-4 bg-primary-600 text-white rounded-xl font-medium hover:bg-primary-700 transition-colors",
                    onclick: move |_| step.set(LandingStep::Login),
                    {t!("landing-have-account")}
                }
                button {
                    class: "w-full py-3 px-4 border-2 border-primary-600 text-primary-600 rounded-xl font-medium hover:bg-primary-50 transition-colors",
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
    let mut login_checked = use_signal(|| false);
    use_effect(move || {
        if *login_checked.peek() { return; }
        login_checked.set(true);

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
                    class: "w-full border border-gray-300 rounded-lg px-3 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-primary-500",
                    r#type: "text",
                    placeholder: "xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx",
                    value: congregation_code.read().clone(),
                    oninput: move |e| congregation_code.set(e.value()),
                }
            }
            FormField { label: t!("form-email"),
                input {
                    class: "w-full border border-gray-300 rounded-lg px-3 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-primary-500",
                    r#type: "email",
                    value: username.read().clone(),
                    oninput: move |e| username.set(e.value()),
                }
            }
            FormField { label: t!("form-password"),
                input {
                    class: "w-full border border-gray-300 rounded-lg px-3 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-primary-500",
                    r#type: "password",
                    value: password.read().clone(),
                    oninput: move |e| password.set(e.value()),
                }
            }

            div { class: "flex items-center justify-between",
                label { class: "flex items-center gap-2 text-sm text-gray-600 cursor-pointer",
                    input {
                        r#type: "checkbox",
                        class: "rounded border-gray-300 text-primary-600",
                        checked: *remember_me.read(),
                        oninput: move |e| remember_me.set(e.checked()),
                    }
                    {t!("form-remember-me")}
                }
                button {
                    class: "text-sm text-primary-600 hover:underline",
                    onclick: move |_| step.set(LandingStep::ForgotPassword),
                    {t!("landing-forgot-password")}
                }
            }

            button {
                class: "w-full py-3 bg-primary-600 text-white rounded-xl font-medium hover:bg-primary-700 transition-colors disabled:opacity-50",
                disabled: *loading.read(),
                onclick: move |_| {
                    if *loading.peek() {
                        return;
                    }
                    loading.set(true);
                    let cid = congregation_code.peek().clone();
                    let user = username.peek().clone();
                    let pass = password.peek().clone();
                    let save = *remember_me.peek();
                    if cid.is_empty() || user.is_empty() || pass.is_empty() {
                        error.set(Some(t!("error-fields-required")));
                        loading.set(false);
                        return;
                    }
                    let config = OnlineConfig {
                        congregation_uid: cid.clone(),
                        username: user.clone(),
                    };
                    spawn(async move {
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
                    class: "w-full py-4 px-4 border-2 border-gray-200 rounded-xl text-left hover:border-primary-400 hover:bg-primary-50 transition-all",
                    onclick: move |_| {
                        onboarding.write().mode = Some(DatabaseMode::Offline);
                        step.set(LandingStep::OnboardingUser);
                    },
                    div { class: "font-medium text-gray-800", {t!("onboarding-mode-offline")} }
                    div { class: "text-sm text-gray-500 mt-0.5", {t!("onboarding-mode-offline-desc")} }
                }
                button {
                    class: "w-full py-4 px-4 border-2 border-gray-200 rounded-xl text-left hover:border-primary-400 hover:bg-primary-50 transition-all",
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
                        class: "w-full border border-gray-300 rounded-lg px-3 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-primary-500",
                        r#type: "text",
                        value: first_name.read().clone(),
                        oninput: move |e| first_name.set(e.value()),
                    }
                }
                FormField { label: t!("form-last-name"),
                    input {
                        class: "w-full border border-gray-300 rounded-lg px-3 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-primary-500",
                        r#type: "text",
                        value: last_name.read().clone(),
                        oninput: move |e| last_name.set(e.value()),
                    }
                }
            }
            FormField { label: t!("form-email"),
                input {
                    class: "w-full border border-gray-300 rounded-lg px-3 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-primary-500",
                    r#type: "email",
                    value: email.read().clone(),
                    oninput: move |e| email.set(e.value()),
                }
            }
            FormField { label: t!("form-password"),
                input {
                    class: "w-full border border-gray-300 rounded-lg px-3 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-primary-500",
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
                    class: "flex-1 py-3 bg-primary-600 text-white rounded-xl font-medium hover:bg-primary-700 transition-colors",
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
    let mut cong_address = use_signal(|| onboarding.read().congregation_address.clone());
    let mut cong_circuit = use_signal(|| onboarding.read().congregation_circuit.clone());
    let mut cong_language = use_signal(|| onboarding.read().congregation_language.clone());
    let mut time_format = use_signal(|| onboarding.read().time_format.clone());
    let mut date_format = use_signal(|| onboarding.read().date_format.clone());
    let mut name_format = use_signal(|| onboarding.read().name_format.clone());
    let mut theme = use_signal(|| onboarding.read().theme.clone());
    let mut accent_color = use_signal(|| onboarding.read().accent_color.clone());

    // Detect browser locale and pre-select language if not already set
    use_effect(move || {
        if cong_language.read().is_empty() {
            spawn(async move {
                let mut eval =
                    document::eval("dioxus.send(navigator.language || 'en-US');");
                let locale = eval
                    .recv::<String>()
                    .await
                    .unwrap_or_else(|_| "en-US".to_string());
                let lang = if locale.starts_with("es") { "es-ES" } else { "en-US" };
                cong_language.set(lang.to_string());
            });
        }
    });

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
                    class: "w-full border border-gray-300 rounded-lg px-3 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-primary-500",
                    r#type: "text",
                    value: cong_name.read().clone(),
                    oninput: move |e| cong_name.set(e.value()),
                }
            }
            FormField { label: t!("onboarding-congregation-address"),
                input {
                    class: "w-full border border-gray-300 rounded-lg px-3 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-primary-500",
                    r#type: "text",
                    value: cong_address.read().clone(),
                    oninput: move |e| cong_address.set(e.value()),
                }
            }
            FormField { label: t!("onboarding-congregation-circuit"),
                input {
                    class: "w-full border border-gray-300 rounded-lg px-3 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-primary-500",
                    r#type: "text",
                    value: cong_circuit.read().clone(),
                    oninput: move |e| cong_circuit.set(e.value()),
                }
            }
            FormField { label: t!("onboarding-congregation-language"),
                select {
                    class: "w-full border border-gray-300 rounded-lg px-3 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-primary-500 bg-white",
                    value: cong_language.read().clone(),
                    onchange: move |e| cong_language.set(e.value()),
                    option { value: "en-US", "\u{1f1fa}\u{1f1f8} English" }
                    option { value: "es-ES", "\u{1f1ea}\u{1f1f8} Español" }
                }
            }
            div { class: "grid grid-cols-2 gap-3",
                FormField { label: t!("onboarding-congregation-time-format"),
                    select {
                        class: "w-full border border-gray-300 rounded-lg px-3 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-primary-500 bg-white",
                        value: match *time_format.read() {
                            TimeFormat::H12 => "12h",
                            TimeFormat::H24 => "24h",
                        },
                        onchange: move |e| {
                            let val = match e.value().as_str() {
                                "12h" => TimeFormat::H12,
                                _ => TimeFormat::H24,
                            };
                            time_format.set(val);
                        },
                        option { value: "12h", "12h (AM/PM)" }
                        option { value: "24h", "24h" }
                    }
                }
                FormField { label: t!("onboarding-congregation-date-format"),
                    select {
                        class: "w-full border border-gray-300 rounded-lg px-3 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-primary-500 bg-white",
                        value: match *date_format.read() {
                            DateFormat::YMD => "YMD",
                            DateFormat::DMY => "DMY",
                            DateFormat::MDY => "MDY",
                        },
                        onchange: move |e| {
                            let val = match e.value().as_str() {
                                "DMY" => DateFormat::DMY,
                                "MDY" => DateFormat::MDY,
                                _ => DateFormat::YMD,
                            };
                            date_format.set(val);
                        },
                        option { value: "YMD", "YYYY-MM-DD" }
                        option { value: "DMY", "DD-MM-YYYY" }
                        option { value: "MDY", "MM-DD-YYYY" }
                    }
                }
            }
            div { class: "grid grid-cols-2 gap-3",
                FormField { label: t!("onboarding-congregation-name-format"),
                    select {
                        class: "w-full border border-gray-300 rounded-lg px-3 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-primary-500 bg-white",
                        value: match *name_format.read() {
                            NameFormat::FirstLast => "FirstLast",
                            NameFormat::LastFirst => "LastFirst",
                        },
                        onchange: move |e| {
                            let val = match e.value().as_str() {
                                "LastFirst" => NameFormat::LastFirst,
                                _ => NameFormat::FirstLast,
                            };
                            name_format.set(val);
                        },
                        option { value: "FirstLast", {t!("format-first-last")} }
                        option { value: "LastFirst", {t!("format-last-first")} }
                    }
                }
                FormField { label: t!("onboarding-congregation-theme"),
                    select {
                        class: "w-full border border-gray-300 rounded-lg px-3 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-primary-500 bg-white",
                        value: match *theme.read() {
                            Theme::Light => "Light",
                            Theme::Dark => "Dark",
                        },
                        onchange: move |e| {
                            let val = match e.value().as_str() {
                                "Dark" => Theme::Dark,
                                _ => Theme::Light,
                            };
                            theme.set(val);
                        },
                        option { value: "Light", {t!("theme-light")} }
                        option { value: "Dark", {t!("theme-dark")} }
                    }
                }
            }
            div { class: "grid grid-cols-1 gap-3",
                FormField { label: t!("onboarding-congregation-accent-color"),
                    select {
                        class: "w-full border border-gray-300 rounded-lg px-3 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-primary-500 bg-white",
                        value: match *accent_color.read() {
                            AccentColor::Blue => "Blue",
                            AccentColor::Green => "Green",
                            AccentColor::Purple => "Purple",
                            AccentColor::Rose => "Rose",
                            AccentColor::Amber => "Amber",
                        },
                        onchange: move |e| {
                            let val = match e.value().as_str() {
                                "Green" => AccentColor::Green,
                                "Purple" => AccentColor::Purple,
                                "Rose" => AccentColor::Rose,
                                "Amber" => AccentColor::Amber,
                                _ => AccentColor::Blue,
                            };
                            accent_color.set(val);
                        },
                        option { value: "Blue", {t!("accent-blue")} }
                        option { value: "Green", {t!("accent-green")} }
                        option { value: "Purple", {t!("accent-purple")} }
                        option { value: "Rose", {t!("accent-rose")} }
                        option { value: "Amber", {t!("accent-amber")} }
                    }
                }
            }

            // ── Theme preview ──────────────────────────────────────────────
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

            div { class: "flex gap-3 pt-1",
                button {
                    class: "flex-1 py-3 border border-gray-300 rounded-xl text-gray-700 font-medium hover:bg-gray-50 transition-colors",
                    onclick: move |_| step.set(LandingStep::OnboardingUser),
                    {t!("btn-back")}
                }
                button {
                    class: "flex-1 py-3 bg-primary-600 text-white rounded-xl font-medium hover:bg-primary-700 transition-colors",
                    onclick: move |_| {
                        let name = cong_name.read().clone();
                        let address = cong_address.read().clone();
                        let circuit = cong_circuit.read().clone();
                        let language = cong_language.read().clone();
                        let t_fmt = time_format.read().clone();
                        let d_fmt = date_format.read().clone();
                        let n_fmt = name_format.read().clone();
                        let thm = theme.read().clone();
                        let acc = accent_color.read().clone();

                        if name.is_empty() || language.is_empty() {
                            error.set(Some(t!("error-fields-required")));
                            return;
                        }
                        let mut ob = onboarding.write();
                        ob.congregation_name = name;
                        ob.congregation_address = address;
                        ob.congregation_circuit = circuit;
                        ob.congregation_language = language;
                        ob.time_format = t_fmt;
                        ob.date_format = d_fmt;
                        ob.name_format = n_fmt;
                        ob.theme = thm;
                        ob.accent_color = acc;
                        drop(ob);
                        step.set(LandingStep::OnboardingEncryption);
                    },
                    {t!("btn-next")}
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// OnboardingEncryptionStep
// ---------------------------------------------------------------------------

#[component]
fn OnboardingEncryptionStep(
    mut step: Signal<LandingStep>,
    mut onboarding: Signal<OnboardingState>,
) -> Element {
    let mut enc_password = use_signal(|| onboarding.read().enc_password.clone());
    let mut enc_confirm = use_signal(|| onboarding.read().enc_confirm_password.clone());
    let mut error: Signal<Option<String>> = use_signal(|| None);

    let strength_pct = use_memo(move || password_strength(&enc_password.read()));

    rsx! {
        div { class: "space-y-4",
            div { class: "flex items-center gap-3 mb-1",
                div { class: "w-10 h-10 bg-primary-100 rounded-xl flex items-center justify-center shrink-0",
                    svg {
                        class: "w-5 h-5 text-primary-600",
                        fill: "none",
                        stroke: "currentColor",
                        view_box: "0 0 24 24",
                        path {
                            stroke_linecap: "round",
                            stroke_linejoin: "round",
                            stroke_width: "2",
                            d: "M12 15v2m-6 4h12a2 2 0 002-2v-6a2 2 0 00-2-2H6a2 2 0 00-2 2v6a2 2 0 002 2zm10-10V7a4 4 0 00-8 0v4h8z",
                        }
                    }
                }
                div {
                    h2 { class: "text-xl font-semibold text-gray-800",
                        {t!("onboarding-encryption-title")}
                    }
                    p { class: "text-gray-500 text-xs", {t!("onboarding-encryption-desc")} }
                }
            }

            div { class: "bg-primary-50 border border-primary-200 rounded-xl p-4 text-sm text-primary-800 leading-relaxed",
                {t!("onboarding-encryption-explanation")}
            }

            div { class: "flex items-start gap-2 bg-amber-50 border border-amber-200 rounded-xl p-3 text-sm text-amber-800",
                span { class: "shrink-0 mt-0.5", "⚠️" }
                span { {t!("onboarding-encryption-warning")} }
            }

            if let Some(err) = error.read().clone() {
                div { class: "bg-red-50 border border-red-200 rounded-lg p-3 text-red-700 text-sm",
                    "{err}"
                }
            }

            FormField { label: t!("onboarding-encryption-password"),
                input {
                    class: "w-full border border-gray-300 rounded-lg px-3 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-primary-500",
                    r#type: "password",
                    value: enc_password.read().clone(),
                    oninput: move |e| enc_password.set(e.value()),
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
                    value: enc_confirm.read().clone(),
                    oninput: move |e| enc_confirm.set(e.value()),
                }
            }

            div { class: "flex gap-3 pt-1",
                button {
                    class: "flex-1 py-3 border border-gray-300 rounded-xl text-gray-700 font-medium hover:bg-gray-50 transition-colors",
                    onclick: move |_| step.set(LandingStep::OnboardingCongregation),
                    {t!("btn-back")}
                }
                button {
                    class: "flex-1 py-3 bg-primary-600 text-white rounded-xl font-medium hover:bg-primary-700 transition-colors",
                    onclick: move |_| {
                        let pw = enc_password.read().clone();
                        let cp = enc_confirm.read().clone();
                        if pw.is_empty() {
                            error.set(Some(t!("error-fields-required")));
                            return;
                        }
                        if pw != cp {
                            error.set(Some(t!("error-passwords-mismatch")));
                            return;
                        }
                        let mut ob = onboarding.write();
                        ob.enc_password = pw;
                        ob.enc_confirm_password = cp;
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

    let mut started = use_signal(|| false);

    use_effect(move || {
        if *started.peek() { return; }
        started.set(true);

        let ob = onboarding.peek().clone();
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
            let (keystore, sym_key) = match crate::crypto::KeyStore::create(&ob.enc_password) {
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
                address: (!ob.congregation_address.is_empty()).then(|| ob.congregation_address.clone()),
                circuit: (!ob.congregation_circuit.is_empty()).then(|| ob.congregation_circuit.clone()),
                language: ob.congregation_language.clone(),
                time_format: ob.time_format.clone(),
                date_format: ob.date_format.clone(),
                name_format: ob.name_format.clone(),
                theme: ob.theme.clone(),
                accent_color: ob.accent_color.clone(),
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
                congregations: vec![cong_id.clone()],
            };
            let created_user = match User::create(&db, &crypto, user_data).await {
                Ok(Some(u)) => u,
                _ => {
                    error.set(Some("Failed to create user".to_string()));
                    return;
                }
            };

            if let Some(user_id) = &created_user.id {
                let key_str = match &user_id.key {
                    surrealdb::types::RecordIdKey::String(s) => s.clone(),
                    surrealdb::types::RecordIdKey::Number(n) => n.to_string(),
                    _ => "unknown".to_string(),
                };
                let id_str = format!("{}:{}", user_id.table.as_str(), key_str);
                ls_set("theo_my_user_id", &id_str);
            }

            // Persist connection state
            let mut state = db_state.write();
            state.congregation_uid = Some(uid.clone());
            state.active_congregation_id = Some(cong_id.clone());

            let workspace = crate::database::Workspace {
                uid: uid.clone(),
                name: ob.congregation_name.clone(),
                mode: ob.mode.clone().unwrap(),
                username: Some(ob.email.clone()),
                theme: match ob.theme { Theme::Dark => "dark".to_string(), _ => "light".to_string() },
                accent_color: match ob.accent_color { AccentColor::Green => "Green".to_string(), AccentColor::Purple => "Purple".to_string(), AccentColor::Rose => "Rose".to_string(), AccentColor::Amber => "Amber".to_string(), _ => "Blue".to_string() },
            };
            crate::database::add_workspace(workspace).await;

            match &ob.mode {
                Some(DatabaseMode::Online) => {
                    ls_set("theo_active_uid", &uid);
                    state.mode = DatabaseMode::Online;
                    state.config = Some(OnlineConfig {
                        congregation_uid: uid,
                        username: ob.email.clone(),
                    });
                    state.db = Some(db);
                }
                _ => {
                    ls_set("theo_active_uid", &uid);
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
                        step.set(LandingStep::OnboardingEncryption);
                    },
                    {t!("btn-back")}
                }
            } else {
                div { class: "flex flex-col items-center gap-4",
                    div { class: "w-10 h-10 border-4 border-primary-600 border-t-transparent rounded-full animate-spin" }
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
    let mut selected_uid = use_signal(|| uid.clone());

    let workspaces = use_resource(move || async move {
        crate::database::get_workspaces().await
    });

    // Apply theme/accent from the selected workspace whenever selection changes.
    use_effect(move || {
        let uid = selected_uid.read().clone();
        if let Some(wks) = workspaces.read().as_ref() {
            if let Some(wk) = wks.iter().find(|w| w.uid == uid) {
                let theme = wk.theme.clone();
                let accent = wk.accent_color.clone();
                let js = format!(
                    "document.body.setAttribute('data-theme', '{}'); document.body.setAttribute('data-accent', '{}');",
                    theme, accent
                );
                let _ = document::eval(&js);
            }
        }
    });

    rsx! {
        div { class: "space-y-4",
            h2 { class: "text-xl font-semibold text-gray-800", {t!("landing-resume-title")} }
            p { class: "text-gray-500 text-sm", {t!("landing-resume-desc")} }

            if let Some(err) = error.read().clone() {
                div { class: "bg-red-50 border border-red-200 rounded-lg p-3 text-red-700 text-sm",
                    "{err}"
                }
            }

            if let Some(wks) = workspaces.read().as_ref() {
                if wks.len() > 1 {
                    FormField { label: t!("congregation-label"),
                        select {
                            class: "w-full border border-gray-300 rounded-lg px-3 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-primary-500 bg-white",
                            value: selected_uid.read().clone(),
                            onchange: move |e| selected_uid.set(e.value()),
                            for wk in wks.iter() {
                                option {
                                    value: "{wk.uid}",
                                    selected: *selected_uid.read() == wk.uid,
                                    {
                                        if wk.mode == crate::database::DatabaseMode::Offline {
                                            "💾 "
                                        } else {
                                            "☁️ "
                                        }
                                    }
                                    "{wk.name}"
                                }
                            }
                        }
                    }
                }
            }

            FormField { label: t!("form-password"),
                input {
                    class: "w-full border border-gray-300 rounded-lg px-3 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-primary-500",
                    r#type: "password",
                    value: password.read().clone(),
                    oninput: move |e| password.set(e.value()),
                }
            }

            button {
                class: "w-full py-3 bg-primary-600 text-white rounded-xl font-medium hover:bg-primary-700 transition-colors disabled:opacity-50",
                disabled: *loading.read(),
                onclick: move |_| {
                    if *loading.peek() {
                        return;
                    }
                    loading.set(true);
                    let pass = password.peek().clone();
                    let uid_clone = selected_uid.peek().clone();
                    if pass.is_empty() {
                        error.set(Some(t!("error-fields-required")));
                        loading.set(false);
                        return;
                    }
                    spawn(async move {
                        error.set(None);
                        // Re-connect always to the selected db (if it's not the already active one)
                        let active_cached = db_state.peek().congregation_uid.clone();
                        let is_new = active_cached != Some(uid_clone.clone());

                        if is_new || db_state.peek().db.is_none() {
                            match connect_offline(&uid_clone).await {
                                Ok(db) => {
                                    let mut state = db_state.write();
                                    if let Some(old) = state.db.take() {
                                        state.leaked_dbs.push(old);
                                    }
                                    state.db = Some(db);
                                    state.mode = DatabaseMode::Offline;
                                    state.congregation_uid = Some(uid_clone.clone());
                                }
                                Err(e) => {
                                    error.set(Some(e.to_string()));
                                    loading.set(false);
                                    return;
                                }
                            }
                        }
                        let db = db_state.peek().db.clone().unwrap();
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

                        // We verify the password is correct manually by trying to decrypt the congregation table
                        let mut temp_crypto = crate::crypto::SessionCrypto::default();
                        temp_crypto.set_key(sym_key.clone());
                        if crate::models::congregation::Congregation::all(&db, &temp_crypto)
                            .await
                            .is_err()
                        {
                            error.set(Some(t!("error-incorrect-password")));
                            loading.set(false);
                            return;
                        }
                        crypto_state.write().set_key(sym_key);
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
