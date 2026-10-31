//! What Ondoku keeps for each kid: where they are in each book, how often
//! they've read it through, how fast they read it (for grown-ups), and the
//! grown-ups' choices.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct BookProgress {
    /// The page to go on from (0 when finished).
    pub page: usize,
    /// Times read to the end.
    pub finished: u32,
    /// Words a minute, the last time through and the best.
    pub last_wpm: Option<u32>,
    pub best_wpm: Option<u32>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Progress {
    pub books: BTreeMap<String, BookProgress>,
    /// Fumi reads aloud and speaks.
    pub voice: bool,
    /// Fumi helps with a word after a pause.
    pub help: bool,
    /// The shelf last looked at.
    pub shelf: Option<u8>,
    /// Extra minutes a grown-up gave, and the day (its start) they're for.
    pub extra: Option<(i64, u32)>,
}

impl Default for Progress {
    fn default() -> Progress {
        Progress { books: BTreeMap::new(), voice: true, help: true, shelf: None, extra: None }
    }
}

impl Progress {
    pub fn book(&self, id: &str) -> BookProgress {
        self.books.get(id).cloned().unwrap_or_default()
    }

    /// Note the reader got to `page`.
    pub fn at(&mut self, id: &str, page: usize) {
        self.books.entry(id.to_owned()).or_default().page = page;
    }

    /// Note the book read to the end, at `wpm` words a minute.
    pub fn finished(&mut self, id: &str, wpm: Option<u32>) {
        let b = self.books.entry(id.to_owned()).or_default();
        b.page = 0;
        b.finished += 1;
        if wpm.is_some() {
            b.last_wpm = wpm;
            b.best_wpm = b.best_wpm.max(wpm);
        }
    }

    /// Extra minutes for the day starting at `day`.
    pub fn extra_minutes(&self, day: i64) -> u32 {
        match self.extra {
            Some((d, m)) if d == day => m,
            _ => 0,
        }
    }
}

/// Words a minute: `words` read between `from` and `to` (milliseconds),
/// if that's long enough to say.
pub fn wpm(words: u32, from: i64, to: i64) -> Option<u32> {
    let minutes = (to - from) as f64 / 60_000.0;
    (words >= 10 && minutes >= 0.1).then(|| (f64::from(words) / minutes).round() as u32)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bookmarks_and_times_through() {
        let mut p = Progress::default();
        p.at("hop-fox", 3);
        assert_eq!(p.book("hop-fox").page, 3);
        p.finished("hop-fox", Some(40));
        p.finished("hop-fox", Some(35));
        let b = p.book("hop-fox");
        assert_eq!((b.page, b.finished, b.last_wpm, b.best_wpm), (0, 2, Some(35), Some(40)));
        assert_eq!(wpm(60, 0, 60_000), Some(60));
        assert_eq!(wpm(5, 0, 60_000), None);
        let old: Progress = serde_json::from_str("{\"voice\": false}").unwrap();
        assert!(!old.voice && old.help);
    }
}
