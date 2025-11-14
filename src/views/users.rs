use dioxus::prelude::*;
use crate::database::models::user::{User, UserType, UserAppointment, UserEmergencyContact};
use crate::database::models::congregation::{Congregation, NameOrder};
use chrono::NaiveDate;

#[derive(Props, Clone, PartialEq)]
pub struct UsersProps {
    pub on_navigate: EventHandler<String>,
}

#[component]
pub fn Users(props: UsersProps) -> Element {
    // State for users list
    let mut users = use_signal(|| Vec::<User>::new());
    let mut filtered_users = use_signal(|| Vec::<User>::new());
    let mut selected_users = use_signal(|| Vec::<String>::new());
    
    // Pagination
    let mut current_page = use_signal(|| 0usize);
    let items_per_page = 12;
    
    // Filters
    let mut search_query = use_signal(|| String::new());
    let mut gender_filter = use_signal(|| None::<bool>);
    let mut appointment_filter = use_signal(|| None::<UserAppointment>);
    let mut type_filter = use_signal(|| None::<UserType>);
    let mut filters_collapsed = use_signal(|| true);
    
    // Modal state
    let mut show_modal = use_signal(|| false);
    let mut modal_mode = use_signal(|| ModalMode::Create);
    let mut editing_user = use_signal(|| None::<User>);
    
    // Confirmation dialogs
    let mut show_delete_confirm = use_signal(|| false);
    let mut show_bulk_delete_confirm = use_signal(|| false);
    let mut deleting_user_id = use_signal(|| None::<String>);
    
    // Messages
    let mut message = use_signal(|| None::<String>);
    
    // Load congregation settings for name order
    let congregation = use_resource(move || async move {
        match Congregation::all().await {
            Ok(congregations) => congregations.into_iter().next(),
            Err(_) => None,
        }
    });
    
    // Load users on mount
    let load_users = move || {
        spawn(async move {
            match User::all().await {
                Ok(all_users) => {
                    users.set(all_users.clone());
                    filtered_users.set(all_users);
                },
                Err(_) => {
                    message.set(Some("Failed to load users".to_string()));
                }
            }
        });
    };
    
    use_effect(move || {
        load_users();
    });
    
    // Apply filters
    let mut apply_filters = move || {
        let query = search_query().to_lowercase();
        let filtered: Vec<User> = users().into_iter().filter(|user| {
            // Search filter
            let name_match = if query.is_empty() {
                true
            } else {
                let full_name = format!("{} {}", user.firstname, user.lastname).to_lowercase();
                full_name.contains(&query)
            };
            
            // Gender filter
            let gender_match = match gender_filter() {
                Some(g) => user.gender == g,
                None => true,
            };
            
            // Appointment filter
            let appointment_match = match appointment_filter() {
                Some(ref a) => match (&user.appointment, a) {
                    (Some(ua), filter_a) => match (ua, filter_a) {
                        (UserAppointment::Elder, UserAppointment::Elder) => true,
                        (UserAppointment::MinisterialServant, UserAppointment::MinisterialServant) => true,
                        _ => false,
                    },
                    _ => false,
                },
                None => true,
            };
            
            // Type filter
            let type_match = match type_filter() {
                Some(ref t) => match (&user.publisher_type, t) {
                    (Some(ut), filter_t) => match (ut, filter_t) {
                        (UserType::Student, UserType::Student) => true,
                        (UserType::UnbaptizedPublisher, UserType::UnbaptizedPublisher) => true,
                        (UserType::BaptizedPublisher, UserType::BaptizedPublisher) => true,
                        (UserType::RegularPioneer, UserType::RegularPioneer) => true,
                        (UserType::SpecialPioneer, UserType::SpecialPioneer) => true,
                        (UserType::ContiniousAuxiliaryPioneer, UserType::ContiniousAuxiliaryPioneer) => true,
                        _ => false,
                    },
                    _ => false,
                },
                None => true,
            };
            
            name_match && gender_match && appointment_match && type_match
        }).collect();
        
        filtered_users.set(filtered);
        current_page.set(0);
    };
    
    // Format user name based on congregation settings
    let format_name = move |user: &User| -> String {
        match congregation() {
            Some(Some(cong)) => {
                match cong.name_order {
                    NameOrder::FirstnameLastname => format!("{} {}", user.firstname, user.lastname),
                    NameOrder::LastnameFirstname => format!("{}, {}", user.lastname, user.firstname),
                }
            },
            _ => format!("{} {}", user.firstname, user.lastname),
        }
    };
    
    // Pagination
    let total_pages = (filtered_users().len() + items_per_page - 1) / items_per_page;
    let start_idx = current_page() * items_per_page;
    let end_idx = (start_idx + items_per_page).min(filtered_users().len());
    let paginated_users: Vec<User> = filtered_users().into_iter().skip(start_idx).take(items_per_page).collect();
    
    // Toggle user selection
    let mut toggle_selection = move |user_id: String| {
        let mut selected = selected_users();
        if selected.contains(&user_id) {
            selected.retain(|id| id != &user_id);
        } else {
            selected.push(user_id);
        }
        selected_users.set(selected);
    };
    
    // Toggle select all / unselect all
    let toggle_select_all = move |_| {
        let all_user_ids: Vec<String> = filtered_users()
            .iter()
            .map(|u| u.id.to_string())
            .collect();
        
        if selected_users().len() == all_user_ids.len() {
            // Unselect all
            selected_users.set(Vec::new());
        } else {
            // Select all
            selected_users.set(all_user_ids);
        }
    };
    
    // Handle create user
    let handle_create = move |_| {
        modal_mode.set(ModalMode::Create);
        editing_user.set(None);
        show_modal.set(true);
    };
    
    // Handle edit user
    let mut handle_edit = move |user: User| {
        modal_mode.set(ModalMode::Edit);
        editing_user.set(Some(user));
        show_modal.set(true);
    };
    
    // Handle delete user
    let mut handle_delete = move |user_id: String| {
        deleting_user_id.set(Some(user_id));
        show_delete_confirm.set(true);
    };
    
    // Confirm delete
    let confirm_delete = move |_| {
        if let Some(id_str) = deleting_user_id() {
            spawn(async move {
                if let Ok(record_id) = id_str.parse() {
                    match User::delete(record_id).await {
                        Ok(_) => {
                            message.set(Some("User deleted successfully".to_string()));
                            load_users();
                            show_delete_confirm.set(false);
                            deleting_user_id.set(None);
                        },
                        Err(_) => {
                            message.set(Some("Failed to delete user".to_string()));
                        }
                    }
                }
            });
        }
    };
    
    // Handle bulk delete
    let handle_bulk_delete = move |_| {
        if !selected_users().is_empty() {
            show_bulk_delete_confirm.set(true);
        }
    };
    
    // Confirm bulk delete
    let confirm_bulk_delete = move |_| {
        let ids = selected_users();
        spawn(async move {
            let mut success_count = 0;
            for id_str in ids {
                if let Ok(record_id) = id_str.parse() {
                    if User::delete(record_id).await.is_ok() {
                        success_count += 1;
                    }
                }
            }
            message.set(Some(format!("{} users deleted successfully", success_count)));
            selected_users.set(Vec::new());
            load_users();
            show_bulk_delete_confirm.set(false);
        });
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
                            onclick: move |_| props.on_navigate.call("publishers-category".to_string()),
                            "Publishers"
                        }
                    }
                    li { "Users" }
                }
            }
            // Header with title and action buttons
            div { class: "flex flex-col lg:flex-row lg:items-center lg:justify-between gap-4 mb-6",
                div {
                    h2 { class: "text-3xl font-bold text-base-content", "Users" }
                    p { class: "text-base-content/70 mt-1", "Manage congregation members" }
                }
                div { class: "flex gap-2",
                    // Select/Unselect all button (only show when there are users)
                    if !filtered_users().is_empty() {
                        button {
                            class: "btn btn-outline btn-sm lg:btn-md",
                            onclick: toggle_select_all,
                            if selected_users().len() == filtered_users().len() {
                                "☐ Unselect All"
                            } else {
                                "☑ Select All"
                            }
                        }
                    }
                    // Bulk delete button (only show when users are selected)
                    if !selected_users().is_empty() {
                        button {
                            class: "btn btn-error btn-sm lg:btn-md",
                            onclick: handle_bulk_delete,
                            "🗑️ Delete Selected ({selected_users().len()})"
                        }
                    }
                    // Create button for desktop
                    button {
                        class: "hidden lg:flex btn btn-primary",
                        onclick: handle_create,
                        "➕ New User"
                    }
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
            // Filters section (sticky and collapsible)
            div { class: "mb-6 bg-base-200",
                div { class: "card bg-base-100 shadow-lg sticky top-0 z-10 pb-4",
                    div { class: "card-body",
                        // Header with toggle button
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
                        // Collapsible content
                        if !filters_collapsed() {
                            div { class: "grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-4",
                                // Search by name
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
                                // Gender filter
                                div { class: "form-control",
                                    label { class: "label",
                                        span { class: "label-text", "Gender" }
                                    }
                                    select {
                                        class: "select select-bordered",
                                        onchange: move |evt| {
                                            match evt.value().as_str() {
                                                "all" => gender_filter.set(None),
                                                "male" => gender_filter.set(Some(true)),
                                                "female" => gender_filter.set(Some(false)),
                                                _ => {}
                                            }
                                            apply_filters();
                                        },
                                        option { value: "all", "All" }
                                        option { value: "male", "Male" }
                                        option { value: "female", "Female" }
                                    }
                                }
                                // Appointment filter
                                div { class: "form-control",
                                    label { class: "label",
                                        span { class: "label-text", "Appointment" }
                                    }
                                    select {
                                        class: "select select-bordered",
                                        onchange: move |evt| {
                                            match evt.value().as_str() {
                                                "all" => appointment_filter.set(None),
                                                "elder" => appointment_filter.set(Some(UserAppointment::Elder)),
                                                "ms" => appointment_filter.set(Some(UserAppointment::MinisterialServant)),
                                                _ => {}
                                            }
                                            apply_filters();
                                        },
                                        option { value: "all", "All" }
                                        option { value: "elder", "Elder" }
                                        option { value: "ms", "Ministerial Servant" }
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
                                                "student" => type_filter.set(Some(UserType::Student)),
                                                "unbaptized" => type_filter.set(Some(UserType::UnbaptizedPublisher)),
                                                "baptized" => type_filter.set(Some(UserType::BaptizedPublisher)),
                                                "regular_pioneer" => type_filter.set(Some(UserType::RegularPioneer)),
                                                "special_pioneer" => type_filter.set(Some(UserType::SpecialPioneer)),
                                                "aux_pioneer" => type_filter.set(Some(UserType::ContiniousAuxiliaryPioneer)),
                                                _ => {}
                                            }
                                            apply_filters();
                                        },
                                        option { value: "all", "All" }
                                        option { value: "student", "Student" }
                                        option { value: "unbaptized", "Unbaptized Publisher" }
                                        option { value: "baptized", "Baptized Publisher" }
                                        option { value: "regular_pioneer", "Regular Pioneer" }
                                        option { value: "special_pioneer", "Special Pioneer" }
                                        option { value: "aux_pioneer", "Auxiliary Pioneer" }
                                    }
                                }
                            }
                            // Clear filters button
                            div { class: "mt-4",
                                button {
                                    class: "btn btn-outline btn-sm",
                                    onclick: move |_| {
                                        search_query.set(String::new());
                                        gender_filter.set(None);
                                        appointment_filter.set(None);
                                        type_filter.set(None);
                                        apply_filters();
                                    },
                                    "Clear Filters"
                                }
                            }
                        }
                    }
                }
            }
            // Users grid
            div { class: "grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4 gap-4",
                for user in paginated_users.iter() {
                    {
                        let user_clone = user.clone();
                        let user_id = user.id.to_string();
                        let user_id_for_checkbox = user_id.clone();
                        let user_id_for_delete = user_id.clone();
                        let is_selected = selected_users().contains(&user_id);

                        rsx! {
                            div {
                                key: "{user.id}",
                                class: format!(
                                    "card bg-base-100 shadow-lg hover:shadow-xl transition-all cursor-pointer {}",
                                    if is_selected { "ring-2 ring-primary" } else { "" },
                                ),
                                div { class: "card-body p-4",
                                    // Selection checkbox
                                    div { class: "flex items-start justify-between mb-2",
                                        input {
                                            r#type: "checkbox",
                                            class: "checkbox checkbox-primary checkbox-sm",
                                            checked: is_selected,
                                            onclick: move |evt| {
                                                evt.stop_propagation();
                                                toggle_selection(user_id_for_checkbox.clone());
                                            },
                                        }
                                        // Gender icon
                                        div {

                                            // Name

                                            // Labels/Tags
                                            // Appointment badge

                                            // Publisher type badge

                                            // Anointed badge

                                            // Family head badge

                                            // Contact info preview

                                            // Action buttons
                                            class: format!(
                                                "badge badge-sm {}",
                                                if user.gender { "badge-info" } else { "badge-secondary" },
                                            ),
                                            if user.gender {
                                                "♂ Male"
                                            } else {
                                                "♀ Female"
                                            }
                                        }
                                    }
                                    h3 {
                                        class: "font-bold text-lg mb-2",
                                        onclick: {
                                            let u = user_clone.clone();
                                            move |_| handle_edit(u.clone())
                                        },
                                        "{format_name(&user)}"
                                    }
                                    div { class: "flex flex-wrap gap-1 mb-3",
                                        if let Some(ref appointment) = user.appointment {
                                            div { class: "badge badge-primary badge-sm",
                                                match appointment {
                                                    UserAppointment::Elder => "Elder",
                                                    UserAppointment::MinisterialServant => "MS",
                                                }
                                            }
                                        }
                                        if let Some(ref pub_type) = user.publisher_type {
                                            div { class: "badge badge-secondary badge-sm",
                                                match pub_type {
                                                    UserType::Student => "Student",
                                                    UserType::UnbaptizedPublisher => "Unbaptized",
                                                    UserType::BaptizedPublisher => "Publisher",
                                                    UserType::RegularPioneer => "Pioneer",
                                                    UserType::SpecialPioneer => "Special Pioneer",
                                                    UserType::ContiniousAuxiliaryPioneer => "Aux Pioneer",
                                                }
                                            }
                                        }
                                        if user.anointed == Some(true) {
                                            div { class: "badge badge-accent badge-sm", "Anointed" }
                                        }
                                        if user.family_head {
                                            div { class: "badge badge-info badge-sm", "Family Head" }
                                        }
                                    }
                                    div { class: "text-xs text-base-content/70 space-y-1",
                                        if let Some(ref phone) = user.phone {
                                            div { "📞 {phone}" }
                                        }
                                    }
                                    div { class: "card-actions justify-end mt-3",
                                        button {
                                            class: "btn btn-error btn-sm btn-circle",
                                            onclick: {
                                                let uid = user_id_for_delete.clone();
                                                move |evt| {
                                                    evt.stop_propagation();
                                                    handle_delete(uid.clone())
                                                }
                                            },
                                            "🗑️"
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
            // Empty state
            if filtered_users().is_empty() {
                div { class: "text-center py-12",
                    div { class: "text-6xl mb-4", "👥" }
                    h3 { class: "text-xl font-bold mb-2", "No users found" }
                    p { class: "text-base-content/70",
                        "Try adjusting your filters or create a new user"
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
            // Floating action button for mobile
            button {
                class: "lg:hidden btn btn-primary btn-circle btn-lg fixed bottom-20 right-4 shadow-2xl z-40",
                onclick: handle_create,
                span { class: "text-2xl text-white", "+" }
            }
            // User modal (create/edit)
            if show_modal() {
                UserModal {
                    mode: modal_mode(),
                    user: editing_user(),
                    on_close: move |_| show_modal.set(false),
                    on_save: move |_| {
                        show_modal.set(false);
                        load_users();
                        message.set(Some("User saved successfully".to_string()));
                    },
                }
            }
            // Delete confirmation dialog
            if show_delete_confirm() {
                div { class: "modal modal-open",
                    div { class: "modal-box",
                        h3 { class: "font-bold text-lg mb-4", "Confirm Delete" }
                        p { class: "py-4",
                            "Are you sure you want to delete this user? This action cannot be undone."
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
            // Bulk delete confirmation dialog
            if show_bulk_delete_confirm() {
                div { class: "modal modal-open",
                    div { class: "modal-box",
                        h3 { class: "font-bold text-lg mb-4", "Confirm Bulk Delete" }
                        p { class: "py-4",
                            "Are you sure you want to delete {selected_users().len()} users? This action cannot be undone."
                        }
                        div { class: "modal-action",
                            button {
                                class: "btn btn-ghost",
                                onclick: move |_| show_bulk_delete_confirm.set(false),
                                "Cancel"
                            }
                            button {
                                class: "btn btn-error",
                                onclick: confirm_bulk_delete,
                                "Delete All"
                            }
                        }
                    }
                }
            }
        }
    }
}

#[derive(Clone, PartialEq)]
enum ModalMode {
    Create,
    Edit,
}

#[derive(Props, Clone, PartialEq)]
struct UserModalProps {
    mode: ModalMode,
    user: Option<User>,
    on_close: EventHandler<()>,
    on_save: EventHandler<()>,
}

#[component]
fn UserModal(props: UserModalProps) -> Element {
    // Clone props values to avoid move issues
    let mode = props.mode.clone();
    let mode_for_save = mode.clone();
    let mode_for_display = mode.clone();
    let mode_for_password = mode.clone();
    let mode_for_button = mode.clone();
    
    let existing_user_prop = props.user.clone();
    let existing_user_for_save = existing_user_prop.clone();
    
    let on_close = props.on_close.clone();
    let on_close_for_cancel = on_close.clone();
    let on_save = props.on_save.clone();
    
    // Form fields
    let mut firstname = use_signal(|| String::new());
    let mut lastname = use_signal(|| String::new());
    let mut gender = use_signal(|| true);
    let mut family_head = use_signal(|| false);
    let mut email = use_signal(|| String::new());
    let mut phone = use_signal(|| String::new());
    let mut address = use_signal(|| String::new());
    let mut city = use_signal(|| String::new());
    let mut country = use_signal(|| String::new());
    let mut zipcode = use_signal(|| String::new());
    let mut birthday = use_signal(|| String::new());
    let mut baptism_date = use_signal(|| String::new());
    let mut anointed = use_signal(|| false);
    let mut publisher_type = use_signal(|| None::<UserType>);
    let mut appointment = use_signal(|| None::<UserAppointment>);
    let mut password = use_signal(|| String::new());
    let mut emergency_contacts = use_signal(|| Vec::<UserEmergencyContact>::new());
    
    // Collapsible sections state
    let mut basic_info_collapsed = use_signal(|| false);
    let mut contact_info_collapsed = use_signal(|| true);
    let mut emergency_contacts_collapsed = use_signal(|| true);
    let mut spiritual_info_collapsed = use_signal(|| true);
    let mut credentials_collapsed = use_signal(|| true);
    
    let mut is_saving = use_signal(|| false);
    let mut error_message = use_signal(|| None::<String>);
    
    // Initialize form with existing user data if editing
    use_effect(move || {
        if let Some(user) = &existing_user_prop {
            firstname.set(user.firstname.clone());
            lastname.set(user.lastname.clone());
            gender.set(user.gender);
            family_head.set(user.family_head);
            email.set(user.email.clone().unwrap_or_default());
            phone.set(user.phone.clone().unwrap_or_default());
            address.set(user.address.clone().unwrap_or_default());
            city.set(user.city.clone().unwrap_or_default());
            country.set(user.country.clone().unwrap_or_default());
            zipcode.set(user.zipcode.clone().unwrap_or_default());
            birthday.set(user.birthday.map(|d| d.to_string()).unwrap_or_default());
            baptism_date.set(user.baptism_date.map(|d| d.to_string()).unwrap_or_default());
            anointed.set(user.anointed.unwrap_or(false));
            publisher_type.set(user.publisher_type.clone());
            appointment.set(user.appointment.clone());
            emergency_contacts.set(user.emergency_contacts.clone());
        }
    });
    
    // Handle save
    let handle_save = move |_| {
        if firstname().trim().is_empty() || lastname().trim().is_empty() {
            error_message.set(Some("First name and last name are required".to_string()));
            return;
        }
        
        is_saving.set(true);
        
        let mode_clone = mode_for_save.clone();
        let existing_user = existing_user_for_save.clone();
        
        let user_data = User {
            id: if let Some(ref user) = existing_user {
                user.id.clone()
            } else {
                // Generate a temporary ID for creation
                "user:temp".parse().unwrap()
            },
            firstname: firstname(),
            lastname: lastname(),
            gender: gender(),
            family_head: family_head(),
            email: if email().is_empty() { None } else { Some(email()) },
            password: if password().is_empty() { None } else { Some(password()) },
            phone: if phone().is_empty() { None } else { Some(phone()) },
            address: if address().is_empty() { None } else { Some(address()) },
            city: if city().is_empty() { None } else { Some(city()) },
            country: if country().is_empty() { None } else { Some(country()) },
            zipcode: if zipcode().is_empty() { None } else { Some(zipcode()) },
            birthday: if birthday().is_empty() {
                None
            } else {
                NaiveDate::parse_from_str(&birthday(), "%Y-%m-%d").ok()
            },
            baptism_date: if baptism_date().is_empty() {
                None
            } else {
                NaiveDate::parse_from_str(&baptism_date(), "%Y-%m-%d").ok()
            },
            anointed: Some(anointed()),
            publisher_type: publisher_type(),
            appointment: appointment(),
            preaching_group: None,
            emergency_contacts: emergency_contacts(),
        };
        
        let on_save_clone = on_save.clone();
        
        spawn(async move {
            let result = match mode_clone {
                ModalMode::Create => User::create(user_data).await,
                ModalMode::Edit => {
                    if let Some(user) = &existing_user {
                        User::update(user.id.clone(), user_data).await
                    } else {
                        Err(surrealdb::Error::Api(surrealdb::error::Api::Query("No user to update".to_string())))
                    }
                }
            };
            
            match result {
                Ok(_) => {
                    is_saving.set(false);
                    on_save_clone.call(());
                },
                Err(_) => {
                    is_saving.set(false);
                    error_message.set(Some("Failed to save user".to_string()));
                }
            }
        });
    };
    
    rsx! {
        div { class: "modal modal-open",
            div { class: "modal-box max-w-4xl max-h-[90vh] overflow-y-auto",
                // Header
                div { class: "flex items-center justify-between mb-6",
                    h3 { class: "font-bold text-2xl",
                        if mode_for_display == ModalMode::Create {
                            "Create New User"
                        } else {
                            "Edit User"
                        }
                    }
                    button {
                        class: "btn btn-sm btn-circle btn-ghost",
                        onclick: move |_| on_close.call(()),
                        "✕"
                    }
                }
                // Error message
                if let Some(err) = error_message() {
                    div { class: "alert alert-error mb-4",
                        span { "{err}" }
                    }
                }
                // Form
                div { class: "space-y-6",
                    // Basic Information
                    // Basic Information
                    div {
                        div {
                            class: "flex items-center justify-between mb-3 cursor-pointer",
                            onclick: move |_| basic_info_collapsed.set(!basic_info_collapsed()),
                            h4 { class: "font-semibold text-lg", "Basic Information" }
                            span { class: "text-lg",
                                if basic_info_collapsed() {
                                    "▼"
                                } else {
                                    "▲"
                                }
                            }
                        }
                        if !basic_info_collapsed() {
                            div { class: "grid grid-cols-1 md:grid-cols-2 gap-4",
                                div { class: "form-control",
                                    label { class: "label",
                                        span { class: "label-text font-semibold", "First Name *" }
                                    }
                                    input {
                                        r#type: "text",
                                        class: "input input-bordered w-full",
                                        value: "{firstname()}",
                                        oninput: move |evt| firstname.set(evt.value()),
                                    }
                                }
                                div { class: "form-control",
                                    label { class: "label",
                                        span { class: "label-text font-semibold", "Last Name *" }
                                    }
                                    input {
                                        r#type: "text",
                                        class: "input input-bordered w-full",
                                        value: "{lastname()}",
                                        oninput: move |evt| lastname.set(evt.value()),
                                    }
                                }
                                div { class: "form-control",
                                    label { class: "label",
                                        span { class: "label-text font-semibold", "Gender" }
                                    }
                                    select {
                                        class: "select select-bordered w-full",
                                        value: if gender() { "male" } else { "female" },
                                        onchange: move |evt| gender.set(evt.value() == "male"),
                                        option { value: "male", "Male" }
                                        option { value: "female", "Female" }
                                    }
                                }
                                div { class: "form-control",
                                    label { class: "label",
                                        span { class: "label-text font-semibold", "Birthday" }
                                    }
                                    input {
                                        r#type: "date",
                                        class: "input input-bordered w-full",
                                        value: "{birthday()}",
                                        oninput: move |evt| birthday.set(evt.value()),
                                    }
                                }
                            }
                            div { class: "grid grid-cols-1 md:grid-cols-2 gap-4 mt-4",
                                div { class: "form-control",
                                    label { class: "label cursor-pointer justify-start gap-2",
                                        input {
                                            r#type: "checkbox",
                                            class: "checkbox checkbox-primary",
                                            checked: family_head(),
                                            onchange: move |evt| family_head.set(evt.checked()),
                                        }
                                        span { class: "label-text", "Family Head" }
                                    }
                                }
                                div { class: "form-control",
                                    label { class: "label cursor-pointer justify-start gap-2",
                                        input {
                                            r#type: "checkbox",
                                            class: "checkbox checkbox-primary",
                                            checked: anointed(),
                                            onchange: move |evt| anointed.set(evt.checked()),
                                        }
                                        span { class: "label-text", "Anointed" }
                                    }
                                }
                            }
                        }
                    }
                    // Contact Information
                    div {
                        div {
                            class: "flex items-center justify-between mb-3 cursor-pointer",
                            onclick: move |_| contact_info_collapsed.set(!contact_info_collapsed()),
                            h4 { class: "font-semibold text-lg", "Contact Information" }
                            span { class: "text-lg",
                                if contact_info_collapsed() {
                                    "▼"
                                } else {
                                    "▲"
                                }
                            }
                        }
                        if !contact_info_collapsed() {
                            div { class: "grid grid-cols-1 md:grid-cols-2 gap-4",
                                div { class: "form-control",
                                    label { class: "label",
                                        span { class: "label-text font-semibold", "Email" }
                                    }
                                    input {
                                        r#type: "email",
                                        class: "input input-bordered w-full",
                                        value: "{email()}",
                                        oninput: move |evt| email.set(evt.value()),
                                    }
                                }
                                div { class: "form-control",
                                    label { class: "label",
                                        span { class: "label-text font-semibold", "Phone" }
                                    }
                                    input {
                                        r#type: "tel",
                                        class: "input input-bordered w-full",
                                        value: "{phone()}",
                                        oninput: move |evt| phone.set(evt.value()),
                                    }
                                }
                                div { class: "form-control md:col-span-2",
                                    label { class: "label",
                                        span { class: "label-text font-semibold", "Address" }
                                    }
                                    input {
                                        r#type: "text",
                                        class: "input input-bordered w-full",
                                        value: "{address()}",
                                        oninput: move |evt| address.set(evt.value()),
                                    }
                                }
                                div { class: "form-control",
                                    label { class: "label",
                                        span { class: "label-text font-semibold", "City" }
                                    }
                                    input {
                                        r#type: "text",
                                        class: "input input-bordered w-full",
                                        value: "{city()}",
                                        oninput: move |evt| city.set(evt.value()),
                                    }
                                }
                                div { class: "form-control",
                                    label { class: "label",
                                        span { class: "label-text font-semibold", "Country" }
                                    }
                                    input {
                                        r#type: "text",
                                        class: "input input-bordered w-full",
                                        value: "{country()}",
                                        oninput: move |evt| country.set(evt.value()),
                                    }
                                }
                                div { class: "form-control",
                                    label { class: "label",
                                        span { class: "label-text font-semibold", "Zip Code" }
                                    }
                                    input {
                                        r#type: "text",
                                        class: "input input-bordered w-full",
                                        value: "{zipcode()}",
                                        oninput: move |evt| zipcode.set(evt.value()),
                                    }
                                }
                            }
                        }
                    }
                    // Emergency Contacts
                    div {
                        div {
                            class: "flex items-center justify-between mb-3 cursor-pointer",
                            onclick: move |_| emergency_contacts_collapsed.set(!emergency_contacts_collapsed()),
                            h4 { class: "font-semibold text-lg", "Emergency Contacts" }
                            span { class: "text-lg",
                                if emergency_contacts_collapsed() {
                                    "▼"
                                } else {
                                    "▲"
                                }
                            }
                        }
                        if !emergency_contacts_collapsed() {
                            div { class: "space-y-4",
                                div { class: "flex justify-end mb-3",
                                    button {
                                        class: "btn btn-primary btn-sm",
                                        r#type: "button",
                                        onclick: move |_| {
                                            let mut contacts = emergency_contacts();
                                            contacts
                                                .push(UserEmergencyContact {
                                                    firstname: String::new(),
                                                    lastname: None,
                                                    email: None,
                                                    phone: None,
                                                    address: None,
                                                    notes: None,
                                                });
                                            emergency_contacts.set(contacts);
                                        },
                                        "➕ Add Contact"
                                    }
                                }
                                for (idx , contact) in emergency_contacts().iter().enumerate() {
                                    {
                                        let contact_clone = contact.clone();
                                        let idx_for_remove = idx;

                                        rsx! {

                                            div { key: "{idx}", class: "card bg-base-200 p-4",
                                                div { class: "flex items-center justify-between mb-3",
                                                    h5 { class: "font-medium", "Contact #{idx + 1}" }
                                                    button {
                                                        class: "btn btn-error btn-xs btn-circle",
                                                        r#type: "button",
                                                        onclick: move |_| {
                                                            let mut contacts = emergency_contacts();
                                                            contacts.remove(idx_for_remove);
                                                            emergency_contacts.set(contacts);
                                                        },
                                                        "✕"
                                                    }
                                                }
                                                div { class: "grid grid-cols-1 md:grid-cols-2 gap-3",
                                                    div { class: "form-control",
                                                        label { class: "label",
                                                            span { class: "label-text", "First Name *" }
                                                        }
                                                        input {
                                                            r#type: "text",
                                                            class: "input input-bordered input-sm w-full",
                                                            value: "{contact_clone.firstname}",
                                                            oninput: move |evt| {
                                                                let mut contacts = emergency_contacts();
                                                                if let Some(c) = contacts.get_mut(idx) {
                                                                    c.firstname = evt.value();
                                                                }
                                                                emergency_contacts.set(contacts);
                                                            },
                                                        }
                                                    }
                                                    div { class: "form-control",
                                                        label { class: "label",
                                                            span { class: "label-text", "Last Name" }
                                                        }
                                                        input {
                                                            r#type: "text",
                                                            class: "input input-bordered input-sm w-full",
                                                            value: "{contact_clone.lastname.clone().unwrap_or_default()}",
                                                            oninput: move |evt| {
                                                                let mut contacts = emergency_contacts();
                                                                if let Some(c) = contacts.get_mut(idx) {
                                                                    c.lastname = if evt.value().is_empty() { None } else { Some(evt.value()) };
                                                                }
                                                                emergency_contacts.set(contacts);
                                                            },
                                                        }
                                                    }
                                                    div { class: "form-control",
                                                        label { class: "label",
                                                            span { class: "label-text", "Phone" }
                                                        }
                                                        input {
                                                            r#type: "tel",
                                                            class: "input input-bordered input-sm w-full",
                                                            value: "{contact_clone.phone.clone().unwrap_or_default()}",
                                                            oninput: move |evt| {
                                                                let mut contacts = emergency_contacts();
                                                                if let Some(c) = contacts.get_mut(idx) {
                                                                    c.phone = if evt.value().is_empty() { None } else { Some(evt.value()) };
                                                                }
                                                                emergency_contacts.set(contacts);
                                                            },
                                                        }
                                                    }
                                                    div { class: "form-control",
                                                        label { class: "label",
                                                            span { class: "label-text", "Email" }
                                                        }
                                                        input {
                                                            r#type: "email",
                                                            class: "input input-bordered input-sm w-full",
                                                            value: "{contact_clone.email.clone().unwrap_or_default()}",
                                                            oninput: move |evt| {
                                                                let mut contacts = emergency_contacts();
                                                                if let Some(c) = contacts.get_mut(idx) {
                                                                    c.email = if evt.value().is_empty() { None } else { Some(evt.value()) };
                                                                }
                                                                emergency_contacts.set(contacts);
                                                            },
                                                        }
                                                    }
                                                    div { class: "form-control md:col-span-2",
                                                        label { class: "label",
                                                            span { class: "label-text", "Address" }
                                                        }
                                                        input {
                                                            r#type: "text",
                                                            class: "input input-bordered input-sm w-full",
                                                            value: "{contact_clone.address.clone().unwrap_or_default()}",
                                                            oninput: move |evt| {
                                                                let mut contacts = emergency_contacts();
                                                                if let Some(c) = contacts.get_mut(idx) {
                                                                    c.address = if evt.value().is_empty() { None } else { Some(evt.value()) };
                                                                }
                                                                emergency_contacts.set(contacts);
                                                            },
                                                        }
                                                    }
                                                    div { class: "form-control md:col-span-2",
                                                        label { class: "label",
                                                            span { class: "label-text", "Notes" }
                                                        }
                                                        textarea {
                                                            class: "textarea textarea-bordered w-full",
                                                            rows: 2,
                                                            value: "{contact_clone.notes.clone().unwrap_or_default()}",
                                                            oninput: move |evt| {
                                                                let mut contacts = emergency_contacts();
                                                                if let Some(c) = contacts.get_mut(idx) {
                                                                    c.notes = if evt.value().is_empty() { None } else { Some(evt.value()) };
                                                                }
                                                                emergency_contacts.set(contacts);
                                                            },
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                                if emergency_contacts().is_empty() {
                                    div { class: "text-center text-base-content/50 py-4",
                                        "No emergency contacts added. Click the 'Add Contact' button to add one."
                                    }
                                }
                            }
                        }
                    }
                    // Spiritual Information
                    div {
                        div {
                            class: "flex items-center justify-between mb-3 cursor-pointer",
                            onclick: move |_| spiritual_info_collapsed.set(!spiritual_info_collapsed()),
                            h4 { class: "font-semibold text-lg", "Spiritual Information" }
                            span { class: "text-lg",
                                if spiritual_info_collapsed() {
                                    "▼"
                                } else {
                                    "▲"
                                }
                            }
                        }
                        if !spiritual_info_collapsed() {
                            div { class: "grid grid-cols-1 md:grid-cols-2 gap-4",
                                div { class: "form-control",
                                    label { class: "label",
                                        span { class: "label-text font-semibold", "Publisher Type" }
                                    }
                                    select {
                                        class: "select select-bordered w-full",
                                        onchange: move |evt| {
                                            publisher_type
                                                .set(
                                                    match evt.value().as_str() {
                                                        "student" => Some(UserType::Student),
                                                        "unbaptized" => Some(UserType::UnbaptizedPublisher),
                                                        "baptized" => Some(UserType::BaptizedPublisher),
                                                        "regular_pioneer" => Some(UserType::RegularPioneer),
                                                        "special_pioneer" => Some(UserType::SpecialPioneer),
                                                        "aux_pioneer" => Some(UserType::ContiniousAuxiliaryPioneer),
                                                        _ => None,
                                                    },
                                                );
                                        },
                                        option { value: "", "Select type" }
                                        option {
                                            value: "student",
                                            selected: matches!(publisher_type(), Some(UserType::Student)),
                                            "Student"
                                        }
                                        option {
                                            value: "unbaptized",
                                            selected: matches!(publisher_type(), Some(UserType::UnbaptizedPublisher)),
                                            "Unbaptized Publisher"
                                        }
                                        option {
                                            value: "baptized",
                                            selected: matches!(publisher_type(), Some(UserType::BaptizedPublisher)),
                                            "Baptized Publisher"
                                        }
                                        option {
                                            value: "regular_pioneer",
                                            selected: matches!(publisher_type(), Some(UserType::RegularPioneer)),
                                            "Regular Pioneer"
                                        }
                                        option {
                                            value: "special_pioneer",
                                            selected: matches!(publisher_type(), Some(UserType::SpecialPioneer)),
                                            "Special Pioneer"
                                        }
                                        option {
                                            value: "aux_pioneer",
                                            selected: matches!(publisher_type(), Some(UserType::ContiniousAuxiliaryPioneer)),
                                            "Auxiliary Pioneer"
                                        }
                                    }
                                }
                                div { class: "form-control",
                                    label { class: "label",
                                        span { class: "label-text font-semibold", "Appointment" }
                                    }
                                    select {
                                        class: "select select-bordered w-full",
                                        onchange: move |evt| {
                                            appointment
                                                .set(
                                                    match evt.value().as_str() {
                                                        "elder" => Some(UserAppointment::Elder),
                                                        "ms" => Some(UserAppointment::MinisterialServant),
                                                        _ => None,
                                                    },
                                                );
                                        },
                                        option { value: "", "No appointment" }
                                        option {
                                            value: "elder",
                                            selected: matches!(appointment(), Some(UserAppointment::Elder)),
                                            "Elder"
                                        }
                                        option {
                                            value: "ms",
                                            selected: matches!(appointment(), Some(UserAppointment::MinisterialServant)),
                                            "Ministerial Servant"
                                        }
                                    }
                                }
                                div { class: "form-control",
                                    label { class: "label",
                                        span { class: "label-text font-semibold", "Baptism Date" }
                                    }
                                    input {
                                        r#type: "date",
                                        class: "input input-bordered w-full",
                                        value: "{baptism_date()}",
                                        oninput: move |evt| baptism_date.set(evt.value()),
                                    }
                                }
                            }
                        }
                    }
                    // Password (only for create mode)
                    if mode_for_password == ModalMode::Create {
                        div {
                            div {
                                class: "flex items-center justify-between mb-3 cursor-pointer",
                                onclick: move |_| credentials_collapsed.set(!credentials_collapsed()),
                                h4 { class: "font-semibold text-lg", "Login Credentials" }
                                span { class: "text-lg",
                                    if credentials_collapsed() {
                                        "▼"
                                    } else {
                                        "▲"
                                    }
                                }
                            }
                            if !credentials_collapsed() {
                                div { class: "form-control",
                                    label { class: "label",
                                        span { class: "label-text font-semibold", "Password (optional)" }
                                    }
                                    input {
                                        r#type: "password",
                                        class: "input input-bordered w-full",
                                        placeholder: "Leave empty if not needed",
                                        value: "{password()}",
                                        oninput: move |evt| password.set(evt.value()),
                                    }
                                    label { class: "label",
                                        span { class: "label-text-alt text-base-content/60",
                                            "Password will be hashed securely"
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                // Action buttons
                div { class: "modal-action",
                    button {
                        class: "btn btn-ghost",
                        onclick: move |_| on_close_for_cancel.call(()),
                        disabled: is_saving(),
                        "Cancel"
                    }
                    button {
                        class: "btn btn-primary",
                        onclick: handle_save,
                        disabled: is_saving(),
                        if is_saving() {
                            span { class: "loading loading-spinner" }
                            " Saving..."
                        } else {
                            if mode_for_button == ModalMode::Create {
                                "Create User"
                            } else {
                                "Update User"
                            }
                        }
                    }
                }
            }
        }
    }
}
