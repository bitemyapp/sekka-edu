//! What Sekka Edu apps share on screen, on Aizome's kit: the kids' look
//! ([`look`], [`stage`], [`art`]), their animals ([`avatar`]), "Who's playing?"
//! ([`who`]) and making a new player ([`newkid`]), the grown-ups' door
//! ([`gate`]), a kid's report for grown-ups ([`report_view`]), and tiles that
//! wrap in a narrow window ([`tiles`]).

pub mod art;
pub mod avatar;
pub mod gate;
pub mod instance;
pub mod look;
pub mod newkid;
pub mod report_view;
pub mod stage;
pub mod tiles;
pub mod who;

pub use look::Look;
