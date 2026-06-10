use dioxus::prelude::*;
use dioxus_primitives::{
    dropdown_menu::{DropdownMenu, DropdownMenuContent, DropdownMenuItem, DropdownMenuTrigger},
    separator::Separator,
};

use crate::{Route, database::{use_db, get_workspaces, DatabaseMode}};
use crate::models::congregation::Congregation;
use dioxus_i18n::t;

// ── Context ───────────────────────────────────────────────────────────────────

/// Wraps the sidebar-open signal so it can be distinguished from other
/// `Signal<bool>` values that might also live in the context tree.
#[derive(Clone, Copy)]
pub struct SidebarCtx(pub Signal<bool>);

/// Read the sidebar open/close signal from any descendant component.
pub fn use_sidebar() -> Signal<bool> {
    use_context::<SidebarCtx>().0
}

// ── AppSidebar ────────────────────────────────────────────────────────────────

/// Full-height sidebar with congregation switcher, sectioned navigation and a
/// user footer.
///
/// Responsive behaviour:
/// - **Desktop (md+):** always visible as a static flex column.
/// - **Mobile:** `position: fixed`, slides in from the left when the
///   `SidebarCtx` signal is `true`. The [`AppLayout`] renders a backdrop that
///   sets the signal to `false` when tapped.
#[component]
pub fn AppSidebar() -> Element {
    let open = use_sidebar();

    // On mobile the sidebar is a fixed overlay that translates in/out.
    // On md+ it becomes a normal static flex child via md:relative / md:translate-x-0.
    let aside_class = if *open.read() {
        "fixed inset-y-0 left-0 z-30 flex flex-col w-64 h-full bg-white border-r border-gray-200 transition-transform duration-300 translate-x-0 md:relative md:z-auto"
    } else {
        "fixed inset-y-0 left-0 z-30 flex flex-col w-64 h-full bg-white border-r border-gray-200 transition-transform duration-300 -translate-x-full md:relative md:z-auto md:translate-x-0"
    };

    rsx! {
        aside { class: aside_class,
            CongregationSwitcher {}

            // ── Primary navigation ─────────────────────────────────────────
            nav { class: "flex-1 overflow-y-auto px-2 py-3",
                NavItem {
                    to: Route::AppDashboard {},
                    icon: "🏠",
                    label: t!("nav-dashboard"),
                }

                NavDivider {}

                // User section
                NavSectionLabel { label: t!("section-users") }
                NavItem {
                    to: Route::AppUsers {},
                    icon: "👥",
                    label: t!("nav-user-list"),
                }
                NavItem {
                    to: Route::AppFieldServiceReports {},
                    icon: "📊",
                    label: t!("nav-field-service-reports"),
                }
                NavItem {
                    to: Route::AppAbsences {},
                    icon: "📅",
                    label: t!("nav-absences"),
                }

                NavDivider {}

                // Ministry section
                NavSectionLabel { label: t!("section-ministry") }
                NavItem {
                    to: Route::AppPublicPreaching {},
                    icon: "📢",
                    label: t!("nav-public-preaching"),
                }
                NavItem {
                    to: Route::AppFieldServiceGroups {},
                    icon: "💼",
                    label: t!("nav-field-service-groups"),
                }
                NavItem {
                    to: Route::AppTerritory {},
                    icon: "🗺️",
                    label: t!("nav-territory"),
                }
                NavItem {
                    to: Route::AppFieldServiceMeetings {},
                    icon: "🤝",
                    label: t!("nav-field-service-meetings"),
                }

                NavDivider {}

                // Meetings section
                NavSectionLabel { label: t!("section-meetings") }
                NavItem {
                    to: Route::AppAttendants {},
                    icon: "🚪",
                    label: t!("nav-attendants"),
                }
                NavItem {
                    to: Route::AppAvPlatform {},
                    icon: "🎧",
                    label: t!("nav-av-platform"),
                }
                NavItem {
                    to: Route::AppCleaning {},
                    icon: "🧹",
                    label: t!("nav-cleaning"),
                }
                NavItem {
                    to: Route::AppWeekdayMeeting {},
                    icon: "📖",
                    label: t!("nav-weekday-meeting"),
                }
                NavItem {
                    to: Route::AppWeekendMeeting {},
                    icon: "🗣️",
                    label: t!("nav-weekend-meeting"),
                }

                NavDivider {}

                // Congregation section
                NavSectionLabel { label: t!("section-congregation") }
                NavItem {
                    to: Route::AppCongregationSettings {},
                    icon: "⚙️",
                    label: t!("nav-settings"),
                }
                NavItem {
                    to: Route::AppCongregationPermissions {
                    },
                    icon: "🔒",
                    label: t!("nav-permissions"),
                }
            }

            // ── User / disconnect footer ───────────────────────────────────
            UserMenu {}
        }
    }
}

