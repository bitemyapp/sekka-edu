//! What Anzan keeps for each kid (in their Sekka Kid folder): stars won on
//! each lesson, which introductions they've heard, how long their peeks
//! last, sound and voice, and extra time a grown-up gave today.

use std::collections::{BTreeMap, BTreeSet};

use sekka_kid::Grade;
use serde::{Deserialize, Serialize};

use crate::lessons::{CHAPTERS, LESSONS};
use crate::round::PEEK_MS;

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct LessonRecord {
    /// The most stars won.
    pub stars: u8,
    /// Rounds finished.
    pub plays: u32,
    pub last: i64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Progress {
    #[serde(default)]
    pub lessons: BTreeMap<String, LessonRecord>,
    #[serde(default = "yes")]
    pub sound: bool,
    #[serde(default = "yes")]
    pub voice: bool,
    #[serde(default)]
    pub intros: BTreeSet<String>,
    #[serde(default = "peek")]
    pub peek_ms: u32,
    /// Minutes more a grown-up allowed, and the day (its start) they're
    /// for.
    #[serde(default)]
    pub extra: Option<(i64, u32)>,
}

fn yes() -> bool {
    true
}

fn peek() -> u32 {
    PEEK_MS
}

impl Default for Progress {
    fn default() -> Progress {
        Progress {
            lessons: BTreeMap::new(),
            sound: true,
            voice: true,
            intros: BTreeSet::new(),
            peek_ms: PEEK_MS,
            extra: None,
        }
    }
}

impl Progress {
    pub fn stars(&self, lesson: usize) -> u8 {
        self.lessons.get(LESSONS[lesson].id).map_or(0, |r| r.stars)
    }

    /// Finished at least once.
    pub fn done(&self, lesson: usize) -> bool {
        self.lessons.get(LESSONS[lesson].id).is_some_and(|r| r.plays > 0)
    }

    /// Whether a kid in `grade` may play `lesson`: the first, any after one
    /// done, and for first graders on, all of kindergarten's and the first
    /// of first grade's.
    pub fn unlocked(&self, lesson: usize, grade: Grade) -> bool {
        let first_grade = LESSONS.iter().position(|l| CHAPTERS[l.chapter].grade >= Grade::First);
        lesson == 0
            || self.done(lesson)
            || self.done(lesson - 1)
            || (grade >= Grade::First
                && (CHAPTERS[LESSONS[lesson].chapter].grade <= Grade::Kindergarten || Some(lesson) == first_grade))
    }

    /// Where the kid's marker stands: the first lesson open and not done,
    /// else the last open.
    pub fn here(&self, grade: Grade) -> usize {
        (0..LESSONS.len())
            .find(|i| self.unlocked(*i, grade) && !self.done(*i))
            .unwrap_or_else(|| (0..LESSONS.len()).rev().find(|i| self.unlocked(*i, grade)).unwrap_or(0))
    }

    /// Record a finished round.
    pub fn finished(&mut self, lesson: usize, stars: u8, now: i64) -> bool {
        let r = self.lessons.entry(LESSONS[lesson].id.to_owned()).or_default();
        let better = stars > r.stars;
        let first = r.plays == 0;
        r.stars = r.stars.max(stars);
        r.plays += 1;
        r.last = now;
        better || first
    }

    /// Lanterns lit.
    pub fn lit(&self) -> usize {
        (0..LESSONS.len()).filter(|i| self.done(*i)).count()
    }

    /// Extra minutes for the day starting at `day`.
    pub fn extra_minutes(&self, day: i64) -> u32 {
        match self.extra {
            Some((d, m)) if d == day => m,
            _ => 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lessons::index_of;

    #[test]
    fn lessons_open_one_after_another() {
        let mut p = Progress::default();
        let k = Grade::Kindergarten;
        assert!(p.unlocked(0, k));
        assert!(!p.unlocked(1, k));
        assert_eq!(p.here(k), 0);
        p.finished(0, 1, 5);
        assert!(p.unlocked(1, k));
        assert_eq!(p.here(k), 1);
        assert!(!p.unlocked(2, k));
    }

    #[test]
    fn first_graders_may_skip_kindergarten() {
        let p = Progress::default();
        let first = Grade::First;
        for (i, l) in LESSONS.iter().enumerate() {
            let k = CHAPTERS[l.chapter].grade == Grade::Kindergarten;
            assert_eq!(p.unlocked(i, first), k || i == index_of("tens").unwrap(), "{}", l.id);
        }
        assert_eq!(p.here(first), 0, "but start at the start");
    }

    #[test]
    fn the_best_stars_are_kept() {
        let mut p = Progress::default();
        assert!(p.finished(3, 2, 1), "the first time counts");
        assert!(!p.finished(3, 1, 2));
        assert!(p.finished(3, 3, 3));
        assert_eq!(p.stars(3), 3);
        assert_eq!(p.lessons[LESSONS[3].id].plays, 3);
    }

    #[test]
    fn old_records_read_with_defaults() {
        let p: Progress = serde_json::from_str("{\"lessons\":{}}").unwrap();
        assert!(p.sound && p.voice);
        assert_eq!(p.peek_ms, PEEK_MS);
    }
}
