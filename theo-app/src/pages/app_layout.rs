use crate::state::use_app_state;
use crate::{nav, route_for, Redirect, Route};
use dioxus::prelude::*;
use theo_ui::*;

/// Active section and page for the shell.
fn active_of(route: &Route) -> (String, Option<String>) {
    match route {
        Route::Me {} => ("me".into(), None),
        Route::UserList {} | Route::UserNew {} | Route::UserDetail { .. } => ("users".into(), Some("all".into())),
        Route::ReportsPage {} => ("ministry".into(), Some("reports".into())),
        Route::PermissionsPage {} | Route::PermissionsDetail { .. } => ("users".into(), Some("permissions".into())),
        Route::PrivilegesPage {} => ("users".into(), Some("privileges".into())),
        Route::FamilyDetail { .. } => ("users".into(), Some("all".into())),
        Route::AbsencesPage {} => ("users".into(), Some("absences".into())),
        Route::CustomTasksPage {} => ("departments".into(), Some("custom-tasks".into())),
        Route::MovedAway {} => ("users".into(), Some("moved-away".into())),
        Route::GroupList {} | Route::GroupNew {} | Route::GroupDetail { .. } => ("ministry".into(), Some("groups".into())),
        Route::SectionHub { section } => (section.clone(), None),
        Route::SectionPage { section, page } => (section.clone(), Some(page.clone())),
        _ => ("home".into(), None),
    }
}

/// Layout of every `/app` page. Guards: signed in, and a congregation unlocked.
#[component]
pub fn AppLayout() -> Element {
    let mut state = use_app_state();
    let navigator = navigator();
    let route = use_route::<Route>();

    if state.session.read().is_none() {
        return rsx! { Redirect { to: Route::Login {} } };
    }
    if state.unlocked.read().is_none() {
        return rsx! { Redirect { to: Route::Unlock {} } };
    }

    let display = state.account().map(|a| a.display_name).unwrap_or_default();
    let congregation = state.active_congregation();
    let (active, active_page) = active_of(&route);
    let workspaces: Vec<SwitcherItem> = state
        .my_congregations()
        .into_iter()
        .map(|c| SwitcherItem { id: c.id.clone(), name: c.name, kind_icon: c.mode.icon(), kind_label: c.mode.label() })
        .collect();

    rsx! {
        AppShell {
            brand: "Theo Manager",
            nav: nav::sections(),
            account: NavEntry::new("me", display, Icon::User),
            account_detail: congregation.as_ref().map(|c| c.mode.label()).unwrap_or_default(),
            active,
            active_page,
            on_navigate: move |id: String| { navigator.push(route_for(&id)); },
            workspaces,
            active_workspace: state.unlocked.read().clone(),
            on_switch_workspace: move |id: String| {
                if state.unlocked.read().as_deref() != Some(id.as_str()) {
                    // Each congregation has its own keys: switching locks the current one.
                    state.unlocked.set(None);
                    state.preselect.set(Some(id));
                    navigator.push(Route::Unlock {});
                }
            },
            on_create_workspace: move |_| { navigator.push(Route::NewCongregation {}); },
            on_workspace_settings: move |_| { navigator.push(Route::SettingsPage {}); },
            transition_key: route.to_string(),
            Outlet::<Route> {}
        }
    }
}
