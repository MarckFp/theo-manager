//! Forms (R-425). `Form` owns layout and buttons; `Field` owns label, help and error.

use crate::icon::{Icon, IconSize, IconView};
use dioxus::prelude::*;

/// A form. Enter submits; the page never reloads. `actions` is rendered in the standard
/// place: bottom right on larger screens (wide buttons), full width on phones (R-424).
///
/// With `floating_save`, phones get a round floating Save button instead of the actions, and the
/// page's sticky back arrow is the way to cancel (edit pages, R-424).
#[component]
pub fn Form(
    onsubmit: EventHandler<()>,
    #[props(default)] actions: Option<Element>,
    #[props(default)] floating_save: bool,
    /// The buttons take the whole width on every screen (sign-in and sign-up forms).
    #[props(default)]
    full_width_actions: bool,
    children: Element,
) -> Element {
    let actions_class = if full_width_actions {
        "flex flex-col gap-2 pt-2 [&>*]:w-full"
    } else if floating_save {
        "hidden gap-2 pt-2 md:flex md:justify-end md:[&>*]:min-w-40"
    } else {
        "flex flex-col-reverse gap-2 pt-2 sm:flex-row sm:justify-end [&>*]:w-full sm:[&>*]:w-auto sm:[&>*]:min-w-40"
    };
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
                div { class: actions_class, {a} }
            }
            if floating_save {
                button {
                    r#type: "submit",
                    class: "btn btn-primary btn-circle btn-lg fixed right-4 z-40 shadow-xl shadow-primary/30 transition-transform active:scale-90 md:hidden",
                    style: "bottom: calc(6.25rem + env(safe-area-inset-bottom));",
                    "aria-label": "Save",
                    title: "Save",
                    IconView { icon: Icon::Save, size: IconSize::Lg }
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
    /// A whole number of 0 or more (a count): digits only, no sign or decimals.
    Count,
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
        InputKind::Count => "number",
    };
    let count = kind == InputKind::Count;
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
                min: if count { Some("0") } else { None },
                step: if count { Some("1") } else { None },
                inputmode: if count { Some("numeric") } else { None },
                // A count keeps only its digits: "-2" becomes "2", "1.5" becomes "15".
                onkeydown: move |e| {
                    if count && matches!(e.key(), Key::Character(ref c) if c == "-" || c == "+" || c == "." || c == "," || c == "e" || c == "E") {
                        e.prevent_default();
                    }
                },
                oninput: move |e| {
                    let v = e.value();
                    oninput.call(if count { v.chars().filter(char::is_ascii_digit).collect() } else { v })
                },
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

/// The swatch colour of a tone (labels).
fn tone_dot(tone: crate::feedback::Tone) -> &'static str {
    use crate::feedback::Tone;
    match tone {
        Tone::Info => "bg-info",
        Tone::Success => "bg-success",
        Tone::Warning => "bg-warning",
        Tone::Error => "bg-error",
        Tone::Neutral => "bg-base-content/40",
    }
}

/// A row of colour circles to choose one (for example the colour of a label). The chosen one has a
/// ring and a check; its name is shown next to the circles.
#[component]
pub fn ColorPicker(value: String, options: Vec<(String, String, crate::feedback::Tone)>, onchange: EventHandler<String>) -> Element {
    let chosen = options.iter().find(|o| o.0 == value).map(|o| o.1.clone()).unwrap_or_default();
    rsx! {
        div { class: "flex flex-wrap items-center gap-2", role: "radiogroup",
            for (v, label, tone) in options.into_iter() {
                {
                    let selected = v == value;
                    let ring = if selected { "ring-2 ring-base-content ring-offset-2 ring-offset-base-100" } else { "hover:scale-110" };
                    rsx! {
                        button {
                            key: "{v}",
                            r#type: "button",
                            role: "radio",
                            "aria-checked": if selected { "true" } else { "false" },
                            "aria-label": "{label}",
                            title: "{label}",
                            class: "grid size-7 place-items-center rounded-full text-white transition-transform duration-150 {tone_dot(tone)} {ring}",
                            onclick: move |_| onchange.call(v.clone()),
                            if selected {
                                IconView { icon: Icon::Check, size: IconSize::Xs }
                            }
                        }
                    }
                }
            }
            span { class: "ml-1 text-sm text-base-content/70", "{chosen}" }
        }
    }
}

