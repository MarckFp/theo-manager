use dioxus::prelude::*;
use dioxus_i18n::prelude::*;

mod components;
mod crypto;
mod database;
mod i18n;
mod models;
mod pages;

use pages::{
    Landing, NotFound,
    app::{
        AppAbsences, AppAttendants, AppAvPlatform, AppCleaning, AppCongregationPermissions,
        AppCongregationSettings, AppDashboard, AppFieldServiceGroups, AppFieldServiceMeetings,
        AppFieldServiceReports, AppLayout, AppMaintenance, AppPrivileges, AppPublicPreaching,
        AppPublicTalks, AppTerritory, AppUsers, AppWeekdayMeeting, AppWeekendMeeting,
        AppNewCongregation, AppUserSettings, AppUserDetail, AppEvents, AppCustom
    },
};

/// All application routes.
///
/// Two logical zones:
/// - `/`         → [`Landing`]: first-time setup / mode selector.
/// - `/app/**`   → authenticated zone wrapped by [`AppLayout`], which
///                  redirects to `/` when no database connection is active.
#[derive(Debug, Clone, Routable, PartialEq)]
#[rustfmt::skip]
pub enum Route {
    // ── Unauthenticated ────────────────────────────────────────────────────
    #[route("/")]
    Landing {},

    // ── Authenticated app ──────────────────────────────────────────────────
    #[layout(AppLayout)]
        #[route("/app")]
        AppDashboard {},

        // Users section
        #[route("/app/users")]
        AppUsers {},
        #[route("/app/user/settings")]
        AppUserSettings {},
        #[route("/app/user/:id")]
        AppUserDetail { id: String },
        #[route("/app/field-service-reports")]
        AppFieldServiceReports {},
        #[route("/app/absences")]
        AppAbsences {},

        // Ministry section
        #[route("/app/public-preaching")]
        AppPublicPreaching {},
        #[route("/app/field-service-groups")]
        AppFieldServiceGroups {},
        #[route("/app/territory")]
        AppTerritory {},
        #[route("/app/ministry/field-service-meetings")]
        AppFieldServiceMeetings {},

        // Meetings section
        #[route("/app/meetings/attendants")]
        AppAttendants {},
        #[route("/app/meetings/av-platform")]
        AppAvPlatform {},
        #[route("/app/meetings/cleaning")]
        AppCleaning {},
        #[route("/app/meetings/weekday")]
        AppWeekdayMeeting {},
        #[route("/app/meetings/weekend")]
        AppWeekendMeeting {},
        #[route("/app/meetings/public-talks")]
        AppPublicTalks {},

        // Congregation section
        #[route("/app/congregation/settings")]
        AppCongregationSettings {},
        #[route("/app/congregation/permissions")]
        AppCongregationPermissions {},
        #[route("/app/congregation/privileges")]
        AppPrivileges {},
        #[route("/app/congregation/maintenance")]
        AppMaintenance {},
        #[route("/app/congregation/events")]
        AppEvents {},
        #[route("/app/congregation/custom")]
        AppCustom {},
        
        #[route("/app/congregation/new")]
        AppNewCongregation {},
    #[end_layout]

    // ── Catch-all 404 ──────────────────────────────────────────────────────
    #[route("/:..segments")]
    NotFound { segments: Vec<String> },
}

const FAVICON: Asset = asset!("/assets/favicon.ico");
const MAIN_CSS: Asset = asset!("/assets/main.css");
const TAILWIND_CSS: Asset = asset!("/assets/tailwind.css");
const MANIFEST: Asset = asset!("/assets/manifest.json");
// Service workers must be served at a fixed path (no content-hash suffix).
const SW_JS: Asset = asset!("/assets/sw.js", AssetOptions::builder().with_hash_suffix(false));

fn main() {
    #[cfg(target_arch = "wasm32")]
    console_error_panic_hook::set_once();
    dioxus::launch(App);
}

#[component]
fn App() -> Element {
    use_init_i18n(i18n::init_config);

    // Register the service worker after the client-side hydration.
    // sw.js is copied to the root of the output by deploy.yml so it can
    // control the full origin — including the /theo-manager/ GitHub Pages path.
    use_effect(|| {
        let _ = document::eval(r#"
            if ('serviceWorker' in navigator) {
                navigator.serviceWorker.register('/sw.js')
                    .catch(function(e) { console.warn('[SW] Registration failed:', e); });
            }
        "#);
    });

    rsx! {
        database::DatabaseProvider {
            document::Link { rel: "icon", href: FAVICON }
            document::Link { rel: "manifest", href: MANIFEST }
            document::Link { rel: "stylesheet", href: MAIN_CSS }
            document::Link { rel: "stylesheet", href: TAILWIND_CSS }
            Router::<Route> {}
        }
    }
}
