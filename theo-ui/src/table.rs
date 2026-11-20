//! A plain table of numbers (for example a yearly summary, or the table view of a chart).

use dioxus::prelude::*;

/// A table: the first column is the row name, the others are numbers (right-aligned, digits in
/// columns). `groups` adds a header row over the columns: (label, number of columns), first the
/// group over the name column. `footer` is a totals row. Scrolls sideways on narrow screens.
#[component]
pub fn DataTable(
    columns: Vec<String>,
    rows: Vec<Vec<String>>,
    #[props(default)] groups: Vec<(String, usize)>,
    #[props(default)] footer: Option<Vec<String>>,
    /// Read by screen readers.
    #[props(default)]
    caption: Option<String>,
) -> Element {
    // Columns that start a group get a divider on their left.
    let mut starts = std::collections::BTreeSet::new();
    let mut at = 0;
    for (_, span) in &groups {
        if at > 0 {
            starts.insert(at);
        }
        at += span;
    }
    let cell = move |i: usize, base: &str| {
        let align = if i == 0 { "text-left" } else { "text-right tabular-nums" };
        let divider = if starts.contains(&i) { "border-l border-base-300" } else { "" };
        format!("{base} {align} {divider}")
    };
    rsx! {
        div { class: "-mx-1 overflow-x-auto",
            table { class: "w-full border-collapse text-sm",
                if let Some(c) = caption {
                    caption { class: "sr-only", "{c}" }
                }
                thead {
                    if !groups.is_empty() {
                        tr {
                            for (i, (label, span)) in groups.iter().enumerate() {
                                th {
                                    key: "{i}",
                                    colspan: "{span}",
                                    scope: "colgroup",
                                    class: if i == 0 { "px-3 pb-1 text-left text-xs font-semibold" } else { "border-l border-base-300 px-3 pb-1 text-center text-xs font-semibold" },
                                    "{label}"
                                }
                            }
                        }
                    }
                    tr { class: "border-b border-base-300",
                        for (i, c) in columns.iter().enumerate() {
                            th { key: "{i}", scope: "col", class: cell(i, "whitespace-nowrap px-3 py-2 text-xs font-medium text-base-content/60"), "{c}" }
                        }
                    }
                }
                tbody {
                    for (r, row) in rows.iter().enumerate() {
                        tr { key: "{r}", class: "border-b border-base-300/60 last:border-0",
                            for (i, v) in row.iter().enumerate() {
                                if i == 0 {
                                    th { key: "{i}", scope: "row", class: cell(i, "whitespace-nowrap px-3 py-2 font-medium"), "{v}" }
                                } else {
                                    td { key: "{i}", class: cell(i, "whitespace-nowrap px-3 py-2"), "{v}" }
                                }
                            }
                        }
                    }
                }
                if let Some(f) = footer {
                    tfoot {
                        tr { class: "border-t-2 border-base-300 font-semibold",
                            for (i, v) in f.iter().enumerate() {
                                td { key: "{i}", class: cell(i, "whitespace-nowrap px-3 py-2"), "{v}" }
                            }
                        }
                    }
                }
            }
        }
    }
}

/// One cell of a [`MatrixTable`]: a name and a detail line (for example the dates).
#[derive(Clone, Debug, PartialEq)]
pub struct MatrixCell {
    /// The main text (for example the publisher).
    pub title: String,
    /// A second, smaller line (for example "12 Mar 2026 – 3 Jun 2026").
    pub detail: String,
    /// Neutral, or a colour for cells that need attention (for example late).
    pub tone: crate::feedback::Tone,
    /// Small labels under the detail.
    pub tags: Vec<crate::card::Tag>,
}

