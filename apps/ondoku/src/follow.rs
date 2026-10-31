//! Following a reader along a page: what Whisper heard is lined up with the
//! words still to read, and the page moves on to just after the last word
//! heard. Words left out or misread on the way are marked for practice;
//! words a reader repeats, or "um"s, cost little. A guess heard while the
//! reader is still going ([`Page::guess`]) only shows how far they've got;
//! what's heard at a pause ([`Page::hear`]) marks the words.

use crate::words::{self, Word};

/// How a word on the page has gone.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Mark {
    Unread,
    Read,
    /// Fumi said it: the reader asked, or was stuck on it.
    Helped,
    /// Read past without it being heard: skipped or misread.
    Missed,
}

/// A page being read.
#[derive(Clone, Debug, PartialEq)]
pub struct Page {
    pub words: Vec<Word>,
    pub marks: Vec<Mark>,
    /// The first word not yet read.
    pub next: usize,
    /// How far a guess heard mid-sentence has the reader (at least `next`).
    pub ahead: usize,
    /// When the first word was heard, and the last.
    pub started: Option<i64>,
    pub ended: Option<i64>,
}

/// One step of lining up.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Step {
    /// The printed word, heard (from one heard word, or two run together).
    Match(usize),
    /// A different word heard in its place.
    Swap,
    /// The printed word, not heard.
    Skip,
    /// A heard word not on the page.
    Extra,
}

/// The cost of `heard` read as `printed[..j]`, at its best, for each `j`;
/// and how to get there.
fn line_up(printed: &[&str], heard: &[String]) -> (Vec<f64>, Vec<Vec<Option<Step>>>) {
    let (n, m) = (printed.len(), heard.len());
    let mut cost = vec![vec![f64::INFINITY; m + 1]; n + 1];
    let mut how = vec![vec![None; m + 1]; n + 1];
    cost[0][0] = 0.0;
    for j in 0..=n {
        for i in 0..=m {
            let here = cost[j][i];
            if !here.is_finite() {
                continue;
            }
            let mut relax = |jj: usize, ii: usize, c: f64, s: Step| {
                if jj <= n && ii <= m && here + c < cost[jj][ii] {
                    cost[jj][ii] = here + c;
                    how[jj][ii] = Some(s);
                }
            };
            if j < n && i < m {
                if words::same(printed[j], &heard[i]) {
                    relax(j + 1, i + 1, 0.0, Step::Match(1));
                } else {
                    relax(j + 1, i + 1, 1.0, Step::Swap);
                }
                // "fir tree" heard for "fir-tree".
                if i + 1 < m && words::same(printed[j], &format!("{}{}", heard[i], heard[i + 1])) {
                    relax(j + 1, i + 2, 0.0, Step::Match(2));
                }
            }
            if j < n {
                relax(j + 1, i, 1.0, Step::Skip);
            }
            if i < m {
                // Repeats and "um"s are cheap, so that a reader going back
                // over a word doesn't lose their place.
                relax(j, i + 1, 0.6, Step::Extra);
            }
        }
    }
    ((0..=n).map(|j| cost[j][m]).collect(), how)
}

impl Page {
    pub fn new(text: &str) -> Page {
        let words = words::words(text);
        let marks = vec![Mark::Unread; words.len()];
        let mut p = Page { words, marks, next: 0, ahead: 0, started: None, ended: None };
        p.pass_unspoken();
        p.ahead = p.next;
        p
    }

    /// Step over what isn't read aloud (a lone dash).
    fn pass_unspoken(&mut self) {
        while self.next < self.words.len() && !self.words[self.next].spoken() {
            self.marks[self.next] = Mark::Read;
            self.next += 1;
        }
    }

    pub fn done(&self) -> bool {
        self.next >= self.words.len()
    }

    /// The spoken words from `next`, as indices into `words`, at most `n`.
    fn window(&self, n: usize) -> Vec<usize> {
        (self.next..self.words.len()).filter(|i| self.words[*i].spoken()).take(n).collect()
    }

