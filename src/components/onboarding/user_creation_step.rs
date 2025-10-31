use dioxus::prelude::*;
use crate::database::models::user::{User, UserType, UserAppointment};

#[derive(Props, Clone, PartialEq)]
pub struct UserCreationStepProps {
    pub on_create: EventHandler<User>,
    pub on_back: EventHandler<()>,
}

#[component]
pub fn UserCreationStep(props: UserCreationStepProps) -> Element {
    let mut firstname = use_signal(|| String::new());
    let mut lastname = use_signal(|| String::new());
    let mut email = use_signal(|| String::new());
    let mut password = use_signal(|| String::new());
    let mut confirm_password = use_signal(|| String::new());
    let mut gender = use_signal(|| true); // true = male, false = female
    let mut phone = use_signal(|| String::new());
    
    let mut error_message = use_signal(|| String::new());

    let handle_submit = move |_| {
        // Validation
        if firstname().trim().is_empty() {
            error_message.set("First name is required".to_string());
            return;
        }
        
        if lastname().trim().is_empty() {
            error_message.set("Last name is required".to_string());
            return;
        }
        
        if email().trim().is_empty() {
            error_message.set("Email is required".to_string());
            return;
        }
        
        // Basic email validation
        if !email().contains('@') {
            error_message.set("Please enter a valid email address".to_string());
            return;
        }
        
        if password().trim().is_empty() {
            error_message.set("Password is required".to_string());
            return;
        }
        
        if password().len() < 6 {
            error_message.set("Password must be at least 6 characters".to_string());
            return;
        }
        
        if password() != confirm_password() {
            error_message.set("Passwords do not match".to_string());
            return;
        }
        
        // Create user object
        let user = User {
            id: surrealdb::RecordId::from(("user", email().trim())),
            firstname: firstname().trim().to_string(),
            lastname: lastname().trim().to_string(),
            gender: gender(),
            family_head: true, // Admin user is family head by default
            email: Some(email().trim().to_string()),
            password: Some(password().clone()), // Will be hashed in User::create
            birthday: None,
            phone: if phone().trim().is_empty() { None } else { Some(phone().trim().to_string()) },
            address: None,
            city: None,
            country: None,
            zipcode: None,
            baptism_date: None,
            anointed: None,
            publisher_type: None,
            appointment: None,
            preaching_group: None,
            emergency_contacts: vec![],
        };
        
        error_message.set(String::new());
        props.on_create.call(user);
    };

    rsx! {
        div { class: "space-y-6",
            // Header
            div { class: "text-center space-y-2",
                h2 { class: "text-2xl sm:text-3xl font-bold text-base-content",
                    "Create Your Account"
                }
                p { class: "text-base-content/70 text-sm sm:text-base",
                    "Set up your administrator account to get started"
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
                // Name fields
                div { class: "grid grid-cols-1 sm:grid-cols-2 gap-4",
                    div { class: "form-control",
                        label { class: "label",
                            span { class: "label-text font-semibold", "First Name *" }
                        }
                        input {
                            r#type: "text",
                            class: "input input-bordered w-full",
                            placeholder: "Enter first name",
                            value: "{firstname}",
                            oninput: move |e| firstname.set(e.value().clone())
                        }
                    }
                    
                    div { class: "form-control",
                        label { class: "label",
                            span { class: "label-text font-semibold", "Last Name *" }
                        }
                        input {
                            r#type: "text",
                            class: "input input-bordered w-full",
                            placeholder: "Enter last name",
                            value: "{lastname}",
                            oninput: move |e| lastname.set(e.value().clone())
                        }
                    }
                }
                
                // Gender
                div { class: "form-control",
                    label { class: "label",
                        span { class: "label-text font-semibold", "Gender" }
                    }
                    div { class: "flex gap-4",
                        label { class: "label cursor-pointer gap-2 flex-1 justify-start",
                            input {
                                r#type: "radio",
                                class: "radio radio-primary",
                                name: "gender",
                                checked: gender(),
                                onchange: move |_| gender.set(true)
                            }
                            span { class: "label-text", "Male" }
                        }
                        label { class: "label cursor-pointer gap-2 flex-1 justify-start",
                            input {
                                r#type: "radio",
                                class: "radio radio-primary",
                                name: "gender",
                                checked: !gender(),
                                onchange: move |_| gender.set(false)
                            }
                            span { class: "label-text", "Female" }
                        }
                    }
                }
                
                // Email
                div { class: "form-control",
                    label { class: "label",
                        span { class: "label-text font-semibold", "Email *" }
                    }
                    input {
                        r#type: "email",
                        class: "input input-bordered w-full",
                        placeholder: "your.email@example.com",
                        value: "{email}",
                        oninput: move |e| email.set(e.value().clone())
                    }
                }
                
                // Phone (optional)
                div { class: "form-control",
                    label { class: "label",
                        span { class: "label-text font-semibold", "Phone (Optional)" }
                    }
                    input {
                        r#type: "tel",
                        class: "input input-bordered w-full",
                        placeholder: "+1 234 567 8900",
                        value: "{phone}",
                        oninput: move |e| phone.set(e.value().clone())
                    }
                }
                
                // Password fields
                div { class: "grid grid-cols-1 sm:grid-cols-2 gap-4",
                    div { class: "form-control",
                        label { class: "label",
                            span { class: "label-text font-semibold", "Password *" }
                        }
                        input {
                            r#type: "password",
                            class: "input input-bordered w-full",
                            placeholder: "Min. 6 characters",
                            value: "{password}",
                            oninput: move |e| password.set(e.value().clone())
                        }
                        label { class: "label",
                            span { class: "label-text-alt text-base-content/60", "At least 6 characters" }
                        }
                    }
                    
                    div { class: "form-control",
                        label { class: "label",
                            span { class: "label-text font-semibold", "Confirm Password *" }
                        }
                        input {
                            r#type: "password",
                            class: "input input-bordered w-full",
                            placeholder: "Re-enter password",
                            value: "{confirm_password}",
                            oninput: move |e| confirm_password.set(e.value().clone())
                        }
                    }
                }
                
                // Info note
                div { class: "alert alert-info",
                    svg { 
                        class: "stroke-current shrink-0 h-6 w-6",
                        fill: "none",
                        view_box: "0 0 24 24",
                        path { 
                            stroke_linecap: "round",
                            stroke_linejoin: "round",
                            stroke_width: "2",
                            d: "M13 16h-1v-4h-1m1-4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z"
                        }
                    }
                    span { class: "text-sm",
                        "This account will have full administrator privileges. You can add more users later."
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
                    svg { 
                        class: "w-5 h-5 mr-2",
                        fill: "none",
                        stroke: "currentColor",
                        view_box: "0 0 24 24",
                        path { 
                            stroke_linecap: "round",
                            stroke_linejoin: "round",
                            stroke_width: "2",
                            d: "M5 13l4 4L19 7"
                        }
                    }
                    "Create & Finish"
                }
            }
        }
    }
}
