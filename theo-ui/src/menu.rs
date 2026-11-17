//! Popovers, the workspace (congregation) switcher and the segmented control.

use crate::button::{Button, ButtonVariant};
use crate::card::Avatar;
use crate::overlay::Dialog;
use crate::icon::{Icon, IconSize, IconView};
use dioxus::prelude::*;

/// Where a popover opens, relative to its trigger.
#[allow(missing_docs)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Placement {
    #[default]
    Below,
    /// Below on small screens, to the right on the icon rail.
    Side,
    /// Below, aligned with the left edge of the trigger (filter menus).
    Start,
    /// Below, aligned with the right edge of the trigger (page action menus).
    End,
}

/// A popover that closes on outside click or Escape.
#[component]
pub fn Popover(
    open: bool,
    onclose: EventHandler<()>,
    #[props(default)] placement: Placement,
    trigger: Element,
    children: Element,
) -> Element {
    let pos = match placement {
        Placement::Below => "left-0 right-0 top-full mt-2 min-w-64",
        Placement::Side => "left-0 top-full mt-2 w-72 md:max-lg:left-full md:max-lg:top-0 md:max-lg:mt-0 md:max-lg:ml-3 lg:right-0",
        Placement::Start => "left-0 top-full mt-2 w-72 max-w-[calc(100vw-2rem)]",
        Placement::End => "right-0 top-full mt-2 w-72 max-w-[calc(100vw-2rem)]",
    };
    rsx! {
        div {
            class: "relative",
            onkeydown: move |e| {
                if e.key() == Key::Escape {
                    onclose.call(())
                }
            },
            {trigger}
            if open {
                div { class: "fixed inset-0 z-40", onclick: move |_| onclose.call(()) }
                div { class: "absolute z-50 {pos} origin-top animate-fade-in rounded-box border border-base-300 bg-base-100 p-1.5 shadow-xl shadow-base-content/10",
                    {children}
                }
            }
        }
    }
}

/// One entry of a [`WorkspaceSwitcher`].
#[derive(Clone, Debug, PartialEq)]
pub struct SwitcherItem {
    /// Identifier sent to `onselect`.
    pub id: String,
    /// Name shown to the user.
    pub name: String,
    /// Icon that tells the kind (for example local or cloud).
    pub kind_icon: Icon,
    /// Text for the kind (for example "On this device").
    pub kind_label: String,
}

