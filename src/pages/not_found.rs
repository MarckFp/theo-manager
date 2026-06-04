use dioxus::prelude::*;
use dioxus_i18n::t;

use crate::Route;

#[component]
pub fn NotFound(segments: Vec<String>) -> Element {
    let path = segments.join("/");
    rsx! {
        div { class: "min-h-screen flex items-center justify-center bg-gray-50",
            div { class: "text-center",
                p { class: "text-8xl font-extrabold text-gray-200 select-none", "404" }
                h1 { class: "mt-4 text-2xl font-semibold text-gray-800", {t!("not-found-title")} }
                p { class: "mt-2 text-gray-500 text-sm", "/{path}" }
                Link {
                    to: Route::Landing {},
                    class: "mt-6 inline-block px-6 py-3 bg-blue-600 text-white rounded-lg hover:bg-blue-700 font-medium transition-colors",
                    {t!("not-found-btn")}
                }
            }
        }
    }
}
