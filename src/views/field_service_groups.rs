use dioxus::prelude::*;
use crate::database::models::field_service_group::FieldServiceGroup;
use crate::database::models::user::User;
use crate::database::models::congregation::{Congregation, NameOrder};
use surrealdb::sql::Thing;

// Helper function to normalize strings by removing accents
fn normalize_string(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            'à' | 'á' | 'â' | 'ã' | 'ä' | 'å' => 'a',
            'è' | 'é' | 'ê' | 'ë' => 'e',
            'ì' | 'í' | 'î' | 'ï' => 'i',
            'ò' | 'ó' | 'ô' | 'õ' | 'ö' => 'o',
            'ù' | 'ú' | 'û' | 'ü' => 'u',
            'ñ' => 'n',
            'ç' => 'c',
            'À' | 'Á' | 'Â' | 'Ã' | 'Ä' | 'Å' => 'A',
            'È' | 'É' | 'Ê' | 'Ë' => 'E',
            'Ì' | 'Í' | 'Î' | 'Ï' => 'I',
            'Ò' | 'Ó' | 'Ô' | 'Õ' | 'Ö' => 'O',
            'Ù' | 'Ú' | 'Û' | 'Ü' => 'U',
            'Ñ' => 'N',
            'Ç' => 'C',
            _ => c,
        })
        .collect::<String>()
        .to_lowercase()
}

// Helper function for natural sorting (handles numbers in strings correctly)
fn natural_sort_key(s: &str) -> Vec<(bool, String)> {
    let mut result = Vec::new();
    let mut current = String::new();
    let mut is_digit = false;
    
    for ch in s.chars() {
        let ch_is_digit = ch.is_ascii_digit();
        
        if current.is_empty() {
            is_digit = ch_is_digit;
            current.push(ch);
        } else if is_digit == ch_is_digit {
            current.push(ch);
        } else {
            if is_digit {
                // Pad numbers with zeros for proper sorting
                result.push((true, format!("{:0>10}", current)));
            } else {
                result.push((false, current.to_lowercase()));
            }
            current.clear();
            current.push(ch);
            is_digit = ch_is_digit;
        }
    }
    
    if !current.is_empty() {
        if is_digit {
            result.push((true, format!("{:0>10}", current)));
        } else {
            result.push((false, current.to_lowercase()));
        }
    }
    
    result
}

#[derive(Props, Clone, PartialEq)]
pub struct FieldServiceGroupsProps {
    pub on_navigate: EventHandler<String>,
}

