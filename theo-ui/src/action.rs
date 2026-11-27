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
    Settings,
    CreateBackup,
    RestoreBackup,
    MoveToCloud,
    MoveToDevice,
    ManageLabels,
    Apply,
    Undo,
    Send,
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
            Action::Settings => "Settings",
            Action::CreateBackup => "Create backup",
            Action::RestoreBackup => "Restore backup",
            Action::MoveToCloud => "Move to the Cloud",
            Action::MoveToDevice => "Move to this device",
            Action::ManageLabels => "Labels",
            Action::Apply => "Apply",
            Action::Undo => "Undo",
            Action::Send => "Send",
        }
    }

    /// The icon of the action, if it has one.
    pub fn icon(self) -> Option<Icon> {
        match self {
            Action::Save => Some(Icon::Save),
            Action::Done => Some(Icon::Check),
            Action::Settings => Some(Icon::Settings),
            Action::CreateBackup => Some(Icon::Download),
            Action::RestoreBackup => Some(Icon::Upload),
            Action::MoveToCloud => Some(Icon::Cloud),
            Action::MoveToDevice => Some(Icon::Local),
            Action::ManageLabels => Some(Icon::Tag),
            Action::Apply => Some(Icon::Check),
            Action::Undo => Some(Icon::MoveBack),
            Action::Send => Some(Icon::Send),
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
            // Sign out ends the session, so it uses the same red style as other leaving actions.
            Action::Delete | Action::SignOut => ButtonVariant::Destructive,
            Action::Cancel
            | Action::Back
            | Action::ResetLayout
            | Action::Regenerate
            | Action::ClearFilters
            | Action::Remove
            | Action::Undo => {
                ButtonVariant::Ghost
            }
            _ => ButtonVariant::Secondary,
        }
    }
}

/// The buttons of a dialog that edits one thing: Delete and Save side by side (Delete only when
/// it exists), Cancel below over the full width. Put it in the `actions` of a `Dialog`.
#[component]
pub fn EditActions(
    on_save: EventHandler<()>,
    on_cancel: EventHandler<()>,
    #[props(default)] on_delete: Option<EventHandler<()>>,
    #[props(default)] save_label: Option<String>,
) -> Element {
    let save_span = if on_delete.is_some() { "" } else { "col-span-2" };
    rsx! {
        div { class: "grid w-full! grid-cols-2 gap-2 [&_button]:w-full",
            if let Some(d) = on_delete {
                ActionButton { action: Action::Delete, onclick: move |_| d.call(()) }
            }
            div { class: save_span,
                ActionButton { action: Action::Save, label: save_label, onclick: move |_| on_save.call(()) }
            }
            div { class: "col-span-2",
                ActionButton { action: Action::Cancel, onclick: move |_| on_cancel.call(()) }
            }
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