/// Dropdown to switch between workspaces (congregations), with a "create" entry at the end.
#[component]
pub fn WorkspaceSwitcher(
    items: Vec<SwitcherItem>,
    active: Option<String>,
    onselect: EventHandler<String>,
    create_label: String,
    oncreate: EventHandler<()>,
    /// Icon-only trigger (mobile top bar).
    #[props(default)]
    compact: bool,
    #[props(default)] placement: Placement,
) -> Element {
    let mut open = use_signal(|| false);
    let current = items.iter().find(|i| Some(&i.id) == active.as_ref()).cloned();
    let name = current.as_ref().map(|c| c.name.clone()).unwrap_or_else(|| "Choose congregation".into());
    let kind = current.as_ref().map(|c| (c.kind_icon, c.kind_label.clone()));
    // Only the sidebar (Placement::Side) becomes an icon rail on tablets; anywhere else, for
    // example on the unlock screen, the switcher keeps its name at every width.
    let rail = placement == Placement::Side;
    let label_class = if compact { "hidden" } else if rail { "flex md:max-lg:hidden" } else { "flex" };
    let trigger_class = if rail {
        "flex w-full items-center gap-2.5 rounded-field border border-base-300 bg-base-100 p-1.5 text-left transition-colors hover:bg-base-200 md:max-lg:justify-center md:max-lg:border-transparent"
    } else {
        "flex w-full items-center gap-2.5 rounded-field border border-base-300 bg-base-100 p-1.5 text-left transition-colors hover:bg-base-200"
    };
    let trigger = rsx! {
        button {
            r#type: "button",
            class: trigger_class,
            "aria-haspopup": "listbox",
            "aria-expanded": if open() { "true" } else { "false" },
            title: "{name}",
            onclick: move |_| open.set(!open()),
            Avatar { name: name.clone() }
            span { class: "{label_class} min-w-0 flex-1 flex-col",
                span { class: "truncate text-sm font-semibold", "{name}" }
                if let Some((icon, label)) = kind.clone() {
                    span { class: "flex items-center gap-1 text-xs text-base-content/60",
                        IconView { icon, size: IconSize::Xs }
                        "{label}"
                    }
                }
            }
            span { class: "{label_class} text-base-content/50", IconView { icon: Icon::ChevronsUpDown, size: IconSize::Sm } }
        }
    };
    rsx! {
        Popover {
            open: open(),
            onclose: move |_| open.set(false),
            placement,
            trigger,
            div { class: "px-2 pb-1 pt-1.5 text-xs font-medium text-base-content/60", "Congregations" }
            ul { role: "listbox", class: "flex flex-col gap-0.5",
                for it in items.iter().cloned() {
                    {
                        let selected = Some(&it.id) == active.as_ref();
                        let id = it.id.clone();
                        rsx! {
                            li { key: "{it.id}",
                                button {
                                    r#type: "button",
                                    role: "option",
                                    "aria-selected": if selected { "true" } else { "false" },
                                    class: "flex w-full items-center gap-2.5 rounded-field px-2 py-2 text-left transition-colors hover:bg-base-200",
                                    onclick: move |_| {
                                        open.set(false);
                                        onselect.call(id.clone());
                                    },
                                    Avatar { name: it.name.clone() }
                                    span { class: "flex min-w-0 flex-1 flex-col",
                                        span { class: "truncate text-sm font-medium", "{it.name}" }
                                        span { class: "flex items-center gap-1 text-xs text-base-content/60",
                                            IconView { icon: it.kind_icon, size: IconSize::Xs }
                                            "{it.kind_label}"
                                        }
                                    }
                                    if selected {
                                        span { class: "text-primary", IconView { icon: Icon::Check, size: IconSize::Sm } }
                                    }
                                }
                            }
                        }
                    }
                }
            }
            div { class: "my-1.5 h-px bg-base-300" }
            button {
                r#type: "button",
                class: "flex w-full items-center gap-2.5 rounded-field px-2 py-2 text-left text-sm font-medium text-primary transition-colors hover:bg-primary/10",
                onclick: move |_| {
                    open.set(false);
                    oncreate.call(());
                },
                span { class: "grid size-8 place-items-center rounded-full border border-dashed border-primary/50",
                    IconView { icon: Icon::Plus, size: IconSize::Sm }
                }
                "{create_label}"
            }
        }
    }
}

/// One option of a [`Segmented`] control.
#[derive(Clone, Debug, PartialEq)]
pub struct Segment {
    /// Value sent to `onchange`.
    pub value: String,
    /// Text shown.
    pub label: String,
    /// Icon shown before the text.
    pub icon: Option<Icon>,
}

/// A segmented control with a sliding indicator.
#[component]
pub fn Segmented(value: String, options: Vec<Segment>, onchange: EventHandler<String>) -> Element {
    let n = options.len().max(1);
    let idx = options.iter().position(|o| o.value == value).unwrap_or(0);
    rsx! {
        div { class: "relative grid rounded-field bg-base-200 p-1", role: "radiogroup",
            style: "grid-template-columns: repeat({n}, minmax(0, 1fr));",
            div {
                class: "absolute bottom-1 left-1 top-1 rounded-[calc(var(--radius-field)-2px)] bg-base-100 shadow-sm transition-transform duration-300 ease-[var(--ease-out-soft)]",
                style: "width: calc((100% - 0.5rem) / {n}); transform: translateX({idx * 100}%);",
            }
            for o in options.iter().cloned() {
                {
                    let selected = o.value == value;
                    let v = o.value.clone();
                    rsx! {
                        button {
                            key: "{o.value}",
                            r#type: "button",
                            role: "radio",
                            "aria-checked": if selected { "true" } else { "false" },
                            class: if selected { "relative z-10 flex items-center justify-center gap-1.5 px-3 py-1.5 text-sm font-medium" } else { "relative z-10 flex items-center justify-center gap-1.5 px-3 py-1.5 text-sm text-base-content/60 hover:text-base-content" },
                            onclick: move |_| onchange.call(v.clone()),
                            if let Some(icon) = o.icon {
                                IconView { icon, size: IconSize::Sm }
                            }
                            "{o.label}"
                        }
                    }
                }
            }
        }
    }
}

