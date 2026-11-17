//! Illustrations drawn as inline SVG with theme colours (no external images, P2).

use dioxus::prelude::*;

/// Available illustrations.
#[allow(missing_docs)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IllustrationKind {
    /// A desktop window and a phone showing a schedule.
    Devices,
    /// A lock between devices and a server that only sees ciphertext.
    Encryption,
}

/// Renders an illustration. Decorative: hidden from screen readers.
#[component]
pub fn Illustration(kind: IllustrationKind) -> Element {
    match kind {
        IllustrationKind::Devices => rsx! { Devices {} },
        IllustrationKind::Encryption => rsx! { Encryption {} },
    }
}

#[component]
fn Devices() -> Element {
    rsx! {
        svg { view_box: "0 0 480 360", class: "h-auto w-full drop-shadow-2xl", "aria-hidden": "true",
            // Desktop window
            rect { x: "10", y: "20", width: "380", height: "260", rx: "16", class: "fill-base-100 stroke-base-300", stroke_width: "2" }
            rect { x: "10", y: "20", width: "380", height: "34", rx: "16", class: "fill-base-200" }
            circle { cx: "32", cy: "37", r: "5", class: "fill-error/60" }
            circle { cx: "48", cy: "37", r: "5", class: "fill-warning/70" }
            circle { cx: "64", cy: "37", r: "5", class: "fill-success/60" }
            // Sidebar
            rect { x: "22", y: "66", width: "80", height: "200", rx: "10", class: "fill-base-200" }
            rect { x: "32", y: "80", width: "60", height: "10", rx: "5", class: "fill-primary/70" }
            rect { x: "32", y: "102", width: "50", height: "8", rx: "4", class: "fill-base-300" }
            rect { x: "32", y: "120", width: "56", height: "8", rx: "4", class: "fill-base-300" }
            rect { x: "32", y: "138", width: "44", height: "8", rx: "4", class: "fill-base-300" }
            // Widgets
            rect { x: "114", y: "66", width: "126", height: "84", rx: "10", class: "fill-base-100 stroke-base-300", stroke_width: "1.5" }
            rect { x: "126", y: "78", width: "50", height: "7", rx: "3.5", class: "fill-base-300" }
            text { x: "126", y: "122", class: "fill-base-content text-[26px] font-semibold", "92%" }
            rect { x: "126", y: "132", width: "80", height: "6", rx: "3", class: "fill-success/40" }
            rect { x: "250", y: "66", width: "128", height: "84", rx: "10", class: "fill-base-100 stroke-base-300", stroke_width: "1.5" }
            rect { x: "262", y: "118", width: "12", height: "22", rx: "3", class: "fill-primary/30" }
            rect { x: "280", y: "104", width: "12", height: "36", rx: "3", class: "fill-primary/30" }
            rect { x: "298", y: "110", width: "12", height: "30", rx: "3", class: "fill-primary/30" }
            rect { x: "316", y: "92", width: "12", height: "48", rx: "3", class: "fill-primary/30" }
            rect { x: "334", y: "84", width: "12", height: "56", rx: "3", class: "fill-primary" }
            rect { x: "114", y: "162", width: "264", height: "104", rx: "10", class: "fill-base-100 stroke-base-300", stroke_width: "1.5" }
            for (i, w) in [150, 120, 170].iter().enumerate() {
                g { key: "{i}",
                    circle { cx: "134", cy: "{186 + i * 26}", r: "8", class: "fill-accent/40" }
                    rect { x: "150", y: "{181 + i * 26}", width: "{w}", height: "9", rx: "4.5", class: "fill-base-300" }
                    rect { x: "334", y: "{181 + i * 26}", width: "30", height: "9", rx: "4.5", class: "fill-primary/20" }
                }
            }
            // Phone
            rect { x: "330", y: "130", width: "140", height: "220", rx: "22", class: "fill-base-content" }
            rect { x: "338", y: "140", width: "124", height: "200", rx: "16", class: "fill-base-100" }
            rect { x: "350", y: "156", width: "70", height: "9", rx: "4.5", class: "fill-base-content/80" }
            rect { x: "350", y: "176", width: "100", height: "42", rx: "8", class: "fill-primary/10" }
            rect { x: "358", y: "186", width: "50", height: "7", rx: "3.5", class: "fill-primary" }
            rect { x: "358", y: "200", width: "76", height: "6", rx: "3", class: "fill-primary/40" }
            rect { x: "350", y: "226", width: "100", height: "42", rx: "8", class: "fill-base-200" }
            rect { x: "358", y: "236", width: "60", height: "7", rx: "3.5", class: "fill-base-300" }
            rect { x: "358", y: "250", width: "40", height: "6", rx: "3", class: "fill-base-300" }
            // Dock
            rect { x: "348", y: "300", width: "104", height: "28", rx: "12", class: "fill-base-200" }
            circle { cx: "366", cy: "314", r: "5", class: "fill-primary" }
            circle { cx: "390", cy: "314", r: "5", class: "fill-base-300" }
            circle { cx: "414", cy: "314", r: "5", class: "fill-base-300" }
            circle { cx: "436", cy: "314", r: "5", class: "fill-base-300" }
        }
    }
}

#[component]
fn Encryption() -> Element {
    rsx! {
        svg { view_box: "0 0 480 260", class: "h-auto w-full", "aria-hidden": "true",
            // Devices (left)
            rect { x: "20", y: "70", width: "120", height: "84", rx: "12", class: "fill-base-100 stroke-base-300", stroke_width: "2" }
            rect { x: "34", y: "86", width: "60", height: "8", rx: "4", class: "fill-base-content/70" }
            rect { x: "34", y: "102", width: "90", height: "6", rx: "3", class: "fill-base-300" }
            rect { x: "34", y: "114", width: "76", height: "6", rx: "3", class: "fill-base-300" }
            rect { x: "34", y: "126", width: "84", height: "6", rx: "3", class: "fill-base-300" }
            text { x: "80", y: "180", text_anchor: "middle", class: "fill-base-content/60 text-[12px]", "Your devices" }
            // Arrow + lock
            path { d: "M150 112 H330", class: "stroke-primary/50", stroke_width: "2", stroke_dasharray: "6 6", fill: "none" }
            circle { cx: "240", cy: "112", r: "34", class: "fill-primary" }
            rect { x: "226", y: "108", width: "28", height: "22", rx: "4", class: "fill-primary-content" }
            path { d: "M231 108 v-7 a9 9 0 0 1 18 0 v7", class: "stroke-primary-content", stroke_width: "3.5", fill: "none" }
            text { x: "240", y: "170", text_anchor: "middle", class: "fill-base-content/60 text-[12px]", "Encrypted before it leaves" }
            // Server (right)
            rect { x: "340", y: "60", width: "120", height: "104", rx: "12", class: "fill-base-200 stroke-base-300", stroke_width: "2" }
            for (i, row) in ["x9#Qa7…", "f$2Lm0…", "Zk8!pR…", "b4@Yt1…"].iter().enumerate() {
                text { key: "{i}", x: "356", y: "{86 + i * 20}", class: "fill-base-content/40 font-mono text-[12px]", "{row}" }
            }
            text { x: "400", y: "190", text_anchor: "middle", class: "fill-base-content/60 text-[12px]", "Server sees only this" }
        }
    }
}
