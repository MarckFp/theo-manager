use dioxus::prelude::*;
use web_sys::window;

mod database;
mod views;
mod components;

use views::{Landing, Home};
use database::models::congregation::Congregation;

fn main() {
    dioxus::launch(App);
}

#[component]
fn App() -> Element {
    // Check if we have any congregation data
    let has_data = use_resource(move || async move {
        match Congregation::all().await {
            Ok(congregations) => !congregations.is_empty(),
            Err(_) => false,
        }
    });

    let document = window().unwrap().document().unwrap();
    let html = document.document_element().unwrap();

    html.set_attribute("lang", "en").unwrap();
    html.set_attribute("data-theme", "aqua").unwrap();

    rsx! {
        // Icons
        document::Link { rel: "icon", type: "image/x-icon", href: asset!("/assets/favicon.ico") }
        document::Link { rel: "icon", type: "image/png", sizes: "32x32", href: asset!("/assets/favicon-32x32.png") }
        document::Link { rel: "icon", type: "image/png", sizes: "16x16", href: asset!("/assets/favicon-16x16.png") }
        document::Link { rel: "icon", href: asset!("/assets/favicon.ico") }
        document::Link { rel: "apple-touch-icon", sizes: "180x180" ,href: asset!("/assets/apple-touch-icon.png") }

        // Stylesheets
        document::Link { rel: "stylesheet", href: asset!("/assets/main.css") }
        document::Link { rel: "stylesheet", href: asset!("/assets/tailwind.css") }

        // Manifest
        document::Link { rel: "manifest", href: asset!("/assets/site.webmanifest") }

        body {
            match has_data() {
                Some(true) => rsx! {
                    // Show main app when data exists
                    Home {}
                },
                Some(false) => rsx! {
                    // Show landing/onboarding when no data exists
                    Landing {}
                },
                None => rsx! {
                    // Loading state
                    div { class: "min-h-screen flex items-center justify-center",
                        span { class: "loading loading-spinner loading-lg text-primary" }
                    }
                }
            }
        }
    }
}
