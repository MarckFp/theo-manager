use dioxus::prelude::*;
use crate::components::menu::Menu;
use crate::components::navigation_header::NavigationHeader;
use crate::database::models::congregation::{Congregation, NameOrder};
use crate::database::models::user::User;
use crate::views::categories::publishers::Publishers;
use crate::views::categories::meetings::Meetings;
use crate::views::categories::congregation::Congregation as CongregationCategory;
use crate::views::categories::settings::Settings as SettingsCategory;
use crate::views::congregation_settings::CongregationSettings;
use crate::views::user_settings::UserSettings;
use crate::views::users::Users;
use crate::views::field_service_groups::FieldServiceGroups;
use crate::views::field_service_reports::FieldServiceReports;
use crate::views::publisher_reports::PublisherReports;

// Helper function to get parent section
fn get_parent_section(section: &str) -> Option<&'static str> {
    match section {
        // Subcategories go back to their category
        "users" | "field-service-reports" | "roles" | "field-service-groups" => Some("publishers-category"),
        "weekday-meeting" | "weekend-meeting" | "field-service-meetings" | "meeting-attendance" => Some("meetings-category"),
        "special-events" | "absences" | "cleaning" | "maintenance" | "attendant" | "audio-video" | "territory" => Some("congregation-category"),
        "user-settings" | "congregation-settings" => Some("settings-category"),
        // Categories go back to dashboard
        "publishers-category" | "meetings-category" | "congregation-category" | "settings-category" => Some("dashboard"),
        // Dashboard has no parent
        "dashboard" => None,
        // Publisher reports go back to field-service-reports
        s if s.starts_with("field_service_reports/") => Some("field-service-reports"),
        _ => Some("dashboard"),
    }
}

#[component]
pub fn Home() -> Element {
    let mut current_section = use_signal(|| "dashboard".to_string());
    
    // Handle navigation
    let on_navigate = move |section: String| {
        current_section.set(section);
    };
    
    // Handle back navigation - go to parent section
    let on_back = move |_| {
        if let Some(parent) = get_parent_section(&current_section()) {
            current_section.set(parent.to_string());
        }
    };
    
    // Handle home navigation
    let on_home = move |_| {
        current_section.set("dashboard".to_string());
    };
    
    // Fetch congregation data
    let congregation = use_resource(move || async move {
        match Congregation::all().await {
            Ok(congregations) => congregations.into_iter().next(),
            Err(_) => None,
        }
    });
    
    // Fetch current user (first user with Owner role for now)
    let current_user = use_resource(move || async move {
        match User::all().await {
            Ok(users) => users.into_iter().next(),
            Err(_) => None,
        }
    });
    
    // Format user name based on congregation name order
    let format_name = move || -> String {
        match (congregation(), current_user()) {
            (Some(Some(cong)), Some(Some(user))) => {
                match cong.name_order {
                    NameOrder::FirstnameLastname => {
                        format!("{} {}", user.firstname, user.lastname)
                    },
                    NameOrder::LastnameFirstname => {
                        format!("{}, {}", user.lastname, user.firstname)
                    }
                }
            },
            _ => "User".to_string()
        }
    };
    
    rsx! {
        div { class: "flex min-h-screen bg-base-200 overflow-x-hidden",
            // Menu Sidebar
            Menu {
                current_section: current_section(),
                on_section_change: move |section: String| {
                    current_section.set(section);
                },
            }
            // Main Content Area
            main { class: "flex-1 lg:ml-64 p-4 lg:p-8 pb-20 lg:pb-8 overflow-x-hidden",
                // Navigation Header (Back + Home buttons) - hide on dashboard
                if current_section() != "dashboard" {
                    NavigationHeader {
                        show_back: get_parent_section(&current_section()).is_some(),
                        show_home: true,
                        on_back,
                        on_home,
                    }
                }
                // Header - only show on dashboard
                if current_section() == "dashboard" {
                    div { class: "mb-8 mt-0 lg:mt-0",
                        match (congregation(), current_user()) {
                            (Some(Some(_)), Some(Some(_))) => rsx! {
                                h1 { class: "text-3xl lg:text-4xl font-bold text-base-content", "Welcome, {format_name()}!" }
                                p { class: "text-base-content/70 mt-2", "Manage your congregation efficiently" }
                            },
                            _ => rsx! {
                                div { class: "flex items-center gap-2",
                                    span { class: "loading loading-spinner loading-md" }
                                    span { "Loading..." }
                                }
                            },
                        }
                    }
                }
                // Content based on current section
                div { key: "{current_section()}", class: "page-transition",
                    match current_section().as_str() {
                        // Category views (no wrapper needed)
                        "publishers-category" => rsx! {
                            Publishers {
                                on_navigate: move |section: String| {
                                    current_section.set(section);
                                },
                            }
                        },
                        "meetings-category" => rsx! {
                            Meetings {
                                on_navigate: move |section: String| {
                                    current_section.set(section);
                                },
                            }
                        },
                        "congregation-category" => rsx! {
                            CongregationCategory {
                                on_navigate: move |section: String| {
                                    current_section.set(section);
                                },
                            }
                        },
                        "settings-category" => rsx! {
                            SettingsCategory {
                                on_navigate: move |section: String| {
                                    current_section.set(section);
                                },
                            }
                        },
                        "congregation-settings" => rsx! {
                            CongregationSettings {
                                on_navigate: move |section: String| {
                                    current_section.set(section);
                                },
                            }
                        },
                        "user-settings" => rsx! {
                            UserSettings {
                                on_navigate: move |section: String| {
                                    current_section.set(section);
                                },
                            }
                        },
                        "users" => rsx! {
                            Users {
                                on_navigate: move |section: String| {
                                    current_section.set(section);
                                },
                            }
                        },
                        "field-service-groups" => rsx! {
                            FieldServiceGroups {
                                on_navigate: move |section: String| {
                                    current_section.set(section);
                                },
                            }
                        },
                        "field-service-reports" => rsx! {
                            FieldServiceReports {
                                on_navigate: move |section: String| {
                                    current_section.set(section);
                                },
                            }
                        },
                        s if s.starts_with("field_service_reports/") => {
                            let publisher_id = s
                                .strip_prefix("field_service_reports/")
                                .unwrap_or("")
                                .to_string();
                            rsx! {
                                PublisherReports {
                                    publisher_id,
                                    on_navigate: move |section: String| {
                                        current_section.set(section);
                                    },
                                }
                            }
                        }
                        _ => rsx! {
                            {render_section_content(current_section())}
                        },
                    }
                }
            }
        }
    }
}