/// A coloured pill to put on or take off (for example a label on a person). On: filled in its
/// colour with a check. Off: an outline with a colour dot.
#[component]
pub fn TagToggle(label: String, tone: crate::feedback::Tone, selected: bool, onclick: EventHandler<()>) -> Element {
    use crate::feedback::Tone;
    let on = match tone {
        Tone::Info => "border-info bg-info/15 text-info",
        Tone::Success => "border-success bg-success/15 text-success",
        Tone::Warning => "border-warning bg-warning/15 text-warning",
        Tone::Error => "border-error bg-error/15 text-error",
        Tone::Neutral => "border-base-content/40 bg-base-content/10 text-base-content",
    };
    let class = if selected { on } else { "border-dashed border-base-300 text-base-content/70 hover:bg-base-200" };
    rsx! {
        button {
            r#type: "button",
            class: "inline-flex min-h-8 items-center gap-1.5 rounded-full border px-3 text-sm font-medium transition-colors duration-150 {class}",
            "aria-pressed": if selected { "true" } else { "false" },
            onclick: move |_| onclick.call(()),
            if selected {
                IconView { icon: Icon::Check, size: IconSize::Xs }
            } else {
                span { class: "size-2.5 rounded-full {tone_dot(tone)}" }
            }
            "{label}"
        }
    }
}

/// A column of a [`NumberGrid`]: a label and an optional hint under it (for example a date).
#[derive(Clone, Debug, PartialEq)]
pub struct GridColumn {
    /// Column label ("Week 1").
    pub label: String,
    /// Small text under the label ("6 Nov").
    pub hint: Option<String>,
}

/// A row of a [`NumberGrid`]: its values (one per column; `None` is empty) and the summary
/// cells shown after the inputs (for example the total and the average).
#[derive(Clone, Debug, PartialEq)]
pub struct GridRow {
    /// Row label ("Midweek meeting").
    pub label: String,
    /// Icon before the label.
    pub icon: Icon,
    /// One value per column.
    pub values: Vec<Option<u32>>,
    /// One text per summary column ("214", "53.5").
    pub summary: Vec<String>,
    /// Grey text in each empty input (for example the date of that meeting: "Thu 5").
    pub placeholders: Vec<String>,
}

