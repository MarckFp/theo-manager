use dioxus::prelude::*;
use dioxus_i18n::t;

#[component]
pub fn AppAbsences() -> Element {
    rsx! {
        div { class: "space-y-6 w-full",
            // Page header
            div { class: "flex items-center justify-between",
                h1 { class: "text-2xl font-bold text-gray-900", {t!("page-absences")} }
                button { class: "px-4 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700 text-sm font-medium transition-colors",
                    // TODO: open record-absence modal / form
                    {t!("btn-record-absence")}
                }
            }

            // Content area
            div { class: "bg-white rounded-xl border border-gray-200",
                div { class: "px-6 py-12 text-center text-gray-400",
                    p { class: "text-4xl mb-3", "📅" }
                    p { class: "font-medium text-gray-600", {t!("empty-absences-title")} }
                    p { class: "text-sm mt-1", {t!("empty-absences-desc")} }
                }
            }
        }
    }
}