/// A table read by columns: one column per item (for example a territory) with its entries
/// below, newest first. Columns can have different lengths. Scrolls sideways; the header stays
/// on top while the page scrolls.
#[component]
pub fn MatrixTable(
    /// (title, subtitle) of each column.
    columns: Vec<(String, String)>,
    /// The cells of each column, top to bottom.
    cells: Vec<Vec<MatrixCell>>,
    /// Called with (column, row) when a cell is tapped.
    #[props(default)]
    on_select: Option<EventHandler<(usize, usize)>>,
    /// Text of an empty column.
    #[props(default)]
    empty: String,
    /// Read by screen readers.
    #[props(default)]
    caption: Option<String>,
) -> Element {
    use crate::feedback::Tone;
    let rows = cells.iter().map(|c| c.len()).max().unwrap_or(0).max(1);
    let tint = |t: Tone| match t {
        Tone::Info => "bg-info/10",
        Tone::Success => "bg-success/10",
        Tone::Warning => "bg-warning/15",
        Tone::Error => "bg-error/10",
        Tone::Neutral => "",
    };
    rsx! {
        div { class: "-mx-1 overflow-x-auto rounded-box border border-base-300",
            table { class: "w-full border-collapse text-sm",
                if let Some(c) = caption {
                    caption { class: "sr-only", "{c}" }
                }
                thead {
                    tr { class: "border-b border-base-300 bg-base-200/60",
                        for (i, (title, sub)) in columns.iter().enumerate() {
                            th { key: "{i}", scope: "col", class: "min-w-36 border-l border-base-300 px-3 py-2 text-left align-bottom first:border-l-0",
                                div { class: "font-semibold", "{title}" }
                                if !sub.is_empty() {
                                    div { class: "whitespace-nowrap text-xs font-normal text-base-content/60", "{sub}" }
                                }
                            }
                        }
                    }
                }
                tbody {
                    for r in 0..rows {
                        tr { key: "{r}", class: "border-b border-base-300/60 last:border-0",
                            for (c, col) in cells.iter().enumerate() {
                                td { key: "{c}", class: "border-l border-base-300/60 p-0 align-top first:border-l-0",
                                    match col.get(r) {
                                        Some(cell) => {
                                            let class = format!("block w-full px-3 py-2 text-left {}", tint(cell.tone));
                                            match on_select {
                                                Some(h) => rsx! {
                                                    button { r#type: "button", class: "{class} transition-colors hover:bg-base-200", onclick: move |_| h.call((c, r)),
                                                        div { class: "whitespace-nowrap font-medium", "{cell.title}" }
                                                        div { class: "whitespace-nowrap text-xs text-base-content/60", "{cell.detail}" }
                                                        {cell_tags(&cell.tags)}
                                                    }
                                                },
                                                None => rsx! {
                                                    div { class,
                                                        div { class: "whitespace-nowrap font-medium", "{cell.title}" }
                                                        div { class: "whitespace-nowrap text-xs text-base-content/60", "{cell.detail}" }
                                                        {cell_tags(&cell.tags)}
                                                    }
                                                },
                                            }
                                        }
                                        None if r == 0 && col.is_empty() => rsx! { div { class: "px-3 py-2 text-xs text-base-content/50", "{empty}" } },
                                        None => rsx! {},
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

fn cell_tags(tags: &[crate::card::Tag]) -> Element {
    use crate::feedback::Tone;
    rsx! {
        if !tags.is_empty() {
            div { class: "mt-1 flex flex-wrap gap-1",
                for (i, t) in tags.iter().enumerate() {
                    span {
                        key: "{i}",
                        class: match t.tone {
                            Tone::Info => "rounded-full bg-info/15 px-1.5 text-[10px] font-medium text-info",
                            Tone::Success => "rounded-full bg-success/15 px-1.5 text-[10px] font-medium text-success",
                            Tone::Warning => "rounded-full bg-warning/15 px-1.5 text-[10px] font-medium text-warning",
                            Tone::Error => "rounded-full bg-error/15 px-1.5 text-[10px] font-medium text-error",
                            Tone::Neutral => "rounded-full bg-base-content/10 px-1.5 text-[10px] font-medium",
                        },
                        "{t.label}"
                    }
                }
            }
        }
    }
}
