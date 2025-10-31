use dioxus::prelude::*;

#[derive(Props, Clone, PartialEq)]
pub struct StepperProps {
    pub current_step: i32,
    pub total_steps: i32,
}

#[component]
pub fn Stepper(props: StepperProps) -> Element {
    rsx! {
        // Desktop stepper
        div { class: "hidden sm:flex items-center justify-center w-full mb-8",
            {(1..=props.total_steps).map(|step| {
                let is_current = step == props.current_step;
                let is_completed = step < props.current_step;
                
                rsx! {
                    div { 
                        key: "{step}",
                        class: "flex items-center",
                        // Step circle
                        div { 
                            class: if is_current {
                                "flex items-center justify-center w-10 h-10 rounded-full bg-primary text-primary-content font-bold"
                            } else if is_completed {
                                "flex items-center justify-center w-10 h-10 rounded-full bg-success text-success-content"
                            } else {
                                "flex items-center justify-center w-10 h-10 rounded-full bg-base-300 text-base-content"
                            },
                            {
                                if is_completed {
                                    rsx! {
                                        svg { 
                                            class: "w-6 h-6",
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
                                    }
                                } else {
                                    rsx! { "{step}" }
                                }
                            }
                        }
                        
                        // Connector line (don't show after last step)
                        if step < props.total_steps {
                            div { 
                                class: if is_completed {
                                    "w-12 sm:w-16 h-1 bg-success mx-2"
                                } else {
                                    "w-12 sm:w-16 h-1 bg-base-300 mx-2"
                                }
                            }
                        }
                    }
                }
            })}
        }
        
        // Mobile stepper - simple progress indicator
        div { class: "sm:hidden mb-6",
            div { class: "text-center mb-3",
                span { class: "text-sm font-semibold text-base-content/70",
                    "Step {props.current_step} of {props.total_steps}"
                }
            }
            div { class: "w-full bg-base-300 rounded-full h-2",
                div { 
                    class: "bg-primary h-2 rounded-full transition-all duration-300",
                    style: "width: {(props.current_step as f32 / props.total_steps as f32 * 100.0)}%"
                }
            }
        }
    }
}
