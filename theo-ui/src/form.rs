//! Forms (R-425). `Form` owns layout and buttons; `Field` owns label, help and error.

use crate::icon::{Icon, IconSize, IconView};
use dioxus::prelude::*;

/// A form. Enter submits; the page never reloads. `actions` is rendered in the standard
/// place: bottom right on desktop, full width on phones (R-424).
#[component]
pub fn Form(
    onsubmit: EventHandler<()>,
    #[props(default)] actions: Option<Element>,
    children: Element,
) -> Element {
    rsx! {
        form {
            class: "flex flex-col gap-5",
            novalidate: true,
            onsubmit: move |e| {
                e.prevent_default();
                onsubmit.call(());
            },
            div { class: "flex flex-col gap-4", {children} }
            if let Some(a) = actions {
                div { class: "flex flex-col-reverse gap-2 pt-2 sm:flex-row sm:justify-end [&>*]:w-full sm:[&>*]:w-auto",
                    {a}
                }
            }
        }
    }
}

/// Label + input + help + error, the same everywhere.
#[component]
pub fn Field(
    label: String,
    #[props(default)] help: Option<String>,
    #[props(default)] error: Option<String>,
    #[props(default)] required: bool,
    children: Element,
) -> Element {
    rsx! {
        label { class: "flex flex-col gap-1.5",
            span { class: "text-sm font-medium",
                "{label}"
                if required {
                    span { class: "text-error ml-0.5", "aria-hidden": "true", "*" }
                }
            }
            {children}
            if let Some(err) = error {
                span { class: "flex items-center gap-1 text-xs text-error", role: "alert",
                    IconView { icon: Icon::Error, size: IconSize::Xs }
                    "{err}"
                }
            } else if let Some(h) = help {
                span { class: "text-xs text-base-content/60", "{h}" }
            }
        }
    }
}

/// Input types we support.
#[allow(missing_docs)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum InputKind {
    #[default]
    Text,
    Email,
    Number,
    Time,
    Url,
    Date,
    Tel,
    Search,
}

/// A single-line text input.
#[component]
pub fn TextInput(
    value: String,
    oninput: EventHandler<String>,
    #[props(default)] kind: InputKind,
    #[props(default)] placeholder: Option<String>,
    #[props(default)] autocomplete: Option<String>,
    #[props(default)] icon: Option<Icon>,
    #[props(default)] invalid: bool,
    #[props(default)] autofocus: bool,
) -> Element {
    let ty = match kind {
        InputKind::Text => "text",
        InputKind::Email => "email",
        InputKind::Number => "number",
        InputKind::Time => "time",
        InputKind::Url => "url",
        InputKind::Date => "date",
        InputKind::Tel => "tel",
        InputKind::Search => "search",
    };
    let invalid_class = if invalid { "input-error" } else { "" };
    rsx! {
        div { class: "input w-full {invalid_class} focus-within:outline-primary",
            if let Some(icon) = icon {
                span { class: "text-base-content/50", IconView { icon, size: IconSize::Sm } }
            }
            input {
                class: "grow",
                r#type: ty,
                value,
                placeholder,
                autocomplete,
                autofocus,
                "aria-invalid": if invalid { "true" } else { "false" },
                oninput: move |e| oninput.call(e.value()),
            }
        }
    }
}

/// A password input with show/hide, and an optional strength meter (0–4).
/// The strength is computed outside the UI (later by theo-crypto / zxcvbn).
#[component]
pub fn PasswordInput(
    value: String,
    oninput: EventHandler<String>,
    #[props(default)] placeholder: Option<String>,
    #[props(default)] autocomplete: Option<String>,
    #[props(default)] strength: Option<u8>,
    #[props(default)] invalid: bool,
    #[props(default)] autofocus: bool,
) -> Element {
    let mut visible = use_signal(|| false);
    let invalid_class = if invalid { "input-error" } else { "" };
    rsx! {
        div { class: "flex flex-col gap-2",
            div { class: "input w-full {invalid_class}",
                span { class: "text-base-content/50", IconView { icon: Icon::Key, size: IconSize::Sm } }
                input {
                    class: "grow",
                    r#type: if visible() { "text" } else { "password" },
                    value,
                    placeholder,
                    autocomplete,
                    autofocus,
                    "aria-invalid": if invalid { "true" } else { "false" },
                    oninput: move |e| oninput.call(e.value()),
                }
                button {
                    r#type: "button",
                    class: "btn btn-ghost btn-xs btn-square",
                    "aria-label": if visible() { "Hide password" } else { "Show password" },
                    onclick: move |_| visible.set(!visible()),
                    IconView { icon: if visible() { Icon::Hide } else { Icon::Show }, size: IconSize::Sm }
                }
            }
            if let Some(s) = strength {
                StrengthMeter { score: s }
            }
        }
    }
}

