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
