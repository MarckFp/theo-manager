use dioxus::prelude::*;
use dioxus_i18n::t;

#[component]
pub fn AppPublicTalks() -> Element {
    rsx! {
        div { class: "space-y-6 w-full",
            h1 { class: "text-2xl font-bold text-gray-900", {t!("page-public-talks")} }
            div { class: "bg-white rounded-xl border border-gray-200",
                div { class: "px-6 py-12 text-center text-gray-400",
                    p { class: "text-4xl mb-3", "🎤" }
                    p { class: "font-medium text-gray-600", {t!("empty-public-talks-title")} }
                    p { class: "text-sm mt-1", {t!("empty-public-talks-desc")} }
                }
            }
        }
    }
}