#[component]
fn StrengthMeter(score: u8) -> Element {
    let (label, color) = match score {
        0 => ("Very weak", "bg-error"),
        1 => ("Weak", "bg-error"),
        2 => ("Fair", "bg-warning"),
        3 => ("Strong", "bg-success"),
        _ => ("Very strong", "bg-success"),
    };
    rsx! {
        div { class: "flex items-center gap-2",
            div { class: "grid flex-1 grid-cols-4 gap-1",
                for i in 0..4u8 {
                    div {
                        key: "{i}",
                        class: if i < score.max(1) { "h-1.5 rounded-full {color} transition-colors duration-300" } else { "h-1.5 rounded-full bg-base-300 transition-colors duration-300" },
                    }
                }
            }
            span { class: "w-20 text-right text-xs text-base-content/60", "{label}" }
        }
    }
}

/// One option of a [`Select`].
#[derive(Clone, Debug, PartialEq)]
pub struct SelectOption {
    /// Value sent to `onchange`.
    pub value: String,
    /// Text shown to the user.
    pub label: String,
    /// Heading of the section this option belongs to. Consecutive options with the same
    /// heading are shown together under it, with a separator.
    pub group: Option<String>,
}

impl SelectOption {
    /// Creates an option.
    pub fn new(value: impl Into<String>, label: impl Into<String>) -> Self {
        Self { value: value.into(), label: label.into(), group: None }
    }

    /// Puts the option under a heading (for example "Elders").
    pub fn in_group(mut self, group: impl Into<String>) -> Self {
        self.group = Some(group.into());
        self
    }
}

/// Consecutive options with the same heading, in order.
fn sections(options: Vec<SelectOption>) -> Vec<(Option<String>, Vec<SelectOption>)> {
    let mut out: Vec<(Option<String>, Vec<SelectOption>)> = Vec::new();
    for o in options {
        match out.last_mut() {
            Some((g, list)) if *g == o.group => list.push(o),
            _ => out.push((o.group.clone(), vec![o])),
        }
    }
    out
}

/// A native select (best accessibility and mobile behaviour). Options with a `group` are shown
/// in sections with a heading (`optgroup`), so phones keep their own picker.
#[component]
pub fn Select(
    value: String,
    options: Vec<SelectOption>,
    onchange: EventHandler<String>,
    #[props(default)] invalid: bool,
) -> Element {
    let invalid_class = if invalid { "select-error" } else { "" };
    rsx! {
        select {
            class: "select w-full {invalid_class}",
            onchange: move |e| onchange.call(e.value()),
            for (i, (group, list)) in sections(options).into_iter().enumerate() {
                if let Some(g) = group {
                    optgroup { key: "g{i}", label: "{g}",
                        for o in list {
                            option { key: "{o.value}", value: "{o.value}", selected: o.value == value, "{o.label}" }
                        }
                    }
                } else {
                    for o in list {
                        option { key: "{o.value}", value: "{o.value}", selected: o.value == value, "{o.label}" }
                    }
                }
            }
        }
    }
}

/// An on/off switch with a label and an optional description.
#[component]
pub fn Switch(
    checked: bool,
    onchange: EventHandler<bool>,
    label: String,
    #[props(default)] description: Option<String>,
) -> Element {
    rsx! {
        label { class: "flex cursor-pointer items-start justify-between gap-4 rounded-box border border-base-300 p-3 transition-colors hover:bg-base-200/60",
            span { class: "flex flex-col gap-0.5",
                span { class: "text-sm font-medium", "{label}" }
                if let Some(d) = description {
                    span { class: "text-xs text-base-content/60", "{d}" }
                }
            }
            input {
                r#type: "checkbox",
                class: "toggle toggle-primary toggle-sm mt-0.5",
                checked,
                onchange: move |e| onchange.call(e.checked()),
            }
        }
    }
}

/// A checkbox with a label.
#[component]
pub fn Checkbox(checked: bool, onchange: EventHandler<bool>, label: String) -> Element {
    rsx! {
        label { class: "flex cursor-pointer items-start gap-3 text-sm",
            input {
                r#type: "checkbox",
                class: "checkbox checkbox-primary checkbox-sm mt-0.5",
                checked,
                onchange: move |e| onchange.call(e.checked()),
            }
            span { "{label}" }
        }
    }
}

