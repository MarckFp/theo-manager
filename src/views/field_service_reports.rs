use dioxus::prelude::*;
use crate::database::models::user::{User, UserType};
use crate::database::models::congregation::{Congregation, NameOrder};
use crate::database::models::field_service_group::FieldServiceGroup;
use crate::database::models::field_service_report::FieldServiceReport;
use chrono::{Utc, Datelike, NaiveDate};

/// Get the first day of the previous month
fn get_previous_month_start() -> NaiveDate {
    let now = Utc::now().naive_utc().date();
    let year = now.year();
    let month = now.month();
    
    if month == 1 {
        NaiveDate::from_ymd_opt(year - 1, 12, 1).unwrap()
    } else {
        NaiveDate::from_ymd_opt(year, month - 1, 1).unwrap()
    }
}

/// Get the last day of the previous month
fn get_previous_month_end() -> NaiveDate {
    let now = Utc::now().naive_utc().date();
    let first_of_current = NaiveDate::from_ymd_opt(now.year(), now.month(), 1).unwrap();
    first_of_current.pred_opt().unwrap()
}

/// Calculate total hours + credits for a user in previous month
async fn get_previous_month_totals(user_id: &str) -> (i16, i16) {
    let start = get_previous_month_start();
    let end = get_previous_month_end();
    
    match FieldServiceReport::all().await {
        Ok(reports) => {
            let mut total_hours = 0i16;
            let mut total_credits = 0i16;
            
            for report in reports {
                if let Some(ref publisher) = report.publisher {
                    let publisher_id = publisher.id.to_string();
                    if publisher_id == user_id && report.date >= start && report.date <= end {
                        total_hours += report.hours.unwrap_or(0);
                        total_credits += report.credits.unwrap_or(0);
                    }
                }
            }
            
            (total_hours, total_credits)
        }
        Err(_) => (0, 0)
    }
}

/// Check if user preached in previous month
async fn did_preach_previous_month(user_id: &str) -> bool {
    let start = get_previous_month_start();
    let end = get_previous_month_end();
    
    match FieldServiceReport::all().await {
        Ok(reports) => {
            reports.iter().any(|report| {
                if let Some(ref publisher) = report.publisher {
                    let publisher_id = publisher.id.to_string();
                    publisher_id == user_id && report.date >= start && report.date <= end && report.preached
                } else {
                    false
                }
            })
        }
        Err(_) => false
    }
}

#[derive(Props, Clone, PartialEq)]
pub struct FieldServiceReportsProps {
    pub on_navigate: EventHandler<String>,
}

