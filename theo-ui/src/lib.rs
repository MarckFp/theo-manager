//! # theo-ui
//!
//! The Theo Manager design system. It is the **only** place where markup and styles live
//! (REQUIREMENTS.md 8.2 and 8.4). Screens in `theo-app` compose these components and never
//! style anything themselves.
//!
//! Rules this crate follows:
//! - One component per UI pattern (8.4). Page templates, forms, menus and messages look the same
//!   everywhere.
//! - Closed components: no free `class` or `style` props (R-430). Variants are enums.
//! - One icon per concept ([`Icon`], R-428) and one action catalogue ([`Action`], R-422).
//! - Styles come from `assets/theo-ui.css`, built by `scripts/build-css.*` from `css/input.css`
//!   (Tailwind CSS v4 + daisyUI 5, no Node.js).

mod action;
mod button;
mod card;
mod charts;
mod dashboard;
mod feedback;
mod filter;
mod form;
mod icon;
mod illustration;
mod landing;
mod layout;
mod menu;
mod overlay;
mod page;
mod period;
mod rows;
mod shell;
mod typography;

pub use action::{Action, ActionButton};
pub use button::{Button, ButtonKind, ButtonSize, ButtonVariant, IconButton};
pub use card::{Avatar, BrandMark, Card, CheckList, ItemList, ListEntry, RecordCard, Tag};
pub use charts::{BarChart, ChartPoint, MonthMark, MonthStrip, ProgressRow, Sparkline, StatTile, Trend};
pub use dashboard::{Dashboard, WidgetLayout};
pub use feedback::{use_toaster, Alert, Badge, EmptyState, Skeleton, Spinner, Toaster, Tone};
pub use filter::{Facet, FacetOption, FilterBar, FilterSelection};
pub use form::{
    Checkbox, ChipToggle, Choice, ChoiceCards, Field, FileButton, Form, InputKind, PasswordInput, RuleChecklist, Select,
    SelectOption, Switch, TextArea, TextInput,
};
pub use icon::{Icon, IconSize, IconView};
pub use illustration::{Illustration, IllustrationKind};
pub use landing::{CompareCard, FeatureCard, Hero, PublicLayout, Section, StepCard};
pub use layout::{Align, Cluster, Container, Divider, Gap, Grid, Justify, Spacer, Split, Stack, StickyBar};
pub use menu::{
    MenuButton, MenuItem, Placement, Popover, Segment, Segmented, SwitcherItem, WorkspaceSwitcher,
};
pub use overlay::{ConfirmDialog, Dialog};
pub use period::{month_name, MonthPicker, YearMonth};
pub use rows::{PickList, Row, RowList};
pub use page::{AuthLayout, Page, TextLink, WizardLayout};
pub use shell::{AppShell, NavEntry, NavHub};
pub use typography::{Code, Heading, HeadingLevel, Text, TextTone};

use dioxus::prelude::*;

/// Waits for `d` without blocking the UI, on every platform (web and native).
pub async fn sleep(d: std::time::Duration) {
    futures_timer::Delay::new(d).await;
}

/// The compiled design-system stylesheet.
pub const STYLESHEET: Asset = asset!("/assets/theo-ui.css");

/// Colour theme.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Theme {
    /// Follow the operating system setting.
    #[default]
    System,
    /// Always light.
    Light,
    /// Always dark.
    Dark,
}

impl Theme {
    /// Stable value for storage and segmented controls.
    pub fn as_str(self) -> &'static str {
        match self {
            Theme::System => "system",
            Theme::Light => "light",
            Theme::Dark => "dark",
        }
    }

    /// Parses [`Theme::as_str`].
    pub fn parse(s: &str) -> Self {
        match s {
            "light" => Theme::Light,
            "dark" => Theme::Dark,
            _ => Theme::System,
        }
    }
}

/// Root of every app that uses theo-ui: loads the stylesheet, applies the theme and hosts
/// the toasts. Use it once, around the router.
#[component]
pub fn ThemeRoot(#[props(default)] theme: Theme, children: Element) -> Element {
    feedback::provide_toaster();
    let data_theme = match theme {
        Theme::System => None,
        Theme::Light => Some("theo-light"),
        Theme::Dark => Some("theo-dark"),
    };
    rsx! {
        document::Stylesheet { href: STYLESHEET }
        document::Meta { name: "viewport", content: "width=device-width, initial-scale=1, viewport-fit=cover" }
        document::Meta { name: "color-scheme", content: "light dark" }
        div { "data-theme": data_theme, class: "min-h-dvh bg-base-100 font-sans text-base-content",
            {children}
            feedback::ToastHost {}
        }
    }
}
