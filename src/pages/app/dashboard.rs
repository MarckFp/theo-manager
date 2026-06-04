use dioxus::prelude::*;
use dioxus_i18n::t;

use crate::Route;

#[component]
pub fn AppDashboard() -> Element {
    rsx! {
        div { class: "space-y-6 w-full",
            h1 { class: "text-2xl font-bold text-gray-900", {t!("page-dashboard")} }

            div { class: "grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-4",
                DashCard {
                    to: Route::AppUsers {},
                    icon: "👤",
                    title: t!("dash-users"),
                    description: t!("dash-users-desc"),
                }
                DashCard {
                    to: Route::AppAbsences {},
                    icon: "📅",
                    title: t!("dash-absences"),
                    description: t!("dash-absences-desc"),
                }
            }
        }
    }
}

/// Clickable summary card that navigates to a section of the app.
#[component]
fn DashCard(to: Route, icon: String, title: String, description: String) -> Element {
    rsx! {
        Link {
            to,
            class: "block bg-white rounded-xl border border-gray-200 p-5 hover:border-blue-300 hover:shadow-sm transition-all",
            div { class: "flex items-start gap-4",
                span { class: "text-3xl", "{icon}" }
                div {
                    p { class: "font-semibold text-gray-900", "{title}" }
                    p { class: "text-sm text-gray-500 mt-0.5", "{description}" }
                }
            }
        }
    }
}