    /// Where `heard` takes the reader: the last printed word heard (as an
    /// index into `window`'s words), and the steps there.
    fn follow(&self, heard: &[String]) -> Option<(Vec<usize>, Vec<Step>, usize)> {
        if heard.is_empty() {
            return None;
        }
        // A reader can't get further than every heard word reading one
        // printed word, and a few skipped.
        let window = self.window(heard.len() + 6);
        let printed: Vec<&str> = window.iter().map(|i| self.words[*i].key.as_str()).collect();
        let (ends, how) = line_up(&printed, heard);
        // The end that costs least, and of those the furthest: trailing
        // printed words not heard would cost, so it ends on a heard word.
        let mut best = 0;
        for (j, c) in ends.iter().enumerate() {
            if *c <= ends[best] + 1e-9 {
                best = j;
            }
        }
        let mut steps = Vec::new();
        let (mut j, mut i) = (best, heard.len());
        while let Some(s) = how[j][i] {
            steps.push(s);
            match s {
                Step::Match(k) => {
                    j -= 1;
                    i -= k;
                }
                Step::Swap => {
                    j -= 1;
                    i -= 1;
                }
                Step::Skip => j -= 1,
                Step::Extra => i -= 1,
            }
        }
        steps.reverse();
        // Up to the last printed word heard; a reader who stopped on a
        // misread word hasn't got past it.
        let mut reached = 0;
        let mut at = 0;
        for s in &steps {
            match s {
                Step::Match(_) => {
                    at += 1;
                    reached = at;
                }
                Step::Swap | Step::Skip => at += 1,
                Step::Extra => {}
            }
        }
        (reached > 0).then_some((window, steps, reached))
    }

    /// What's heard so far of a sentence still being read: only how far
    /// along the reader seems to be.
    pub fn guess(&mut self, heard: &str) {
        let heard = words::heard(heard);
        if let Some((window, _, reached)) = self.follow(&heard) {
            self.ahead = self.ahead.max(window[reached - 1] + 1);
        }
    }

    /// What's heard at a pause: the words read are marked, and the page
    /// moves on past the last word heard. Whether any word was.
    pub fn hear(&mut self, heard: &str, now: i64) -> bool {
        let heard = words::heard(heard);
        let Some((window, steps, reached)) = self.follow(&heard) else { return false };
        let mut at = 0;
        for s in steps {
            if at >= reached {
                break;
            }
            let i = window[at];
            match s {
                Step::Match(_) => {
                    if self.marks[i] == Mark::Unread {
                        self.marks[i] = Mark::Read;
                    }
                    at += 1;
                }
                Step::Swap | Step::Skip => {
                    if self.marks[i] == Mark::Unread {
                        self.marks[i] = Mark::Missed;
                    }
                    at += 1;
                }
                Step::Extra => {}
            }
        }
        // Dashes between the words passed.
        for i in self.next..window[reached - 1] {
            if !self.words[i].spoken() {
                self.marks[i] = Mark::Read;
            }
        }
        self.next = window[reached - 1] + 1;
        self.pass_unspoken();
        self.ahead = self.ahead.max(self.next);
        self.started.get_or_insert(now);
        self.ended = Some(now);
        true
    }

    /// Fumi says word `i`: it's helped, and if it's the next to read, the
    /// reader goes on from after it.
    pub fn help(&mut self, i: usize, now: i64) {
        if i >= self.words.len() {
            return;
        }
        if self.marks[i] != Mark::Read {
            self.marks[i] = Mark::Helped;
        }
        if i == self.next {
            self.next += 1;
            self.pass_unspoken();
            self.ahead = self.ahead.max(self.next);
            self.started.get_or_insert(now);
            self.ended = Some(now);
        }
    }

    /// Spoken words on the page.
    pub fn spoken(&self) -> u32 {
        self.words.iter().filter(|w| w.spoken()).count() as u32
    }

