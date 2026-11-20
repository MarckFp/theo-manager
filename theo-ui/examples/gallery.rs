//! Component gallery (R-414): every component in its variants.
//! Run: `dx serve --example gallery --platform web --features web`

use dioxus::prelude::*;
use theo_ui::*;
use theo_ui::Action; // explicit: dioxus::prelude also exports an `Action`

fn main() {
    dioxus::launch(Gallery);
}

#[component]
fn Gallery() -> Element {
    let mut theme = use_signal(|| Theme::System);
    let mut text = use_signal(String::new);
    let mut pw = use_signal(String::new);
    let mut on = use_signal(|| true);
    let mut mode = use_signal(|| "local".to_string());
    let mut editing = use_signal(|| false);
    let mut widgets = use_signal(|| {
        vec![
            WidgetLayout::new("a", "Stat", Icon::Users, 1, 1),
            WidgetLayout::new("b", "Bars", Icon::Chart, 2, 1),
            WidgetLayout::new("c", "List", Icon::Assignment, 1, 2),
        ]
    });
    rsx! {
        ThemeRoot { theme: theme(),
            Container {
                Stack { gap: Gap::Xl,
                    Page {
                        title: "theo-ui gallery",
                        subtitle: "Every component, in every variant.",
                        actions: rsx! {
                            Segmented {
                                value: theme().as_str(),
                                options: vec![
                                    Segment { value: "light".into(), label: "Light".into(), icon: Some(Icon::Light) },
                                    Segment { value: "dark".into(), label: "Dark".into(), icon: Some(Icon::Dark) },
                                    Segment { value: "system".into(), label: "System".into(), icon: Some(Icon::System) },
                                ],
                                onchange: move |v: String| theme.set(Theme::parse(&v)),
                            }
                        },
                        Card { title: "Buttons",
                            Cluster {
                                Button { variant: ButtonVariant::Primary, "Primary" }
                                Button { "Secondary" }
                                Button { variant: ButtonVariant::Ghost, "Ghost" }
                                Button { variant: ButtonVariant::Destructive, "Destructive" }
                                Button { variant: ButtonVariant::Primary, loading: true, "Loading" }
                                ActionButton { action: Action::Save }
                                ActionButton { action: Action::Delete }
                                IconButton { icon: Icon::Settings, label: "Settings" }
                            }
                        }
                        Card { title: "Form",
                            Form { onsubmit: move |_| {},
                                actions: rsx! { ActionButton { action: Action::Cancel } ActionButton { action: Action::Save, kind: ButtonKind::Submit } },
                                Field { label: "Name", required: true, help: "Help text",
                                    TextInput { value: text(), oninput: move |v| text.set(v), placeholder: "Type here" }
                                }
                                Field { label: "Password", error: if pw().is_empty() { Some("Required".to_string()) } else { None },
                                    PasswordInput { value: pw(), oninput: move |v| pw.set(v), strength: Some((pw().len() / 3).min(4) as u8) }
                                }
                                Switch { checked: on(), onchange: move |v| on.set(v), label: "A switch", description: "With a description" }
                                ChoiceCards {
                                    value: mode(),
                                    onchange: move |v| mode.set(v),
                                    choices: vec![
                                        Choice { value: "local".into(), title: "Local".into(), description: "On this device".into(), icon: Icon::Local },
                                        Choice { value: "cloud".into(), title: "Cloud".into(), description: "On a server".into(), icon: Icon::Cloud },
                                    ],
                                }
                            }
                        }
                        Card { title: "Feedback",
                            Stack {
                                Alert { tone: Tone::Info, title: "Info", "Something to know." }
                                Alert { tone: Tone::Warning, "Be careful." }
                                Cluster { Badge { tone: Tone::Success, "Success" } Badge { tone: Tone::Neutral, icon: Icon::Cloud, "Cloud" } Spinner {} }
                                Skeleton {}
                                EmptyState { icon: Icon::Users, title: "Nothing yet", description: "Empty states look like this." }
                            }
                        }
                        Card { title: "Dashboard",
                            actions: rsx! { ActionButton { action: if editing() { Action::Done } else { Action::Customize }, onclick: move |_| editing.set(!editing()) } },
                            Dashboard {
                                widgets: widgets(),
                                editing: editing(),
                                on_change: move |w| widgets.set(w),
                                render: move |w: WidgetLayout| match w.id.as_str() {
                                    "a" => rsx! { StatTile { label: "Publishers", value: "84", icon: Icon::Users, delta: "+3", trend: Trend::Up } },
                                    "b" => rsx! { BarChart { data: vec![ChartPoint::new("J", 3.0), ChartPoint::new("F", 5.0), ChartPoint::new("M", 4.0)], highlight_last: true } },
                                    _ => rsx! { Skeleton { lines: 5 } },
                                },
                            }
                        }
                    }
                }
            }
        }
    }
}
