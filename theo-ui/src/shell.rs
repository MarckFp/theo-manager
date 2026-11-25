//! The app shell (R-401): sidebar on desktop, icon rail on tablets, dock on phones,
//! sub-menus, and an animated page transition for the content.
//!
//! Sub-menus, the same model on every screen size:
//! - **Desktop (lg+):** sidebar sections expand in place (accordion).
//! - **Tablet (md):** icon rail; clicking a section opens a flyout with its pages.
//! - **Phone:** the dock opens the section's hub page ([`NavHub`]) with its pages. To change
//!   page, tap the dock item again (it returns to the hub) and choose another page.
//!
//! Navigation ids are opaque to theo-ui: `"users"` for a section, `"users/reports"` for a page.

use crate::card::{Avatar, BrandMark};
use crate::icon::{Icon, IconSize, IconView};
use crate::menu::{Placement, Popover, SwitcherItem, WorkspaceSwitcher};
use dioxus::prelude::*;
use std::collections::HashSet;

/// One navigation destination. A section has `children` (its pages).
#[derive(Clone, Debug, PartialEq)]
pub struct NavEntry {
    /// Identifier sent to `on_navigate`.
    pub id: String,
    /// Label shown to the user.
    pub label: String,
    /// Icon shown to the user.
    pub icon: Icon,
    /// One sentence shown on hub tiles.
    pub description: String,
    /// Pages of this section.
    pub children: Vec<NavEntry>,
}

impl NavEntry {
    /// Creates an entry without description and children.
    pub fn new(id: impl Into<String>, label: impl Into<String>, icon: Icon) -> Self {
        Self { id: id.into(), label: label.into(), icon, description: String::new(), children: Vec::new() }
    }

    /// Adds the description shown on hub tiles.
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = description.into();
        self
    }

    /// Adds the pages of a section.
    pub fn with_children(mut self, children: Vec<NavEntry>) -> Self {
        self.children = children;
        self
    }
}

fn child_id(section: &str, page: &str) -> String {
    format!("{section}/{page}")
}