// ── Congregation switcher ─────────────────────────────────────────────────────

/// Top-of-sidebar dropdown that shows the active congregation and allows
/// switching or adding a new one.
///
/// Congregation data will be loaded from the DB in a future iteration;
/// for now it uses a placeholder signal.



#[component]
fn CongregationSwitcher() -> Element {
    let mut db_state = use_db();
    let nav = use_navigator();

    let mut workspaces = use_resource({
        let db_state = db_state.clone();
        move || async move {
            let _trigger_reload = db_state.read().congregation_uid.clone();
            get_workspaces().await
        }
    });

    let current_uid = db_state.read().congregation_uid.clone();
    
    let active_name = if let Some(wks) = workspaces.read().as_ref() {
        if let Some(uid) = current_uid.as_ref() {
            wks.iter().find(|w| w.uid == *uid).map(|w| w.name.clone())
        } else {
            None
        }
    } else {
        None
    };

    let congregation_name = active_name.unwrap_or_else(|| "My Congregation".to_string());

    let initial = congregation_name
        .chars()
        .next()
        .map(|c| c.to_ascii_uppercase().to_string())
        .unwrap_or_else(|| "?".to_string());

    rsx! {
        div { class: "p-2 border-b border-gray-200",
            DropdownMenu { class: "relative",
                DropdownMenuTrigger { class: "w-full flex items-center gap-3 px-3 py-2 rounded-lg hover:bg-gray-100 active:bg-gray-200 transition-colors text-left",
                    // Avatar / initial
                    div { class: "flex items-center justify-center w-9 h-9 rounded-lg bg-primary-600 text-white text-sm font-bold shrink-0",
                        "{initial}"
                    }
                    // Name + role
                    div { class: "flex-1 min-w-0",
                        p { class: "text-sm font-semibold text-gray-900 truncate",
                            "{congregation_name}"
                        }
                        p { class: "text-xs text-gray-500", {t!("congregation-label")} }
                    }
                    span { class: "text-gray-400 text-xs shrink-0", "⌄" }
                }
                DropdownMenuContent { class: "absolute left-0 top-full mt-1 w-full bg-white border border-gray-200 rounded-xl shadow-lg overflow-hidden py-1 z-50",
                    div { class: "px-3 py-2 text-xs font-medium text-gray-400 uppercase tracking-wider border-b border-gray-100",
                        {t!("switch-congregation")}
                    }

                    if let Some(wks) = workspaces.read().as_ref() {
                        for (i , wk) in wks.iter().enumerate() {
                            DropdownMenuItem::<String> {
                                index: i + 1,
                                value: wk.uid.clone(),
                                class: "w-full flex items-center px-4 py-2 text-sm text-gray-700 hover:bg-gray-50 cursor-pointer",
                                on_select: {
                                    let iter_uid = current_uid.clone();
                                    move |value: String| {
                                        if iter_uid.as_ref() != Some(&value) {
                                            crate::database::ls_set("theo_active_uid", &value);
                                            // Leak the current DB before overwriting it so panic is prevented
                                            let mut state = db_state.write();
                                            if let Some(old) = state.db.take() {
                                                state.leaked_dbs.push(old);
                                            }
                                            let _ = document::eval("window.location.reload();");
                                        }
                                    }
                                },
                                div { class: "flex items-center gap-2 truncate",
                                    span { class: "text-lg shrink-0",
                                        {
                                            if wk.mode == crate::database::DatabaseMode::Offline {
                                                "💾 "
                                            } else {
                                                "☁️ "
                                            }
                                        }
                                    }
                                    span { class: "truncate font-medium text-gray-800",
                                        "{wk.name}"
                                    }
                                }
                            }
                        }
                    }

                    Separator { class: "h-px w-full bg-gray-100 my-1" }

                    DropdownMenuItem::<String> {
                        index: 0usize,
                        value: "new".to_string(),
                        class: "w-full cursor-pointer hover:bg-gray-50 flex items-center transition-colors",
                        on_select: move |_| {
                            nav.push(Route::AppNewCongregation {});
                        },
                        div { class: "flex items-center gap-3 px-4 py-2.5 text-sm text-primary-600 font-semibold",
                            span { "＋" }
                            span { {t!("sidebar-congregation-new")} }
                        }
                    }
                }
            }
        }
    }
}
/// Footer dropdown with account settings and disconnect.
#[component]
fn UserMenu() -> Element {
    let mut db = use_db();
    let crypto = crate::database::use_crypto();
    let nav = use_navigator();

    let mut current_user = use_resource(move || async move {
        if let Some(db_ref) = db.read().db.clone() {
            let crypto_ref = crypto.read().clone();
            
            let mut eval = document::eval("
                try { dioxus.send(localStorage.getItem('theo_my_user_id')); } 
                catch(e) { dioxus.send(null); }
            ");
            let user_id_str = eval.recv::<serde_json::Value>().await.ok().and_then(|v| {
                match v {
                    serde_json::Value::String(val) => Some(val),
                    _ => None,
                }
            });

            if let Some(id_str) = user_id_str {
                if let Ok(record_id) = surrealdb::types::RecordId::parse_simple(&id_str) {
                    if let Ok(Some(user)) = crate::models::user::User::get(&db_ref, &crypto_ref, record_id).await {
                        return Some(user);
                    }
                }
            }
            
            // Fallback to first user
            if let Ok(users) = crate::models::user::User::all(&db_ref, &crypto_ref).await {
                return users.into_iter().next();
            }
        }
        None
    });

    let (initial, name) = if let Some(Some(u)) = current_user.read().as_ref() {
        (
            u.first_name.chars().next().unwrap_or('?').to_uppercase().to_string(),
            u.first_name.clone()
        )
    } else {
        ("U".to_string(), t!("user-label").to_string())
    };

    rsx! {
        div { class: "p-2 border-t border-gray-200",
            DropdownMenu { class: "relative",
                DropdownMenuTrigger { class: "w-full flex items-center gap-3 px-3 py-2 rounded-lg hover:bg-gray-100 transition-colors text-left",
                    div { class: "flex items-center justify-center w-9 h-9 rounded-full bg-gray-200 text-gray-600 text-sm font-medium shrink-0",
                        "{initial}"
                    }
                    div { class: "flex-1 min-w-0",
                        p { class: "text-sm font-medium text-gray-900 truncate", "{name}" }
                        p { class: "text-xs text-gray-500 truncate", {t!("account-settings")} }
                    }
                    span { class: "text-gray-400 text-xs shrink-0", "⌄" }
                }
                // Opens upward so it doesn't overflow the viewport bottom
                DropdownMenuContent { class: "absolute left-0 bottom-full mb-1 w-full bg-white border border-gray-200 rounded-xl shadow-lg overflow-hidden py-1 z-50",
                    DropdownMenuItem::<String> {
                        index: 0usize,
                        value: "settings".to_string(),
                        class: "w-full cursor-pointer hover:bg-gray-50 flex items-center transition-colors",
                        on_select: move |_: String| {
                            nav.push(Route::AppUserSettings {});
                        },
                        div { class: "flex items-center gap-3 px-3 py-2.5 text-sm text-gray-700",
                            span { "⚙️" }
                            span { {t!("menu-settings")} }
                        }
                    }
                    DropdownMenuItem::<String> {
                        index: 1usize,
                        value: "disconnect".to_string(),
                        class: "w-full cursor-pointer hover:bg-red-50 flex items-center transition-colors",
                        on_select: move |_: String| {
                            db.write().db = None;
                            nav.push(Route::Landing {});
                        },
                        div { class: "flex items-center gap-3 px-3 py-2.5 text-sm text-red-600 font-medium",
                            span { "⎋" }
                            span { {t!("menu-disconnect")} }
                        }
                    }
                }
            }
        }
    }
}

// ── Nav primitives ────────────────────────────────────────────────────────────

/// Section heading inside the nav — uppercase, muted, small.
#[component]
fn NavSectionLabel(label: String) -> Element {
    rsx! {
        p { class: "px-3 pt-4 pb-1.5 text-xs font-semibold text-gray-400 uppercase tracking-wider select-none",
            "{label}"
        }
    }
}

/// Thin horizontal rule between nav groups.
#[component]
fn NavDivider() -> Element {
    rsx! {
        Separator {
            class: "h-px bg-gray-100 my-2 mx-1",
            horizontal: true,
            decorative: true,
        }
    }
}

/// Single navigation link. Highlights when the current route matches `to`.
/// On mobile, clicking any link also closes the sidebar via [`use_sidebar`].
#[component]
fn NavItem(to: Route, icon: String, label: String) -> Element {
    let current = use_route::<Route>();
    let mut sidebar_open = use_sidebar();

    let is_active = current == to;

    let class = if is_active {
        "flex items-center gap-3 px-3 py-2 rounded-lg bg-primary-50 text-primary-700 font-semibold text-sm w-full"
    } else {
        "flex items-center gap-3 px-3 py-2 rounded-lg text-gray-600 hover:bg-gray-100 hover:text-gray-900 transition-colors text-sm w-full"
    };

    rsx! {
        Link {
            to,
            class,
            // Close the mobile sidebar when navigating
            onclick: move |_| sidebar_open.set(false),
            span { class: "w-5 text-center text-base shrink-0", "{icon}" }
            span { class: "flex-1 truncate", "{label}" }
        }
    }
}
