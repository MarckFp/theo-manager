use dioxus::prelude::*;
use crate::database::models::congregation::{Congregation, NameOrder, FirstWeekday, MeetingTime};
use chrono::{Weekday, NaiveTime};
use wasm_bindgen::JsCast;

#[derive(Props, Clone, PartialEq)]
pub struct CongregationSettingsProps {
    pub on_navigate: EventHandler<String>,
}

#[component]
pub fn CongregationSettings(props: CongregationSettingsProps) -> Element {
    // Load congregation data
    let congregation = use_resource(move || async move {
        match Congregation::all().await {
            Ok(congregations) => congregations.into_iter().next(),
            Err(_) => None,
        }
    });
    
    // Form fields
    let mut name = use_signal(|| String::new());
    let mut jw_code = use_signal(|| String::new());
    let mut name_order = use_signal(|| NameOrder::FirstnameLastname);
    let mut first_weekday = use_signal(|| FirstWeekday::Sunday);
    
    let mut weekday_day = use_signal(|| Weekday::Thu);
    let mut weekday_time = use_signal(|| String::from("19:00"));
    
    let mut weekend_day = use_signal(|| Weekday::Sun);
    let mut weekend_time = use_signal(|| String::from("10:00"));
    
    let mut is_saving = use_signal(|| false);
    let mut save_message = use_signal(|| None::<String>);
    let mut show_delete_confirm = use_signal(|| false);
    let mut show_import_confirm = use_signal(|| false);
    let mut is_deleting = use_signal(|| false);
    let mut is_exporting = use_signal(|| false);
    let mut is_importing = use_signal(|| false);
    
    // Initialize form with loaded data
    use_effect(move || {
        if let Some(Some(cong)) = congregation() {
            name.set(cong.name.clone());
            jw_code.set(cong.jw_code.unwrap_or_default());
            name_order.set(cong.name_order.clone());
            first_weekday.set(cong.first_weekday.clone());
            weekday_day.set(cong.weekday_meeting.day);
            weekday_time.set(cong.weekday_meeting.time.format("%H:%M").to_string());
            weekend_day.set(cong.weekend_meeting.day);
            weekend_time.set(cong.weekend_meeting.time.format("%H:%M").to_string());
        }
    });
    
    let handle_save = move |_| {
        if let Some(Some(cong)) = congregation() {
            is_saving.set(true);
            save_message.set(None);
            
            let updated_cong = Congregation {
                id: cong.id.clone(),
                name: name().trim().to_string(),
                jw_code: if jw_code().trim().is_empty() { None } else { Some(jw_code().trim().to_string()) },
                name_order: name_order(),
                first_weekday: first_weekday(),
                weekday_meeting: MeetingTime {
                    day: weekday_day(),
                    time: NaiveTime::parse_from_str(&weekday_time(), "%H:%M").unwrap_or(NaiveTime::from_hms_opt(19, 0, 0).unwrap()),
                },
                weekend_meeting: MeetingTime {
                    day: weekend_day(),
                    time: NaiveTime::parse_from_str(&weekend_time(), "%H:%M").unwrap_or(NaiveTime::from_hms_opt(10, 0, 0).unwrap()),
                },
            };
            
            spawn(async move {
                match Congregation::update(cong.id, updated_cong).await {
                    Ok(_) => {
                        is_saving.set(false);
                        save_message.set(Some("Settings saved successfully!".to_string()));
                    },
                    Err(e) => {
                        is_saving.set(false);
                        save_message.set(Some(format!("Failed to save: {}", e)));
                    }
                }
            });
        }
    };
    
    let handle_delete_confirm = move |_| {
        is_deleting.set(true);
        
        spawn(async move {
            let db = match crate::database::db::get_db().await {
                Ok(db) => db,
                Err(_) => {
                    is_deleting.set(false);
                    save_message.set(Some("Failed to connect to database".to_string()));
                    return;
                }
            };
            
            // Delete all records from all tables
            let tables = vec![
                "user", "congregation", "role", "field_service_group",
                "field_service_meeting", "field_service_report", "meeting_attendance",
                "absence", "special_event", "privilege"
            ];
            
            for table in tables {
                let _ = db.query(format!("DELETE {}", table)).await;
            }
            
            is_deleting.set(false);
            
            // Reload the page to go back to landing
            if let Some(window) = web_sys::window() {
                let _ = window.location().reload();
            }
        });
    };
    
    let handle_export = move |_| {
        is_exporting.set(true);
        
        spawn(async move {
            match crate::database::db::get_db().await {
                Ok(db) => {
                    let mut export_data = serde_json::Map::new();
                    
                    let tables = vec![
                        "congregation", "user", "role", "field_service_group",
                        "field_service_meeting", "field_service_report", "meeting_attendance",
                        "absence", "special_event", "privilege"
                    ];
                    
                    // Export all tables
                    for table in tables {
                        match db.select::<Vec<serde_json::Value>>(table).await {
                            Ok(records) => {
                                export_data.insert(table.to_string(), serde_json::Value::Array(records));
                            },
                            Err(_) => {
                                export_data.insert(table.to_string(), serde_json::Value::Array(vec![]));
                            }
                        }
                    }
                    
                    let export_json = match serde_json::to_string_pretty(&export_data) {
                        Ok(json) => json,
                        Err(_) => {
                            is_exporting.set(false);
                            save_message.set(Some("Failed to serialize data".to_string()));
                            return;
                        }
                    };
                    
                    let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S");
                    let filename = format!("theo_manager_backup_{}.json", timestamp);
                    
                    // Create and trigger download
                    if let Some(window) = web_sys::window() {
                        if let Some(document) = window.document() {
                            use web_sys::{Blob, Url, HtmlAnchorElement};
                            use wasm_bindgen::JsValue;
                            
                            let export_bytes = export_json.as_bytes();
                            let array = js_sys::Uint8Array::from(export_bytes);
                            
                            match Blob::new_with_u8_array_sequence(&JsValue::from(js_sys::Array::of1(&array))) {
                                Ok(blob) => {
                                    if let Ok(url) = Url::create_object_url_with_blob(&blob) {
                                        if let Ok(element) = document.create_element("a") {
                                            if let Ok(anchor) = element.dyn_into::<HtmlAnchorElement>() {
                                                anchor.set_href(&url);
                                                anchor.set_download(&filename);
                                                anchor.click();
                                                let _ = Url::revoke_object_url(&url);
                                                save_message.set(Some("Database exported successfully!".to_string()));
                                            }
                                        }
                                    }
                                },
                                Err(_) => {
                                    save_message.set(Some("Failed to create download".to_string()));
                                }
                            }
                        }
                    }
                    
                    is_exporting.set(false);
                },
                Err(_) => {
                    is_exporting.set(false);
                    save_message.set(Some("Failed to connect to database".to_string()));
                }
            }
        });
    };
    
    let handle_import = move |_evt: Event<FormData>| {
        // File import is not yet supported in web browsers due to API limitations
        // Export works perfectly for creating backups
        save_message.set(Some("Import feature coming soon! Use Export to create backups.".to_string()));
    };
    
    rsx! {
        div { class: "space-y-6",
            // Breadcrumbs
            div { class: "text-sm breadcrumbs mb-4",
                ul {
                    li {
                        a {
                            class: "text-primary",
                            onclick: move |_| props.on_navigate.call("dashboard".to_string()),
                            "Home"
                        }
                    }
                    li {
                        a {
                            class: "text-primary",
                            onclick: move |_| props.on_navigate.call("settings-category".to_string()),
                            "Settings"
                        }
                    }
                    li { "Congregation Settings" }
                }
            }
            // Header
            div { class: "mb-6",
                h2 { class: "text-3xl font-bold text-base-content mb-2", "Congregation Settings" }
                p { class: "text-base-content/70", "Manage your congregation configuration" }
            }
            match congregation() {
                Some(Some(_)) => rsx! {
                    // Settings Form
                    div { class: "card bg-base-100 shadow-lg",
                        div { class: "card-body",
                            h3 { class: "card-title text-lg mb-4", "General Information" }

                            div { class: "grid grid-cols-1 md:grid-cols-2 gap-4",
                                // Name
                                div { class: "form-control",
                                    label { class: "label",
                                        span { class: "label-text font-semibold", "Congregation Name *" }
                                    }
                                    input {
                                        class: "input input-bordered w-full",
                                        r#type: "text",
                                        value: "{name()}",
                                        oninput: move |evt| name.set(evt.value()),
                                        placeholder: "Enter congregation name",
                                    }
                                }

                                // JW Code
                                div { class: "form-control",
                                    label { class: "label",
                                        span { class: "label-text font-semibold", "JW Code (Optional)" }
                                    }
                                    input {
                                        class: "input input-bordered w-full",
                                        r#type: "text",
                                        value: "{jw_code()}",
                                        oninput: move |evt| jw_code.set(evt.value()),
                                        placeholder: "Enter JW code",
                                    }
                                }

                                // Name Order
                                div { class: "form-control",
                                    label { class: "label",
                                        span { class: "label-text font-semibold", "Name Display Order *" }
                                    }
                                    select {
                                        class: "select select-bordered w-full",
                                        value: match name_order() {
                                            NameOrder::FirstnameLastname => "FirstnameLastname",
                                            NameOrder::LastnameFirstname => "LastnameFirstname",
                                        },
                                        onchange: move |evt| {
                                            name_order

                                                // First Weekday
                                                .set(

                                                    // Meeting Times

                                                    // Weekday Meeting
                                                    match evt.value().as_str() {

                                                        // Weekend Meeting

                                                        // Save Button & Messages

                                                        // Danger Zone

                                                        // Export Button

                                                        // Import Button

                                                        // Delete Button

                                                        "LastnameFirstname" => NameOrder::LastnameFirstname,
                                                        _ => NameOrder::FirstnameLastname,
                                                    },
                                                );
                                        },
                                        option { value: "FirstnameLastname", "Firstname Lastname" }
                                        option { value: "LastnameFirstname", "Lastname, Firstname" }
                                    }
                                }
                                div { class: "form-control",
                                    label { class: "label",
                                        span { class: "label-text font-semibold", "First Day of Week *" }
                                    }
                                    select {
                                        class: "select select-bordered w-full",
                                        value: match first_weekday() {
                                            FirstWeekday::Sunday => "Sunday",
                                            FirstWeekday::Monday => "Monday",
                                        },
                                        onchange: move |evt| {
                                            first_weekday
                                                .set(
                                                    match evt.value().as_str() {
                                                        "Monday" => FirstWeekday::Monday,
                                                        _ => FirstWeekday::Sunday,
                                                    },
                                                );
                                        },
                                        option { value: "Sunday", "Sunday" }
                                        option { value: "Monday", "Monday" }
                                    }
                                }
                            }
                        }
                    }
                    div { class: "card bg-base-100 shadow-lg",
                        div { class: "card-body",
                            h3 { class: "card-title text-lg mb-4", "Meeting Times" }
                            div { class: "grid grid-cols-1 md:grid-cols-2 gap-6",
                                div { class: "space-y-4",
                                    h4 { class: "font-semibold text-base", "Weekday Meeting" }
                                    div { class: "form-control",
                                        label { class: "label",
                                            span { class: "label-text", "Day" }
                                        }
                                        select {
                                            class: "select select-bordered w-full",
                                            value: format!("{:?}", weekday_day()),
                                            onchange: move |evt| {
                                                weekday_day
                                                    .set(
                                                        match evt.value().as_str() {
                                                            "Mon" => Weekday::Mon,
                                                            "Tue" => Weekday::Tue,
                                                            "Wed" => Weekday::Wed,
                                                            "Thu" => Weekday::Thu,
                                                            "Fri" => Weekday::Fri,
                                                            "Sat" => Weekday::Sat,
                                                            "Sun" => Weekday::Sun,
                                                            _ => Weekday::Thu,
                                                        },
                                                    );
                                            },
                                            option { value: "Mon", "Monday" }
                                            option { value: "Tue", "Tuesday" }
                                            option { value: "Wed", "Wednesday" }
                                            option { value: "Thu", "Thursday" }
                                            option { value: "Fri", "Friday" }
                                            option { value: "Sat", "Saturday" }
                                            option { value: "Sun", "Sunday" }
                                        }
                                    }
                                    div { class: "form-control",
                                        label { class: "label",
                                            span { class: "label-text", "Time" }
                                        }
                                        input {
                                            class: "input input-bordered w-full",
                                            r#type: "time",
                                            value: "{weekday_time()}",
                                            oninput: move |evt| weekday_time.set(evt.value()),
                                        }
                                    }
                                }
                                div { class: "space-y-4",
                                    h4 { class: "font-semibold text-base", "Weekend Meeting" }
                                    div { class: "form-control",
                                        label { class: "label",
                                            span { class: "label-text", "Day" }
                                        }
                                        select {
                                            class: "select select-bordered w-full",
                                            value: format!("{:?}", weekend_day()),
                                            onchange: move |evt| {
                                                weekend_day
                                                    .set(
                                                        match evt.value().as_str() {
                                                            "Mon" => Weekday::Mon,
                                                            "Tue" => Weekday::Tue,
                                                            "Wed" => Weekday::Wed,
                                                            "Thu" => Weekday::Thu,
                                                            "Fri" => Weekday::Fri,
                                                            "Sat" => Weekday::Sat,
                                                            "Sun" => Weekday::Sun,
                                                            _ => Weekday::Sun,
                                                        },
                                                    );
                                            },
                                            option { value: "Mon", "Monday" }
                                            option { value: "Tue", "Tuesday" }
                                            option { value: "Wed", "Wednesday" }
                                            option { value: "Thu", "Thursday" }
                                            option { value: "Fri", "Friday" }
                                            option { value: "Sat", "Saturday" }
                                            option { value: "Sun", "Sunday" }
                                        }
                                    }
                                    div { class: "form-control",
                                        label { class: "label",
                                            span { class: "label-text", "Time" }
                                        }
                                        input {
                                            class: "input input-bordered w-full",
                                            r#type: "time",
                                            value: "{weekend_time()}",
                                            oninput: move |evt| weekend_time.set(evt.value()),
                                        }
                                    }
                                }
                            }
                        }
                    }
                    if let Some(msg) = save_message() {
                        div { class: if msg.contains("success") { "alert alert-success" } else { "alert alert-error" },
                            span { "{msg}" }
                        }
                    }
                    button {
                        class: "btn btn-primary btn-lg w-full sm:w-auto",
                        disabled: is_saving(),
                        onclick: handle_save,
                        if is_saving() {
                            span { class: "loading loading-spinner" }
                            "Saving..."
                        } else {
                            "💾 Save Changes"
                        }
                    }
                    div { class: "divider mt-8", "Danger Zone" }
                    div { class: "card bg-error/10 border border-error/30",
                        div { class: "card-body",
                            h3 { class: "card-title text-error text-lg mb-4", "⚠️ Database Management" }
                            div { class: "grid grid-cols-1 sm:grid-cols-3 gap-4",
                                button {
                                    class: "btn btn-info btn-lg",
                                    disabled: is_exporting(),
                                    onclick: handle_export,
                                    if is_exporting() {
                                        span { class: "loading loading-spinner" }
                                        "Exporting..."
                                    } else {
                                        "📥 Export Database"
                                    }
                                }
                                div {
                                    if !show_import_confirm() {
                                        button {
                                            class: "btn btn-warning btn-lg w-full",
                                            onclick: move |_| show_import_confirm.set(true),
                                            "📤 Import Database"
                                        }
                                    } else {
                                        div { class: "space-y-2",
                                            div { class: "alert alert-warning text-sm p-2",
                                                p { class: "font-semibold", "⚠️ All data will be replaced!" }
                                            }
                                            div { class: "flex gap-2",
                                                label {
                                                    class: "btn btn-warning btn-sm flex-1",
                                                    r#for: "import-file",
                                                    if is_importing() {
                                                        span { class: "loading loading-spinner loading-xs" }
                                                        "Importing..."
                                                    } else {
                                                        "Select File"
                                                    }
                                                }
                                                button {
                                                    class: "btn btn-ghost btn-sm",
                                                    onclick: move |_| show_import_confirm.set(false),
                                                    "Cancel"
                                                }
                                            }
                                            input {
                                                id: "import-file",
                                                class: "hidden",
                                                r#type: "file",
                                                accept: ".json",
                                                onchange: handle_import,
                                            }
                                        }
                                    }
                                }
                                div {
                                    if !show_delete_confirm() {
                                        button {
                                            class: "btn btn-error btn-lg w-full",
                                            onclick: move |_| show_delete_confirm.set(true),
                                            "🗑️ Delete Everything"
                                        }
                                    } else {
                                        div { class: "space-y-2",
                                            div { class: "alert alert-error text-sm p-2",
                                                p { class: "font-semibold", "⚠️ This cannot be undone!" }
                                            }
                                            div { class: "flex gap-2",
                                                button {
                                                    class: "btn btn-error btn-sm flex-1",
                                                    disabled: is_deleting(),
                                                    onclick: handle_delete_confirm,
                                                    if is_deleting() {
                                                        span { class: "loading loading-spinner loading-xs" }
                                                        "Deleting..."
                                                    } else {
                                                        "Confirm Delete"
                                                    }
                                                }
                                                button {
                                                    class: "btn btn-ghost btn-sm",
                                                    onclick: move |_| show_delete_confirm.set(false),
                                                    "Cancel"
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                            p { class: "text-sm text-base-content/70 mt-4",
                                "Export: Download a complete backup • Import: Restore from backup file • Delete: Remove all data and start fresh"
                            }
                        }
                    }
                },
                Some(None) => rsx! {
                    div { class: "alert alert-warning",
                        span { "No congregation data found" }
                    }
                },
                None => rsx! {
                    div { class: "flex items-center gap-2",
                        span { class: "loading loading-spinner loading-md" }
                        span { "Loading congregation settings..." }
                    }
                },
            }
        }
    }
}