#[component]
pub fn FieldServiceReports(props: FieldServiceReportsProps) -> Element {
    // State
    let mut users = use_signal(|| Vec::<User>::new());
    let mut filtered_users = use_signal(|| Vec::<User>::new());
    let mut field_service_groups = use_signal(|| Vec::<FieldServiceGroup>::new());
    
    // Filters
    let mut search_query = use_signal(|| String::new());
    let mut type_filter = use_signal(|| None::<UserType>);
    let mut group_filter = use_signal(|| None::<String>);
    let mut filters_collapsed = use_signal(|| true);
    
    // Pagination
    let mut current_page = use_signal(|| 0usize);
    let items_per_page = 20;
    
    // Messages
    let mut message = use_signal(|| None::<String>);
    
    // Load congregation settings
    let congregation = use_resource(move || async move {
        match Congregation::all().await {
            Ok(congregations) => congregations.into_iter().next(),
            Err(_) => None,
        }
    });
    
    // Load users
    let load_users = move || {
        spawn(async move {
            match User::all().await {
                Ok(all_users) => {
                    // Filter only publishers (not students)
                    let publishers: Vec<User> = all_users.into_iter()
                        .filter(|u| u.publisher_type.is_some())
                        .collect();
                    users.set(publishers);
                },
                Err(_) => {
                    message.set(Some("Failed to load users".to_string()));
                }
            }
        });
    };
    
    // Load field service groups
    let load_field_service_groups = move || {
        spawn(async move {
            match FieldServiceGroup::all().await {
                Ok(groups) => {
                    field_service_groups.set(groups);
                },
                Err(_) => {}
            }
        });
    };
    
    use_effect(move || {
        load_users();
        load_field_service_groups();
    });
    
    // Apply filters
    let mut apply_filters = move || {
        let mut result = users();
        
        // Search by name
        let query = search_query().to_lowercase();
        if !query.is_empty() {
            result = result.into_iter()
                .filter(|u| {
                    let fullname = format!("{} {}", u.firstname, u.lastname).to_lowercase();
                    fullname.contains(&query)
                })
                .collect();
        }
        
        // Filter by type
        if let Some(ref t) = type_filter() {
            result = result.into_iter()
                .filter(|u| u.publisher_type.as_ref() == Some(t))
                .collect();
        }
        
        // Filter by field service group
        if let Some(ref group_id) = group_filter() {
            // Extract just the ID part from the filter (in case it's "field_service_group:id")
            let group_id_only = group_id.split(':').last().unwrap_or(group_id);
            
            result = result.into_iter()
                .filter(|u| {
                    if let Some(user_id_val) = &u.id {
                        let user_id_str = user_id_val.to_string();
                        let user_id_only = user_id_str.split(':').last().unwrap_or(&user_id_str);
                        
                        // Find the group and check if user is a member, supervisor, or auxiliary
                        field_service_groups().iter().any(|g| {
                            if let Some(ref g_id) = g.id {
                                let g_id_str = g_id.to_string();
                                let g_id_only = g_id_str.split(':').last().unwrap_or(&g_id_str);
                                
                                if g_id_only == group_id_only {
                                    // Check if user is supervisor
                                    let is_supervisor = g.supervisor.as_ref().map_or(false, |sup| {
                                        let sup_str = sup.to_string();
                                        let sup_id = sup_str.split(':').last().unwrap_or(&sup_str);
                                        sup_id == user_id_only
                                    });
                                    
                                    // Check if user is auxiliary
                                    let is_auxiliary = g.auxiliar.as_ref().map_or(false, |aux| {
                                        let aux_str = aux.to_string();
                                        let aux_id = aux_str.split(':').last().unwrap_or(&aux_str);
                                        aux_id == user_id_only
                                    });
                                    
                                    // Check if user is a member
                                    let is_member = g.members.iter().any(|member| {
                                        let member_str = member.to_string();
                                        let member_id = member_str.split(':').last().unwrap_or(&member_str);
                                        member_id == user_id_only
                                    });
                                    
                                    return is_supervisor || is_auxiliary || is_member;
                                }
                            }
                            false
                        })
                    } else {
                        false
                    }
                })
                .collect();
        }
        
        // Sort alphabetically
        if let Some(Some(cong)) = congregation.read().as_ref() {
            match cong.name_order {
                NameOrder::FirstnameLastname => {
                    result.sort_by(|a, b| {
                        let a_name = format!("{} {}", a.firstname, a.lastname);
                        let b_name = format!("{} {}", b.firstname, b.lastname);
                        a_name.to_lowercase().cmp(&b_name.to_lowercase())
                    });
                }
                NameOrder::LastnameFirstname => {
                    result.sort_by(|a, b| {
                        let a_name = format!("{} {}", a.lastname, a.firstname);
                        let b_name = format!("{} {}", b.lastname, b.firstname);
                        a_name.to_lowercase().cmp(&b_name.to_lowercase())
                    });
                }
            }
        }
        
        filtered_users.set(result);
        current_page.set(0);
    };
    
    use_effect(move || {
        apply_filters();
    });
    
    // Pagination
    let total_pages = (filtered_users().len() + items_per_page - 1) / items_per_page;
    let paginated_users: Vec<User> = filtered_users()
        .into_iter()
        .skip(current_page() * items_per_page)
        .take(items_per_page)
        .collect();
    
    // Format user name based on congregation settings
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
    
    // Get field service group name
    let get_group_name = move |user: &User| -> String {
        if let Some(ref group_ref) = user.preaching_group {
            let group_id = group_ref.id.to_string();
            field_service_groups()
                .iter()
                .find(|g| {
                    if let Some(ref id) = g.id {
                        id.to_string() == group_id
                    } else {
                        false
                    }
                })
                .map(|g| g.name.clone())
                .unwrap_or_else(|| "—".to_string())
        } else {
            "—".to_string()
        }
    };
    
    // Get publisher type label
    let get_type_label = move |user_type: &UserType| -> &str {
        match user_type {
            UserType::BaptizedPublisher => "Publisher",
            UserType::UnbaptizedPublisher => "Unbaptized",
            UserType::RegularPioneer => "Regular Pioneer",
            UserType::SpecialPioneer => "Special Pioneer",
            UserType::ContiniousAuxiliaryPioneer => "Auxiliary Pioneer",
            UserType::Student => "Student",
        }
    };
    
    // Check if user is a pioneer
    let is_pioneer = |user_type: &Option<UserType>| -> bool {
        matches!(
            user_type,
            Some(UserType::RegularPioneer) | Some(UserType::SpecialPioneer) | Some(UserType::ContiniousAuxiliaryPioneer)
        )
    };
    
    rsx! {
        div { class: "space-y-6",
            // Header
            div { class: "flex flex-col lg:flex-row lg:items-center lg:justify-between gap-4 mb-6",
                div {
                    h1 { class: "text-3xl font-bold", "📊 Field Service Reports" }
                    p { class: "text-base-content/70 mt-1", "View and manage field service reports" }
                }
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
            // Filters
            div { class: "mb-6 bg-base-200",
                div { class: "card bg-base-100 shadow-lg sticky top-0 z-10 pb-4",
                    div { class: "card-body",
                        div { class: "flex items-center justify-between mb-2",
                            h3 { class: "card-title text-lg", "🔍 Filters" }
                            button {
                                class: "btn btn-ghost btn-sm btn-circle",
                                onclick: move |_| filters_collapsed.set(!filters_collapsed()),
                                if filters_collapsed() {
                                    "▼"
                                } else {
                                    "▲"
                                }
                            }
                        }
                        if !filters_collapsed() {
                            div { class: "grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4",
                                // Search
                                div { class: "form-control",
                                    label { class: "label",
                                        span { class: "label-text", "Search by name" }
                                    }
                                    input {
                                        r#type: "text",
                                        class: "input input-bordered",
                                        placeholder: "John Doe",
                                        value: "{search_query()}",
                                        oninput: move |evt| {
                                            search_query.set(evt.value());
                                            apply_filters();
                                        },
                                    }
                                }
                                // Type filter
                                div { class: "form-control",
                                    label { class: "label",
                                        span { class: "label-text", "Publisher Type" }
                                    }
                                    select {
                                        class: "select select-bordered",
                                        onchange: move |evt| {
                                            match evt.value().as_str() {
                                                "all" => type_filter.set(None),
                                                "baptized" => type_filter.set(Some(UserType::BaptizedPublisher)),
                                                "unbaptized" => type_filter.set(Some(UserType::UnbaptizedPublisher)),
                                                "regular_pioneer" => type_filter.set(Some(UserType::RegularPioneer)),
                                                "special_pioneer" => type_filter.set(Some(UserType::SpecialPioneer)),
                                                "aux_pioneer" => type_filter.set(Some(UserType::ContiniousAuxiliaryPioneer)),
                                                _ => {}
                                            }
                                            apply_filters();
                                        },
                                        option { value: "all", "All" }
                                        option { value: "baptized", "Publisher" }
                                        option { value: "unbaptized", "Unbaptized" }
                                        option { value: "regular_pioneer", "Regular Pioneer" }
                                        option { value: "special_pioneer", "Special Pioneer" }
                                        option { value: "aux_pioneer", "Auxiliary Pioneer" }
                                    }
                                }
                                // Group filter
                                div { class: "form-control",
                                    label { class: "label",
                                        span { class: "label-text", "Field Service Group" }
                                    }
                                    select {
                                        class: "select select-bordered",
                                        onchange: move |evt| {
                                            let value = evt.value();
                                            if value == "all" {
                                                group_filter.set(None);
                                            } else {
                                                group_filter.set(Some(value));
                                            }
                                            apply_filters();
                                        },
                                        option { value: "all", "All" }
                                        {
                                            let mut sorted_groups = field_service_groups();
                                            sorted_groups.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));

                                            sorted_groups
                                                .into_iter()
                                                .filter_map(|group| {
                                                    if let Some(ref group_id) = group.id {
                                                        let id_str = group_id.to_string();
                                                        let name = group.name.clone();
                                                        Some(rsx! {
                                                            option { value: "{id_str}", "{name}" }
                                                        })
                                                    } else {
                                                        None
                                                    }
                                                })
                                        }
                                    }
                                }
                            }
                            div { class: "mt-4",
                                button {
                                    class: "btn btn-outline btn-sm",
                                    onclick: move |_| {
                                        search_query.set(String::new());
                                        type_filter.set(None);
                                        group_filter.set(None);
                                        apply_filters();
                                    },
                                    "Clear Filters"
                                }
                            }
                        }
                    }
                }
            }
            // Publishers list
            div { class: "overflow-x-auto",
                table { class: "table table-zebra w-full",
                    thead {
                        tr {
                            th { "Name" }
                            th { "Type" }
                            th { "Previous Month" }
                            th { class: "hidden md:table-cell", "Field Service Group" }
                        }
                    }
                    tbody {
                        for user in paginated_users {
                            {
                                let user_id = if let Some(ref id) = user.id {
                                    id.to_string().split(':').last().unwrap_or("").to_string()
                                } else {
                                    String::new()
                                };

                                let user_type = user.publisher_type.clone();
                                let is_pioneer_type = is_pioneer(&user_type);
                                let user_clone = user.clone();
                                let user_clone2 = user.clone();

                                rsx! {
                                    tr {
                                        key: "{user_id}",
                                        class: "hover:bg-base-200 cursor-pointer",
                                        onclick: move |_| {
                                            props.on_navigate.call(format!("field_service_reports/{}", user_id));
                                        },
                                        td {
                                            div { class: "font-medium", "{format_user_name(&user)}" }
                                        }
                                        td {
                                            if let Some(ref ut) = user_type {
                                                // Show previous month data as pill

                                                span { class: "badge badge-primary badge-sm", "{get_type_label(ut)}" }
                                            }
                                        }
                                        td {
                                            {
                                                let user_id_for_async = user_id.clone();
                                                let previous_month_data = use_resource(move || {
                                                    let uid = user_id_for_async.clone();
                                                    async move {
                                                        if is_pioneer_type {
                                                            let (hours, credits) = get_previous_month_totals(&uid).await;
                                                            let total = hours + credits;
                                                            (format!("{} hours", total), total > 0)
                                                        } else {
                                                            if did_preach_previous_month(&uid).await {
                                                                ("Preached".to_string(), true)
                                                            } else {
                                                                ("Not preached".to_string(), false)
                                                            }
                                                        }
                                                    }
                                                });
        
                                                match previous_month_data.read().as_ref() {
                                                    Some((data, preached)) => rsx! {
                                                        span { class: if *preached { "badge badge-success" } else { "badge badge-error" }, "{data}" }
                                                    },
                                                    None => rsx! {
                                                        span { class: "loading loading-spinner loading-xs" }
                                                    },
                                                }
                                            }
                                        }
                                        td { class: "hidden md:table-cell", "{get_group_name(&user_clone)}" }
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
                            class: "join-item btn btn-sm",
                            disabled: current_page() == 0,
                            onclick: move |_| current_page.set(current_page().saturating_sub(1)),
                            "«"
                        }
                        for page in 0..total_pages {
                            button {
                                key: "{page}",
                                class: format!(
                                    "join-item btn btn-sm {}",
                                    if page == current_page() { "btn-active" } else { "" },
                                ),
                                onclick: move |_| current_page.set(page),
                                "{page + 1}"
                            }
                        }
                        button {
                            class: "join-item btn btn-sm",
                            disabled: current_page() >= total_pages - 1,
                            onclick: move |_| current_page.set((current_page() + 1).min(total_pages - 1)),
                            "»"
                        }
                    }
                }
            }
        }
    }
}
