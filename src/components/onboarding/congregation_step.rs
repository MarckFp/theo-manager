use dioxus::prelude::*;
use crate::database::models::congregation::{Congregation, NameOrder, FirstWeekday, MeetingTime};

#[derive(Props, Clone, PartialEq)]
pub struct CongregationStepProps {
    pub on_next: EventHandler<Congregation>,
    pub on_back: EventHandler<()>,
}

#[component]
pub fn CongregationStep(props: CongregationStepProps) -> Element {
    let mut name = use_signal(|| String::new());
    let mut jw_code = use_signal(|| String::new());
    let mut name_order = use_signal(|| NameOrder::FirstnameLastname);
    let mut first_weekday = use_signal(|| FirstWeekday::Sunday);
    
    let mut weekday_day = use_signal(|| chrono::Weekday::Mon);
    let mut weekday_hour = use_signal(|| String::from("19"));
    let mut weekday_minute = use_signal(|| String::from("00"));
    
    let mut weekend_day = use_signal(|| chrono::Weekday::Sat);
    let mut weekend_hour = use_signal(|| String::from("10"));
    let mut weekend_minute = use_signal(|| String::from("00"));
    
    let mut error_message = use_signal(|| String::new());

    let handle_submit = move |_| {
        // Validation
        if name().trim().is_empty() {
            error_message.set("Congregation name is required".to_string());
            return;
        }
        
        // Parse meeting times
        let weekday_meeting = match (weekday_hour().parse::<u32>(), weekday_minute().parse::<u32>()) {
            (Ok(h), Ok(m)) if h < 24 && m < 60 => {
                match chrono::NaiveTime::from_hms_opt(h, m, 0) {
                    Some(time) => MeetingTime {
                        day: weekday_day(),
                        time,
                    },
                    None => {
                        error_message.set("Invalid weekday meeting time".to_string());
                        return;
                    }
                }
            },
            _ => {
                error_message.set("Invalid weekday meeting time format".to_string());
                return;
            }
        };
        
        let weekend_meeting = match (weekend_hour().parse::<u32>(), weekend_minute().parse::<u32>()) {
            (Ok(h), Ok(m)) if h < 24 && m < 60 => {
                match chrono::NaiveTime::from_hms_opt(h, m, 0) {
                    Some(time) => MeetingTime {
                        day: weekend_day(),
                        time,
                    },
                    None => {
                        error_message.set("Invalid weekend meeting time".to_string());
                        return;
                    }
                }
            },
            _ => {
                error_message.set("Invalid weekend meeting time format".to_string());
                return;
            }
        };
        
        // Create congregation object
        let congregation = Congregation {
            id: surrealdb::RecordId::from(("congregation", "temp")),
            name: name().trim().to_string(),
            jw_code: if jw_code().trim().is_empty() { None } else { Some(jw_code().trim().to_string()) },
            name_order: name_order(),
            first_weekday: first_weekday(),
            weekday_meeting,
            weekend_meeting,
        };
        
        error_message.set(String::new());
        props.on_next.call(congregation);
    };

    rsx! {
        div { class: "space-y-6",
            // Header
            div { class: "text-center space-y-2",
                h2 { class: "text-2xl sm:text-3xl font-bold text-base-content",
                    "Congregation Setup"
                }
                p { class: "text-base-content/70 text-sm sm:text-base",
                    "Enter your congregation's basic information"
                }
            }
            
            // Error message
            if !error_message().is_empty() {
                div { class: "alert alert-error",
                    svg { 
                        class: "stroke-current shrink-0 h-6 w-6",
                        fill: "none",
                        view_box: "0 0 24 24",
                        path { 
                            stroke_linecap: "round",
                            stroke_linejoin: "round",
                            stroke_width: "2",
                            d: "M10 14l2-2m0 0l2-2m-2 2l-2-2m2 2l2 2m7-2a9 9 0 11-18 0 9 9 0 0118 0z"
                        }
                    }
                    span { "{error_message}" }
                }
            }
            
            // Form
            div { class: "space-y-4",
                // Congregation Name
                div { class: "form-control",
                    label { class: "label",
                        span { class: "label-text font-semibold", "Congregation Name *" }
                    }
                    input {
                        r#type: "text",
                        class: "input input-bordered w-full",
                        placeholder: "Enter congregation name",
                        value: "{name}",
                        oninput: move |e| name.set(e.value().clone())
                    }
                }
                
                // JW Code (optional)
                div { class: "form-control",
                    label { class: "label",
                        span { class: "label-text font-semibold", "JW Code (Optional)" }
                    }
                    input {
                        r#type: "text",
                        class: "input input-bordered w-full",
                        placeholder: "e.g., 12345",
                        value: "{jw_code}",
                        oninput: move |e| jw_code.set(e.value().clone())
                    }
                }
                
                // Name Order
                div { class: "form-control",
                    label { class: "label",
                        span { class: "label-text font-semibold", "Name Display Order" }
                    }
                    select {
                        class: "select select-bordered w-full",
                        onchange: move |e| {
                            name_order.set(match e.value().as_str() {
                                "lastname" => NameOrder::LastnameFirstname,
                                _ => NameOrder::FirstnameLastname,
                            });
                        },
                        option { value: "firstname", selected: true, "Firstname Lastname" }
                        option { value: "lastname", "Lastname, Firstname" }
                    }
                }
                
                // First Weekday
                div { class: "form-control",
                    label { class: "label",
                        span { class: "label-text font-semibold", "Week Starts On" }
                    }
                    select {
                        class: "select select-bordered w-full",
                        onchange: move |e| {
                            first_weekday.set(match e.value().as_str() {
                                "monday" => FirstWeekday::Monday,
                                _ => FirstWeekday::Sunday,
                            });
                        },
                        option { value: "sunday", selected: true, "Sunday" }
                        option { value: "monday", "Monday" }
                    }
                }
                
                // Weekday Meeting
                div { class: "form-control border border-base-300 rounded-lg p-4 space-y-3",
                    label { class: "label",
                        span { class: "label-text font-semibold", "Weekday Meeting *" }
                    }
                    
                    div { class: "grid grid-cols-1 sm:grid-cols-3 gap-3",
                        div { class: "form-control",
                            label { class: "label",
                                span { class: "label-text text-xs", "Day" }
                            }
                            select {
                                class: "select select-bordered select-sm w-full",
                                onchange: move |e| {
                                    weekday_day.set(match e.value().as_str() {
                                        "Mon" => chrono::Weekday::Mon,
                                        "Tue" => chrono::Weekday::Tue,
                                        "Wed" => chrono::Weekday::Wed,
                                        "Thu" => chrono::Weekday::Thu,
                                        "Fri" => chrono::Weekday::Fri,
                                        _ => chrono::Weekday::Mon,
                                    });
                                },
                                option { value: "Mon", selected: true, "Monday" }
                                option { value: "Tue", "Tuesday" }
                                option { value: "Wed", "Wednesday" }
                                option { value: "Thu", "Thursday" }
                                option { value: "Fri", "Friday" }
                            }
                        }
                        div { class: "form-control",
                            label { class: "label",
                                span { class: "label-text text-xs", "Hour" }
                            }
                            input {
                                r#type: "number",
                                class: "input input-bordered input-sm w-full",
                                min: "0",
                                max: "23",
                                value: "{weekday_hour}",
                                oninput: move |e| weekday_hour.set(e.value().clone())
                            }
                        }
                        div { class: "form-control",
                            label { class: "label",
                                span { class: "label-text text-xs", "Minute" }
                            }
                            input {
                                r#type: "number",
                                class: "input input-bordered input-sm w-full",
                                min: "0",
                                max: "59",
                                value: "{weekday_minute}",
                                oninput: move |e| weekday_minute.set(e.value().clone())
                            }
                        }
                    }
                }
                
                // Weekend Meeting
                div { class: "form-control border border-base-300 rounded-lg p-4 space-y-3",
                    label { class: "label",
                        span { class: "label-text font-semibold", "Weekend Meeting *" }
                    }
                    
                    div { class: "grid grid-cols-1 sm:grid-cols-3 gap-3",
                        div { class: "form-control",
                            label { class: "label",
                                span { class: "label-text text-xs", "Day" }
                            }
                            select {
                                class: "select select-bordered select-sm w-full",
                                onchange: move |e| {
                                    weekend_day.set(match e.value().as_str() {
                                        "Sat" => chrono::Weekday::Sat,
                                        "Sun" => chrono::Weekday::Sun,
                                        _ => chrono::Weekday::Sat,
                                    });
                                },
                                option { value: "Sat", selected: true, "Saturday" }
                                option { value: "Sun", "Sunday" }
                            }
                        }
                        div { class: "form-control",
                            label { class: "label",
                                span { class: "label-text text-xs", "Hour" }
                            }
                            input {
                                r#type: "number",
                                class: "input input-bordered input-sm w-full",
                                min: "0",
                                max: "23",
                                value: "{weekend_hour}",
                                oninput: move |e| weekend_hour.set(e.value().clone())
                            }
                        }
                        div { class: "form-control",
                            label { class: "label",
                                span { class: "label-text text-xs", "Minute" }
                            }
                            input {
                                r#type: "number",
                                class: "input input-bordered input-sm w-full",
                                min: "0",
                                max: "59",
                                value: "{weekend_minute}",
                                oninput: move |e| weekend_minute.set(e.value().clone())
                            }
                        }
                    }
                }
            }
            
            // Action buttons
            div { class: "flex flex-col sm:flex-row gap-3 mt-8 pt-6 border-t border-base-300",
                button { 
                    class: "btn btn-outline btn-lg flex-1",
                    onclick: move |_| props.on_back.call(()),
                    svg { 
                        class: "w-5 h-5 mr-2",
                        fill: "none",
                        stroke: "currentColor",
                        view_box: "0 0 24 24",
                        path { 
                            stroke_linecap: "round",
                            stroke_linejoin: "round",
                            stroke_width: "2",
                            d: "M11 17l-5-5m0 0l5-5m-5 5h12"
                        }
                    }
                    "Back"
                }
                
                button { 
                    class: "btn btn-primary btn-lg flex-1",
                    onclick: handle_submit,
                    "Next"
                    svg { 
                        class: "w-5 h-5 ml-2",
                        fill: "none",
                        stroke: "currentColor",
                        view_box: "0 0 24 24",
                        path { 
                            stroke_linecap: "round",
                            stroke_linejoin: "round",
                            stroke_width: "2",
                            d: "M13 7l5 5m0 0l-5 5m5-5H6"
                        }
                    }
                }
            }
        }
    }
}
