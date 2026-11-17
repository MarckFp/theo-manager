//! Dashboard grid with widgets that users reorder and resize by dragging.
//!
//! Pointer events (not HTML drag-and-drop) are used, so it works with mouse, pen and touch.
//! Columns follow the grid's own width (container queries in `input.css`):
//! < 600 px → 1 column, < 1000 px → 2 columns, otherwise 4 columns. Rows are 10rem high.

use crate::icon::{Icon, IconSize, IconView};
use dioxus::prelude::*;
use std::collections::HashMap;
use std::rc::Rc;

/// Position and size of one widget. The order of the list is the order on screen.
#[derive(Clone, Debug, PartialEq)]
pub struct WidgetLayout {
    /// Identifier passed to `render`.
    pub id: String,
    /// Title in the widget header.
    pub title: String,
    /// Icon in the widget header.
    pub icon: Icon,
    /// Width in columns (1–4).
    pub cols: u8,
    /// Height in rows (1–3).
    pub rows: u8,
}

impl WidgetLayout {
    /// Creates a widget layout.
    pub fn new(id: impl Into<String>, title: impl Into<String>, icon: Icon, cols: u8, rows: u8) -> Self {
        Self { id: id.into(), title: title.into(), icon, cols: cols.clamp(1, 4), rows: rows.clamp(1, 3) }
    }
}

const GAP_PX: f64 = 16.0;
const ROW_PX: f64 = 160.0;

#[derive(Clone, Copy, Debug, PartialEq)]
enum Gesture {
    Move { from: usize, start: (f64, f64), cur: (f64, f64) },
    Resize { index: usize, start: (f64, f64), start_cols: u8, start_rows: u8, col_step: f64, max_cols: u8 },
}

type Rect = (f64, f64, f64, f64);

/// Gesture state machine shared by all handles. Everything inside is `Copy`.
#[derive(Clone, Copy)]
struct Ctl {
    gesture: Signal<Option<Gesture>>,
    preview: Signal<Option<Vec<WidgetLayout>>>,
    target: Signal<Option<usize>>,
    rects: Signal<Vec<(String, Rect)>>,
    current: CopyValue<Vec<WidgetLayout>>,
    on_change: EventHandler<Vec<WidgetLayout>>,
}

impl Ctl {
    fn moved(mut self, p: (f64, f64)) {
        let g = *self.gesture.peek();
        match g {
            Some(Gesture::Move { from, start, .. }) => {
                self.gesture.set(Some(Gesture::Move { from, start, cur: p }));
                let widgets = self.current.read().clone();
                let hit = self
                    .rects
                    .peek()
                    .iter()
                    .find(|(_, r)| p.0 >= r.0 && p.0 <= r.0 + r.2 && p.1 >= r.1 && p.1 <= r.1 + r.3)
                    .and_then(|(id, _)| widgets.iter().position(|w| &w.id == id));
                self.target.set(hit);
            }
            Some(Gesture::Resize { index, start, start_cols, start_rows, col_step, max_cols }) => {
                if col_step <= 0.0 {
                    return;
                }
                let mut next = self.current.read().clone();
                if let Some(w) = next.get_mut(index) {
                    let eff = start_cols.min(max_cols);
                    let c = (f64::from(eff) + ((p.0 - start.0) / col_step).round())
                        .clamp(1.0, f64::from(max_cols)) as u8;
                    w.cols = if c == eff { start_cols } else { c };
                    w.rows = (f64::from(start_rows) + ((p.1 - start.1) / (ROW_PX + GAP_PX)).round())
                        .clamp(1.0, 3.0) as u8;
                }
                if self.preview.peek().as_ref() != Some(&next) {
                    self.preview.set(Some(next));
                }
            }
            None => {}
        }
    }

