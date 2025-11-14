use dioxus::prelude::*;

#[derive(Clone, PartialEq, Debug)]
pub enum WorkingMode {
    Offline,
    Online,
}

#[derive(Props, Clone, PartialEq)]
pub struct ModeSelectionStepProps {
    pub on_next: EventHandler<WorkingMode>,
    pub on_back: EventHandler<()>,
}

#[component]
pub fn ModeSelectionStep(props: ModeSelectionStepProps) -> Element {
    let mut selected_mode = use_signal(|| None::<WorkingMode>);

    let handle_submit = move |_| {
        if let Some(mode) = selected_mode() {
            props.on_next.call(mode);
        }
    };

    rsx! {
        div { class: "space-y-6",
            // Header
            div { class: "text-center space-y-2",
                h2 { class: "text-2xl sm:text-3xl font-bold text-base-content",
                    "Choose Your Working Mode"
                }
                p { class: "text-base-content/70 text-sm sm:text-base",
                    "Select how you want to use Theo Manager"
                }
            }
            // Mode selection cards
            div { class: "grid grid-cols-1 md:grid-cols-2 gap-4 mt-8",
                // Offline Mode
                div {
                    class: if selected_mode() == Some(WorkingMode::Offline) { "card bg-primary text-primary-content shadow-lg cursor-pointer border-2 border-primary transform scale-105 transition-all" } else { "card bg-base-100 border-2 border-base-300 shadow-md cursor-pointer hover:border-primary hover:shadow-lg transition-all" },
                    onclick: move |_| selected_mode.set(Some(WorkingMode::Offline)),
                    div { class: "card-body p-6",
                        div { class: "flex items-center justify-between mb-4",
                            h3 { class: "card-title text-xl",
                                svg {
                                    class: "w-6 h-6 mr-2",
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
                                "Offline Mode"
                            }
                            if selected_mode() == Some(WorkingMode::Offline) {
                                div { class: "badge badge-secondary",
                                    svg {
                                        class: "w-4 h-4",
                                        fill: "none",
                                        stroke: "currentColor",
                                        view_box: "0 0 24 24",
                                        path {
                                            stroke_linecap: "round",
                                            stroke_linejoin: "round",
                                            stroke_width: "2",
                                            d: "M5 13l4 4L19 7",
                                        }
                                    }
                                }
                            }
                        }
                        p { class: "mb-4 text-sm opacity-90",
                            "All data stored locally on your device. Perfect for single-device usage."
                        }
                        div { class: "space-y-2",
                            div { class: "flex items-start gap-2",
                                svg {
                                    class: "w-5 h-5 mt-0.5 flex-shrink-0",
                                    fill: "none",
                                    stroke: "currentColor",
                                    view_box: "0 0 24 24",
                                    path {
                                        stroke_linecap: "round",
                                        stroke_linejoin: "round",
                                        stroke_width: "2",
                                        d: "M5 13l4 4L19 7",
                                    }
                                }
                                span { class: "text-sm", "No internet required" }
                            }
                            div { class: "flex items-start gap-2",
                                svg {
                                    class: "w-5 h-5 mt-0.5 flex-shrink-0",
                                    fill: "none",
                                    stroke: "currentColor",
                                    view_box: "0 0 24 24",
                                    path {
                                        stroke_linecap: "round",
                                        stroke_linejoin: "round",
                                        stroke_width: "2",
                                        d: "M5 13l4 4L19 7",
                                    }
                                }
                                span { class: "text-sm", "Complete privacy" }
                            }
                            div { class: "flex items-start gap-2",
                                svg {
                                    class: "w-5 h-5 mt-0.5 flex-shrink-0",
                                    fill: "none",
                                    stroke: "currentColor",
                                    view_box: "0 0 24 24",
                                    path {
                                        stroke_linecap: "round",
                                        stroke_linejoin: "round",
                                        stroke_width: "2",
                                        d: "M5 13l4 4L19 7",
                                    }
                                }
                                span { class: "text-sm", "Fast performance" }
                            }
                            div { class: "flex items-start gap-2",
                                svg {
                                    class: "w-5 h-5 mt-0.5 flex-shrink-0",
                                    fill: "none",
                                    stroke: "currentColor",
                                    view_box: "0 0 24 24",
                                    path {
                                        stroke_linecap: "round",
                                        stroke_linejoin: "round",
                                        stroke_width: "2",
                                        d: "M6 18L18 6M6 6l12 12",
                                    }
                                }
                                span { class: "text-sm", "No sync across devices" }
                            }
                            div { class: "flex items-start gap-2",
                                svg {
                                    class: "w-5 h-5 mt-0.5 flex-shrink-0",
                                    fill: "none",
                                    stroke: "currentColor",
                                    view_box: "0 0 24 24",
                                    path {
                                        stroke_linecap: "round",
                                        stroke_linejoin: "round",
                                        stroke_width: "2",
                                        d: "M6 18L18 6M6 6l12 12",
                                    }
                                }
                                span { class: "text-sm", "Manual backups required" }
                            }
                        }
                    }
                }
                // Online Mode
                div {
                    class: if selected_mode() == Some(WorkingMode::Online) { "card bg-primary text-primary-content shadow-lg cursor-pointer border-2 border-primary transform scale-105 transition-all" } else { "card bg-base-100 border-2 border-base-300 shadow-md cursor-pointer hover:border-primary hover:shadow-lg transition-all" },
                    onclick: move |_| selected_mode.set(Some(WorkingMode::Online)),
                    div { class: "card-body p-6",
                        div { class: "flex items-center justify-between mb-4",
                            h3 { class: "card-title text-xl",
                                svg {
                                    class: "w-6 h-6 mr-2",
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
                                "Online Mode"
                            }
                            if selected_mode() == Some(WorkingMode::Online) {
                                div { class: "badge badge-secondary",
                                    svg {
                                        class: "w-4 h-4",
                                        fill: "none",
                                        stroke: "currentColor",
                                        view_box: "0 0 24 24",
                                        path {
                                            stroke_linecap: "round",
                                            stroke_linejoin: "round",
                                            stroke_width: "2",
                                            d: "M5 13l4 4L19 7",
                                        }
                                    }
                                }
                            }
                        }
                        p { class: "mb-4 text-sm opacity-90",
                            "Data synced to the cloud. Access from anywhere, anytime."
                        }
                        div { class: "space-y-2",
                            div { class: "flex items-start gap-2",
                                svg {
                                    class: "w-5 h-5 mt-0.5 flex-shrink-0",
                                    fill: "none",
                                    stroke: "currentColor",
                                    view_box: "0 0 24 24",
                                    path {
                                        stroke_linecap: "round",
                                        stroke_linejoin: "round",
                                        stroke_width: "2",
                                        d: "M5 13l4 4L19 7",
                                    }
                                }
                                span { class: "text-sm", "Sync across devices" }
                            }
                            div { class: "flex items-start gap-2",
                                svg {
                                    class: "w-5 h-5 mt-0.5 flex-shrink-0",
                                    fill: "none",
                                    stroke: "currentColor",
                                    view_box: "0 0 24 24",
                                    path {
                                        stroke_linecap: "round",
                                        stroke_linejoin: "round",
                                        stroke_width: "2",
                                        d: "M5 13l4 4L19 7",
                                    }
                                }
                                span { class: "text-sm", "Automatic backups" }
                            }
                            div { class: "flex items-start gap-2",
                                svg {
                                    class: "w-5 h-5 mt-0.5 flex-shrink-0",
                                    fill: "none",
                                    stroke: "currentColor",
                                    view_box: "0 0 24 24",
                                    path {
                                        stroke_linecap: "round",
                                        stroke_linejoin: "round",
                                        stroke_width: "2",
                                        d: "M5 13l4 4L19 7",
                                    }
                                }
                                span { class: "text-sm", "Access from anywhere" }
                            }
                            div { class: "flex items-start gap-2",
                                svg {
                                    class: "w-5 h-5 mt-0.5 flex-shrink-0",
                                    fill: "none",
                                    stroke: "currentColor",
                                    view_box: "0 0 24 24",
                                    path {
                                        stroke_linecap: "round",
                                        stroke_linejoin: "round",
                                        stroke_width: "2",
                                        d: "M6 18L18 6M6 6l12 12",
                                    }
                                }
                                span { class: "text-sm", "Internet required" }
                            }
                            div { class: "flex items-start gap-2",
                                svg {
                                    class: "w-5 h-5 mt-0.5 flex-shrink-0",
                                    fill: "none",
                                    stroke: "currentColor",
                                    view_box: "0 0 24 24",
                                    path {
                                        stroke_linecap: "round",
                                        stroke_linejoin: "round",
                                        stroke_width: "2",
                                        d: "M6 18L18 6M6 6l12 12",
                                    }
                                }
                                span { class: "text-sm", "Requires account setup" }
                            }
                        }
                    }
                }
            }
            // Info note
            if selected_mode().is_some() {
                div { class: "alert alert-info mt-4",
                    svg {
                        class: "stroke-current shrink-0 h-6 w-6",
                        fill: "none",
                        view_box: "0 0 24 24",
                        path {
                            stroke_linecap: "round",
                            stroke_linejoin: "round",
                            stroke_width: "2",
                            d: "M13 16h-1v-4h-1m1-4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z",
                        }
                    }
                    span { class: "text-sm",
                        "You can change this setting later in the application preferences."
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
                            d: "M11 17l-5-5m0 0l5-5m-5 5h12",
                        }
                    }
                    "Back"
                }
                button {
                    class: if selected_mode().is_some() { "btn btn-primary btn-lg flex-1" } else { "btn btn-primary btn-lg flex-1 btn-disabled" },
                    disabled: selected_mode().is_none(),
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
                            d: "M13 7l5 5m0 0l-5 5m5-5H6",
                        }
                    }
                }
            }
        }
    }
}
