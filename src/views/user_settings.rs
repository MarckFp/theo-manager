use dioxus::prelude::*;
use web_sys::{window, Storage};

#[derive(Props, Clone, PartialEq)]
pub struct UserSettingsProps {
    pub on_navigate: EventHandler<String>,
}

// Available DaisyUI themes
const THEMES: &[(&str, &str)] = &[
    ("light", "Light"),
    ("dark", "Dark"),
    ("cupcake", "Cupcake"),
    ("bumblebee", "Bumblebee"),
    ("emerald", "Emerald"),
    ("corporate", "Corporate"),
    ("synthwave", "Synthwave"),
    ("retro", "Retro"),
    ("cyberpunk", "Cyberpunk"),
    ("valentine", "Valentine"),
    ("halloween", "Halloween"),
    ("garden", "Garden"),
    ("forest", "Forest"),
    ("aqua", "Aqua"),
    ("lofi", "Lofi"),
    ("pastel", "Pastel"),
    ("fantasy", "Fantasy"),
    ("wireframe", "Wireframe"),
    ("black", "Black"),
    ("luxury", "Luxury"),
    ("dracula", "Dracula"),
    ("cmyk", "CMYK"),
    ("autumn", "Autumn"),
    ("business", "Business"),
    ("acid", "Acid"),
    ("lemonade", "Lemonade"),
    ("night", "Night"),
    ("coffee", "Coffee"),
    ("winter", "Winter"),
    ("dim", "Dim"),
    ("nord", "Nord"),
    ("sunset", "Sunset"),
];

// Available languages (prepared for future i18n)
const LANGUAGES: &[(&str, &str)] = &[
    ("en", "English"),
    ("es", "Español"),
];

// LocalStorage keys
const THEME_KEY: &str = "theo_manager_theme";
const LANGUAGE_KEY: &str = "theo_manager_language";

#[component]
pub fn UserSettings(props: UserSettingsProps) -> Element {
    // Load current settings from localStorage
    let current_theme = use_signal(|| {
        if let Some(window) = window() {
            if let Ok(Some(storage)) = window.local_storage() {
                storage.get_item(THEME_KEY).ok().flatten().unwrap_or_else(|| "black".to_string())
            } else {
                "black".to_string()
            }
        } else {
            "black".to_string()
        }
    });
    
    let current_language = use_signal(|| {
        if let Some(window) = window() {
            if let Ok(Some(storage)) = window.local_storage() {
                storage.get_item(LANGUAGE_KEY).ok().flatten().unwrap_or_else(|| "en".to_string())
            } else {
                "en".to_string()
            }
        } else {
            "en".to_string()
        }
    });
    
    let mut selected_theme = use_signal(|| current_theme());
    let mut selected_language = use_signal(|| current_language());
    let mut save_message = use_signal(|| None::<String>);
    
    // Handle save
    let handle_save = move |_| {
        if let Some(window) = window() {
            if let Ok(Some(storage)) = window.local_storage() {
                // Save to localStorage
                let _ = storage.set_item(THEME_KEY, &selected_theme());
                let _ = storage.set_item(LANGUAGE_KEY, &selected_language());
                
                // Apply theme immediately
                if let Some(document) = window.document() {
                    if let Some(html) = document.document_element() {
                        let _ = html.set_attribute("data-theme", &selected_theme());
                        let _ = html.set_attribute("lang", &selected_language());
                    }
                }
                
                save_message.set(Some("Settings saved successfully!".to_string()));
                
                // Clear message after 3 seconds
                spawn(async move {
                    gloo_timers::future::TimeoutFuture::new(3000).await;
                    save_message.set(None);
                });
            }
        }
    };
    
    // Check if settings have changed
    let has_changes = selected_theme() != current_theme() || selected_language() != current_language();
    
    rsx! {
        div { class: "space-y-6 max-w-4xl mx-auto w-full px-2 sm:px-0 overflow-x-hidden",
            // Breadcrumbs
            div { class: "text-sm breadcrumbs mb-4",
                ul {
                    li {
                        a {
                            class: "text-primary",
                            onclick: move |_| props.on_navigate.call("dashboard".to_string()),
                            "Home"
                        }
                    }
                    li {
                        a {
                            class: "text-primary",
                            onclick: move |_| props.on_navigate.call("settings-category".to_string()),
                            "Settings"
                        }
                    }
                    li { "User Settings" }
                }
            }
            
            // Header
            div { class: "mb-6",
                h2 { class: "text-3xl font-bold text-base-content", "User Settings" }
                p { class: "text-base-content/70 mt-1", "Customize your experience" }
            }
            
            // Success message
            if let Some(msg) = save_message() {
                div { class: "alert alert-success shadow-lg mb-4",
                    span { "{msg}" }
                }
            }
            
            // Settings Card
            div { class: "card bg-base-100 shadow-lg w-full",
                div { class: "card-body p-4 sm:p-8",
                    // Theme Selection
                    div { class: "form-control mb-6",
                        label { class: "label",
                            span { class: "label-text font-semibold text-lg", "Theme" }
                        }
                        select {
                            class: "select select-bordered w-full",
                            value: "{selected_theme()}",
                            onchange: move |evt| selected_theme.set(evt.value()),
                            for (theme_id, theme_name) in THEMES.iter() {
                                option {
                                    value: "{theme_id}",
                                    selected: selected_theme() == *theme_id,
                                    "{theme_name}"
                                }
                            }
                        }
                        label { class: "label",
                            span { class: "label-text-alt text-base-content/60",
                                "Choose your preferred color theme"
                            }
                        }
                    }
                    
                    // Language Selection
                    div { class: "form-control mb-6",
                        label { class: "label",
                            span { class: "label-text font-semibold text-lg", "Language" }
                        }
                        select {
                            class: "select select-bordered w-full",
                            value: "{selected_language()}",
                            onchange: move |evt| selected_language.set(evt.value()),
                            for (lang_id, lang_name) in LANGUAGES.iter() {
                                option {
                                    value: "{lang_id}",
                                    selected: selected_language() == *lang_id,
                                    "{lang_name}"
                                }
                            }
                        }
                        label { class: "label",
                            span { class: "label-text-alt text-base-content/60",
                                "Select your preferred language"
                            }
                        }
                    }
                    
                    // Theme Preview
                    div { class: "mb-6 overflow-x-hidden",
                        label { class: "label",
                            span { class: "label-text font-semibold text-lg", "Preview" }
                        }
                        div {
                            "data-theme": "{selected_theme()}",
                            class: "p-4 rounded-lg bg-base-200 space-y-3 overflow-x-hidden",
                            div { class: "flex flex-wrap gap-2",
                                button { class: "btn btn-primary btn-sm", "Primary" }
                                button { class: "btn btn-secondary btn-sm", "Secondary" }
                                button { class: "btn btn-accent btn-sm", "Accent" }
                            }
                            div { class: "flex flex-wrap gap-2",
                                div { class: "badge badge-primary", "Badge" }
                                div { class: "badge badge-secondary", "Badge" }
                                div { class: "badge badge-accent", "Badge" }
                            }
                            div { class: "alert alert-info",
                                span { "This is how alerts will look in this theme" }
                            }
                        }
                    }
                    
                    // Save Button
                    div { class: "flex justify-center mt-6",
                        button {
                            class: format!(
                                "btn btn-primary btn-wide {}",
                                if !has_changes { "btn-disabled" } else { "" }
                            ),
                            disabled: !has_changes,
                            onclick: handle_save,
                            "💾 Save Settings"
                        }
                    }
                }
            }
        }
    }
}
