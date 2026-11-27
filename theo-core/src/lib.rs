//! # theo-core
//!
//! The domain of Theo Manager (`REQUIREMENTS.md` 9.1): the models of every module, their rules,
//! eligibility and the permission model. No UI, no network, no async runtime (R-121); it builds for
//! `wasm32-unknown-unknown` and every native target.
//!
//! [`people::Directory`] holds a congregation's records of every module. Each module file follows its
//! `docs/model/mNN-*.md` description.

pub mod absences;
pub mod access;
pub mod assignments;
pub mod attendance;
pub mod collections;
pub mod demo;
pub mod duties;
pub mod events;
pub mod invites;
pub mod midweek;
pub mod ministry;
pub mod people;
pub mod prefs;
pub mod privileges;
pub mod public_witnessing;
pub mod reports;
pub mod scheduling;
pub mod service_meetings;
pub mod store;
pub mod talks;
pub mod tasks;
pub mod territories;
pub mod weekend;
pub mod workbook;
