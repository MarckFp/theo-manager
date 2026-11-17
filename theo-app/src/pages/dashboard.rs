use crate::mock;
use crate::state::use_app_state;
use dioxus::prelude::*;
use theo_ui::*;
use theo_ui::Action; // explicit: dioxus::prelude also exports an `Action`

/// `/app` home: widgets that the user reorders and resizes.
#[component]
pub fn Dashboard() -> Element {
    let mut state = use_app_state();
    let mut editing = use_signal(|| false);
    let name = state.account().map(|a| a.display_name).unwrap_or_default();
    let first = name.split_whitespace().next().unwrap_or("").to_string();
    let congregation = state.active_congregation().map(|c| c.name).unwrap_or_default();

    rsx! {
        Page {
            title: "Welcome back, {first}",
            subtitle: "{congregation} · Some widgets still show sample data",
            actions: rsx! {
                if editing() {
                    ActionButton { action: Action::ResetLayout, onclick: move |_| state.reset_layout() }
                }
                ActionButton {
                    action: if editing() { Action::Done } else { Action::Customize },
                    variant: if editing() { ButtonVariant::Primary } else { ButtonVariant::Secondary },
                    onclick: move |_| editing.set(!editing()),
                }
            },
            if editing() {
                Alert { tone: Tone::Info, "Drag a widget by its handle to move it. Drag the corner to resize it." }
            }
            theo_ui::Dashboard {
                widgets: state.layout(),
                editing: editing(),
                on_change: move |layout| state.set_layout(layout),
                render: move |id: String| widget(&id, &state),
            }
        }
    }
}

fn widget(id: &str, state: &crate::state::AppState) -> Element {
    let p = state.preferences();
    let meeting = state
        .active_congregation()
        .map(|c| format!("Midweek meeting · {} {}", c.midweek.day, p.format_hhmm(&c.midweek.time)))
        .unwrap_or_default();
    let names = format!(
        "Chairman: {} · Bible reading: {} · Congregation Bible Study: {}",
        p.format_name("John", "Smith"),
        p.format_name("Mark", "Brown"),
        p.format_name("Anna", "Lee")
    );
    match id {
        "next-meeting" => rsx! {
            Stack { gap: Gap::Sm,
                Cluster { justify: Justify::Between,
                    Heading { level: HeadingLevel::H3, "{meeting}" }
                    Badge { tone: Tone::Success, icon: Icon::Check, "All parts assigned" }
                }
                Text { tone: TextTone::Muted, "{names}" }
            }
        },
        "publishers" => {
            let (value, delta) = super::reports::publishers_widget_values(&state.directory());
            rsx! { StatTile { label: "Publishers", value, icon: Icon::Users, delta, trend: Trend::Flat } }
        }
        "reports" => {
            let (label, value, delta, done) = super::reports::reports_widget_values(&state.directory());
            rsx! { StatTile { label, value, icon: Icon::Footprints, delta, trend: if done { Trend::Up } else { Trend::Down } } }
        }
        "assignments" => rsx! { ItemList { items: mock::my_assignments() } },
        "attendance" => rsx! { BarChart { data: mock::attendance(), highlight_last: true } },
        "territories" => rsx! {
            StatTile { label: "Checked out", value: "18 / 42", icon: Icon::Map, delta: "5 overdue", trend: Trend::Down }
        },
        "duties" => rsx! { ItemList { items: mock::duties() } },
        "events" => rsx! { ItemList { items: mock::events() } },
        _ => rsx! { Skeleton {} },
    }
}
