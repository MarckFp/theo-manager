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
        AppFieldServiceReports, AppLayout, AppPublicPreaching, AppTerritory, AppUsers,
        AppWeekdayMeeting, AppWeekendMeeting, AppNewCongregation, AppUserSettings, AppUserDetail
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

        // Congregation section
        #[route("/app/congregation/settings")]
        AppCongregationSettings {},
        #[route("/app/congregation/permissions")]
        AppCongregationPermissions {},
        
        #[route("/app/congregation/new")]
        AppNewCongregation {},

        #[route("/app/user/settings")]
        AppUserSettings {},
    #[end_layout]

    // ── Catch-all 404 ──────────────────────────────────────────────────────
    #[route("/:..segments")]
    NotFound { segments: Vec<String> },
}

const FAVICON: Asset = asset!("/assets/favicon.ico");
const MAIN_CSS: Asset = asset!("/assets/main.css");
const TAILWIND_CSS: Asset = asset!("/assets/tailwind.css");

fn main() {
    dioxus::launch(App);
}

#[component]
fn App() -> Element {
    use_init_i18n(i18n::init_config);
    rsx! {
        database::DatabaseProvider {
            document::Link { rel: "icon", href: FAVICON }
            document::Link { rel: "stylesheet", href: MAIN_CSS }
            document::Link { rel: "stylesheet", href: TAILWIND_CSS }
            Router::<Route> {}
        }
    }
}
