use dioxus::prelude::*;
use dioxus_i18n::t;

#[component]
pub fn AppFieldServiceReports() -> Element {
    rsx! {
        div { class: "space-y-6 w-full",
            div { class: "flex items-center justify-between",
                h1 { class: "text-2xl font-bold text-gray-900", {t!("page-field-service-reports")} }
                button { class: "px-4 py-2 bg-primary-600 text-white rounded-lg hover:bg-primary-700 text-sm font-medium transition-colors",
                    {t!("btn-add-report")}
                }
            }
            div { class: "bg-white rounded-xl border border-gray-200",
                div { class: "px-6 py-12 text-center text-gray-400",
                    p { class: "text-4xl mb-3", "📊" }
                    p { class: "font-medium text-gray-600", {t!("empty-reports-title")} }
                    p { class: "text-sm mt-1", {t!("empty-reports-desc")} }
                }
            }
        }
    }
}