/// Whole numbers in a grid of rows × columns, with summary cells per row. A table on larger
/// screens; on phones each row is a block with its inputs side by side. `onchange` gets
/// (row, column, value); an emptied input gives `None`. Values above `max` are capped.
#[component]
pub fn NumberGrid(
    columns: Vec<GridColumn>,
    summary_columns: Vec<String>,
    rows: Vec<GridRow>,
    onchange: EventHandler<(usize, usize, Option<u32>)>,
    #[props(default = 9999)] max: u32,
) -> Element {
    let n = columns.len().max(1);
    let parse = move |v: String| -> Option<Option<u32>> {
        let t = v.trim();
        if t.is_empty() {
            Some(None)
        } else {
            t.parse::<u32>().ok().map(|x| Some(x.min(max)))
        }
    };
    let input = move |r: usize, c: usize, value: Option<u32>, label: String, placeholder: String| {
        rsx! {
            input {
                r#type: "number",
                inputmode: "numeric",
                min: "0",
                max: "{max}",
                class: "input input-sm w-full px-1 text-center tabular-nums",
                "aria-label": "{label}",
                placeholder: "{placeholder}",
                value: value.map(|v| v.to_string()).unwrap_or_default(),
                oninput: move |e| {
                    if let Some(v) = parse(e.value()) {
                        onchange.call((r, c, v));
                    }
                },
            }
        }
    };
    let aria = move |row: &GridRow, col: &GridColumn, c: usize| {
        let when = row.placeholders.get(c).filter(|p| !p.is_empty()).or(col.hint.as_ref());
        match when {
            Some(h) => format!("{}, {} ({h})", row.label, col.label),
            None => format!("{}, {}", row.label, col.label),
        }
    };
    rsx! {
        // Larger screens: a table.
        div { class: "hidden md:block",
            table { class: "w-full border-collapse",
                thead {
                    tr {
                        th { class: "w-48" }
                        for (c, col) in columns.iter().enumerate() {
                            th { key: "{c}", scope: "col", class: "px-1.5 pb-2 text-center align-bottom",
                                div { class: "text-xs font-medium", "{col.label}" }
                                if let Some(h) = &col.hint {
                                    div { class: "text-[11px] font-normal text-base-content/60", "{h}" }
                                }
                            }
                        }
                        for (s, label) in summary_columns.iter().enumerate() {
                            th { key: "s{s}", scope: "col", class: "px-3 pb-2 text-right align-bottom text-xs font-medium text-base-content/60", "{label}" }
                        }
                    }
                }
                tbody {
                    for (r, row) in rows.iter().enumerate() {
                        tr { key: "{r}", class: "border-t border-base-300",
                            th { scope: "row", class: "py-3 pr-3 text-left text-sm font-medium",
                                span { class: "inline-flex items-center gap-2",
                                    span { class: "text-base-content/60", IconView { icon: row.icon, size: IconSize::Sm } }
                                    "{row.label}"
                                }
                            }
                            for (c, col) in columns.iter().enumerate() {
                                td { key: "{c}", class: "px-1.5 py-3", {input(r, c, row.values.get(c).copied().flatten(), aria(row, col, c), row.placeholders.get(c).cloned().unwrap_or_default())} }
                            }
                            for (s, v) in row.summary.iter().enumerate() {
                                td { key: "s{s}", class: "px-3 py-3 text-right text-sm font-semibold tabular-nums", "{v}" }
                            }
                        }
                    }
                }
            }
        }
        // Phones: one block per row.
        div { class: "flex flex-col gap-3 md:hidden",
            for (r, row) in rows.iter().enumerate() {
                div { key: "{r}", class: "flex flex-col gap-3 rounded-box border border-base-300 p-3",
                    span { class: "inline-flex items-center gap-2 text-sm font-medium",
                        span { class: "text-base-content/60", IconView { icon: row.icon, size: IconSize::Sm } }
                        "{row.label}"
                    }
                    div { class: "grid gap-1.5", style: "grid-template-columns: repeat({n}, minmax(0, 1fr));",
                        for (c, col) in columns.iter().enumerate() {
                            label { key: "{c}", class: "flex min-w-0 flex-col items-center gap-1",
                                span { class: "truncate text-[11px] font-medium", "{col.label}" }
                                {input(r, c, row.values.get(c).copied().flatten(), aria(row, col, c), row.placeholders.get(c).cloned().unwrap_or_default())}
                                if let Some(h) = &col.hint {
                                    span { class: "truncate text-[10px] text-base-content/60", "{h}" }
                                }
                            }
                        }
                    }
                    div { class: "flex justify-end gap-4 text-sm",
                        for (s, v) in row.summary.iter().enumerate() {
                            span { key: "{s}", class: "inline-flex items-baseline gap-1.5",
                                span { class: "text-xs text-base-content/60", {summary_columns.get(s).cloned().unwrap_or_default()} }
                                span { class: "font-semibold tabular-nums", "{v}" }
                            }
                        }
                    }
                }
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

/// A button that opens the file chooser and sends the chosen file's name and bytes.
#[component]
pub fn FileButton(
    label: String,
    on_file: EventHandler<(String, Vec<u8>)>,
    /// File types to offer, for example ".theobackup".
    #[props(default)]
    accept: Option<String>,
    #[props(default)] icon: Option<Icon>,
) -> Element {
    rsx! {
        label { class: "btn btn-soft gap-2 border-base-300 bg-base-100 font-medium hover:bg-base-200",
            if let Some(icon) = icon {
                IconView { icon, size: IconSize::Sm }
            }
            "{label}"
            input {
                r#type: "file",
                class: "hidden",
                accept,
                onchange: move |e| {
                    let files = e.files();
                    spawn(async move {
                        if let Some(f) = files.into_iter().next() {
                            let name = f.name();
                            if let Ok(bytes) = f.read_bytes().await {
                                on_file.call((name, bytes.to_vec()));
                            }
                        }
                    });
                },
            }
        }
    }
}