#[component]
pub fn FieldServiceGroups(props: FieldServiceGroupsProps) -> Element {
    // State for groups list
    let mut groups = use_signal(|| Vec::<FieldServiceGroup>::new());
    let mut filtered_groups = use_signal(|| Vec::<FieldServiceGroup>::new());
    let mut all_users = use_signal(|| Vec::<User>::new());
    let mut congregation = use_signal(|| Option::<Congregation>::None);
    
    // Search
    let mut search_query = use_signal(|| String::new());
    
    // Modal state
    let mut show_modal = use_signal(|| false);
    let mut modal_mode = use_signal(|| ModalMode::Create);
    let mut editing_group = use_signal(|| None::<FieldServiceGroup>);
    
    // Confirmation dialogs
    let mut show_delete_confirm = use_signal(|| false);
    let mut deleting_group_id = use_signal(|| None::<String>);
    
    // Messages
    let mut message = use_signal(|| None::<String>);
    
    // Load groups and users on mount
    let load_groups = move || {
        spawn(async move {
            match FieldServiceGroup::all().await {
                Ok(all_groups) => {
                    groups.set(all_groups.clone());
                    filtered_groups.set(all_groups);
                },
                Err(_) => {
                    message.set(Some("Failed to load field service groups".to_string()));
                }
            }
        });
    };
    
    let load_users = move || {
        spawn(async move {
            match User::all().await {
                Ok(mut users) => {
                    // Sort users based on congregation settings
                    if let Some(cong) = congregation() {
                        users.sort_by(|a, b| {
                            match cong.name_order {
                                NameOrder::FirstnameLastname => {
                                    let name_a = format!("{} {}", a.firstname.to_lowercase(), a.lastname.to_lowercase());
                                    let name_b = format!("{} {}", b.firstname.to_lowercase(), b.lastname.to_lowercase());
                                    name_a.cmp(&name_b)
                                },
                                NameOrder::LastnameFirstname => {
                                    let name_a = format!("{} {}", a.lastname.to_lowercase(), a.firstname.to_lowercase());
                                    let name_b = format!("{} {}", b.lastname.to_lowercase(), b.firstname.to_lowercase());
                                    name_a.cmp(&name_b)
                                }
                            }
                        });
                    } else {
                        // Default: sort by firstname lastname
                        users.sort_by(|a, b| {
                            let name_a = format!("{} {}", a.firstname.to_lowercase(), a.lastname.to_lowercase());
                            let name_b = format!("{} {}", b.firstname.to_lowercase(), b.lastname.to_lowercase());
                            name_a.cmp(&name_b)
                        });
                    }
                    all_users.set(users);
                },
                Err(_) => {}
            }
        });
    };
    
    let load_congregation = move || {
        spawn(async move {
            match Congregation::all().await {
                Ok(congs) => {
                    congregation.set(congs.first().cloned());
                },
                Err(_) => {}
            }
        });
    };
    
    use_effect(move || {
        load_congregation();
        load_groups();
        load_users();
    });
    
    // Apply search filter
    let mut apply_search = move || {
        let query = normalize_string(&search_query());
        let mut filtered: Vec<FieldServiceGroup> = if query.is_empty() {
            groups()
        } else {
            // Search in group name, supervisor, auxiliary, and members
            groups().into_iter().filter(|group| {
                // Search by group name
                if normalize_string(&group.name).contains(&query) {
                    return true;
                }
                
                // Search by supervisor name
                if let Some(supervisor_thing) = &group.supervisor {
                    let thing_str = supervisor_thing.to_string();
                    let thing_id = thing_str.split(':').last().unwrap_or(&thing_str);
                    
                    if let Some(user) = all_users().iter().find(|u| {
                        if let Some(ref id) = u.id {
                            let user_id_str = id.to_string();
                            let user_id = user_id_str.split(':').last().unwrap_or(&user_id_str);
                            user_id == thing_id
                        } else {
                            false
                        }
                    }) {
                        let full_name = format!("{} {}", user.firstname, user.lastname);
                        if normalize_string(&full_name).contains(&query) {
                            return true;
                        }
                    }
                }
                
                // Search by auxiliary name
                if let Some(auxiliary_thing) = &group.auxiliar {
                    let thing_str = auxiliary_thing.to_string();
                    let thing_id = thing_str.split(':').last().unwrap_or(&thing_str);
                    
                    if let Some(user) = all_users().iter().find(|u| {
                        if let Some(ref id) = u.id {
                            let user_id_str = id.to_string();
                            let user_id = user_id_str.split(':').last().unwrap_or(&user_id_str);
                            user_id == thing_id
                        } else {
                            false
                        }
                    }) {
                        let full_name = format!("{} {}", user.firstname, user.lastname);
                        if normalize_string(&full_name).contains(&query) {
                            return true;
                        }
                    }
                }
                
                // Search by member names
                for member_thing in &group.members {
                    let thing_str = member_thing.to_string();
                    let thing_id = thing_str.split(':').last().unwrap_or(&thing_str);
                    
                    if let Some(user) = all_users().iter().find(|u| {
                        if let Some(ref id) = u.id {
                            let user_id_str = id.to_string();
                            let user_id = user_id_str.split(':').last().unwrap_or(&user_id_str);
                            user_id == thing_id
                        } else {
                            false
                        }
                    }) {
                        let full_name = format!("{} {}", user.firstname, user.lastname);
                        if normalize_string(&full_name).contains(&query) {
                            return true;
                        }
                    }
                }
                
                false
            }).collect()
        };
        
        // Sort groups alphabetically by name using natural sort
        filtered.sort_by(|a, b| natural_sort_key(&a.name).cmp(&natural_sort_key(&b.name)));
        
        filtered_groups.set(filtered);
    };
    
    // Apply sorting when groups change
    use_effect(move || {
        if !groups().is_empty() {
            apply_search();
        }
    });
    
    // Handle create
    let handle_create = move |_| {
        modal_mode.set(ModalMode::Create);
        editing_group.set(None);
        show_modal.set(true);
    };
    
    // Handle edit
    let mut handle_edit = move |group: FieldServiceGroup| {
        modal_mode.set(ModalMode::Edit);
        editing_group.set(Some(group));
        show_modal.set(true);
    };
    
    // Handle delete
    let mut handle_delete = move |group_id: String| {
        deleting_group_id.set(Some(group_id));
        show_delete_confirm.set(true);
    };
    
    let confirm_delete = move |_| {
        if let Some(id_str) = deleting_group_id() {
            spawn(async move {
                // Parse the ID (id_str already contains table:id format)
                if let Ok(record_id) = id_str.parse() {
                    match FieldServiceGroup::delete(record_id).await {
                        Ok(_) => {
                            message.set(Some("Group deleted successfully".to_string()));
                            load_groups();
                        },
                        Err(_) => {
                            message.set(Some("Failed to delete group".to_string()));
                        }
                    }
                }
                show_delete_confirm.set(false);
                deleting_group_id.set(None);
            });
        }
    };
    
    // Get user by Thing reference
    let get_user_name = move |user_thing: &Option<Thing>| -> String {
        if let Some(thing) = user_thing {
            let thing_str = thing.to_string();
            // Extract the ID part after the colon
            let thing_id = thing_str.split(':').last().unwrap_or(&thing_str);
            
            if let Some(user) = all_users().iter().find(|u| {
                if let Some(ref id) = u.id {
                    let user_id_str = id.to_string();
                    // Extract the ID part after the colon (or use the whole string if no colon)
                    let user_id = user_id_str.split(':').last().unwrap_or(&user_id_str);
                    user_id == thing_id
                } else {
                    false
                }
            }) {
                return format!("{} {}", user.firstname, user.lastname);
            }
        }
        "Not assigned".to_string()
    };
    
    // Count members
    let count_members = move |members: &Vec<Thing>| -> usize {
        members.len()
    };
    
    // Get sorted member names based on congregation settings
    let get_sorted_member_names = move |members: &Vec<Thing>| -> Vec<String> {
        let mut member_names: Vec<String> = members.iter().filter_map(|member_thing| {
            let thing_str = member_thing.to_string();
            let thing_id = thing_str.split(':').last().unwrap_or(&thing_str);
            
            all_users().iter().find(|u| {
                if let Some(ref id) = u.id {
                    let user_id_str = id.to_string();
                    let user_id = user_id_str.split(':').last().unwrap_or(&user_id_str);
                    user_id == thing_id
                } else {
                    false
                }
            }).map(|user| {
                // Format name based on congregation settings
                if let Some(ref cong) = congregation() {
                    match cong.name_order {
                        NameOrder::LastnameFirstname => format!("{}, {}", user.lastname, user.firstname),
                        NameOrder::FirstnameLastname => format!("{} {}", user.firstname, user.lastname),
                    }
                } else {
                    format!("{} {}", user.firstname, user.lastname)
                }
            })
        }).collect();
        
        // Sort alphabetically
        member_names.sort_by(|a, b| a.to_lowercase().cmp(&b.to_lowercase()));
        member_names
    };
    
    rsx! {
        div { class: "space-y-6 max-w-7xl mx-auto w-full px-2 sm:px-0",
            // Success message
            if let Some(msg) = message() {
                div { class: "alert alert-success shadow-lg mb-4",
                    span { "{msg}" }
                    button {
                        class: "btn btn-sm btn-ghost",
                        onclick: move |_| message.set(None),
                        "✕"
                    }
                }
            }
            // Header
            div { class: "flex flex-col sm:flex-row justify-between items-start sm:items-center gap-4 mb-6",
                div {
                    h2 { class: "text-3xl font-bold text-base-content", "Field Service Groups" }
                    p { class: "text-base-content/70",
                        "Organize publishers into field service groups"
                    }
                }
                button {
                    class: "hidden sm:flex btn btn-primary gap-2",
                    onclick: handle_create,
                    span { class: "text-lg", "+" }
                    "Create Group"
                }
            }
            // Search bar
            div { class: "form-control w-full mb-6",
                div { class: "input-group",
                    input {
                        r#type: "text",
                        placeholder: "Search by group name or publisher name...",
                        class: "input input-bordered flex-1",
                        value: "{search_query()}",
                        oninput: move |evt| {
                            search_query.set(evt.value());
                            apply_search();
                        },
                    }
                    button {
                        class: "btn btn-square",
                        onclick: move |_| {
                            search_query.set(String::new());
                            apply_search();
                        },
                        if search_query().is_empty() {
                            "🔍"
                        } else {
                            "✕"
                        }
                    }
                }
            }
            // Groups grid
            if filtered_groups().is_empty() {
                div { class: "text-center py-12",
                    div { class: "text-6xl mb-4", "📋" }
                    p { class: "text-xl text-base-content/70",
                        if search_query().is_empty() {
                            "No field service groups yet. Create your first one!"
                        } else {
                            "No groups found matching your search."
                        }
                    }
                }
            } else {
                div { class: "grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4",
                    for group in filtered_groups() {
                        {
                            if let Some(ref group_id) = group.id {
                                let group_id_str = group_id.to_string();
                                rsx! {
                                    div {
                                        key: "{group_id}",
                                        class: "card bg-base-100 shadow-xl hover:shadow-2xl transition-shadow",
                                        div { class: "card-body",
                                            h3 { class: "card-title text-lg", "{group.name}" }

            

                                            // Members dropdown

                                            div { class: "space-y-2 mt-2",
                                                div { class: "flex items-center gap-2 text-sm",
                                                    span { class: "font-semibold", "👤 Supervisor:" }
                                                    span { class: "text-base-content/70", "{get_user_name(&group.supervisor)}" }
                                                }
                                                div { class: "flex items-center gap-2 text-sm",
                                                    span { class: "font-semibold", "🤝 Auxiliary:" }
                                                    span { class: "text-base-content/70", "{get_user_name(&group.auxiliar)}" }
                                                }
            
                                                {
                                                    let member_names = get_sorted_member_names(&group.members);
                                                    let member_count = member_names.len();
                                                    rsx! {
                                                        div { class: "collapse collapse-arrow bg-base-200 rounded-box",
                                                            input { r#type: "checkbox" }
                                                            div { class: "collapse-title text-sm font-medium flex items-center gap-2",
                                                                span { class: "font-semibold", "👥 Members:" }
                                                                span { class: "badge badge-primary", "{member_count}" }
                                                            }
                                                            div { class: "collapse-content",
                                                                if member_names.is_empty() {
                                                                    p { class: "text-sm text-base-content/70 italic", "No members assigned" }
                                                                } else {
                                                                    ul { class: "list-disc list-inside space-y-1 text-sm",
                                                                        for name in member_names {
                                                                            li { class: "text-base-content/70", "{name}" }
                                                                        }
                                                                    }
                                                                }
                                                            }
                                                        }
                                                    }
                                                }
                                            }
            
                                            div { class: "card-actions justify-end mt-4",
                                                button {
                                                    class: "btn btn-sm btn-ghost",
                                                    onclick: {
                                                        let group_clone = group.clone();
                                                        move |_| handle_edit(group_clone.clone())
                                                    },
                                                    "✏️ Edit"
                                                }
                                                button {
                                                    class: "btn btn-sm btn-error btn-ghost",
                                                    onclick: {
                                                        let group_id_clone = group_id_str.clone();
                                                        move |_| handle_delete(group_id_clone.clone())
                                                    },
                                                    "🗑️ Delete"
                                                }
                                            }
                                        }
                                    }
                                }
                            } else {
                                rsx! {}
                            }
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
            // Group modal (create/edit)
            if show_modal() {
                GroupModal {
                    mode: modal_mode(),
                    group: editing_group(),
                    all_users: all_users(),
                    existing_groups: groups(),
                    on_close: move |_| show_modal.set(false),
                    on_save: move |_| {
                        show_modal.set(false);
                        load_groups();
                        message.set(Some("Group saved successfully".to_string()));
                    },
                }
            }
            // Delete confirmation dialog
            if show_delete_confirm() {
                div { class: "modal modal-open",
                    div { class: "modal-box",
                        h3 { class: "font-bold text-lg mb-4", "Confirm Delete" }
                        p { class: "py-4",
                            "Are you sure you want to delete this group? This action cannot be undone."
                        }
                        div { class: "modal-action w-full flex gap-2",
                            button {
                                class: "btn btn-ghost flex-1",
                                onclick: move |_| show_delete_confirm.set(false),
                                "Cancel"
                            }
                            button {
                                class: "btn btn-error flex-1",
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

#[derive(Clone, PartialEq)]
enum ModalMode {
    Create,
    Edit,
}

#[derive(Props, Clone, PartialEq)]
struct GroupModalProps {
    mode: ModalMode,
    group: Option<FieldServiceGroup>,
    all_users: Vec<User>,
    existing_groups: Vec<FieldServiceGroup>,
    on_close: EventHandler<()>,
    on_save: EventHandler<()>,
}

#[component]
fn GroupModal(props: GroupModalProps) -> Element {
    let mode = props.mode.clone();
    let mode_for_button = mode.clone();
    
    let existing_group = props.group.clone();
    let existing_group_for_save = existing_group.clone();
    let existing_group_for_assigned = existing_group.clone();
    
    let on_close = props.on_close.clone();
    let on_close_for_cancel = on_close.clone();
    let on_save = props.on_save.clone();
    
    let available_users = props.all_users.clone();
    let existing_groups = props.existing_groups.clone();
    
    // Form fields
    let mut group_name = use_signal(|| String::new());
    let mut supervisor_id = use_signal(|| String::new());
    let mut auxiliar_id = use_signal(|| String::new());
    let mut selected_members = use_signal(|| Vec::<String>::new());
    let mut is_saving = use_signal(|| false);
    
    // Search field for filtering members
    let mut members_search = use_signal(|| String::new());
    
    // Initialize form with existing group data
    use_effect(move || {
        if let Some(group) = existing_group.clone() {
            group_name.set(group.name.clone());
            
            if let Some(sup) = &group.supervisor {
                // Extract just the ID part from Thing (last part after colon)
                let thing_str = sup.to_string();
                if let Some(id_part) = thing_str.split(':').last() {
                    supervisor_id.set(id_part.to_string());
                }
            }
            
            if let Some(aux) = &group.auxiliar {
                // Extract just the ID part from Thing (last part after colon)
                let thing_str = aux.to_string();
                if let Some(id_part) = thing_str.split(':').last() {
                    auxiliar_id.set(id_part.to_string());
                }
            }
            
            let member_ids: Vec<String> = group.members.iter()
                .filter_map(|thing| {
                    // Extract just the ID part from Thing (last part after colon)
                    let thing_str = thing.to_string();
                    thing_str.split(':').last().map(|s| s.to_string())
                })
                .collect();
            selected_members.set(member_ids);
        }
    });
    
    // Clone for each closure to avoid ownership issues
    let existing_groups_for_members = existing_groups.clone();
    let existing_group_for_members = existing_group_for_assigned.clone();
    let existing_groups_for_supervisors = existing_groups.clone();
    let existing_group_for_supervisors = existing_group_for_assigned.clone();
    let existing_groups_for_auxiliaries = existing_groups.clone();
    let existing_group_for_auxiliaries = existing_group_for_assigned.clone();
    
    // Get users already assigned to other groups (only checks members, not supervisor/auxiliary)
    let get_assigned_users = move || -> Vec<String> {
        let mut assigned = Vec::new();
        for group in &existing_groups_for_members {
            // Skip the current group being edited
            if let Some(editing) = &existing_group_for_members {
                // Compare IDs if both are present
                if let (Some(group_id), Some(editing_id)) = (&group.id, &editing.id) {
                    if group_id == editing_id {
                        continue;
                    }
                }
            }
            
            // Only add members to assigned list (supervisor and auxiliary can be in multiple groups)
            for member in &group.members {
                assigned.push(member.id.to_string());
            }
        }
        assigned
    };
    
    // Check if user is already assigned
    let is_user_assigned = move |user_id: &str| -> bool {
        get_assigned_users().contains(&user_id.to_string())
    };
    
    // Get users already assigned as supervisors in other groups
    let get_assigned_supervisors = move || -> Vec<String> {
        let mut assigned = Vec::new();
        for group in &existing_groups_for_supervisors {
            // Skip the current group being edited
            if let Some(editing) = &existing_group_for_supervisors {
                if let (Some(group_id), Some(editing_id)) = (&group.id, &editing.id) {
                    if group_id == editing_id {
                        continue;
                    }
                }
            }
            
            // Add supervisor to assigned list
            if let Some(supervisor_thing) = &group.supervisor {
                let thing_str = supervisor_thing.to_string();
                if let Some(id_part) = thing_str.split(':').last() {
                    assigned.push(id_part.to_string());
                }
            }
        }
        assigned
    };
    
    // Get users already assigned as auxiliaries in other groups
    let get_assigned_auxiliaries = move || -> Vec<String> {
        let mut assigned = Vec::new();
        for group in &existing_groups_for_auxiliaries {
            // Skip the current group being edited
            if let Some(editing) = &existing_group_for_auxiliaries {
                if let (Some(group_id), Some(editing_id)) = (&group.id, &editing.id) {
                    if group_id == editing_id {
                        continue;
                    }
                }
            }
            
            // Add auxiliary to assigned list
            if let Some(auxiliary_thing) = &group.auxiliar {
                let thing_str = auxiliary_thing.to_string();
                if let Some(id_part) = thing_str.split(':').last() {
                    assigned.push(id_part.to_string());
                }
            }
        }
        assigned
    };
    
    // Toggle member selection
    let mut toggle_member = move |user_id: String| {
        let mut current = selected_members();
        if current.contains(&user_id) {
            current.retain(|id| id != &user_id);
        } else {
            current.push(user_id);
        }
        selected_members.set(current);
    };
    
    // Handle save
    let handle_save = move |_| {
        let mode_clone = mode.clone();
        let existing_group_clone = existing_group_for_save.clone();
        spawn(async move {
            is_saving.set(true);
            
            // Ensure supervisor and auxiliary are included in members
            let mut all_member_ids = selected_members();
            if !supervisor_id().is_empty() && !all_member_ids.contains(&supervisor_id()) {
                all_member_ids.push(supervisor_id());
            }
            if !auxiliar_id().is_empty() && !all_member_ids.contains(&auxiliar_id()) {
                all_member_ids.push(auxiliar_id());
            }
            
            // Convert member IDs to Things
            let member_things: Vec<Thing> = all_member_ids.iter()
                .filter_map(|id| format!("user:{}", id).parse().ok())
                .collect();
            
            let supervisor_thing: Option<Thing> = if supervisor_id().is_empty() {
                None
            } else {
                format!("user:{}", supervisor_id()).parse().ok()
            };
            
            let auxiliar_thing: Option<Thing> = if auxiliar_id().is_empty() {
                None
            } else {
                format!("user:{}", auxiliar_id()).parse().ok()
            };
            
            let result = match mode_clone {
                ModalMode::Create => {
                    let new_group = FieldServiceGroup {
                        id: None, // Let SurrealDB auto-generate the ID
                        name: group_name(),
                        supervisor: supervisor_thing,
                        auxiliar: auxiliar_thing,
                        members: member_things,
                    };
                    FieldServiceGroup::create(new_group).await
                },
                ModalMode::Edit => {
                    if let Some(group) = existing_group_clone {
                        if let Some(group_id) = group.id.clone() {
                            let updated_group = FieldServiceGroup {
                                id: Some(group_id.clone()),
                                name: group_name(),
                                supervisor: supervisor_thing,
                                auxiliar: auxiliar_thing,
                                members: member_things,
                            };
                            FieldServiceGroup::update(group_id, updated_group).await
                        } else {
                            Err(surrealdb::Error::Api(surrealdb::error::Api::Query("Group has no ID".to_string())))
                        }
                    } else {
                        Err(surrealdb::Error::Api(surrealdb::error::Api::Query("No group to update".to_string())))
                    }
                }
            };
            
            is_saving.set(false);
            
            match result {
                Ok(_) => {
                    on_save.call(());
                },
                Err(_) => {
                    // Error handled in parent
                }
            }
        });
    };
    
    rsx! {
        div { class: "modal modal-open",
            div { class: "modal-box max-w-2xl max-h-[90vh] overflow-y-auto",
                h3 { class: "font-bold text-2xl mb-6",
                    if mode_for_button == ModalMode::Create {
                        "Create Field Service Group"
                    } else {
                        "Edit Field Service Group"
                    }
                }
                div { class: "space-y-6",
                    // Group name
                    div { class: "form-control",
                        label { class: "label",
                            span { class: "label-text font-semibold", "Group Name" }
                        }
                        input {
                            r#type: "text",
                            placeholder: "e.g., Group 1",
                            class: "input input-bordered w-full",
                            value: "{group_name()}",
                            oninput: move |evt| group_name.set(evt.value()),
                        }
                    }
                    // Supervisor selection
                    div { class: "form-control",
                        label { class: "label",
                            span { class: "label-text font-semibold", "Supervisor" }
                        }
                        select {
                            class: "select select-bordered w-full",
                            value: "{supervisor_id()}",
                            onchange: move |evt| {
                                let new_id = evt.value();
                                supervisor_id.set(new_id.clone());
                                // Automatically add supervisor to members if not empty
                                if !new_id.is_empty() && !selected_members().contains(&new_id) {
                                    let mut members = selected_members();
                                    members.push(new_id);
                                    selected_members.set(members);
                                }
                            },
                            option { value: "", "-- Select Supervisor --" }
                            for user in &available_users {
                                {
                                    // Only show male users for supervisor role who are not already supervisors in other groups
                                    if user.gender && user.id.is_some() {
                                        if let Some(ref user_id) = user.id {
                                            // Extract just the ID part (after colon) to match what we store
                                            let id_str = user_id.to_string();
                                            let id_only = id_str.split(':').last().unwrap_or(&id_str);

                                            // Check if this user is already a supervisor in another group
                                            let is_assigned_supervisor = get_assigned_supervisors()

                                                .contains(&id_only.to_string());
                                            if !is_assigned_supervisor {
                                                rsx! {
                                                    option { value: "{id_only}", "{user.firstname} {user.lastname}" }
                                                }
                                            } else {
                                                rsx! {}
                                            }
                                        } else {
                                            rsx! {
                                                option {}
                                            }
                                        }
                                    } else {
                                        rsx! {}
                                    }
                                }
                            }
                        }
                    }
                    // Auxiliary selection
                    div { class: "form-control",
                        label { class: "label",
                            span { class: "label-text font-semibold", "Auxiliary" }
                        }
                        select {
                            class: "select select-bordered w-full",
                            value: "{auxiliar_id()}",
                            onchange: move |evt| {
                                let new_id = evt.value();
                                auxiliar_id.set(new_id.clone());
                                // Automatically add auxiliary to members if not empty
                                if !new_id.is_empty() && !selected_members().contains(&new_id) {
                                    let mut members = selected_members();
                                    members.push(new_id);
                                    selected_members.set(members);
                                }
                            },
                            option { value: "", "-- Select Auxiliary --" }
                            for user in &available_users {
                                {
                                    // Only show male users for auxiliary role who are not already auxiliaries in other groups
                                    if user.gender && user.id.is_some() {
                                        if let Some(ref user_id) = user.id {
                                            // Extract just the ID part (after colon) to match what we store
                                            let id_str = user_id.to_string();
                                            let id_only = id_str.split(':').last().unwrap_or(&id_str);

                                            // Check if this user is already an auxiliary in another group
                                            let is_assigned_auxiliary = get_assigned_auxiliaries()

                                                .contains(&id_only.to_string());
                                            if !is_assigned_auxiliary {
                                                rsx! {
                                                    option { value: "{id_only}", "{user.firstname} {user.lastname}" }
                                                }
                                            } else {
                                                rsx! {}
                                            }
                                        } else {
                                            rsx! {
                                                option {}
                                            }
                                        }
                                    } else {
                                        rsx! {}
                                    }
                                }
                            }
                        }
                    }
                    // Members selection
                    div { class: "form-control",
                        label { class: "label",
                            span { class: "label-text font-semibold", "Members" }
                            span { class: "label-text-alt", "({selected_members().len()} selected)" }
                        }
                        input {
                            r#type: "text",
                            placeholder: "Search members...",
                            class: "input input-bordered w-full mb-2",
                            value: "{members_search()}",
                            oninput: move |evt| members_search.set(evt.value()),
                        }
                        div { class: "border border-base-300 rounded-lg p-4 max-h-64 overflow-y-auto space-y-2",
                            for user in &available_users {
                                {
                                    if let Some(ref user_id) = user.id {
                                        let id_str = user_id.to_string();
                                        let id_only = id_str.split(':').last().unwrap_or(&id_str).to_string();
                                        let user_name = format!("{} {}", user.firstname, user.lastname);
                                        let normalized_search = normalize_string(&members_search());
                                        let matches_search = normalized_search.is_empty()

                                            || normalize_string(&user_name).contains(&normalized_search);
                                        if matches_search {
                                            let is_selected = selected_members().contains(&id_only);
                                            let is_assigned = is_user_assigned(&id_only);
                                            rsx! {
                                                div { key: "{id_only}", class: "form-control",
                                                    label {
                                                        class: format!(
                                                            "label cursor-pointer justify-start gap-3 hover:bg-base-200 rounded-lg p-2 {}",
                                                            if is_assigned { "opacity-50" } else { "" },
                                                        ),
                                                        input {
                                                            r#type: "checkbox",
                                                            class: "checkbox checkbox-primary",
                                                            checked: is_selected,
                                                            disabled: is_assigned && !is_selected,
                                                            onchange: {
                                                                let user_id_clone = id_only.clone();
                                                                move |_| toggle_member(user_id_clone.clone())
                                                            },
                                                        }
                                                        span { class: "label-text",
                                                            "{user.firstname} {user.lastname}"
                                                            if is_assigned && !is_selected {
                                                                span { class: "text-xs text-error ml-2", "(Already assigned)" }
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        } else {
                                            rsx! {}
                                        }
                                    } else {
                                        rsx! {}
                                    }
                                }
                            }
                            if available_users.is_empty() {
                                div { class: "text-center text-base-content/50 py-4",
                                    "No users available. Create users first."
                                }
                            }
                        }
                    }
                }
                // Action buttons
                div { class: "modal-action w-full flex gap-2 mt-6",
                    button {
                        class: "btn btn-ghost flex-1",
                        onclick: move |_| on_close_for_cancel.call(()),
                        disabled: is_saving(),
                        "Cancel"
                    }
                    button {
                        class: "btn btn-primary flex-1",
                        onclick: handle_save,
                        disabled: is_saving() || group_name().trim().is_empty(),
                        if is_saving() {
                            span { class: "loading loading-spinner" }
                            " Saving..."
                        } else {
                            if mode_for_button == ModalMode::Create {
                                "Create Group"
                            } else {
                                "Update Group"
                            }
                        }
                    }
                }
            }
        }
    }
}
