use dioxus::prelude::*;

#[derive(Props, Clone, PartialEq)]
pub struct NavigationHeaderProps {
    /// Whether to show the back button
    pub show_back: bool,
    /// Whether to show the home button (hidden on dashboard)
    pub show_home: bool,
    /// Callback for back button
    pub on_back: EventHandler<()>,
    /// Callback for home button
    pub on_home: EventHandler<()>,
}

#[component]
pub fn NavigationHeader(props: NavigationHeaderProps) -> Element {
    rsx! {
        div { class: "flex items-center justify-between mb-6 fade-in",
            // Left side - Back button
            div { class: "flex-none",
                if props.show_back {
                    button {
                        class: "btn btn-circle bg-secondary hover:bg-secondary/80 text-secondary-content shadow-lg hover:shadow-xl transition-all duration-200 hover:scale-110 active:scale-95",
                        onclick: move |_| props.on_back.call(()),
                        title: "Go back",
                        svg {
                            class: "w-6 h-6",
                            xmlns: "http://www.w3.org/2000/svg",
                            fill: "none",
                            view_box: "0 0 24 24",
                            stroke: "currentColor",
                            path {
                                stroke_linecap: "round",
                                stroke_linejoin: "round",
                                stroke_width: "2",
                                d: "M15 19l-7-7 7-7",
                            }
                        }
                    }
                } else {
                    // Empty placeholder to maintain spacing
                    div { class: "w-12" }
                }
            }
            // Right side - Home button
            div { class: "flex-none",
                if props.show_home {
                    button {
                        class: "btn btn-circle bg-primary hover:bg-primary-focus text-primary-content shadow-lg hover:shadow-xl transition-all duration-200 hover:scale-110 active:scale-95",
                        onclick: move |_| props.on_home.call(()),
                        title: "Go to home",
                        svg {
                            class: "w-5 h-5",
                            xmlns: "http://www.w3.org/2000/svg",
                            fill: "none",
                            view_box: "0 0 24 24",
                            stroke: "currentColor",
                            path {
                                stroke_linecap: "round",
                                stroke_linejoin: "round",
                                stroke_width: "2",
                                d: "M3 12l2-2m0 0l7-7 7 7M5 10v10a1 1 0 001 1h3m10-11l2 2m-2-2v10a1 1 0 01-1 1h-3m-6 0a1 1 0 001-1v-4a1 1 0 011-1h2a1 1 0 011 1v4a1 1 0 001 1m-6 0h6",
                            }
                        }
                    }
                }
            }
        }
    }
}
