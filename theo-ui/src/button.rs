//! Buttons. Closed component: no `class` or `style` prop (R-430).

use crate::feedback::Spinner;
use crate::icon::{Icon, IconSize, IconView};
use dioxus::prelude::*;

/// Visual hierarchy of a button (R-423).
#[allow(missing_docs)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ButtonVariant {
    Primary,
    #[default]
    Secondary,
    Ghost,
    Destructive,
    Link,
}

/// What the button does in a form.
#[allow(missing_docs)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ButtonKind {
    #[default]
    Button,
    Submit,
}

/// Button sizes.
#[allow(missing_docs)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ButtonSize {
    Sm,
    #[default]
    Md,
    Lg,
}

/// A button. For catalogue actions, prefer [`crate::ActionButton`].
#[component]
pub fn Button(
    #[props(default)] variant: ButtonVariant,
    #[props(default)] size: ButtonSize,
    #[props(default)] kind: ButtonKind,
    #[props(default)] icon: Option<Icon>,
    #[props(default)] loading: bool,
    #[props(default)] disabled: bool,
    #[props(default)] full_width: bool,
    onclick: Option<EventHandler<MouseEvent>>,
    children: Element,
) -> Element {
    let variant_class = match variant {
        ButtonVariant::Primary => "btn-primary shadow-sm",
        ButtonVariant::Secondary => "btn-soft border-base-300 bg-base-100 hover:bg-base-200",
        ButtonVariant::Ghost => "btn-ghost",
        ButtonVariant::Destructive => "btn-error",
        ButtonVariant::Link => "btn-link px-0",
    };
    let size_class = match size {
        ButtonSize::Sm => "btn-sm",
        ButtonSize::Md => "",
        ButtonSize::Lg => "btn-lg",
    };
    let width = if full_width { "w-full" } else { "" };
    let kind_attr = match kind {
        ButtonKind::Button => "button",
        ButtonKind::Submit => "submit",
    };
    rsx! {
        button {
            r#type: kind_attr,
            class: "btn {variant_class} {size_class} {width} gap-2 font-medium transition-transform duration-150 active:scale-[0.97]",
            disabled: disabled || loading,
            "aria-busy": if loading { "true" } else { "false" },
            onclick: move |e| {
                if let Some(h) = &onclick {
                    h.call(e)
                }
            },
            if loading {
                Spinner { small: true }
            } else if let Some(icon) = icon {
                IconView { icon, size: IconSize::Sm }
            }
            {children}
        }
    }
}

/// A square icon-only button. `label` is required for screen readers (R-418).
#[component]
pub fn IconButton(
    icon: Icon,
    label: String,
    #[props(default)] active: bool,
    onclick: Option<EventHandler<MouseEvent>>,
) -> Element {
    let active_class = if active { "btn-active text-primary" } else { "" };
    rsx! {
        button {
            r#type: "button",
            class: "btn btn-ghost btn-square btn-sm {active_class} transition-transform duration-150 active:scale-90",
            "aria-label": "{label}",
            title: "{label}",
            onclick: move |e| {
                if let Some(h) = &onclick {
                    h.call(e)
                }
            },
            IconView { icon, size: IconSize::Sm }
        }
    }
}
