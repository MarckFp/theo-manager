use dioxus::prelude::*;

#[derive(Props, Clone, PartialEq)]
pub struct WelcomeStepProps {
    pub on_next: EventHandler<()>,
}

#[component]
pub fn WelcomeStep(props: WelcomeStepProps) -> Element {
    rsx! {
        div { class: "space-y-6",
            // Welcome header
            div { class: "text-center space-y-4",
                h1 { class: "text-3xl sm:text-4xl font-bold text-primary", "Welcome to Theo Manager" }
                div { class: "w-20 h-1 bg-primary mx-auto" }
                p { class: "text-base-content/80 text-base sm:text-lg leading-relaxed px-4",
                    "Your comprehensive congregation management solution. "
                    "Theo Manager helps you organize field service, track meeting attendance, "
                    "manage privileges, and keep your congregation running smoothly."
                }
            }
            // Features highlight
            div { class: "grid grid-cols-1 sm:grid-cols-2 gap-4 mt-8",
                div { class: "flex items-start space-x-3 p-3 rounded-lg bg-base-200",
                    svg {
                        class: "w-6 h-6 text-primary flex-shrink-0 mt-1",
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
                    div {
                        p { class: "font-semibold text-sm sm:text-base", "Easy Setup" }
                        p { class: "text-xs sm:text-sm text-base-content/70",
                            "Get started in minutes with our guided setup"
                        }
                    }
                }
                div { class: "flex items-start space-x-3 p-3 rounded-lg bg-base-200",
                    svg {
                        class: "w-6 h-6 text-primary flex-shrink-0 mt-1",
                        fill: "none",
                        stroke: "currentColor",
                        view_box: "0 0 24 24",
                        path {
                            stroke_linecap: "round",
                            stroke_linejoin: "round",
                            stroke_width: "2",
                            d: "M12 15v2m-6 4h12a2 2 0 002-2v-6a2 2 0 00-2-2H6a2 2 0 00-2 2v6a2 2 0 002 2zm10-10V7a4 4 0 00-8 0v4h8z",
                        }
                    }
                    div {
                        p { class: "font-semibold text-sm sm:text-base", "Secure & Private" }
                        p { class: "text-xs sm:text-sm text-base-content/70",
                            "Your congregation data stays private and secure"
                        }
                    }
                }
                div { class: "flex items-start space-x-3 p-3 rounded-lg bg-base-200",
                    svg {
                        class: "w-6 h-6 text-primary flex-shrink-0 mt-1",
                        fill: "none",
                        stroke: "currentColor",
                        view_box: "0 0 24 24",
                        path {
                            stroke_linecap: "round",
                            stroke_linejoin: "round",
                            stroke_width: "2",
                            d: "M3 15a4 4 0 004 4h9a5 5 0 10-.1-9.999 5.002 5.002 0 10-9.78 2.096A4.001 4.001 0 003 15z",
                        }
                    }
                    div {
                        p { class: "font-semibold text-sm sm:text-base", "Offline or Online" }
                        p { class: "text-xs sm:text-sm text-base-content/70",
                            "Work offline or sync with the cloud"
                        }
                    }
                }
                div { class: "flex items-start space-x-3 p-3 rounded-lg bg-base-200",
                    svg {
                        class: "w-6 h-6 text-primary flex-shrink-0 mt-1",
                        fill: "none",
                        stroke: "currentColor",
                        view_box: "0 0 24 24",
                        path {
                            stroke_linecap: "round",
                            stroke_linejoin: "round",
                            stroke_width: "2",
                            d: "M12 18h.01M8 21h8a2 2 0 002-2V5a2 2 0 00-2-2H8a2 2 0 00-2 2v14a2 2 0 002 2z",
                        }
                    }
                    div {
                        p { class: "font-semibold text-sm sm:text-base", "Mobile Friendly" }
                        p { class: "text-xs sm:text-sm text-base-content/70",
                            "Access from any device, anywhere"
                        }
                    }
                }
            }
            // Action buttons
            div { class: "flex flex-col sm:flex-row gap-3 mt-8 pt-6 border-t border-base-300",
                button {
                    class: "btn btn-outline btn-lg flex-1 order-2 sm:order-1",
                    onclick: move |_| {},
                    svg {
                        class: "w-5 h-5 mr-2",
                        fill: "none",
                        stroke: "currentColor",
                        view_box: "0 0 24 24",
                        path {
                            stroke_linecap: "round",
                            stroke_linejoin: "round",
                            stroke_width: "2",
                            d: "M12 6.253v13m0-13C10.832 5.477 9.246 5 7.5 5S4.168 5.477 3 6.253v13C4.168 18.477 5.754 18 7.5 18s3.332.477 4.5 1.253m0-13C13.168 5.477 14.754 5 16.5 5c1.747 0 3.332.477 4.5 1.253v13C19.832 18.477 18.247 18 16.5 18c-1.746 0-3.332.477-4.5 1.253",
                        }
                    }
                    "Documentation"
                }
                button {
                    class: "btn btn-primary btn-lg flex-1 order-1 sm:order-2",
                    onclick: move |_| props.on_next.call(()),
                    "Get Started"
                    svg {
                        class: "w-5 h-5 ml-2",
                        fill: "none",
                        stroke: "currentColor",
                        view_box: "0 0 24 24",
                        path {
                            stroke_linecap: "round",
                            stroke_linejoin: "round",
                            stroke_width: "2",
                            d: "M13 7l5 5m0 0l-5 5m5-5H6",
                        }
                    }
                }
            }
        }
    }
}
