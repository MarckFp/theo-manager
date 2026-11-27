//! Screens. Each one is built from `theo-ui` page templates and components only.

mod absences;
mod app_layout;
mod attendance;
mod auth;
mod dashboard;
mod events;
mod families;
mod groups;
mod join;
mod landing;
mod midweek;
mod ministry;
mod my_assignments;
mod permissions;
mod person_extras;
mod privileges;
mod territories;
mod witnessing;
mod reports;
mod sections;
mod service_meetings;
mod tasks;
mod unlock;
mod users;
mod wizard;

pub use absences::AbsencesPage;
pub use app_layout::AppLayout;
pub use attendance::{AttendanceMonth, AttendancePage};
pub use auth::{Login, Signup};
pub use dashboard::Dashboard;
pub use events::EventsPage;
pub use families::FamilyDetail;
pub use groups::{GroupDetail, GroupList, GroupNew};
pub use join::Join;
pub use landing::Landing;
pub use midweek::MidweekPage;
pub use ministry::MyMinistryPage;
pub use my_assignments::MyAssignmentsPage;
pub use permissions::{PermissionsDetail, PermissionsPage};
pub use privileges::PrivilegesPage;
pub use territories::{TerritoriesPage, TerritoryDetail, TerritoryEdit, TerritoryNew, TerritorySettingsPage};
pub use witnessing::{WitnessingPage, WitnessingPresetsPage};
pub use reports::ReportsPage;
pub use tasks::CustomTasksPage;
pub use service_meetings::{ServiceMeetingsPage, ServicePresetsPage};
pub use sections::{Me, NotFound, SectionHub, SectionPage, SettingsPage};
pub use unlock::Unlock;
pub use users::{MovedAway, UserDetail, UserList, UserNew};
pub use wizard::NewCongregation;

use dioxus::prelude::*;
use theo_ui::{use_toaster, ActionButton, ButtonVariant};

/// Copies `text` (a generated password) to the clipboard, and says so.
#[component]
pub(crate) fn CopyButton(text: String) -> Element {
    let toaster = use_toaster();
    rsx! {
        ActionButton {
            action: theo_ui::Action::Copy,
            variant: ButtonVariant::Secondary,
            onclick: move |_| {
                let text = text.clone();
                spawn(async move {
                    if crate::platform::copy_text(&text).await {
                        toaster.success("Copied. Paste it somewhere safe.");
                    } else {
                        toaster.error("It could not be copied. Select it and copy it.");
                    }
                });
            },
        }
    }
}
