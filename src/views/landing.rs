use dioxus::prelude::*;
use crate::components::stepper::Stepper;
use crate::components::onboarding::{WelcomeStep, CongregationStep, ModeSelectionStep, UserCreationStep, WorkingMode};
use crate::database::models::congregation::Congregation;
use crate::database::models::user::User;
use crate::database::models::role::{Role, RoleType};

#[component]
pub fn Landing() -> Element {
    let mut current_step = use_signal(|| 1);
    let mut congregation_data = use_signal(|| None::<Congregation>);
    let mut working_mode = use_signal(|| None::<WorkingMode>);
    let mut is_submitting = use_signal(|| false);
    let mut submission_error = use_signal(|| None::<String>);
    let mut setup_complete = use_signal(|| false);
    const TOTAL_STEPS: i32 = 4;

    rsx! {
        div { class: "min-h-screen w-full flex items-center justify-center bg-base-200 p-4",
            // Centered card container
            div { class: "card w-full max-w-2xl bg-base-100 shadow-xl",
                div { class: "card-body p-6 sm:p-8",
                    // Stepper component
                    Stepper {
                        current_step: current_step(),
                        total_steps: TOTAL_STEPS,
                    }
                    // Step content
                    div { class: "mt-6",
                        match current_step() {
                            1 => rsx! {
                                WelcomeStep {
                                    on_next: move |_| {
                                        current_step.set(2);
                                    },
                                }
                            },
                            2 => rsx! {
                                CongregationStep {
                                    on_next: move |congregation: Congregation| {
                                        congregation_data.set(Some(congregation));
                                        current_step.set(3);
                                    },
                                    on_back: move |_| {
                                        current_step.set(1);
                                    },
                                }
                            },
                            3 => rsx! {
                                ModeSelectionStep {
                                    on_next: move |mode: WorkingMode| {
                                        working_mode.set(Some(mode));
                                        current_step.set(4);
                                    },
                                    on_back: move |_| {
                                        current_step.set(2);
                                    },
                                }
                            },
                            4 => rsx! {
                                if setup_complete() {
                                    div { class: "text-center p-8 space-y-6",
                                        div { class: "flex justify-center",
                                            svg {
                                                class: "w-24 h-24 text-success",
                                                fill: "none",
                                                stroke: "currentColor",
                                                view_box: "0 0 24 24",
                                                path {
                                                    stroke_linecap: "round",
                                                    stroke_linejoin: "round",
                                                    stroke_width: "2",
                                                    d: "M9 12l2 2 4-4m6 2a9 9 0 11-18 0 9 9 0 0118 0z",
                                                }
                                            }
                                        }

                                // Spawn async task to save data

                                // Database is already initialized from main.rs
                                // Just save the data

                                // Save congregation
                                // Save user
                                // Create Owner role for the user
                                // Extract table and id from RecordId

                                // Wait 2 seconds to show success message, then reload
                                // Force a full page reload to re-check database

                                // On desktop, the app will automatically re-render and check congregation data
                                // No need for delay or manual reload

        

                                        h2 { class: "text-3xl font-bold text-base-content", "Setup Complete!" }
                                        p { class: "text-base-content/70", "Your congregation has been successfully configured." }
                                        p { class: "text-sm text-base-content/60", "Refreshing application..." }
                                        span { class: "loading loading-spinner loading-md text-primary" }
                                    }
                                } else if is_submitting() {
                                    div { class: "text-center p-8 space-y-4",
                                        span { class: "loading loading-spinner loading-lg text-primary" }
                                        p { class: "text-lg font-semibold", "Setting up your congregation..." }
                                        p { class: "text-base-content/70 text-sm", "This will only take a moment" }
                                    }
                                } else if let Some(error) = submission_error() {
                                    div { class: "text-center p-8 space-y-4",
                                        div { class: "alert alert-error",
                                            svg {
                                                class: "stroke-current shrink-0 h-6 w-6",
                                                fill: "none",
                                                view_box: "0 0 24 24",
                                                path {
                                                    stroke_linecap: "round",
                                                    stroke_linejoin: "round",
                                                    stroke_width: "2",
                                                    d: "M10 14l2-2m0 0l2-2m-2 2l-2-2m2 2l2 2m7-2a9 9 0 11-18 0 9 9 0 0118 0z",
                                                }
                                            }
                                            div {
                                                p { class: "font-semibold", "Setup Failed" }
                                                p { class: "text-sm", "{error}" }
                                            }
                                        }
                                        button {
                                            class: "btn btn-primary btn-lg",
                                            onclick: move |_| {
                                                submission_error.set(None);
                                                current_step.set(3);
                                            },
                                            "Go Back"
                                        }
                                    }
                                } else {
                                    UserCreationStep {
                                        on_create: move |user: User| {
                                            is_submitting.set(true);
        
                                            let congregation = congregation_data().unwrap();
                                            let _mode = working_mode().unwrap();
        
                                            spawn(async move {
        
                                                match Congregation::create(congregation).await {
                                                    Ok(_) => {
                                                        match User::create(user.clone()).await {
                                                            Ok(created_user) => {
                                                                if let Some(user_id) = &created_user.id {
                                                                    let user_record_string = user_id.to_string();
                                                                    let user_thing = surrealdb::sql::Thing::from((
        
                                                                        "user".to_string(),
                                                                        user_record_string.clone(),
                                                                    ));
                                                                    let owner_role = Role {
                                                                        id: surrealdb::RecordId::from((
                                                                            "role",
                                                                            format!("owner_{}", user_record_string),
                                                                        )),
                                                                        publisher: Some(user_thing),
                                                                        r#type: RoleType::Owner,
                                                                        start_date: None,
                                                                        end_date: None,
                                                                        notes: None,
                                                                    };
                                                                    match Role::create(owner_role).await {
                                                                        Ok(_) => {
                                                                            is_submitting.set(false);
                                                                            setup_complete.set(true);
                                                                            #[cfg(target_arch = "wasm32")]
                                                                            {
                                                                                spawn(async move {
                                                                                    gloo_timers::future::TimeoutFuture::new(2_000).await;
                                                                                    if let Some(window) = web_sys::window() {
                                                                                        let _ = window.location().reload();
                                                                                    }
                                                                                });
                                                                            }
                                                                            #[cfg(not(target_arch = "wasm32"))] {}
                                                                        }
                                                                        Err(e) => {
                                                                            is_submitting.set(false);
                                                                            submission_error
                                                                                .set(
                                                                                    Some(
                                                                                        format!(
                                                                                            "User created but failed to assign Owner role: {}",
                                                                                            e,
                                                                                        ),
                                                                                    ),
                                                                                );
                                                                        }
                                                                    }
                                                                } else {
                                                                    is_submitting.set(false);
                                                                    submission_error
                                                                        .set(Some("User created without ID".to_string()));
                                                                }
                                                            }
                                                            Err(e) => {
                                                                is_submitting.set(false);
                                                                submission_error
                                                                    .set(Some(format!("Failed to create user: {}", e)));
                                                            }
                                                        }
                                                    }
                                                    Err(e) => {
                                                        is_submitting.set(false);
                                                        submission_error
                                                            .set(Some(format!("Failed to create congregation: {}", e)));
                                                    }
                                                }
                                            });
                                        },
                                        on_back: move |_| {
                                            current_step.set(3);
                                        },
                                    }
                                }
                            },
                            _ => rsx! {
                                div { "Invalid step" }
                            },
                        }
                    }
                }
            }
        }
    }
}
