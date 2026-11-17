//! The action catalogue (R-422): one label, one icon and one style per action, everywhere.

use crate::button::{Button, ButtonKind, ButtonVariant};
use crate::icon::Icon;
use dioxus::prelude::*;

/// Every user action the app offers. Screens use these, never their own button texts.
#[allow(missing_docs)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    Save,
    Cancel,
    Back,
    Next,
    Finish,
    Edit,
    Delete,
    Add,
    Customize,
    Done,
    ResetLayout,
    SignIn,
    SignUp,
    SignOut,
    Lock,
    Unlock,
    UnlockBiometric,
    CreateCongregation,
    TryDemo,
    Regenerate,
    Export,
    Invite,
    Filters,
    ClearFilters,
    Remove,
    MakeFamilyHead,
    CopyLink,
    MoveAway,
    MoveBack,
    View,
    AddMembers,
}

impl Action {
    /// The label (English for now; i18n keys follow in R-610 / R-429).
    pub fn label(self) -> &'static str {
        match self {
            Action::Save => "Save",
            Action::Cancel => "Cancel",
            Action::Back => "Back",
            Action::Next => "Next",
            Action::Finish => "Create congregation",
            Action::Edit => "Edit",
            Action::Delete => "Delete",
            Action::Add => "Add",
            Action::Customize => "Customize",
            Action::Done => "Done",
            Action::ResetLayout => "Reset layout",
            Action::SignIn => "Log in",
            Action::SignUp => "Sign up",
            Action::SignOut => "Sign out",
            Action::Lock => "Lock",
            Action::Unlock => "Unlock",
            Action::UnlockBiometric => "Unlock with biometrics",
            Action::CreateCongregation => "New congregation",
            Action::TryDemo => "Try the demo",
            Action::Regenerate => "Generate another",
            Action::Export => "Export",
            Action::Invite => "Invite",
            Action::Filters => "Filters",
            Action::ClearFilters => "Clear filters",
            Action::Remove => "Remove",
            Action::MakeFamilyHead => "Make family head",
            Action::CopyLink => "Copy link",
            Action::MoveAway => "Mark as moved away",
            Action::MoveBack => "Move back",
            Action::View => "Show",
            Action::AddMembers => "Add members",
        }
    }

    /// The icon of the action, if it has one.
    pub fn icon(self) -> Option<Icon> {
        match self {
            Action::Save | Action::Done => Some(Icon::Check),
            Action::Next => Some(Icon::ArrowRight),
            Action::Finish => Some(Icon::Check),
            Action::Edit => Some(Icon::Edit),
            Action::Delete => Some(Icon::Trash),
            Action::Export => Some(Icon::Download),
            Action::Invite => Some(Icon::Invitation),
            Action::Filters => Some(Icon::Filter),
            Action::ClearFilters | Action::Remove => Some(Icon::Close),
            Action::MakeFamilyHead => Some(Icon::FamilyHead),
            Action::CopyLink => Some(Icon::Copy),
            Action::MoveAway => Some(Icon::MovedAway),
            Action::MoveBack => Some(Icon::MoveBack),
            Action::View => Some(Icon::ArrowRight),
            Action::AddMembers => Some(Icon::SignUp),
            Action::Add | Action::CreateCongregation => Some(Icon::Plus),
            Action::Customize => Some(Icon::Settings),
            Action::ResetLayout | Action::Regenerate => Some(Icon::Refresh),
            Action::SignIn => Some(Icon::SignIn),
            Action::SignUp => Some(Icon::SignUp),
            Action::SignOut => Some(Icon::SignOut),
            Action::Lock => Some(Icon::Lock),
            Action::Unlock => Some(Icon::Unlock),
            Action::UnlockBiometric => Some(Icon::Fingerprint),
            Action::TryDemo => Some(Icon::Sparkles),
            Action::Cancel | Action::Back => None,
        }
    }

    /// The default button style of the action (R-423).
    pub fn variant(self) -> ButtonVariant {
        match self {
            Action::Save
            | Action::Next
            | Action::Finish
            | Action::SignIn
            | Action::SignUp
            | Action::Unlock
            | Action::CreateCongregation
            | Action::Add
            | Action::AddMembers
            | Action::Done => ButtonVariant::Primary,
            Action::Delete => ButtonVariant::Destructive,
            Action::Cancel
            | Action::Back
            | Action::ResetLayout
            | Action::Regenerate
            | Action::ClearFilters
            | Action::Remove => {
                ButtonVariant::Ghost
            }
            _ => ButtonVariant::Secondary,
        }
    }
}

/// A button for a catalogue [`Action`]. Use `variant` only to demote a primary action
/// when a screen already has one (R-423: at most one primary per screen).
#[component]
pub fn ActionButton(
    action: Action,
    #[props(default)] variant: Option<ButtonVariant>,
    #[props(default)] kind: ButtonKind,
    #[props(default)] loading: bool,
    #[props(default)] disabled: bool,
    #[props(default)] full_width: bool,
    #[props(default)] label: Option<String>,
    onclick: Option<EventHandler<MouseEvent>>,
) -> Element {
    rsx! {
        Button {
            variant: variant.unwrap_or(action.variant()),
            kind,
            icon: action.icon(),
            loading,
            disabled,
            full_width,
            onclick: move |e| {
                if let Some(h) = &onclick {
                    h.call(e)
                }
            },
            {label.unwrap_or_else(|| action.label().to_string())}
        }
    }
}