    fn up(mut self) {
        let g = *self.gesture.peek();
        match g {
            Some(Gesture::Move { from, .. }) => {
                let to = *self.target.peek();
                if let Some(to) = to
                    && to != from
                {
                    let mut next = self.current.read().clone();
                    if from < next.len() && to < next.len() {
                        let w = next.remove(from);
                        next.insert(to, w);
                        self.on_change.call(next);
                    }
                }
            }
            Some(Gesture::Resize { .. }) => {
                let next = self.preview.peek().clone();
                if let Some(next) = next {
                    self.on_change.call(next);
                }
            }
            None => return,
        }
        self.gesture.set(None);
        self.preview.set(None);
        self.target.set(None);
    }
}

/// The dashboard grid. In `editing` mode each widget shows a drag handle (top left) and a
/// resize handle (bottom right). Changes are reported through `on_change`.
#[component]
pub fn Dashboard(
    widgets: Vec<WidgetLayout>,
    editing: bool,
    on_change: EventHandler<Vec<WidgetLayout>>,
    /// Renders the body of a widget, by id.
    render: Callback<String, Element>,
) -> Element {
    let mut gesture = use_signal(|| None::<Gesture>);
    let preview = use_signal(|| None::<Vec<WidgetLayout>>);
    let mut target = use_signal(|| None::<usize>);
    let mut rects = use_signal(Vec::<(String, Rect)>::new);
    let mut nodes = use_signal(HashMap::<String, Rc<MountedData>>::new);
    let mut grid = use_signal(|| None::<Rc<MountedData>>);

    // Latest props, readable from event handlers without re-rendering.
    let current = use_hook(|| CopyValue::new(Vec::<WidgetLayout>::new()));
    let mut writer = current;
    writer.set(widgets.clone());

    let list = preview().unwrap_or_else(|| widgets.clone());
    let active = gesture();
    let ctl = Ctl { gesture, preview, target, rects, current, on_change };
    let on_move = move |p: (f64, f64)| ctl.moved(p);
    let on_up = move || ctl.up();

    rsx! {
        div {
            class: "theo-dash-wrap",
            onmounted: move |e| grid.set(Some(e.data())),
            div { class: "theo-dash",
                for (i, w) in list.iter().cloned().enumerate() {
                    {
                        let id = w.id.clone();
                        let id_mount = w.id.clone();
                        let (style, state_class) = match active {
                            Some(Gesture::Move { from, start, cur }) if from == i => (
                                format!("transform: translate({}px, {}px) scale(1.02); z-index: 30;", cur.0 - start.0, cur.1 - start.1),
                                "shadow-2xl shadow-base-content/20 opacity-90 cursor-grabbing",
                            ),
                            Some(Gesture::Move { .. }) if target() == Some(i) => (String::new(), "ring-2 ring-primary ring-offset-2 ring-offset-base-200"),
                            Some(Gesture::Resize { index, .. }) if index == i => (String::new(), "ring-2 ring-primary/60"),
                            _ => (String::new(), "transition-[transform,box-shadow] duration-300"),
                        };
                        let edit_class = if editing { "border-dashed border-primary/40" } else { "border-base-300" };
                        rsx! {
                            div {
                                key: "{w.id}",
                                class: "relative flex min-w-0 flex-col overflow-hidden rounded-box border bg-base-100 shadow-xs {edit_class} {state_class}",
                                "data-span": "{w.cols}",
                                "data-rows": "{w.rows}",
                                style,
                                onmounted: move |e| {
                                    nodes.write().insert(id_mount.clone(), e.data());
                                },
                                div { class: "flex items-center gap-2 px-4 pt-3.5 pb-1",
                                    if editing {
                                        button {
                                            r#type: "button",
                                            class: "-ml-1.5 cursor-grab rounded-field p-1 text-base-content/50 hover:bg-base-200 hover:text-base-content active:cursor-grabbing",
                                            style: "touch-action: none;",
                                            "aria-label": "Move {w.title}",
                                            onpointerdown: move |e: PointerEvent| {
                                                e.prevent_default();
                                                let p = e.client_coordinates();
                                                gesture.set(Some(Gesture::Move { from: i, start: (p.x, p.y), cur: (p.x, p.y) }));
                                                target.set(Some(i));
                                                spawn(async move {
                                                    let snapshot: Vec<(String, Rc<MountedData>)> = nodes.peek().iter().map(|(k, v)| (k.clone(), v.clone())).collect();
                                                    let mut out = Vec::new();
                                                    for (k, n) in snapshot {
                                                        if let Ok(r) = n.get_client_rect().await {
                                                            out.push((k, (r.origin.x, r.origin.y, r.size.width, r.size.height)));
                                                        }
                                                    }
                                                    rects.set(out);
                                                });
                                            },
                                            onpointermove: move |e: PointerEvent| { let p = e.client_coordinates(); on_move((p.x, p.y)) },
                                            onpointerup: move |_| on_up(),
                                            onpointercancel: move |_| on_up(),
                                            IconView { icon: Icon::Drag, size: IconSize::Sm }
                                        }
                                    }
                                    span { class: "text-base-content/50", IconView { icon: w.icon, size: IconSize::Sm } }
                                    h3 { class: "truncate text-sm font-medium text-base-content/80", "{w.title}" }
                                }
                                div { class: if editing { "min-h-0 flex-1 overflow-auto px-4 pb-8 pt-2 pr-8" } else { "min-h-0 flex-1 overflow-auto px-4 pb-4 pt-2" }, {render.call(id)} }
                                if editing {
                                    button {
                                        r#type: "button",
                                        class: "absolute bottom-1 right-1 cursor-nwse-resize rounded-field p-1 text-base-content/40 hover:bg-base-200 hover:text-primary",
                                        style: "touch-action: none;",
                                        "aria-label": "Resize {w.title}",
                                        onpointerdown: move |e: PointerEvent| {
                                            e.prevent_default();
                                            let p = e.client_coordinates();
                                            let (cols, rows) = (w.cols, w.rows);
                                            gesture.set(Some(Gesture::Resize { index: i, start: (p.x, p.y), start_cols: cols, start_rows: rows, col_step: 0.0, max_cols: 4 }));
                                            spawn(async move {
                                                let node = grid.peek().clone();
                                                if let Some(n) = node
                                                    && let Ok(r) = n.get_client_rect().await
                                                {
                                                    let gw = r.size.width;
                                                    let max_cols: u8 = if gw < 600.0 { 1 } else if gw < 1000.0 { 2 } else { 4 };
                                                    let mc = f64::from(max_cols);
                                                    let step = (gw - GAP_PX * (mc - 1.0)) / mc + GAP_PX;
                                                    let g = *gesture.peek();
                                                    if let Some(Gesture::Resize { index, start, start_cols, start_rows, .. }) = g {
                                                        gesture.set(Some(Gesture::Resize { index, start, start_cols, start_rows, col_step: step, max_cols }));
                                                    }
                                                }
                                            });
                                        },
                                        onpointermove: move |e: PointerEvent| { let p = e.client_coordinates(); on_move((p.x, p.y)) },
                                        onpointerup: move |_| on_up(),
                                        onpointercancel: move |_| on_up(),
                                        IconView { icon: Icon::Resize, size: IconSize::Sm }
                                    }
                                }
                            }
                        }
                    }
                }
            }
            // While dragging with a mouse, this layer receives the moves (touch keeps them on the handle).
            if active.is_some() {
                div {
                    class: "fixed inset-0 z-50 cursor-grabbing",
                    style: "touch-action: none;",
                    onpointermove: move |e: PointerEvent| { let p = e.client_coordinates(); on_move((p.x, p.y)) },
                    onpointerup: move |_| on_up(),
                    onpointercancel: move |_| on_up(),
                }
            }
        }
    }
}
