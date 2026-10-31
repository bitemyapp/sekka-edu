//! Sekka Kid profiles, shared by every app in the Sekka Edu catalogue.
//!
//! A profile is a kid: a name, an animal and a colour to recognise
//! themselves by, a grade, and optionally how many minutes a day a grown-up
//! allows. Each kid has an activity log that every app appends to: stretches
//! of active play ([`Event::Active`]), rounds played with their scores
//! ([`Event::Round`]) and milestones ([`Event::Milestone`]). Reports for
//! grown-ups ([`report`]) are built from the log, across apps.
//!
//! Everything lives under one folder ([`Store::default_root`]):
//!
//! ```text
//! profiles.json                 the kids
//! catalogue/<app id>.json       each app that has run, by name
//! kids/<kid id>/activity.jsonl  the log, one event a line, appended
//! kids/<kid id>/apps/<id>.json  each app's own progress for the kid
//! ```
//!
//! Apps run side by side and write the same log: an event is one `write`
//! to a file opened for appending, and whole files (profiles, progress) are
//! replaced by renaming a finished copy over them.

pub mod activity;
pub mod clock;
pub mod profile;
pub mod report;
pub mod store;

pub use activity::{Event, Round, Tracker};
pub use profile::{Animal, Grade, KidColor, NewProfile, Profile};
pub use report::{Report, report};
pub use store::{AppEntry, Store};