/// The logged-in layout of the app.
#[component]
pub fn AppShell(
    /// Product name shown next to the logo.
    brand: String,
    /// Main sections (sidebar and dock). Five at most, so they fit the dock.
    nav: Vec<NavEntry>,
    /// The "me" destination, at the bottom of the sidebar and in the phone top bar.
    account: NavEntry,
    /// Secondary line under the account name.
    account_detail: String,
    /// Id of the active section (or of `account`).
    active: String,
    /// Id of the active page inside the section, if any.
    #[props(default)]
    active_page: Option<String>,
    /// Called with `"section"` or `"section/page"`.
    on_navigate: EventHandler<String>,
    /// Congregations for the switcher.
    workspaces: Vec<SwitcherItem>,
    /// Active congregation id.
    active_workspace: Option<String>,
    /// Called with the chosen congregation id.
    on_switch_workspace: EventHandler<String>,
    /// Called when the user wants to create a congregation.
    on_create_workspace: EventHandler<()>,
    /// Opens the settings of the open workspace (cog in the switcher).
    #[props(default)]
    on_workspace_settings: Option<EventHandler<()>>,
    /// Changes on every navigation, to replay the page transition.
    transition_key: String,
    /// With it, the account opens a menu (*My settings*, *Sign out*) instead of going to `account`.
    #[props(default)]
    on_sign_out: Option<EventHandler<()>>,
    children: Element,
) -> Element {
    let mut side_menu = use_signal(|| false);
    let mut top_menu = use_signal(|| false);
    let account_items = move |mut open: Signal<bool>, id: String| {
        rsx! {
            div { class: "flex flex-col", role: "menu",
                button {
                    r#type: "button",
                    role: "menuitem",
                    class: "flex items-center gap-3 rounded-field px-3 py-2 text-left text-sm transition-colors hover:bg-base-200",
                    onclick: move |_| {
                        open.set(false);
                        on_navigate.call(id.clone());
                    },
                    IconView { icon: Icon::Settings, size: IconSize::Sm }
                    "My settings"
                }
                if let Some(h) = on_sign_out {
                    button {
                        r#type: "button",
                        role: "menuitem",
                        class: "flex items-center gap-3 rounded-field px-3 py-2 text-left text-sm text-error transition-colors hover:bg-error/10",
                        onclick: move |_| {
                            open.set(false);
                            h.call(());
                        },
                        IconView { icon: Icon::SignOut, size: IconSize::Sm }
                        "Sign out"
                    }
                }
            }
        }
    };
    let has_menu = on_sign_out.is_some();
    // Sections the user opened or closed by hand. The active section is open unless closed.
    let mut opened = use_signal(HashSet::<String>::new);
    let mut closed = use_signal(HashSet::<String>::new);
    let mut flyout = use_signal(|| None::<String>);

    let dock_idx = nav.iter().position(|n| n.id == active);
    let account_active = account.id == active;
    let dock_n = nav.len().max(1);

    rsx! {
        div { class: "flex min-h-dvh bg-base-200/50",
            // ---------------- Sidebar (md+) ----------------
            aside { class: "sticky top-0 z-30 hidden h-dvh w-20 shrink-0 flex-col gap-4 border-r border-base-300 bg-base-100 p-3 md:flex lg:w-64",
                div { class: "flex items-center gap-2.5 px-1.5 pt-1 md:max-lg:justify-center",
                    BrandMark {}
                    span { class: "hidden text-base font-semibold tracking-tight lg:inline", "{brand}" }
                }
                WorkspaceSwitcher {
                    items: workspaces.clone(),
                    active: active_workspace.clone(),
                    onselect: move |id| on_switch_workspace.call(id),
                    create_label: "Create congregation",
                    oncreate: move |_| on_create_workspace.call(()),
                            on_settings: on_workspace_settings,
                    placement: Placement::Side,
                }
                // Scroll only on desktop: on the tablet rail, a scroll container would clip the flyouts.
                nav { class: "-mx-1 flex min-h-0 flex-1 flex-col gap-1 px-1 lg:overflow-y-auto", "aria-label": "Main",
                    for n in nav.iter().cloned() {
                        {
                            let is_active = n.id == active;
                            let has_children = !n.children.is_empty();
                            let is_open = has_children && (opened().contains(&n.id) || (is_active && !closed().contains(&n.id)));
                            let flyout_open = flyout().as_deref() == Some(n.id.as_str());
                            let id = n.id.clone();
                            let row_class = match (is_active, account_active) {
                                (true, _) => "group relative flex h-11 w-full items-center gap-3 rounded-field bg-primary/10 px-3 text-sm font-semibold text-primary transition-colors md:max-lg:justify-center",
                                _ => "group relative flex h-11 w-full items-center gap-3 rounded-field px-3 text-sm font-medium text-base-content/70 transition-colors hover:bg-base-200 hover:text-base-content md:max-lg:justify-center",
                            };
                            rsx! {
                                div { key: "{n.id}", class: "relative",
                                    button {
                                        r#type: "button",
                                        class: row_class,
                                        "aria-current": if is_active && active_page.is_none() { "page" } else { "false" },
                                        "aria-expanded": if has_children { if is_open { "true" } else { "false" } } else { "false" },
                                        title: "{n.label}",
                                        onclick: move |_| {
                                            if has_children {
                                                // Rail (tablet): toggle the flyout. Desktop: accordion.
                                                flyout.set(if flyout_open { None } else { Some(id.clone()) });
                                                if !is_active {
                                                    opened.write().insert(id.clone());
                                                    closed.write().remove(&id);
                                                    on_navigate.call(id.clone());
                                                } else if is_open {
                                                    opened.write().remove(&id);
                                                    closed.write().insert(id.clone());
                                                } else {
                                                    closed.write().remove(&id);
                                                    opened.write().insert(id.clone());
                                                }
                                            } else {
                                                flyout.set(None);
                                                on_navigate.call(id.clone());
                                            }
                                        },
                                        span { class: if is_active { "animate-pop" } else { "" },
                                            IconView { icon: n.icon, size: IconSize::Md }
                                        }
                                        span { class: "hidden flex-1 text-left lg:inline", "{n.label}" }
                                        if has_children {
                                            span { class: if is_open { "hidden rotate-180 text-base-content/50 transition-transform duration-300 lg:inline" } else { "hidden text-base-content/50 transition-transform duration-300 lg:inline" },
                                                IconView { icon: Icon::ChevronDown, size: IconSize::Sm }
                                            }
                                        }
                                    }
                                    // Desktop accordion (animated with grid rows 0fr → 1fr).
                                    if has_children {
                                        div { class: if is_open { "hidden grid-rows-[1fr] transition-[grid-template-rows] duration-300 ease-[var(--ease-out-soft)] lg:grid" } else { "hidden grid-rows-[0fr] transition-[grid-template-rows] duration-300 ease-[var(--ease-out-soft)] lg:grid" },
                                            div { class: "min-h-0 overflow-hidden",
                                                ul { class: "ml-5 mt-1 flex flex-col gap-0.5 border-l border-base-300 pb-1 pl-3",
                                                    for c in n.children.iter().cloned() {
                                                        {
                                                            let selected = is_active && active_page.as_deref() == Some(c.id.as_str());
                                                            let target = child_id(&n.id, &c.id);
                                                            rsx! {
                                                                li { key: "{c.id}",
                                                                    button {
                                                                        r#type: "button",
                                                                        class: if selected { "relative flex h-9 w-full items-center rounded-field px-2.5 text-left text-sm font-semibold text-primary before:absolute before:-left-[13px] before:top-2 before:h-5 before:w-0.5 before:rounded-full before:bg-primary" } else { "flex h-9 w-full items-center rounded-field px-2.5 text-left text-sm text-base-content/65 transition-colors hover:bg-base-200 hover:text-base-content" },
                                                                        "aria-current": if selected { "page" } else { "false" },
                                                                        onclick: move |_| on_navigate.call(target.clone()),
                                                                        "{c.label}"
                                                                    }
                                                                }
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                        // Tablet rail flyout.
                                        if flyout_open {
                                            div { class: "fixed inset-0 z-40 lg:hidden", onclick: move |_| flyout.set(None) }
                                            div { class: "absolute left-full top-0 z-50 ml-3 w-64 origin-left animate-fade-in rounded-box border border-base-300 bg-base-100 p-1.5 shadow-xl shadow-base-content/10 lg:hidden",
                                                div { class: "px-2.5 pb-1 pt-1.5 text-xs font-semibold text-base-content/60", "{n.label}" }
                                                for c in n.children.iter().cloned() {
                                                    {
                                                        let selected = is_active && active_page.as_deref() == Some(c.id.as_str());
                                                        let target = child_id(&n.id, &c.id);
                                                        rsx! {
                                                            button {
                                                                key: "{c.id}",
                                                                r#type: "button",
                                                                class: if selected { "flex w-full items-center gap-2.5 rounded-field bg-primary/10 px-2.5 py-2 text-left text-sm font-semibold text-primary" } else { "flex w-full items-center gap-2.5 rounded-field px-2.5 py-2 text-left text-sm transition-colors hover:bg-base-200" },
                                                                onclick: move |_| {
                                                                    flyout.set(None);
                                                                    on_navigate.call(target.clone());
                                                                },
                                                                span { class: "text-base-content/60", IconView { icon: c.icon, size: IconSize::Sm } }
                                                                "{c.label}"
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
                    }
                }
                {
                    let id = account.id.clone();
                    let id_menu = account.id.clone();
                    rsx! {
                        Popover {
                            open: side_menu(),
                            onclose: move |_| side_menu.set(false),
                            placement: Placement::Above,
                            trigger: rsx! {
                                button {
                                    r#type: "button",
                                    class: if account_active { "flex w-full items-center gap-3 rounded-field bg-primary/10 p-2 text-left md:max-lg:justify-center" } else { "flex w-full items-center gap-3 rounded-field p-2 text-left transition-colors hover:bg-base-200 md:max-lg:justify-center" },
                                    title: "{account.label}",
                                    "aria-haspopup": if has_menu { "menu" } else { "false" },
                                    "aria-expanded": if side_menu() { "true" } else { "false" },
                                    onclick: move |_| if has_menu { side_menu.set(!side_menu()) } else { on_navigate.call(id.clone()) },
                                    Avatar { name: account.label.clone() }
                                    span { class: "hidden min-w-0 flex-1 flex-col lg:flex",
                                        span { class: "truncate text-sm font-semibold", "{account.label}" }
                                        span { class: "truncate text-xs text-base-content/60", "{account_detail}" }
                                    }
                                    if has_menu {
                                        span { class: "hidden text-base-content/50 lg:block", IconView { icon: Icon::ChevronsUpDown, size: IconSize::Sm } }
                                    }
                                }
                            },
                            {account_items(side_menu, id_menu)}
                        }
                    }
                }
            }

            // ---------------- Content column ----------------
            div { class: "flex min-w-0 flex-1 flex-col",
                // Phone top bar
                header { class: "sticky top-0 z-30 flex items-center gap-3 border-b border-base-300 bg-base-100/85 px-4 py-2.5 backdrop-blur md:hidden",
                    style: "padding-top: max(0.625rem, env(safe-area-inset-top));",
                    div { class: "min-w-0 max-w-[15rem]",
                        WorkspaceSwitcher {
                            items: workspaces.clone(),
                            active: active_workspace.clone(),
                            onselect: move |id| on_switch_workspace.call(id),
                            create_label: "Create congregation",
                            oncreate: move |_| on_create_workspace.call(()),
                            on_settings: on_workspace_settings,
                        }
                    }
                    div { class: "flex-1" }
                    {
                        let id = account.id.clone();
                        let id_menu = account.id.clone();
                        rsx! {
                            Popover {
                                open: top_menu(),
                                onclose: move |_| top_menu.set(false),
                                placement: Placement::End,
                                trigger: rsx! {
                                    button {
                                        r#type: "button",
                                        class: "rounded-full transition-transform active:scale-90",
                                        "aria-label": "{account.label}",
                                        "aria-haspopup": if has_menu { "menu" } else { "false" },
                                        onclick: move |_| if has_menu { top_menu.set(!top_menu()) } else { on_navigate.call(id.clone()) },
                                        Avatar { name: account.label.clone() }
                                    }
                                },
                                {account_items(top_menu, id_menu)}
                            }
                        }
                    }
                }
                main { class: "flex-1 px-4 pb-32 pt-6 sm:px-6 md:pb-10 lg:px-10 lg:pt-8",
                    div { class: "mx-auto w-full max-w-7xl",
                        // Keyed wrapper: a new key re-mounts the page and replays the enter animation.
                        for k in std::iter::once(transition_key.clone()) {
                            div { key: "{k}", class: "theo-page", {children.clone()} }
                        }
                    }
                }
            }

            // ---------------- Dock (phones) ----------------
            nav {
                class: "fixed inset-x-3 bottom-3 z-40 md:hidden",
                style: "margin-bottom: env(safe-area-inset-bottom);",
                "aria-label": "Main",
                div { class: "relative grid overflow-hidden rounded-2xl border border-base-300 bg-base-100/90 p-1 shadow-xl shadow-base-content/10 backdrop-blur",
                    style: "grid-template-columns: repeat({dock_n}, minmax(0, 1fr));",
                    // Sliding highlight. No overshoot: a spring would fly past the dock edge on
                    // long jumps (first ↔ last item).
                    div {
                        class: "pointer-events-none absolute bottom-1 left-1 top-1 rounded-xl bg-primary/12 transition-[transform,opacity] duration-[420ms] ease-[var(--ease-out-soft)]",
                        style: match dock_idx {
                            Some(i) => format!("width: calc((100% - 0.5rem) / {dock_n}); transform: translateX({}%); opacity: 1;", i * 100),
                            None => format!("width: calc((100% - 0.5rem) / {dock_n}); opacity: 0;"),
                        },
                    }
                    for n in nav.iter().cloned() {
                        {
                            let is_active = n.id == active;
                            let id = n.id.clone();
                            rsx! {
                                button {
                                    key: "{n.id}",
                                    r#type: "button",
                                    class: if is_active { "relative z-10 flex min-h-14 flex-col items-center justify-center gap-0.5 rounded-xl text-primary transition-transform duration-150 active:scale-90" } else { "relative z-10 flex min-h-14 flex-col items-center justify-center gap-0.5 rounded-xl text-base-content/60 transition-transform duration-150 active:scale-90" },
                                    "aria-current": if is_active { "page" } else { "false" },
                                    // Tapping the active item again goes back to the section hub.
                                    onclick: move |_| on_navigate.call(id.clone()),
                                    span { class: if is_active { "animate-pop" } else { "" },
                                        IconView { icon: n.icon, size: IconSize::Md }
                                    }
                                    span { class: if is_active { "max-w-full truncate text-[10.5px] font-semibold tracking-tight" } else { "max-w-full truncate text-[10.5px] font-medium tracking-tight" }, "{n.label}" }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

/// A section's hub: large tiles for its pages (the phone entry point for sub-menus).
#[component]
pub fn NavHub(section: NavEntry, on_navigate: EventHandler<String>) -> Element {
    rsx! {
        div { class: "grid grid-cols-2 gap-3 sm:gap-4 lg:grid-cols-3",
            for c in section.children.iter().cloned() {
                {
                    let target = child_id(&section.id, &c.id);
                    rsx! {
                        button {
                            key: "{c.id}",
                            r#type: "button",
                            class: "group flex min-h-32 flex-col items-start gap-2.5 rounded-box border border-base-300 bg-base-100 p-4 text-left transition-all duration-200 hover:-translate-y-0.5 hover:border-primary/40 hover:shadow-lg hover:shadow-base-content/5 active:scale-[0.98] sm:p-5",
                            onclick: move |_| on_navigate.call(target.clone()),
                            span { class: "grid size-11 place-items-center rounded-field bg-primary/10 text-primary transition-transform duration-200 group-hover:scale-110",
                                IconView { icon: c.icon, size: IconSize::Lg }
                            }
                            span { class: "flex w-full items-center justify-between gap-2",
                                span { class: "font-semibold leading-tight", "{c.label}" }
                                span { class: "hidden text-base-content/40 transition-transform group-hover:translate-x-0.5 sm:inline",
                                    IconView { icon: Icon::ArrowRight, size: IconSize::Sm }
                                }
                            }
                            if !c.description.is_empty() {
                                span { class: "line-clamp-2 text-xs leading-5 text-base-content/60 sm:text-sm", "{c.description}" }
                            }
                        }
                    }
                }
            }
        }
    }
}