    /// Spoken words read by the reader alone.
    pub fn read(&self) -> u32 {
        self.words.iter().zip(&self.marks).filter(|(w, m)| w.spoken() && **m == Mark::Read).count() as u32
    }

    pub fn helped(&self) -> u32 {
        self.marks.iter().filter(|m| **m == Mark::Helped).count() as u32
    }

    /// Read from the page again: every mark cleared.
    pub fn again(&mut self) {
        *self = Page {
            words: std::mem::take(&mut self.words),
            marks: Vec::new(),
            next: 0,
            ahead: 0,
            started: None,
            ended: None,
        };
        self.marks = vec![Mark::Unread; self.words.len()];
        self.pass_unspoken();
        self.ahead = self.next;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn marks(p: &Page) -> String {
        p.marks
            .iter()
            .map(|m| match m {
                Mark::Unread => '.',
                Mark::Read => 'r',
                Mark::Helped => 'h',
                Mark::Missed => 'm',
            })
            .collect()
    }

    #[test]
    fn reading_a_page_in_two_breaths() {
        let mut p = Page::new("Sam has a cat. The cat is fat.");
        assert!(p.hear("Sam has a cat.", 1_000));
        assert_eq!((marks(&p).as_str(), p.next), ("rrrr....", 4));
        assert!(p.hear("The cat is fat.", 3_000));
        assert!(p.done());
        assert_eq!(p.read(), 8);
        assert_eq!((p.started, p.ended), (Some(1_000), Some(3_000)));
    }

    #[test]
    fn a_skipped_word_is_marked_and_passed() {
        let mut p = Page::new("The big red hen sat on the nest.");
        p.hear("The big hen sat", 0);
        assert_eq!(marks(&p), "rrmrr...");
        assert_eq!(p.next, 5);
    }

    #[test]
    fn a_misread_last_word_isnt_passed() {
        let mut p = Page::new("The pig can dig.");
        p.hear("The pig can dog", 0);
        assert_eq!(marks(&p), "rrr.");
        assert_eq!(p.next, 3);
        p.hear("dig", 0);
        assert!(p.done());
    }

    #[test]
    fn going_back_over_words_keeps_the_place() {
        let mut p = Page::new("Ten red hens sat in a pen.");
        p.hear("Ten red hens", 0);
        p.hear("hens sat in a pen", 0);
        assert!(p.done());
        assert_eq!(marks(&p), "rrrrrrr");
        let mut q = Page::new("Ten red hens sat in a pen.");
        q.hear("um ten ten red hens sat", 0);
        assert_eq!(q.next, 4);
        assert_eq!(marks(&q), "rrrr...");
    }

    #[test]
    fn noise_and_nothing_move_nothing() {
        let mut p = Page::new("The sun is hot.");
        assert!(!p.hear("", 0));
        assert!(!p.hear("[BLANK_AUDIO]", 0));
        assert!(!p.hear("thank you for watching", 0));
        assert_eq!(p.next, 0);
    }

    #[test]
    fn guesses_show_the_way_without_marking() {
        let mut p = Page::new("Kate has a kite. It is white.");
        p.guess("Kate has a");
        assert_eq!((p.next, p.ahead), (0, 3));
        assert_eq!(marks(&p), ".......");
        p.hear("Kate has a kite", 0);
        assert_eq!((p.next, p.ahead), (4, 4));
    }

    #[test]
    fn dashes_compounds_and_help() {
        let mut p = Page::new("Flopsy, Mopsy, Cotton-tail — and Peter.");
        assert!(p.hear("Flopsy Mopsy cotton tail and", 0));
        assert_eq!(p.next, 5);
        assert_eq!(marks(&p), "rrrrr.");
        p.help(5, 10);
        assert!(p.done());
        assert_eq!((p.read(), p.helped()), (4, 1));
        p.again();
        assert_eq!((p.next, marks(&p).as_str()), (0, "......"));
    }
}