fn render_section_content(section: String) -> Element {
    // Map sections to their parent categories
    let section_name = match section.as_str() {
        // Publishers subcategories
        "users" => "Users",
        "field-service-reports" => "Field Service Reports",
        "roles" => "Privileges",
        "field-service-groups" => "Field Service Groups",
        
        // Meetings subcategories
        "weekday-meeting" => "Weekday Meeting",
        "weekend-meeting" => "Weekend Meeting",
        "field-service-meetings" => "Field Service Meetings",
        "meeting-attendance" => "Meeting Attendance",
        
        // Congregation subcategories
        "special-events" => "Special Events",
        "absences" => "Absences",
        "cleaning" => "Cleaning Schedule",
        "maintenance" => "Maintenance",
        "attendant" => "Attendant Schedule",
        "audio-video" => "Audio & Video",
        "territory" => "Territory",
        
        // Settings subcategories
        "user-settings" => "User Settings",
        
        // Dashboard has no parent
        "dashboard" => "Dashboard",
        
        _ => "Unknown",
    };
    
    match section.as_str() {
        "dashboard" => rsx! {
            div { class: "bg-base-100 rounded-lg shadow-lg p-6",
                h2 { class: "text-2xl font-bold mb-4", "Dashboard" }
                div { class: "grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4",
                    // Quick Stats Cards
                    div { class: "stat bg-primary text-primary-content rounded-lg shadow",
                        div { class: "stat-title text-primary-content/80", "Publishers" }
                        div { class: "stat-value", "0" }
                        div { class: "stat-desc text-primary-content/70", "Active members" }
                    }
                    div { class: "stat bg-secondary text-secondary-content rounded-lg shadow",
                        div { class: "stat-title text-secondary-content/80", "Groups" }
                        div { class: "stat-value", "0" }
                        div { class: "stat-desc text-secondary-content/70", "Field service groups" }
                    }
                    div { class: "stat bg-accent text-accent-content rounded-lg shadow",
                        div { class: "stat-title text-accent-content/80", "Reports" }
                        div { class: "stat-value", "0" }
                        div { class: "stat-desc text-accent-content/70", "Missing reports this month" }
                    }
                }
            }
        },
        _ => {
            // For all other sections, show content card
            rsx! {
                div { class: "bg-base-100 rounded-lg shadow-lg p-6",
                    h2 { class: "text-2xl font-bold mb-4", "{section_name}" }
                    p { class: "text-base-content/70", "This feature is currently in development" }
                    div { class: "alert alert-info mt-4",
                        span { "👷 This section is under construction" }
                    }
                }
            }
        }
    }
}
