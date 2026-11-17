//! Theo Manager app: routes, screens and wiring.
//!
//! UI rule (REQUIREMENTS.md 8.4, R-431): this crate never writes markup or styles. Screens are
//! built only from `theo-ui` components. `tests/ui_rules.rs` enforces it.

mod export;
mod mock;
mod nav;
mod pages;
mod password;
mod people;
mod platform;
mod prefs;
mod state;

use dioxus::prelude::*;
use dioxus_i18n::prelude::*;
use pages::*;
use state::AppState;
use theo_ui::ThemeRoot;
use unic_langid::LanguageIdentifier;

fn main() {
    dioxus::launch(App);
}

/// All routes. Logged-in screens live under `/app` inside `AppLayout`.
#[derive(Routable, Clone, Debug, PartialEq)]
#[rustfmt::skip]
#[allow(unreachable_code)] // emitted inside the Routable derive (dioxus-router 0.7), not our code
pub enum Route {
    /// Landing page on the web; other platforms go straight to log in (R-063).
    #[route("/")]
    Landing {},
    #[route("/login")]
    Login {},
    #[route("/signup")]
    Signup {},
    #[route("/unlock")]
    Unlock {},
    #[route("/congregations/new")]
    NewCongregation {},
    #[nest("/app")]
        #[layout(AppLayout)]
            #[route("/")]
            Dashboard {},
            #[route("/me")]
            Me {},
            /// Users › All users: list, new person, one person (docs/model/m01-people.md).
            #[route("/users/all")]
            UserList {},
            #[route("/users/all/new")]
            UserNew {},
            #[route("/users/all/:id")]
            UserDetail { id: u32 },
            #[route("/users/moved-away")]
            MovedAway {},
            /// Users › Field service groups (docs/model/m09-field-service-groups.md).
            #[route("/users/groups")]
            GroupList {},
            #[route("/users/groups/new")]
            GroupNew {},
            #[route("/users/groups/:id")]
            GroupDetail { id: u32 },
            /// Section hub: big tiles for the section's pages.
            #[route("/:section")]
            SectionHub { section: String },
            /// A page of a section.
            #[route("/:section/:page")]
            SectionPage { section: String, page: String },
        #[end_layout]
    #[end_nest]
    #[route("/:..segments")]
    NotFound { segments: Vec<String> },
}

/// The English Fluent file. Every language uses it until translations exist.
const EN_FTL: &str = include_str!("../locales/en.ftl");

fn lang_id(code: &str) -> LanguageIdentifier {
    code.parse().unwrap_or_else(|_| unic_langid::langid!("en"))
}

#[component]
fn App() -> Element {
    let state = use_context_provider(AppState::new);
    let mut i18n = use_init_i18n(|| {
        prefs::LANGUAGES.iter().fold(
            I18nConfig::new(lang_id(&prefs::detected_language())).with_fallback(unic_langid::langid!("en")),
            |cfg, (code, _)| cfg.with_locale((lang_id(code), EN_FTL)),
        )
    });
    // Follow the effective language (user override, else congregation default, else device).
    use_effect(move || {
        let lang = state.preferences().language;
        i18n.set_language(lang_id(&lang));
        let shown = if prefs::TRANSLATED.contains(&lang.as_str()) { lang.as_str() } else { "en" };
        platform::set_document_language(shown);
    });
    rsx! {
        ThemeRoot { theme: (state.theme)(),
            Router::<Route> {}
        }
    }
}

/// Navigates to `to` after the first render (used by route guards). Renders nothing.
#[component]
pub fn Redirect(to: Route) -> Element {
    let nav = navigator();
    use_effect(move || {
        nav.replace(to.clone());
    });
    rsx! {}
}

/// Where a signed-in user should go next.
pub fn next_after_sign_in(state: &AppState) -> Route {
    if state.my_congregations().is_empty() { Route::NewCongregation {} } else { Route::Unlock {} }
}

/// Route for a navigation id from the shell: `"home"`, `"me"`, `"users"` or `"users/reports"`.
pub fn route_for(id: &str) -> Route {
    match id.split_once('/') {
        Some(("users", "all")) => Route::UserList {},
        Some(("users", "moved-away")) => Route::MovedAway {},
        Some(("users", "groups")) => Route::GroupList {},
        Some((section, page)) => Route::SectionPage { section: section.into(), page: page.into() },
        None => match id {
            "home" => Route::Dashboard {},
            "me" => Route::Me {},
            section => Route::SectionHub { section: section.into() },
        },
    }
}
