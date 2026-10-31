//! What a kid did: the events of their log, and the [`Tracker`] that turns
//! an app's input into stretches of active play.

use serde::{Deserialize, Serialize};

use crate::store::Store;

/// One line of a kid's log.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum Event {
    /// Played `app` actively from `from` to `to` (Unix milliseconds).
    Active { app: String, from: i64, to: i64 },
    /// A round of an activity, finished or left.
    Round(Round),
    /// Something worth telling a grown-up ("Lit the lantern for Big
    /// friends").
    Milestone { app: String, at: i64, text: String },
}

/// A round: a handful of questions of one activity.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Round {
    pub app: String,
    /// The activity's stable id within the app.
    pub activity: String,
    /// Its name, as the kid saw it.
    pub title: String,
    /// What it practises (`numbers-to-9`, `little-friends`), for reports
    /// across apps.
    #[serde(default)]
    pub skills: Vec<String>,
    pub started: i64,
    pub ended: i64,
    /// Questions asked.
    pub items: u32,
    /// Answered right the first time, without a hint.
    pub first_try: u32,
    /// Hints asked for or given.
    #[serde(default)]
    pub hints: u32,
    /// 0 to 3.
    #[serde(default)]
    pub stars: u8,
    /// Played to the end, or left part way.
    pub finished: bool,
    /// The middle time from a question to its right answer.
    #[serde(default)]
    pub response_ms: Option<u32>,
}

/// After this long without input, a kid has stopped playing.
pub const IDLE_MS: i64 = 60_000;
/// Time after the last input still counted as playing: looking, thinking,
/// listening.
pub const GRACE_MS: i64 = 10_000;

/// Counts a kid's active time in one app and writes it to their log.
///
/// The app calls [`touch`](Tracker::touch) on input and
/// [`flush`](Tracker::flush) now and then (and when the kid stops or
/// switches): each flush writes what's been played since the last one, so a
/// crash loses at most that.
#[derive(Debug)]
pub struct Tracker {
    store: Store,
    kid: String,
    app: String,
    /// Where the unwritten stretch starts, while the kid is playing.
    start: Option<i64>,
    last: i64,
}

impl Tracker {
    pub fn new(store: Store, kid: &str, app: &str) -> Tracker {
        Tracker { store, kid: kid.to_owned(), app: app.to_owned(), start: None, last: 0 }
    }

    pub fn kid(&self) -> &str {
        &self.kid
    }

    /// Input at `now`.
    pub fn touch(&mut self, now: i64) {
        match self.start {
            None => self.start = Some(now),
            Some(s) if now - self.last > IDLE_MS => {
                self.write(s, self.last + GRACE_MS);
                self.start = Some(now);
            }
            Some(_) => {}
        }
        self.last = now;
    }

    /// Write the play so far; the kid may go on.
    pub fn flush(&mut self, now: i64) {
        let Some(s) = self.start else { return };
        let end = (self.last + GRACE_MS).min(now).max(s);
        self.write(s, end);
        self.start = (now - self.last <= IDLE_MS).then_some(end);
    }

    /// Write the play so far; the kid has stopped.
    pub fn finish(&mut self, now: i64) {
        self.flush(now);
        self.start = None;
    }

    /// Played but not yet written, in milliseconds.
    pub fn unwritten(&self, now: i64) -> i64 {
        self.start.map_or(0, |s| ((self.last + GRACE_MS).min(now) - s).max(0))
    }

    fn write(&self, from: i64, to: i64) {
        if to <= from {
            return;
        }
        let event = Event::Active { app: self.app.clone(), from, to };
        if let Err(e) = self.store.append(&self.kid, &event) {
            log::warn!("{}: {e}", self.store.root().display());
        }
    }
}
