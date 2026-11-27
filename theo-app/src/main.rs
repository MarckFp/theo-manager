//! Theo Manager app: routes, screens and wiring.
//!
//! UI rule (REQUIREMENTS.md 8.4, R-431): this crate never writes markup or styles. Screens are
//! built only from `theo-ui` components. `tests/ui_rules.rs` enforces it.

mod assignments;
mod backup;
mod cloud;
mod export;
mod local;
mod mock;
mod nav;
mod pages;
mod password;
mod people;
mod platform;
mod state;
mod territories;
mod territory_card;
mod widgets;
// The domain lives in theo-core; these keep the `crate::module` paths the screens use.
pub use theo_core::{absences, access, attendance, events, midweek, ministry, prefs, privileges, public_witnessing, reports, service_meetings, tasks};
mod workbook;

use dioxus::prelude::*;
use dioxus_i18n::prelude::*;
use pages::*;
use state::AppState;
use theo_ui::{AuthLayout, Cluster, Justify, Spinner, ThemeRoot};
use unic_langid::LanguageIdentifier;

fn main() {
    prefs::set_device_locale(&sys_locale::get_locale().unwrap_or_default());
    // Before the router starts: it drops the fragment, where invite links keep their token.
    platform::capture_location_fragment();
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
    /// Join a Cloud congregation with an invite link (5.6); the invite is in the fragment.
    #[route("/join")]
    Join {},
    #[route("/congregations/new")]
    NewCongregation {},
    #[nest("/app")]
        #[layout(AppLayout)]
            #[route("/")]
            Dashboard {},
            #[route("/me")]
            Me {},
            /// Me › My ministry: daily log and sending the report (src/ministry.rs).
            #[route("/me/ministry")]
            MyMinistryPage {},
            /// Settings of the open congregation (cog in the congregation switcher).
            #[route("/settings")]
            SettingsPage {},
            /// Everything assigned to me (from the home widget "My assignments").
            #[route("/me/assignments")]
            MyAssignmentsPage {},
            /// Users › All users: list, new person, one person (docs/model/m01-people.md).
            #[route("/users/all")]
            UserList {},
            #[route("/users/all/new")]
            UserNew {},
            #[route("/users/all/:id")]
            UserDetail { id: u32 },
            #[route("/users/moved-away")]
            MovedAway {},
            /// Users › Permissions (docs/model/m02-users-and-access.md).
            #[route("/users/permissions")]
            PermissionsPage {},
            #[route("/users/permissions/:id")]
            PermissionsDetail { id: u32 },
            /// Users › Privileges (docs/model/m01-people.md, "Privileges").
            #[route("/users/privileges")]
            PrivilegesPage {},
            /// Users › Absences (docs/model/m01-people.md, "Absences").
            #[route("/users/absences")]
            AbsencesPage {},
            /// A family, opened from the Families tab of All users (docs/model/m01-people.md, "Family").
            #[route("/users/families/:id")]
            FamilyDetail { id: u32 },
            /// Meetings › Events (docs/model/m12-events.md).
            #[route("/meetings/events")]
            EventsPage {},
            /// Meetings › Meeting attendance (docs/model/m11-meeting-attendance.md).
            #[route("/meetings/attendance")]
            AttendancePage {},
            /// One month of attendance, `month` as `YYYY-MM`.
            #[route("/meetings/attendance/:month")]
            AttendanceMonth { month: String },
            /// Meetings › Midweek meeting (docs/model/m03-midweek-meeting.md).
            #[route("/meetings/midweek")]
            MidweekPage {},
            /// Ministry › Field service reports (docs/model/m10-field-service-reports.md).
            #[route("/ministry/reports")]
            ReportsPage {},
            /// Ministry › Public witnessing (docs/model/m16-public-witnessing.md).
            #[route("/ministry/witnessing")]
            WitnessingPage {},
            #[route("/ministry/witnessing/presets")]
            WitnessingPresetsPage {},
            /// Ministry › Meetings for field service (docs/model/m15-field-service-meetings.md).
            #[route("/ministry/service-meetings")]
            ServiceMeetingsPage {},
            #[route("/ministry/service-meetings/presets")]
            ServicePresetsPage {},
            /// Ministry › Territories (docs/model/m07-territories.md).
            #[route("/ministry/territories")]
            TerritoriesPage {},
            #[route("/ministry/territories/new")]
            TerritoryNew {},
            #[route("/ministry/territories/settings")]
            TerritorySettingsPage {},
            #[route("/ministry/territories/:id")]
            TerritoryDetail { id: u32 },
            #[route("/ministry/territories/:id/edit")]
            TerritoryEdit { id: u32 },
            /// Ministry › Field service groups (docs/model/m09-field-service-groups.md).
            #[route("/ministry/groups")]
            GroupList {},
            #[route("/ministry/groups/new")]
            GroupNew {},
            #[route("/ministry/groups/:id")]
            GroupDetail { id: u32 },
            /// Departments › Custom tasks (docs/model/m01-people.md, "Custom tasks").
            #[route("/departments/custom-tasks")]
            CustomTasksPage {},
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

/// The app icon (the only icon source; platform icons are generated from it: scripts/icons.sh).
/// Through `asset!` the link gets the base path, so the favicon also works on GitHub Pages.
const FAVICON: Asset = asset!("/icons/icon.svg");

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
        document::Link { rel: "icon", r#type: "image/svg+xml", href: FAVICON }
        ThemeRoot { theme: (state.theme)(),
            // *Stay unlocked* reopens the congregation after a reload before any screen decides
            // where to go (otherwise the router would send you to log in first).
            if (state.restoring)() {
                AuthLayout { title: "Opening your congregation",
                    Cluster { justify: Justify::Center, Spinner {} }
                }
            } else {
                Router::<Route> {}
            }
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
        Some(("users", "absences")) => Route::AbsencesPage {},
        Some(("departments", "custom-tasks")) => Route::CustomTasksPage {},
        Some(("users", "privileges")) => Route::PrivilegesPage {},
        Some(("users", "permissions")) => Route::PermissionsPage {},
        Some(("ministry", "groups")) => Route::GroupList {},
        Some(("ministry", "reports")) => Route::ReportsPage {},
        Some(("ministry", "service-meetings")) => Route::ServiceMeetingsPage {},
        Some(("ministry", "witnessing")) => Route::WitnessingPage {},
        Some(("ministry", "territories")) => Route::TerritoriesPage {},
        Some(("meetings", "attendance")) => Route::AttendancePage {},
        Some(("meetings", "events")) => Route::EventsPage {},
        Some(("meetings", "midweek")) => Route::MidweekPage {},
        Some((section, page)) => Route::SectionPage { section: section.into(), page: page.into() },
        None => match id {
            "home" => Route::Dashboard {},
            "me" => Route::Me {},
            "settings" => Route::SettingsPage {},
            section => Route::SectionHub { section: section.into() },
        },
    }
}
