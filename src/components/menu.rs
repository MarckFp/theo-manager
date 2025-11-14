use dioxus::prelude::*;

// Helper function to check if current section belongs to a category
fn is_in_category(section: &str, category: &str) -> bool {
    if section == category {
        return true;
    }
    
    // Check if section is a subcategory of this category
    match category {
        "publishers-category" => matches!(section, "users" | "field-service-reports" | "roles" | "field-service-groups"),
        "meetings-category" => matches!(section, "weekday-meeting" | "weekend-meeting" | "field-service-meetings" | "meeting-attendance"),
        "congregation-category" => matches!(section, "special-events" | "absences" | "cleaning" | "maintenance" | "attendant" | "audio-video" | "territory"),
        "settings-category" => matches!(section, "user-settings" | "congregation-settings"),
        _ => false,
    }
}

#[derive(Props, Clone, PartialEq)]
pub struct MenuProps {
    pub current_section: String,
    pub on_section_change: EventHandler<String>,
}

#[component]
pub fn Menu(props: MenuProps) -> Element {
    let menu_items = vec![
        ("dashboard", "Dashboard", "🏠"),
        ("publishers-category", "Publishers", "👥"),
        ("meetings-category", "Meetings", "📖"),
        ("congregation-category", "Congregation", "🎉"),
        ("settings-category", "Settings", "⚙️"),
    ];
    
    // Mobile bottom bar - 4 main categories
    let mobile_menu_items = vec![
        ("publishers-category", "Publishers", "👥"),
        ("meetings-category", "Meetings", "📖"),
        ("congregation-category", "More", "🎉"),
        ("settings-category", "Settings", "⚙️"),
    ];
    
    let handle_menu_click = move |section: String| {
        props.on_section_change.call(section);
    };
    
    rsx! {
        // Desktop Sidebar
        aside {
            class: "hidden lg:block fixed top-0 left-0 h-screen w-64 bg-gradient-to-b from-base-100 to-base-200 border-r border-base-300 z-30",
            
            // Header
            div { class: "h-20 flex items-center justify-center border-b border-base-300/50 bg-base-100/50 backdrop-blur",
                div { class: "text-center",
                    h1 { class: "text-2xl font-bold bg-gradient-to-r from-primary to-secondary bg-clip-text text-transparent",
                        "Theo"
                    }
                    p { class: "text-xs text-base-content/60 font-medium tracking-wide uppercase",
                        "Manager"
                    }
                }
            }
            
            // Navigation
            nav { class: "flex-1 overflow-y-auto p-3 space-y-1",
                for (id, name, icon) in menu_items.iter() {
                    button {
                        key: "{id}",
                        class: format!(
                            "group w-full flex items-center gap-3 px-3 py-2.5 rounded-xl transition-all duration-200 {}",
                            if is_in_category(&props.current_section, id) {
                                "bg-primary text-primary-content shadow-lg shadow-primary/30 scale-[1.02]"
                            } else {
                                "text-base-content/70 hover:text-base-content hover:bg-base-100 hover:shadow-md hover:scale-[1.01]"
                            }
                        ),
                        onclick: {
                            let section = id.to_string();
                            move |_| handle_menu_click(section.clone())
                        },
                        span { class: "text-xl transition-transform group-hover:scale-110", "{icon}" }
                        span { class: "text-sm font-medium", "{name}" }
                    }
                }
            }
            
            // Footer
            div { class: "h-16 border-t border-base-300/50 flex items-center justify-center bg-base-100/30",
                p { class: "text-xs text-base-content/50",
                    "v1.0.0"
                }
            }
        }
        
        // Mobile Bottom Navigation Bar
        nav {
            class: "lg:hidden fixed bottom-0 left-0 right-0 z-50 bg-base-100 border-t border-base-300 shadow-2xl",
            div { class: "flex items-center justify-around h-16 px-2 safe-bottom",
                for (id, name, icon) in mobile_menu_items.iter() {
                    button {
                        key: "{id}",
                        class: format!(
                            "relative flex flex-col items-center justify-center gap-1 px-3 py-2 rounded-lg transition-all duration-200 min-w-[64px] {}",
                            if is_in_category(&props.current_section, id) {
                                "text-primary scale-105"
                            } else {
                                "text-base-content/60 active:scale-95"
                            }
                        ),
                        onclick: {
                            let section = id.to_string();
                            move |_| handle_menu_click(section.clone())
                        },
                        span { class: "text-2xl", "{icon}" }
                        span { 
                            class: format!(
                                "text-[10px] font-medium {}",
                                if is_in_category(&props.current_section, id) { "font-semibold" } else { "" }
                            ),
                            "{name}"
                        }
                        // Active indicator
                        if is_in_category(&props.current_section, id) {
                            div { class: "absolute bottom-0 w-12 h-1 bg-primary rounded-t-full" }
                        }
                    }
                }
            }
        }
    }
}
