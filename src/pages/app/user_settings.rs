use dioxus::prelude::*;
use dioxus_i18n::{prelude::i18n, t, unic_langid::LanguageIdentifier};
use serde::{Deserialize, Serialize};

use crate::components::ThemePreview;
use crate::database::{use_db, ls_get, ls_set};
use crate::models::congregation::{AccentColor, Congregation, DateFormat, NameFormat, Theme, TimeFormat};

// ── Persisted user prefs ──────────────────────────────────────────────────────

/// User-level overrides stored in `localStorage` as JSON.
/// Each field is `None` when the congregation default is used.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct UserPrefs {
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub theme: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub accent_color: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub name_format: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub date_format: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub time_format: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub language: Option<String>,
}

pub fn prefs_storage_key(uid: &str) -> String {
    format!("theo_user_prefs_{}", uid)
}

pub async fn load_prefs(uid: &str) -> UserPrefs {
    let key = prefs_storage_key(uid);
    if let Some(json) = ls_get(&key).await {
        serde_json::from_str(&json).unwrap_or_default()
    } else {
        UserPrefs::default()
    }
}

pub fn save_prefs(uid: &str, prefs: &UserPrefs) {
    let key = prefs_storage_key(uid);
    if let Ok(json) = serde_json::to_string(prefs) {
        ls_set(&key, &json);
    }
}

/// Apply theme + accent from prefs (or congregation fallback) to `document.body`.
pub fn apply_prefs_to_body(prefs: &UserPrefs, congregation: Option<&Congregation>) {
    let theme_str = prefs
        .theme
        .as_deref()
        .filter(|s| !s.is_empty())
        .or_else(|| {
            congregation.map(|c| match c.theme {
                Theme::Dark => "dark",
                _ => "light",
            })
        })
        .unwrap_or("light");

    let accent_str = prefs
        .accent_color
        .as_deref()
        .filter(|s| !s.is_empty())
        .or_else(|| {
            congregation.map(|c| match c.accent_color {
                AccentColor::Green => "Green",
                AccentColor::Purple => "Purple",
                AccentColor::Rose => "Rose",
                AccentColor::Amber => "Amber",
                _ => "Blue",
            })
        })
        .unwrap_or("Blue");

    let js = format!(
        "document.body.setAttribute('data-theme','{}');document.body.setAttribute('data-accent','{}');",
        theme_str, accent_str
    );
    let _ = document::eval(&js);
}

// ── Helper component ──────────────────────────────────────────────────────────

#[component]
fn FormField(label: String, children: Element) -> Element {
    rsx! {
        div {
            label { class: "block text-sm font-medium text-gray-700 mb-1", "{label}" }
            {children}
        }
    }
}

// ── Page ──────────────────────────────────────────────────────────────────────

