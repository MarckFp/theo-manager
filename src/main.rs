use dioxus::prelude::*;

const FAVICON: Asset = asset!("/assets/favicon.ico");
const MAIN_CSS: Asset = asset!("/assets/main.css");
const TAILWIND_CSS: Asset = asset!("/assets/tailwind.css");

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

    rsx! {
        document::Link { rel: "icon", href: FAVICON }
        document::Link { rel: "stylesheet", href: MAIN_CSS }
        document::Link { rel: "stylesheet", href: TAILWIND_CSS }
        
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