/// One card of a [`ChoiceCards`] group.
#[derive(Clone, Debug, PartialEq)]
pub struct Choice {
    /// Value sent to `onchange`.
    pub value: String,
    /// Title of the card.
    pub title: String,
    /// One or two sentences that explain the choice.
    pub description: String,
    /// Icon of the card.
    pub icon: Icon,
}

/// Large radio cards for important choices (for example Local vs Cloud).
#[component]
pub fn ChoiceCards(value: String, choices: Vec<Choice>, onchange: EventHandler<String>) -> Element {
    rsx! {
        div { class: "grid grid-cols-1 gap-3 sm:grid-cols-2", role: "radiogroup",
            for c in choices {
                {
                    let selected = c.value == value;
                    let v = c.value.clone();
                    rsx! {
                        button {
                            key: "{c.value}",
                            r#type: "button",
                            role: "radio",
                            "aria-checked": if selected { "true" } else { "false" },
                            class: if selected { "relative flex flex-col items-start gap-2 rounded-box border-2 border-primary bg-primary/5 p-4 text-left transition-all duration-200" } else { "relative flex flex-col items-start gap-2 rounded-box border-2 border-base-300 p-4 text-left transition-all duration-200 hover:border-base-content/30" },
                            onclick: move |_| onchange.call(v.clone()),
                            span { class: if selected { "grid size-10 place-items-center rounded-field bg-primary text-primary-content transition-colors" } else { "grid size-10 place-items-center rounded-field bg-base-200 transition-colors" },
                                IconView { icon: c.icon, size: IconSize::Md }
                            }
                            span { class: "font-semibold", "{c.title}" }
                            span { class: "text-sm text-base-content/70", "{c.description}" }
                            if selected {
                                span { class: "absolute right-3 top-3 text-primary animate-pop",
                                    IconView { icon: Icon::Success, size: IconSize::Md }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

/// A multi-line text input (notes).
#[component]
pub fn TextArea(
    value: String,
    oninput: EventHandler<String>,
    #[props(default)] placeholder: Option<String>,
    #[props(default = 4)] rows: u8,
    #[props(default)] invalid: bool,
) -> Element {
    let invalid_class = if invalid { "textarea-error" } else { "" };
    rsx! {
        textarea {
            class: "textarea w-full {invalid_class}",
            rows: "{rows}",
            value,
            placeholder,
            "aria-invalid": if invalid { "true" } else { "false" },
            oninput: move |e| oninput.call(e.value()),
        }
    }
}

/// A toggle chip (filter values, quick choices). Selected chips are filled.
#[component]
pub fn ChipToggle(
    label: String,
    selected: bool,
    onclick: EventHandler<()>,
    /// A small count shown after the label.
    #[props(default)]
    count: Option<usize>,
) -> Element {
    let class = if selected {
        "border-primary bg-primary text-primary-content"
    } else {
        "border-base-300 bg-base-100 hover:bg-base-200"
    };
    rsx! {
        button {
            r#type: "button",
            class: "inline-flex min-h-9 items-center gap-1.5 rounded-full border px-3 text-sm transition-colors duration-150 {class}",
            "aria-pressed": if selected { "true" } else { "false" },
            onclick: move |_| onclick.call(()),
            if selected {
                IconView { icon: Icon::Check, size: IconSize::Xs }
            }
            "{label}"
            if let Some(n) = count {
                span { class: if selected { "text-xs opacity-80" } else { "text-xs text-base-content/50" }, "{n}" }
            }
        }
    }
}

/// Rules an input must meet, each marked as met or not (for example password rules).
#[component]
pub fn RuleChecklist(rules: Vec<(String, bool)>) -> Element {
    rsx! {
        ul { class: "grid grid-cols-1 gap-1 sm:grid-cols-2", "aria-live": "polite",
            for (i, (text, ok)) in rules.into_iter().enumerate() {
                li { key: "{i}", class: if ok { "flex items-center gap-1.5 text-xs text-success" } else { "flex items-center gap-1.5 text-xs text-base-content/60" },
                    IconView { icon: if ok { Icon::Success } else { Icon::Clock }, size: IconSize::Xs }
                    span { class: "sr-only", if ok { "Done: " } else { "Missing: " } }
                    "{text}"
                }
            }
        }
    }
}
