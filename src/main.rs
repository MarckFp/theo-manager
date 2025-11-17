use dioxus::prelude::*;

mod database;
mod views;
mod components;

use views::{Landing, Home};
use database::models::congregation::Congregation;
use database::models::user_settings::UserSettings;

fn main() {
    dioxus::launch(App);
}

#[component]
fn App() -> Element {
    // Initialize database first - this must complete before anything else
    let db_init = use_resource(move || async move {
        match database::db::get_db().await {
            Ok(_) => Some(true),
            Err(e) => {
                eprintln!("Database initialization error: {:?}", e);
                Some(false)
            }
        }
    });

    // Check if we have any congregation data
    let has_data = use_resource(move || async move {
        // Wait for DB to be initialized successfully
        match db_init.read().as_ref() {
            Some(Some(true)) => {
                match database::models::congregation::Congregation::all().await {
                    Ok(congregations) => Some(!congregations.is_empty()),
                    Err(_) => Some(false),
                }
            }
            _ => None
        }
    });

    // Load theme and language from database
    let settings = use_resource(move || async move {
        // Wait for DB to be initialized successfully
        match db_init.read().as_ref() {
            Some(Some(true)) => UserSettings::get_or_create().await.ok(),
            _ => None
        }
    });

    // Set theme and language attributes when settings are loaded
    use_effect(move || {
        if let Some(Some(user_settings)) = settings.read().as_ref() {
            let lang = user_settings.language.clone();
            let theme = user_settings.theme.clone();

            // Set language attribute
            let lang_script = format!("document.documentElement.setAttribute('lang', '{}');", lang);
            document::eval(&lang_script);
            
            // Set theme attribute
            let theme_script = format!("document.documentElement.setAttribute('data-theme', '{}');", theme);
            document::eval(&theme_script);
        }
    });

    rsx! {
        // Set default theme in head - this will be overridden by the effect above
        document::Script {
            "
            // Set default theme immediately to avoid flicker (will be updated from DB)
            if (!document.documentElement.hasAttribute('data-theme')) {{
                document.documentElement.setAttribute('data-theme', 'dark');
            }}
            if (!document.documentElement.hasAttribute('lang')) {{
                document.documentElement.setAttribute('lang', 'en');
            }}
            "
        }

        // Icons
        document::Link {
            rel: "icon",
            r#type: "image/x-icon",
            href: asset!("/assets/favicon.ico", AssetOptions::builder().with_hash_suffix(false)),
        }
        document::Link {
            rel: "icon",
            r#type: "image/png",
            sizes: "32x32",
            href: asset!("/assets/favicon-32x32.png", AssetOptions::builder().with_hash_suffix(false)),
        }
        document::Link {
            rel: "icon",
            r#type: "image/png",
            sizes: "16x16",
            href: asset!("/assets/favicon-16x16.png", AssetOptions::builder().with_hash_suffix(false)),
        }
        document::Link {
            rel: "icon",
            href: asset!("/assets/favicon.ico", AssetOptions::builder().with_hash_suffix(false)),
        }
        document::Link {
            rel: "apple-touch-icon",
            sizes: "180x180",
            href: asset!(
                "/assets/apple-touch-icon.png", AssetOptions::builder().with_hash_suffix(false)
            ),
        }
        document::Link {
            rel: "image/png",
            sizes: "192x192",
            href: asset!(
                "/assets/android-chrome-192x192.png", AssetOptions::builder()
                .with_hash_suffix(false)
            ),
        }
        document::Link {
            rel: "image/png",
            sizes: "512x512",
            href: asset!(
                "/assets/android-chrome-512x512.png", AssetOptions::builder()
                .with_hash_suffix(false)
            ),
        }

        // Stylesheets
        document::Link { rel: "stylesheet", href: asset!("/assets/main.css") }
        document::Link { rel: "stylesheet", href: asset!("/assets/tailwind.css") }
        document::Link { rel: "stylesheet", href: "https://cdn.jsdelivr.net/npm/daisyui@5" }
        document::Link {
            rel: "stylesheet",
            href: "https://cdn.jsdelivr.net/npm/daisyui@5/themes.css",
        }

        // Manifest
        document::Link {
            rel: "manifest",
            href: asset!("/assets/site.webmanifest", AssetOptions::builder().with_hash_suffix(false)),
        }

        body {
            match has_data() {
                Some(Some(true)) => rsx! {
                    // Show main app when data exists
                    Home {}
                },
                Some(Some(false)) => rsx! {
                    // Show landing/onboarding when no data exists
                    Landing {}
                },
                _ => rsx! {
                    // Loading state
                    div { class: "min-h-screen flex items-center justify-center",
                        span { class: "loading loading-spinner loading-lg text-primary" }
                    }
                },
            }
        }
    }
}
