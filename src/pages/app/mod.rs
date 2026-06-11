pub mod absences;
pub mod attendants;
pub mod av_platform;
pub mod cleaning;
pub mod congregation_permissions;
pub mod congregation_settings;
pub mod custom;
pub mod dashboard;
pub mod events;
pub mod field_service_groups;
pub mod field_service_meetings;
pub mod field_service_reports;
pub mod maintenance;
pub mod privileges;
pub mod public_preaching;
pub mod public_talks;
pub mod territory;
pub mod user;
pub mod user_detail;
pub mod weekday_meeting;
pub mod weekend_meeting;
pub mod new_congregation;
pub mod user_settings;

pub use absences::AppAbsences;
pub use attendants::AppAttendants;
pub use av_platform::AppAvPlatform;
pub use cleaning::AppCleaning;
pub use congregation_permissions::AppCongregationPermissions;
pub use congregation_settings::AppCongregationSettings;
pub use dashboard::AppDashboard;
pub use field_service_groups::AppFieldServiceGroups;
pub use field_service_meetings::AppFieldServiceMeetings;
pub use field_service_reports::AppFieldServiceReports;
pub use maintenance::AppMaintenance;
pub use custom::AppCustom;
pub use events::AppEvents;
pub use privileges::AppPrivileges;
pub use public_preaching::AppPublicPreaching;
pub use public_talks::AppPublicTalks;
pub use territory::AppTerritory;
pub use user::AppUsers;
pub use user_detail::AppUserDetail;
pub use weekday_meeting::AppWeekdayMeeting;
pub use weekend_meeting::AppWeekendMeeting;
pub use new_congregation::AppNewCongregation;
pub use user_settings::AppUserSettings;

use dioxus::prelude::*;

use crate::{
    Route,
    components::sidebar::{AppSidebar, SidebarCtx},
    database::{use_db, use_crypto},
};
use crate::models::congregation::{Congregation, Theme, AccentColor};
use crate::pages::app::user_settings::load_prefs;
use dioxus_i18n::{prelude::i18n, t, unic_langid::LanguageIdentifier};

/// Authenticated app shell.
///
/// Responsibilities:
/// 1. **Route guard** — redirects to `/` when `AppDatabase.db` is `None`.
/// 2. **Sidebar context** — provides [`SidebarCtx`] so any descendant can
///    read or toggle the mobile sidebar open state.
/// 3. **Responsive layout** — sidebar is a fixed overlay on mobile, static
///    flex column on desktop (md+).
#[component]
pub fn AppLayout() -> Element {
    let db = use_db();
    let nav = use_navigator();
    let crypto = use_crypto();

    let congregation = use_resource(move || async move {
        if let Some(db_ref) = db.read().db.clone() {
            let crypto_ref = crypto.read().clone();
            if let Ok(congregations) = Congregation::all(&db_ref, &crypto_ref).await {
                return congregations.into_iter().next();
            }
        }
        None
    });

    // Provide the congregation resource as context so child pages can restart it.
    use_context_provider(|| congregation);

    use_effect(move || {
        if let Some(Some(c)) = congregation.read().as_ref() {
            let theme_str = match c.theme {
                Theme::Dark => "dark",
                _ => "light",
            };
            let accent_str = match c.accent_color {
                AccentColor::Green => "Green",
                AccentColor::Purple => "Purple",
                AccentColor::Rose => "Rose",
                AccentColor::Amber => "Amber",
                _ => "Blue",
            };
            let js = format!(
                "document.body.setAttribute('data-theme', '{}'); document.body.setAttribute('data-accent', '{}');",
                theme_str, accent_str
            );
            let _ = document::eval(&js);
        }
    });

    // Restore user prefs (theme/accent/language overrides) from localStorage.
    {
        let uid = db.read().congregation_uid.clone().unwrap_or_default();
        use_effect(move || {
            let uid = uid.clone();
            let db_opt = db.read().db.clone();
            spawn(async move {
                let prefs = load_prefs(&uid, db_opt).await;
                // Apply theme/accent overrides.
                crate::pages::app::user_settings::apply_prefs_to_body(
                    &prefs,
                    congregation.read().as_ref().and_then(|c| c.as_ref()),
                );
                // Apply language override.
                let lang = prefs.language.as_deref().filter(|s| !s.is_empty());
                if let Some(lang_str) = lang {
                    if let Ok(lang_id) = LanguageIdentifier::from_bytes(lang_str.as_bytes()) {
                        i18n().set_language(lang_id);
                    }
                }
            });
        });
    }

    // Sidebar open state — provided as context so AppSidebar and NavItem can
    // both read/write it without prop drilling.
    let mut sidebar_open = use_signal(|| false);
    use_context_provider(|| SidebarCtx(sidebar_open));

    // Guard: redirect to landing whenever the DB connection is lost.
    use_effect(move || {
        if db.read().db.is_none() {
            nav.push(Route::Landing {});
        }
    });

    // Avoid a flash of protected content while the redirect is in-flight.
    if db.read().db.is_none() {
        return rsx! {};
    }

    rsx! {
        div { class: "flex h-screen overflow-hidden bg-gray-50",
            // ── Mobile backdrop ────────────────────────────────────────────
            // Shown only on small screens when the sidebar is open.
            // Tapping it closes the sidebar.
            if *sidebar_open.read() {
                div {
                    class: "fixed inset-0 z-20 bg-black/50 md:hidden",
                    onclick: move |_| sidebar_open.set(false),
                }
            }

            // ── Sidebar ────────────────────────────────────────────────────
            AppSidebar {}

            // ── Main content column ────────────────────────────────────────
            div { class: "flex flex-col flex-1 min-w-0 overflow-hidden",
                // Top bar visible only on mobile — provides the hamburger toggle
                // and app name when the sidebar is hidden.
                header { class: "flex items-center gap-3 h-14 px-4 border-b border-gray-200 bg-white shrink-0 md:hidden",
                    // Hamburger button
                    button {
                        class: "p-2 rounded-lg hover:bg-gray-100 text-gray-600 transition-colors",
                        aria_label: t!("nav-toggle"),
                        onclick: move |_| {
                            let current = *sidebar_open.read();
                            sidebar_open.set(!current);
                        },
                        div { class: "flex flex-col gap-1.5 w-5",
                            div { class: "h-0.5 bg-current rounded-full" }
                            div { class: "h-0.5 bg-current rounded-full" }
                            div { class: "h-0.5 bg-current rounded-full" }
                        }
                    }
                    span { class: "font-semibold text-gray-900", {t!("app-name")} }
                }

                // Scrollable page content area
                main { class: "flex-1 overflow-y-auto p-4 sm:p-6", Outlet::<Route> {} }
            }
        }
    }
}
