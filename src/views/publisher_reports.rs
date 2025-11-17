use dioxus::prelude::*;
use crate::database::models::user::{User, UserType};
use crate::database::models::congregation::{Congregation, NameOrder};
use crate::database::models::field_service_report::{FieldServiceReport, FieldServiceReportCommitment};
use chrono::{Utc, Datelike, NaiveDate};
use surrealdb::sql::Thing;

#[derive(Clone, PartialEq)]
enum ModalMode {
    Create,
    Edit,
}

/// Format month and year
fn format_month_year(date: NaiveDate) -> String {
    let month = match date.month() {
        1 => "January",
        2 => "February",
        3 => "March",
        4 => "April",
        5 => "May",
        6 => "June",
        7 => "July",
        8 => "August",
        9 => "September",
        10 => "October",
        11 => "November",
        12 => "December",
        _ => "",
    };
    format!("{} {}", month, date.year())
}

#[derive(Props, Clone, PartialEq)]
pub struct PublisherReportsProps {
    pub publisher_id: String,
    pub on_navigate: EventHandler<String>,
}

#[component]
pub fn PublisherReports(props: PublisherReportsProps) -> Element {
    // State
    let mut publisher = use_signal(|| None::<User>);
    let mut reports = use_signal(|| Vec::<FieldServiceReport>::new());
    let mut filtered_reports = use_signal(|| Vec::<FieldServiceReport>::new());
    
    // Pagination
    let mut current_page = use_signal(|| 0usize);
    let items_per_page = 10;
    
    // Modal state
    let mut show_modal = use_signal(|| false);
    let mut modal_mode = use_signal(|| ModalMode::Create);
    let mut editing_report = use_signal(|| None::<FieldServiceReport>);
    
    // Form fields
    let mut form_date = use_signal(|| Utc::now().naive_utc().date());
    let mut form_preached = use_signal(|| false);
    let mut form_hours = use_signal(|| String::new());
    let mut form_credits = use_signal(|| String::new());
    let mut form_commitment = use_signal(|| None::<FieldServiceReportCommitment>);
    let mut form_notes = use_signal(|| String::new());
    
    // Confirmation dialogs
    let mut show_delete_confirm = use_signal(|| false);
    let mut deleting_report_id = use_signal(|| None::<String>);
    
    // Messages
    let mut message = use_signal(|| None::<String>);
    
    // Load congregation settings
    let congregation = use_resource(move || async move {
        match Congregation::all().await {
            Ok(congregations) => congregations.into_iter().next(),
            Err(_) => None,
        }
    });
    
    // Clone publisher_id for use in multiple closures
    let publisher_id_for_load = props.publisher_id.clone();
    let publisher_id_for_reports = props.publisher_id.clone();
    let publisher_id_for_save = props.publisher_id.clone();
    let publisher_id_for_delete = props.publisher_id.clone();
    
    // Load publisher
    let load_publisher = move || {
        let publisher_id = publisher_id_for_load.clone();
        spawn(async move {
            let record_id = surrealdb::RecordId::from(("user", publisher_id.as_str()));
            match User::find(record_id).await {
                Ok(Some(user)) => {
                    publisher.set(Some(user));
                },
                _ => {
                    message.set(Some("Failed to load publisher".to_string()));
                }
            }
        });
    };
    
    // Load reports
    let load_reports = move || {
        let publisher_id = publisher_id_for_reports.clone();
        spawn(async move {
            match FieldServiceReport::all().await {
                Ok(all_reports) => {
                    let user_reports: Vec<FieldServiceReport> = all_reports
                        .into_iter()
                        .filter(|r| {
                            if let Some(ref pub_ref) = r.publisher {
                                let pub_id = pub_ref.id.to_string();
                                pub_id == publisher_id || pub_id.ends_with(&format!(":{}", publisher_id))
                            } else {
                                false
                            }
                        })
                        .collect();
                    
                    reports.set(user_reports);
                },
                Err(_) => {
                    message.set(Some("Failed to load reports".to_string()));
                }
            }
        });
    };
    
    use_effect(move || {
        load_publisher();
        load_reports();
    });
    
    // Sort and filter reports
    use_effect(move || {
        let mut sorted = reports();
        sorted.sort_by(|a, b| b.date.cmp(&a.date)); // Most recent first
        filtered_reports.set(sorted);
    });
    
    // Pagination
    let total_pages = (filtered_reports().len() + items_per_page - 1) / items_per_page;
    let paginated_reports: Vec<FieldServiceReport> = filtered_reports()
        .into_iter()
        .skip(current_page() * items_per_page)
        .take(items_per_page)
        .collect();
    
    // Format user name
    let format_user_name = move |user: &User| -> String {
        if let Some(Some(cong)) = congregation.read().as_ref() {
            match cong.name_order {
                NameOrder::FirstnameLastname => format!("{} {}", user.firstname, user.lastname),
                NameOrder::LastnameFirstname => format!("{}, {}", user.lastname, user.firstname),
            }
        } else {
            format!("{} {}", user.firstname, user.lastname)
        }
    };
    
    // Check if user is a pioneer
    let is_pioneer = move || -> bool {
        if let Some(ref user) = publisher() {
            matches!(
                user.publisher_type,
                Some(UserType::RegularPioneer) | Some(UserType::SpecialPioneer) | Some(UserType::ContiniousAuxiliaryPioneer)
            )
        } else {
            false
        }
    };
    
    // Handle create
    let handle_create = move |_| {
        modal_mode.set(ModalMode::Create);
        editing_report.set(None);
        form_date.set(Utc::now().naive_utc().date());
        form_preached.set(false);
        form_hours.set(String::new());
        form_credits.set(String::new());
        form_commitment.set(None);
        form_notes.set(String::new());
        show_modal.set(true);
    };
    
    // Handle edit
    let mut handle_edit = move |report: FieldServiceReport| {
        modal_mode.set(ModalMode::Edit);
        form_date.set(report.date);
        form_preached.set(report.preached);
        form_hours.set(report.hours.map(|h| h.to_string()).unwrap_or_default());
        form_credits.set(report.credits.map(|c| c.to_string()).unwrap_or_default());
        form_commitment.set(report.commitment.clone());
        form_notes.set(report.notes.clone().unwrap_or_default());
        editing_report.set(Some(report));
        show_modal.set(true);
    };
    
    // Handle save
    let handle_save = move |_| {
        let publisher_id = publisher_id_for_save.clone();
        let mut reports_signal = reports.clone();
        let publisher_clone = publisher.clone();
        spawn(async move {
            let publisher_thing = Thing::from(("user".to_string(), publisher_id.clone()));
            
            let hours_val = form_hours().parse::<i16>().ok();
            let credits_val = form_credits().parse::<i16>().ok();
            
            // Determine if preached should be set to true
            let preached = if let Some(ref user) = publisher_clone() {
                match user.publisher_type {
                    // For pioneers, if they have hours + credits > 0, set preached to true
                    Some(UserType::RegularPioneer) | Some(UserType::SpecialPioneer) | Some(UserType::ContiniousAuxiliaryPioneer) => {
                        let total = hours_val.unwrap_or(0) + credits_val.unwrap_or(0);
                        total > 0
                    }
                    // For baptized/unbaptized with commitment, if they have hours > 0, set preached to true
                    Some(UserType::BaptizedPublisher) | Some(UserType::UnbaptizedPublisher) => {
                        if form_commitment().is_some() {
                            hours_val.unwrap_or(0) > 0
                        } else {
                            form_preached()
                        }
                    }
                    _ => form_preached()
                }
            } else {
                form_preached()
            };
            
            let report = FieldServiceReport {
                id: editing_report().as_ref().and_then(|r| r.id.clone()),
                date: form_date(),
                publisher: Some(publisher_thing),
                preached,
                hours: hours_val,
                credits: credits_val,
                commitment: form_commitment(),
                notes: if form_notes().is_empty() { None } else { Some(form_notes()) },
            };
            
            let result = if let Some(ref existing) = editing_report() {
                if let Some(ref id) = existing.id {
                    FieldServiceReport::update(id.clone(), report).await
                } else {
                    return; // Can't update without an ID
                }
            } else {
                FieldServiceReport::create(report).await
            };
            
            match result {
                Ok(_) => {
                    message.set(Some("Report saved successfully".to_string()));
                    show_modal.set(false);
                    
                    // Reload reports
                    match FieldServiceReport::all().await {
                        Ok(all_reports) => {
                            let user_reports: Vec<FieldServiceReport> = all_reports
                                .into_iter()
                                .filter(|r| {
                                    if let Some(ref pub_ref) = r.publisher {
                                        let pub_id = pub_ref.id.to_string();
                                        pub_id == publisher_id || pub_id.ends_with(&format!(":{}", publisher_id))
                                    } else {
                                        false
                                    }
                                })
                                .collect();
                            
                            reports_signal.set(user_reports);
                        },
                        Err(_) => {}
                    }
                },
                Err(e) => {
                    message.set(Some(format!("Failed to save report: {:?}", e)));
                }
            }
        });
    };
    
    // Handle delete
    let mut handle_delete = move |report_id: String| {
        deleting_report_id.set(Some(report_id));
        show_delete_confirm.set(true);
    };
    
    let confirm_delete = move |_| {
        if let Some(report_id) = deleting_report_id() {
            let publisher_id = publisher_id_for_delete.clone();
            let mut reports_signal = reports.clone();
            spawn(async move {
                let record_id = surrealdb::RecordId::from(("field_service_report", report_id.as_str()));
                match FieldServiceReport::delete(record_id).await {
                    Ok(_) => {
                        message.set(Some("Report deleted successfully".to_string()));
                        show_delete_confirm.set(false);
                        
                        // Reload reports
                        match FieldServiceReport::all().await {
                            Ok(all_reports) => {
                                let user_reports: Vec<FieldServiceReport> = all_reports
                                    .into_iter()
                                    .filter(|r| {
                                        if let Some(ref pub_ref) = r.publisher {
                                            let pub_id = pub_ref.id.to_string();
                                            pub_id == publisher_id || pub_id.ends_with(&format!(":{}", publisher_id))
                                        } else {
                                            false
                                        }
                                    })
                                    .collect();
                                
                                reports_signal.set(user_reports);
                            },
                            Err(_) => {}
                        }
                    },
                    Err(_) => {
                        message.set(Some("Failed to delete report".to_string()));
                    }
                }
            });
        }
    };
    
    rsx! {
        div { class: "space-y-6",
            // Header
            div { class: "flex flex-col lg:flex-row lg:items-center lg:justify-between gap-4 mb-6",
                div {
                    h1 { class: "text-3xl font-bold",
                        if let Some(ref user) = publisher() {
                            "Reports for {format_user_name(user)}"
                        } else {
                            "Loading..."
                        }
                    }
                    p { class: "text-base-content/70 mt-1", "View and manage field service reports" }
                }
                button { class: "btn btn-primary", onclick: handle_create, "➕ New Report" }
            }
            // Message banner
            if let Some(msg) = message() {
                div { class: "alert alert-info shadow-lg",
                    span { "{msg}" }
                    button {
                        class: "btn btn-sm btn-ghost",
                        onclick: move |_| message.set(None),
                        "✕"
                    }
                }
            }
            // Reports list
            if paginated_reports.is_empty() {
                // Empty state
                div { class: "text-center py-16",
                    div { class: "text-6xl mb-4", "📋" }
                    h3 { class: "text-xl font-semibold mb-2", "No Field Service Reports Yet" }
                    p { class: "text-base-content/70",
                        "Click 'New Report' to create your first report"
                    }
                }
            } else {
                div { class: "overflow-x-auto",
                    table { class: "table table-zebra w-full",
                        thead {
                            tr {
                                th { "Month" }
                                th { class: "hidden sm:table-cell", "Commitment" }
                                th { "Status" }
                                if is_pioneer() {
                                    th { "Hours" }
                                    th { class: "hidden sm:table-cell", "Credits" }
                                }
                                th { class: "hidden md:table-cell", "Notes" }
                                th { "Actions" }
                            }
                        }
                        tbody {
                            for report in paginated_reports {
                                {
                                    let report_id = if let Some(ref id) = report.id {
                                        id.to_string().split(':').last().unwrap_or("").to_string()
                                    } else {
                                        String::new()
                                    };
                                    let report_clone = report.clone();
                                    let report_clone2 = report.clone();

                                    rsx! {
                                        tr { key: "{report_id}",
                                            td {
                                                div { class: "font-medium", "{format_month_year(report.date)}" }
                                            }
                                            td { class: "hidden sm:table-cell",
                                                if let Some(ref commitment) = report.commitment {
                                                    span { class: "badge badge-secondary badge-sm",
                                                        match commitment {
                                                            FieldServiceReportCommitment::Fifteen => "15 hrs",
                                                            FieldServiceReportCommitment::Thirty => "30 hrs",
                                                        }
                                                    }
                                                } else {
                                                    "—"
                                                }
                                            }
                                            td {
                                                if report.preached {
                                                    span { class: "badge badge-success badge-sm", "✅ Preached" }
                                                } else {
                                                    span { class: "badge badge-error badge-sm", "❌ Not preached" }
                                                }
                                            }
                                            if is_pioneer() {
                                                td {
                                                    {
                                                        let hours = report.hours.unwrap_or(0);
                                                        let credits = report.credits.unwrap_or(0);
                                                        let total = hours + credits;
                                                        rsx! { "{total}" }
                                                    }
                                                }
                                                td { class: "hidden sm:table-cell",
                                                    if let Some(credits) = report.credits {
                                                        "{credits}"
                                                    } else {
                                                        "0"
                                                    }
                                                }
                                            }
                                            td { class: "hidden md:table-cell",
                                                if let Some(ref notes) = report.notes {
                                                    div { class: "truncate max-w-xs", title: "{notes}", "{notes}" }
                                                } else {
                                                    "—"
                                                }
                                            }
                                            td {
                                                div { class: "flex gap-2",
                                                    button {
                                                        class: "btn btn-sm btn-ghost",
                                                        onclick: move |_| handle_edit(report_clone.clone()),
                                                        "✏️"
                                                    }
                                                    button {
                                                        class: "btn btn-sm btn-ghost text-error",
                                                        onclick: move |_| handle_delete(report_id.clone()),
                                                        "🗑️"
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
            }
            // Pagination
            if total_pages > 1 {
                div { class: "flex justify-center mt-6",
                    div { class: "join",
                        button {
                            class: "join-item btn",
                            disabled: current_page() == 0,
                            onclick: move |_| current_page.set(current_page().saturating_sub(1)),
                            "«"
                        }
                        button { class: "join-item btn", "Page {current_page() + 1} of {total_pages}" }
                        button {
                            class: "join-item btn",
                            disabled: current_page() >= total_pages - 1,
                            onclick: move |_| current_page.set((current_page() + 1).min(total_pages - 1)),
                            "»"
                        }
                    }
                }
            }
            // Create/Edit Modal
            if show_modal() {
                div { class: "modal modal-open",
                    div { class: "modal-box max-w-2xl",
                        h3 { class: "font-bold text-lg mb-4",
                            match modal_mode() {
                                ModalMode::Create => "New Report",
                                ModalMode::Edit => "Edit Report",
                            }
                        }
                        div { class: "space-y-4",
                            // Month and Year selector
                            div { class: "grid grid-cols-2 gap-4",
                                div { class: "form-control w-full",
                                    label { class: "label",
                                        span { class: "label-text", "Month" }
                                    }
                                    select {
                                        class: "select select-bordered w-full",
                                        value: "{form_date().month()}",
                                        onchange: move |evt| {
                                            if let Ok(month) = evt.value().parse::<u32>() {
                                                if let Some(new_date) = NaiveDate::from_ymd_opt(
                                                    form_date().year(),
                                                    month,
                                                    1,
                                                ) {
                                                    form_date.set(new_date);
                                                }
                                            }
                                        },
                                        option { value: "1", "January" }
                                        option { value: "2", "February" }
                                        option { value: "3", "March" }
                                        option { value: "4", "April" }
                                        option { value: "5", "May" }
                                        option { value: "6", "June" }
                                        option { value: "7", "July" }
                                        option { value: "8", "August" }
                                        option { value: "9", "September" }
                                        option { value: "10", "October" }
                                        option { value: "11", "November" }
                                        option { value: "12", "December" }
                                    }
                                }
                                div { class: "form-control w-full",
                                    label { class: "label",
                                        span { class: "label-text", "Year" }
                                    }
                                    input {
                                        r#type: "number",
                                        class: "input input-bordered w-full",
                                        min: "2000",
                                        max: "2100",
                                        value: "{form_date().year()}",
                                        oninput: move |evt| {
                                            if let Ok(year) = evt.value().parse::<i32>() {
                                                if let Some(new_date) = NaiveDate::from_ymd_opt(
                                                    year,
                                                    form_date().month(),
                                                    1,
                                                ) {
                                                    form_date.set(new_date);
                                                }
                                            }
                                        },
                                    }
                                }
                            }
                            // Pioneer-specific fields (Regular, Special, Auxiliary)
                            if is_pioneer() {
                                // Hours (required for pioneers with commitment or always for regular/special)
                                div { class: "form-control w-full",
                                    label { class: "label",
                                        span { class: "label-text", "Hours" }
                                    }
                                    input {
                                        r#type: "number",
                                        class: "input input-bordered w-full",
                                        min: "0",
                                        value: "{form_hours()}",
                                        oninput: move |evt| form_hours.set(evt.value()),
                                    }
                                }
                                // Credits (only for Regular and Special pioneers)
                                if let Some(ref user) = publisher() {
                                    if matches!(
                                        user.publisher_type,
                                        Some(UserType::RegularPioneer) | Some(UserType::SpecialPioneer)
                                    )
                                    {
                                        div { class: "form-control w-full",
                                            label { class: "label",
                                                span { class: "label-text", "Credits" }
                                            }
                                            input {
                                                r#type: "number",
                                                class: "input input-bordered w-full",
                                                min: "0",
                                                value: "{form_credits()}",
                                                oninput: move |evt| form_credits.set(evt.value()),
                                            }
                                        }
                                    }
                                }
                            }
                            // Commitment and Preached logic for baptized publishers
                            if let Some(ref user) = publisher() {
                                if matches!(
                                    user.publisher_type,
                                    Some(UserType::BaptizedPublisher) | Some(UserType::UnbaptizedPublisher)
                                )
                                {
                                    // Commitment selector
                                    div { class: "form-control w-full",
                                        label { class: "label",
                                            span { class: "label-text", "Commitment (Optional)" }
                                        }
                                        select {
                                            class: "select select-bordered w-full",
                                            value: match form_commitment() {
                                                Some(FieldServiceReportCommitment::Fifteen) => "fifteen",
                                                Some(FieldServiceReportCommitment::Thirty) => "thirty",
                                                None => "none",
                                            },
                                            onchange: move |evt| {
                                                match evt.value().as_str() {
                                                    "fifteen" => form_commitment.set(Some(FieldServiceReportCommitment::Fifteen)),
                                                    "thirty" => form_commitment.set(Some(FieldServiceReportCommitment::Thirty)),
                                                    _ => form_commitment.set(None),
                                                }
                                            },
                                            option { value: "none", "None" }
                                            option { value: "fifteen", "15 hours" }
                                            option { value: "thirty", "30 hours" }
                                        }
                                    }
                                    // If commitment is selected, show hours field
                                    if form_commitment().is_some() {
                                        div { class: "form-control w-full",
                                            label { class: "label",
                                                span { class: "label-text", "Hours" }
                                            }
                                            input {
                                                r#type: "number",
                                                class: "input input-bordered w-full",
                                                min: "0",
                                                value: "{form_hours()}",
                                                oninput: move |evt| form_hours.set(evt.value()),
                                            }
                                        }
                                    } else {
                                        // If no commitment, show preached checkbox
                                        div { class: "form-control w-full",
                                            label { class: "label cursor-pointer justify-start gap-4",
                                                input {
                                                    r#type: "checkbox",
                                                    class: "checkbox",
                                                    checked: form_preached(),
                                                    onchange: move |evt| form_preached.set(evt.value() == "true"),
                                                }
                                                span { class: "label-text", "Preached this month" }
                                            }
                                        }
                                    }
                                }
                            }
                            // Notes
                            div { class: "form-control w-full",
                                label { class: "label",
                                    span { class: "label-text", "Notes" }
                                }
                                textarea {
                                    class: "textarea textarea-bordered h-24 w-full",
                                    placeholder: "Optional notes...",
                                    value: "{form_notes()}",
                                    oninput: move |evt| form_notes.set(evt.value()),
                                }
                            }
                        }
                        div { class: "modal-action w-full flex gap-2",
                            button {
                                class: "btn btn-ghost flex-1",
                                onclick: move |_| show_modal.set(false),
                                "Cancel"
                            }
                            button {
                                class: "btn btn-primary flex-1",
                                onclick: handle_save,
                                "Save"
                            }
                        }
                    }
                }
            }
            // Delete Confirmation Dialog
            if show_delete_confirm() {
                div { class: "modal modal-open",
                    div { class: "modal-box",
                        h3 { class: "font-bold text-lg", "Delete Report" }
                        p { class: "py-4",
                            "Are you sure you want to delete this report? This action cannot be undone."
                        }
                        div { class: "modal-action",
                            button {
                                class: "btn btn-ghost",
                                onclick: move |_| show_delete_confirm.set(false),
                                "Cancel"
                            }
                            button {
                                class: "btn btn-error",
                                onclick: confirm_delete,
                                "Delete"
                            }
                        }
                    }
                }
            }
        }
    }
}