/// One entry of a [`MenuButton`].
#[derive(Clone, Debug, PartialEq)]
pub struct MenuItem {
    /// Identifier sent to `onselect`.
    pub id: String,
    /// Text shown.
    pub label: String,
    /// Icon shown before the text.
    pub icon: Icon,
    /// A short second line.
    pub description: Option<String>,
}

impl MenuItem {
    /// Creates an entry.
    pub fn new(id: impl Into<String>, label: impl Into<String>, icon: Icon) -> Self {
        Self { id: id.into(), label: label.into(), icon, description: None }
    }

    /// Adds a second line.
    pub fn with_description(mut self, d: impl Into<String>) -> Self {
        self.description = Some(d.into());
        self
    }
}

/// A button that opens a list of choices (for example "Export" → PDF, DOCX, CSV). Larger screens
/// get a dropdown; phones get a bottom sheet, like the filters, so it never leaves the screen.
#[component]
pub fn MenuButton(
    label: String,
    icon: Icon,
    items: Vec<MenuItem>,
    onselect: EventHandler<String>,
    /// A message above the entries (for example a warning).
    #[props(default)]
    note: Option<String>,
    #[props(default)] variant: ButtonVariant,
    #[props(default = Placement::End)] placement: Placement,
) -> Element {
    let mut open = use_signal(|| false);
    let trigger = rsx! {
        Button {
            variant,
            icon: Some(icon),
            onclick: move |_| open.set(!open()),
            "{label}"
            span { class: if open() { "rotate-180 transition-transform duration-200" } else { "transition-transform duration-200" },
                IconView { icon: Icon::ChevronDown, size: IconSize::Xs }
            }
        }
    };
    let choose = move |id: String| {
        open.set(false);
        onselect.call(id);
    };
    rsx! {
        // sm and up: dropdown.
        div { class: "max-sm:hidden",
            Popover {
                open: open(),
                onclose: move |_| open.set(false),
                placement,
                trigger: trigger.clone(),
                MenuEntries { note: note.clone(), items: items.clone(), onselect: choose }
            }
        }
        // Phones: the same button, and the entries in a bottom sheet.
        div { class: "sm:hidden",
            {trigger}
            Dialog {
                open: open(),
                onclose: move |_| open.set(false),
                title: label.clone(),
                MenuEntries { note, items, onselect: choose }
            }
        }
    }
}

#[component]
fn MenuEntries(note: Option<String>, items: Vec<MenuItem>, onselect: EventHandler<String>) -> Element {
    rsx! {
        if let Some(n) = note {
            div { class: "m-1 mb-1.5 flex items-start gap-2 rounded-field bg-warning/10 p-2 text-xs text-base-content/80",
                span { class: "mt-px text-warning", IconView { icon: Icon::Warning, size: IconSize::Xs } }
                span { "{n}" }
            }
        }
        ul { role: "menu", class: "flex flex-col gap-0.5",
            for it in items.iter().cloned() {
                {
                    let id = it.id.clone();
                    rsx! {
                        li { key: "{it.id}",
                            button {
                                r#type: "button",
                                role: "menuitem",
                                class: "flex w-full items-center gap-3 rounded-field px-2 py-2.5 text-left transition-colors hover:bg-base-200",
                                onclick: move |_| onselect.call(id.clone()),
                                span { class: "grid size-9 shrink-0 place-items-center rounded-field bg-base-200 text-base-content/70",
                                    IconView { icon: it.icon, size: IconSize::Sm }
                                }
                                span { class: "flex min-w-0 flex-col",
                                    span { class: "text-sm font-medium", "{it.label}" }
                                    if let Some(d) = it.description.clone() {
                                        span { class: "text-xs text-base-content/60", "{d}" }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
