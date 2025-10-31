use dioxus::prelude::*;

#[derive(Props, Clone, PartialEq)]
pub struct MeetingsProps {
    pub on_navigate: EventHandler<String>,
}

#[component]
pub fn Meetings(props: MeetingsProps) -> Element {
    let submenu_items = vec![
        ("weekday-meeting", "Weekday Meeting", "📖", "Life & Ministry meeting schedule"),
        ("weekend-meeting", "Weekend Meeting", "🎤", "Public talk and Watchtower study"),
        ("field-service-meetings", "Field Service Meetings", "🚪", "Preaching arrangements"),
        ("meeting-attendance", "Meeting Attendance", "✅", "Track meeting attendance"),
    ];
    
    rsx! {
        div { class: "space-y-6",
            // Breadcrumbs
            div { class: "text-sm breadcrumbs",
                ul {
                    li { a { class: "text-primary", onclick: move |_| props.on_navigate.call("dashboard".to_string()), "Home" } }
                    li { "Meetings" }
                }
            }
            
            // Header
            div { class: "mb-8",
                h2 { class: "text-3xl font-bold text-base-content mb-2", "Meetings" }
                p { class: "text-base-content/70", "Organize and track congregation meetings" }
            }
            
            // Submenu Grid
            div { class: "grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-2 gap-4",
                for (id, name, icon, description) in submenu_items.iter() {
                    button {
                        key: "{id}",
                        class: "card bg-base-100 shadow-lg hover:shadow-xl transition-all duration-200 hover:scale-[1.02] active:scale-95",
                        onclick: {
                            let section = id.to_string();
                            move |_| props.on_navigate.call(section.clone())
                        },
                        div { class: "card-body",
                            div { class: "flex items-start gap-4",
                                span { class: "text-5xl", "{icon}" }
                                div { class: "flex-1",
                                    h3 { class: "card-title text-lg mb-1", "{name}" }
                                    p { class: "text-sm text-base-content/70", "{description}" }
                                }
                                svg {
                                    class: "w-5 h-5 text-base-content/30",
                                    xmlns: "http://www.w3.org/2000/svg",
                                    fill: "none",
                                    view_box: "0 0 24 24",
                                    stroke: "currentColor",
                                    path {
                                        stroke_linecap: "round",
                                        stroke_linejoin: "round",
                                        stroke_width: "2",
                                        d: "M9 5l7 7-7 7"
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
