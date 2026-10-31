//! Anzan 暗算, the mental abacus: a soroban for kindergarten and first
//! grade. Kids meet the beads, read and set numbers, peek and remember,
//! add and take away with little and big friends, and finally put the
//! abacus away and move the beads in their heads.
//!
//! Each kid plays as their Sekka Kid profile (`sekka-kid`): their stars and
//! where they're up to are kept in their folder, and their time and rounds
//! go to their log for grown-ups' reports in Sekka Kids.

pub mod abacus;
pub mod app;
pub mod art;
pub mod lessons;
pub mod method;
pub mod progress;
pub mod round;
pub mod scene;
pub mod sound;

pub use app::{APP_ID, Anzan, Msg};