#[component]
pub fn AppUserSettings() -> Element {
    let db_state = use_db();
    let congregation_res = use_context::<Resource<Option<Congregation>>>();

    // The UID for the active workspace — used as the localStorage key.
    let uid = db_state.read().congregation_uid.clone().unwrap_or_default();

    // Signals for each preference field.
    // Value is "" when "congregation default" is selected.
    let mut pref_theme = use_signal(String::new);
    let mut pref_accent = use_signal(String::new);
    let mut pref_name_format = use_signal(String::new);
    let mut pref_date_format = use_signal(String::new);
    let mut pref_time_format = use_signal(String::new);
    let mut pref_language = use_signal(String::new);

    let mut saved = use_signal(|| false);

    // Load saved prefs once mounted.
    {
        let uid = uid.clone();
        use_effect(move || {
            let uid = uid.clone();
            spawn(async move {
                let prefs = load_prefs(&uid).await;
                pref_theme.set(prefs.theme.unwrap_or_default());
                pref_accent.set(prefs.accent_color.unwrap_or_default());
                pref_name_format.set(prefs.name_format.unwrap_or_default());
                pref_date_format.set(prefs.date_format.unwrap_or_default());
                pref_time_format.set(prefs.time_format.unwrap_or_default());
                pref_language.set(prefs.language.unwrap_or_default());
            });
        });
    }

    // Derive the congregation's own values for display as "(Congregation default)" labels.
    let cong = congregation_res.read();
    let cong_ref = cong.as_ref().and_then(|c| c.as_ref());

    let cong_theme_label = cong_ref.map(|c| match c.theme {
        Theme::Dark => t!("theme-dark"),
        _ => t!("theme-light"),
    }).unwrap_or_default();

    let cong_accent_label = cong_ref.map(|c| match c.accent_color {
        AccentColor::Green => t!("accent-green"),
        AccentColor::Purple => t!("accent-purple"),
        AccentColor::Rose => t!("accent-rose"),
        AccentColor::Amber => t!("accent-amber"),
        _ => t!("accent-blue"),
    }).unwrap_or_default();

    let cong_name_format_label = cong_ref.map(|c| match c.name_format {
        NameFormat::LastFirst => t!("format-last-first"),
        _ => t!("format-first-last"),
    }).unwrap_or_default();

    let cong_date_format_label = cong_ref.map(|c| match c.date_format {
        DateFormat::DMY => t!("format-dmy"),
        DateFormat::MDY => t!("format-mdy"),
        _ => t!("format-ymd"),
    }).unwrap_or_default();

    let cong_time_format_label = cong_ref.map(|c| match c.time_format {
        TimeFormat::H12 => t!("format-12h"),
        _ => t!("format-24h"),
    }).unwrap_or_default();

    let cong_language_label = cong_ref.map(|c| match c.language.as_str() {
        "es-ES" => t!("lang-es"),
        _ => t!("lang-en"),
    }).unwrap_or_default();

    // Save handler
    let mut save = {
        let uid = uid.clone();
        move || {
            let prefs = UserPrefs {
                theme: Some(pref_theme.read().clone()).filter(|s| !s.is_empty()),
                accent_color: Some(pref_accent.read().clone()).filter(|s| !s.is_empty()),
                name_format: Some(pref_name_format.read().clone()).filter(|s| !s.is_empty()),
                date_format: Some(pref_date_format.read().clone()).filter(|s| !s.is_empty()),
                time_format: Some(pref_time_format.read().clone()).filter(|s| !s.is_empty()),
                language: Some(pref_language.read().clone()).filter(|s| !s.is_empty()),
            };
            save_prefs(&uid, &prefs);

            // Apply theme/accent immediately.
            let cong = congregation_res.read();
            let cong_ref = cong.as_ref().and_then(|c| c.as_ref());
            apply_prefs_to_body(&prefs, cong_ref);

            // Apply language immediately.
            let lang_str = prefs.language.as_deref()
                .filter(|s| !s.is_empty())
                .or_else(|| cong_ref.map(|c| c.language.as_str()))
                .unwrap_or("en-US");
            if let Ok(lang_id) = LanguageIdentifier::from_bytes(lang_str.as_bytes()) {
                i18n().set_language(lang_id);
            }

            saved.set(true);
        }
    };

    // Reset handler — clears all overrides.
    let mut reset = {
        let uid = uid.clone();
        move || {
            pref_theme.set(String::new());
            pref_accent.set(String::new());
            pref_name_format.set(String::new());
            pref_date_format.set(String::new());
            pref_time_format.set(String::new());
            pref_language.set(String::new());
            save_prefs(&uid, &UserPrefs::default());

            // Re-apply congregation defaults.
            let cong = congregation_res.read();
            let cong_ref = cong.as_ref().and_then(|c| c.as_ref());
            apply_prefs_to_body(&UserPrefs::default(), cong_ref);

            // Reset language to congregation default.
            let lang_str = cong_ref.map(|c| c.language.as_str()).unwrap_or("en-US");
            if let Ok(lang_id) = LanguageIdentifier::from_bytes(lang_str.as_bytes()) {
                i18n().set_language(lang_id);
            }

            saved.set(true);
        }
    };

    let cong_default_label = t!("default");
    let select_class = "w-full border border-gray-300 rounded-lg px-3 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-primary-500 bg-white";

    rsx! {
        div { class: "max-w-2xl mx-auto space-y-6 w-full pb-24",
            h1 { class: "text-2xl font-bold text-gray-900", {t!("user-settings-title")} }
            p { class: "text-sm text-gray-500", {t!("user-settings-desc")} }

            if *saved.read() {
                div { class: "bg-primary-50 border border-primary-200 rounded-lg p-3 text-primary-700 text-sm",
                    {t!("user-settings-saved")}
                }
            }

            div { class: "bg-white rounded-xl border border-gray-200 p-6 space-y-5",

                // ── Theme ──────────────────────────────────────────────────
                FormField { label: t!("onboarding-congregation-theme"),
                    select {
                        class: select_class,
                        value: pref_theme.read().clone(),
                        onchange: move |e| {
                            pref_theme.set(e.value());
                            saved.set(false);
                        },
                        option { value: "", "{cong_theme_label} ({cong_default_label})" }
                        option { value: "light", {t!("theme-light")} }
                        option { value: "dark", {t!("theme-dark")} }
                    }
                }

                // ── Accent Color ───────────────────────────────────────────
                FormField { label: t!("onboarding-congregation-accent-color"),
                    select {
                        class: select_class,
                        value: pref_accent.read().clone(),
                        onchange: move |e| {
                            pref_accent.set(e.value());
                            saved.set(false);
                        },
                        option { value: "", "{cong_accent_label} ({cong_default_label})" }
                        option { value: "Blue", {t!("accent-blue")} }
                        option { value: "Green", {t!("accent-green")} }
                        option { value: "Purple", {t!("accent-purple")} }
                        option { value: "Rose", {t!("accent-rose")} }
                        option { value: "Amber", {t!("accent-amber")} }
                    }
                }
                // ── Theme preview ──────────────────────────────────────────
                {
                    let effective_theme = {
                        let t = pref_theme.read();
                        if t.is_empty() {
                            cong_ref
                                .map(|c| match c.theme {
                                    Theme::Dark => "dark",
                                    _ => "light",
                                })
                                .unwrap_or("light")
                                .to_string()
                        } else {
                            t.clone()
                        }
                    };
                    let effective_accent = {
                        let a = pref_accent.read();
                        if a.is_empty() {
                            cong_ref
                                .map(|c| match c.accent_color {
                                    AccentColor::Green => "Green",
                                    AccentColor::Purple => "Purple",
                                    AccentColor::Rose => "Rose",
                                    AccentColor::Amber => "Amber",
                                    _ => "Blue",
                                })
                                .unwrap_or("Blue")
                                .to_string()
                        } else {
                            a.clone()
                        }
                    };
                    rsx! {
                        ThemePreview { theme: effective_theme, accent: effective_accent }
                    }
                }
                // ── Name Format ────────────────────────────────────────────
                FormField { label: t!("onboarding-congregation-name-format"),
                    select {
                        class: select_class,
                        value: pref_name_format.read().clone(),
                        onchange: move |e| {
                            pref_name_format.set(e.value());
                            saved.set(false);
                        },
                        option { value: "", "{cong_name_format_label} ({cong_default_label})" }
                        option { value: "FirstLast", {t!("format-first-last")} }
                        option { value: "LastFirst", {t!("format-last-first")} }
                    }
                }

                // ── Date Format ────────────────────────────────────────────
                FormField { label: t!("onboarding-congregation-date-format"),
                    select {
                        class: select_class,
                        value: pref_date_format.read().clone(),
                        onchange: move |e| {
                            pref_date_format.set(e.value());
                            saved.set(false);
                        },
                        option { value: "", "{cong_date_format_label} ({cong_default_label})" }
                        option { value: "YMD", {t!("format-ymd")} }
                        option { value: "DMY", {t!("format-dmy")} }
                        option { value: "MDY", {t!("format-mdy")} }
                    }
                }

                // ── Time Format ────────────────────────────────────────────
                FormField { label: t!("onboarding-congregation-time-format"),
                    select {
                        class: select_class,
                        value: pref_time_format.read().clone(),
                        onchange: move |e| {
                            pref_time_format.set(e.value());
                            saved.set(false);
                        },
                        option { value: "", "{cong_time_format_label} ({cong_default_label})" }
                        option { value: "24h", {t!("format-24h")} }
                        option { value: "12h", {t!("format-12h")} }
                    }
                }

                // ── Language ───────────────────────────────────────────────
                FormField { label: t!("onboarding-congregation-language"),
                    select {
                        class: select_class,
                        value: pref_language.read().clone(),
                        onchange: move |e| {
                            pref_language.set(e.value());
                            saved.set(false);
                        },
                        option { value: "", "{cong_language_label} ({cong_default_label})" }
                        option { value: "en-US", {t!("lang-en")} }
                        option { value: "es-ES", {t!("lang-es")} }
                    }
                }
            }

            // ── Actions ────────────────────────────────────────────────────
            div { class: "flex gap-3",
                button {
                    class: "px-6 py-2 bg-primary-600 text-white rounded-lg font-medium hover:bg-primary-700 transition-colors",
                    onclick: move |_| save(),
                    {t!("btn-save")}
                }
                button {
                    class: "px-6 py-2 border border-gray-300 text-gray-700 rounded-lg font-medium hover:bg-gray-50 transition-colors",
                    onclick: move |_| reset(),
                    {t!("user-settings-reset")}
                }
            }
        }
    }
}
