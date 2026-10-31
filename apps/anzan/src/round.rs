//! A round: eight questions of one lesson, asked one at a time, with the
//! beads moved by the kid (or by themselves, to show how), hints, and the
//! score kept.
//!
//! Time comes in from outside (`now`, Unix milliseconds), so tests can play
//! a round in an instant; [`Round::deadline`] says when it next wants
//! [`Round::tick`].

use crate::abacus::{Abacus, Bead};
use crate::lessons::{Kind, LESSONS, Op, Question};
use crate::method::{self, Move, Step};

/// How long the beads show before the curtain closes, at first.
pub const PEEK_MS: u32 = 2600;
/// The shortest peek, for kids who've got good at it.
pub const PEEK_MIN_MS: u32 = 1200;
/// How long each flashed number shows, and the blank between.
pub const FLASH_MS: i64 = 1300;
pub const FLASH_GAP_MS: i64 = 350;
/// The cheer after a right answer.
pub const RIGHT_MS: i64 = 1400;
/// Between the pushes the beads make by themselves, showing how.
pub const SHOW_STEP_MS: i64 = 700;

/// Where the round is.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Phase {
    /// The beads show the number until the curtain closes.
    Peeking { until: i64 },
    /// Numbers flash one at a time: which shows, and whether it's the
    /// blank after it.
    Flashing { shown: usize, gap: bool, until: i64 },
    /// The kid is working.
    Asking,
    /// Right: the beads cheer.
    Right { until: i64 },
    /// The beads show how, a push at a time.
    Showing { steps: Vec<Step>, done: usize, next: i64 },
    /// Every question asked.
    Done,
}

/// A hint showing.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Hint {
    /// Push these beads, this way.
    Push { rod: usize, beads: Vec<Bead>, to_beam: bool, words: String },
    /// The beads have gone astray: back to the start.
    Again,
    /// One wrong choice taken away.
    Strike(u32),
    /// The faded beads, shown again.
    Look,
}

/// What just happened, for sound and speech.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Feedback {
    Nothing,
    Moved,
    Right,
    Miss,
    /// Missed twice: the answer is shown.
    Shown,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Round {
    pub lesson: usize,
    pub questions: Vec<Question>,
    pub index: usize,
    pub phase: Phase,
    /// The beads the kid sees.
    pub abacus: Abacus,
    pub hint: Option<Hint>,
    /// On this question: wrong answers, hints, pushes.
    pub misses: u32,
    pub hinted: bool,
    pub pushes: u32,
    /// The last choice made, and whether it was right.
    pub chosen: Option<(u32, bool)>,
    /// Over the round.
    pub results: Vec<bool>,
    pub hints: u32,
    pub responses: Vec<u32>,
    pub started: i64,
    pub asked_at: i64,
    /// How long peeks last; shorter as they're got right.
    pub peek_ms: u32,
}

impl Round {
    pub fn new(lesson: usize, questions: Vec<Question>, now: i64, peek_ms: u32) -> Round {
        let rods = LESSONS[lesson].rods;
        let mut r = Round {
            lesson,
            questions,
            index: 0,
            phase: Phase::Asking,
            abacus: Abacus::new(rods),
            hint: None,
            misses: 0,
            hinted: false,
            pushes: 0,
            chosen: None,
            results: Vec::new(),
            hints: 0,
            responses: Vec::new(),
            started: now,
            asked_at: now,
            peek_ms: peek_ms.clamp(PEEK_MIN_MS, PEEK_MS),
        };
        r.ask(now);
        r
    }

    pub fn question(&self) -> &Question {
        &self.questions[self.index.min(self.questions.len() - 1)]
    }

    pub fn kind(&self) -> Kind {
        LESSONS[self.lesson].kind
    }

    pub fn is_done(&self) -> bool {
        self.phase == Phase::Done
    }

    /// Whether the kid can move the beads now.
    pub fn beads_open(&self) -> bool {
        self.phase == Phase::Asking
            && matches!(self.question(), Question::Set { .. } | Question::PeekSet { .. } | Question::Sum { .. })
    }

    /// Whether the kid answers by choosing now.
    pub fn choosing(&self) -> bool {
        self.phase == Phase::Asking && self.question().choices().is_some()
    }

    /// Whether the curtain is drawn over the beads.
    pub fn curtained(&self) -> bool {
        matches!(
            (self.question(), &self.phase),
            (Question::PeekRead { .. }, Phase::Asking)
                | (Question::Flash { .. }, Phase::Flashing { .. } | Phase::Asking)
        )
    }

    fn ask(&mut self, now: i64) {
        self.misses = 0;
        self.hinted = false;
        self.pushes = 0;
        self.chosen = None;
        self.hint = None;
        let rods = self.abacus.len();
        let q = self.question().clone();
        self.abacus = Abacus::showing(rods, q.start());
        self.phase = match q {
            Question::PeekRead { value, .. } | Question::PeekSet { value } => {
                self.abacus = Abacus::showing(rods, value);
                Phase::Peeking { until: now + i64::from(self.peek_ms) }
            }
            Question::Flash { .. } => Phase::Flashing { shown: 0, gap: false, until: now + FLASH_MS },
            _ => Phase::Asking,
        };
        self.asked_at = now;
    }

    /// Ask the first question afresh at `now` (after Pon's introduction).
    pub fn begin(&mut self, now: i64) {
        self.started = now;
        self.ask(now);
    }

    /// Put the beads back where the question started them.
    pub fn restart(&mut self) {
        if self.beads_open() && matches!(self.question(), Question::Sum { .. }) {
            self.abacus = Abacus::showing(self.abacus.len(), self.question().start());
            self.hint = None;
            if self.hinted {
                self.give_hint();
            }
        }
    }

    /// When the round next wants a tick.
    pub fn deadline(&self) -> Option<i64> {
        match self.phase {
            Phase::Peeking { until } | Phase::Flashing { until, .. } | Phase::Right { until } => Some(until),
            Phase::Showing { next, .. } => Some(next),
            Phase::Asking | Phase::Done => None,
        }
    }

    /// Move on if it's time; whether anything changed.
    pub fn tick(&mut self, now: i64) -> bool {
        match self.phase.clone() {
            Phase::Peeking { until } if now >= until => {
                if matches!(self.question(), Question::PeekSet { .. }) {
                    self.abacus = Abacus::new(self.abacus.len());
                }
                self.phase = Phase::Asking;
                self.asked_at = now;
                true
            }
            Phase::Flashing { shown, gap, until } if now >= until => {
                let Question::Flash { numbers, .. } = self.question() else { return false };
                let count = numbers.len();
                self.phase = if !gap {
                    Phase::Flashing { shown, gap: true, until: now + FLASH_GAP_MS }
                } else if shown + 1 < count {
                    Phase::Flashing { shown: shown + 1, gap: false, until: now + FLASH_MS }
                } else {
                    self.asked_at = now;
                    Phase::Asking
                };
                true
            }
            Phase::Right { until } if now >= until => {
                self.next(now);
                true
            }
            Phase::Showing { steps, done, next } if now >= next => {
                if done < steps.len() {
                    method::apply(&mut self.abacus, steps[done]);
                    self.phase = Phase::Showing { steps, done: done + 1, next: now + SHOW_STEP_MS };
                } else {
                    self.results.push(false);
                    self.next(now);
                }
                true
            }
            _ => false,
        }
    }

    fn next(&mut self, now: i64) {
        self.index += 1;
        if self.index >= self.questions.len() {
            self.index = self.questions.len() - 1;
            self.hint = None;
            self.phase = Phase::Done;
        } else {
            self.ask(now);
        }
    }

    /// The pushes from the question's start to its answer.
    fn plan(&self) -> Vec<Step> {
        let rods = self.abacus.len();
        let q = self.question();
        match q {
            Question::Sum { start, op, n } | Question::Ghost { start, op, n, .. } => {
                let a = Abacus::showing(rods, *start);
                match op {
                    Op::Add => method::add_steps(&a, *n as u8),
                    Op::Take => method::take_steps(&a, *n as u8),
                }
            }
            _ => set_steps(&Abacus::showing(rods, q.start()), q.answer()),
        }
    }

    /// The pushes the beads make by themselves to show the answer, from
    /// where they are.
    fn show_steps(&self) -> Vec<Step> {
        match self.question() {
            Question::Sum { start, .. } if self.abacus.value() == *start => self.plan(),
            Question::Ghost { .. } => self.plan(),
            q => set_steps(&self.abacus, q.answer()),
        }
    }

    fn right(&mut self, now: i64) {
        let fewest = match self.question() {
            Question::Sum { .. } => self.plan().len() as u32,
            q => Abacus::showing(self.abacus.len(), q.start()).pushes_to(q.answer()),
        };
        // Some slack: a bead at a time (by keys) is fine, wandering isn't.
        let clean = self.misses == 0 && !self.hinted && self.pushes <= fewest * 2 + 3;
        self.results.push(clean);
        self.responses.push((now - self.asked_at).clamp(0, u32::MAX as i64) as u32);
        if matches!(self.question(), Question::PeekRead { .. } | Question::PeekSet { .. }) && clean {
            self.peek_ms = (self.peek_ms * 85 / 100).max(PEEK_MIN_MS);
        }
        self.hint = None;
        self.phase = Phase::Right { until: now + RIGHT_MS };
    }

    fn miss(&mut self, now: i64) -> Feedback {
        self.misses += 1;
        if self.misses >= 2 {
            // Show how: the beads move themselves to the answer.
            self.hint = None;
            if let Question::Sum { start, .. } = self.question() {
                // From the start, so the pushes are the lesson's.
                self.abacus = Abacus::showing(self.abacus.len(), *start);
            }
            if let Question::Ghost { start, .. } = self.question() {
                self.abacus = Abacus::showing(self.abacus.len(), *start);
            }
            if matches!(self.question(), Question::PeekRead { .. } | Question::Read { .. } | Question::Flash { .. }) {
                self.abacus = Abacus::new(self.abacus.len());
            }
            self.phase = Phase::Showing { steps: self.show_steps(), done: 0, next: now + SHOW_STEP_MS };
            Feedback::Shown
        } else {
            self.give_hint();
            Feedback::Miss
        }
    }

    /// The kid flicked a bead.
    pub fn flick(&mut self, rod: usize, bead: Bead, now: i64) -> Feedback {
        if !self.beads_open() || !self.abacus.flick(rod, bead) {
            return Feedback::Nothing;
        }
        self.moved(now)
    }

    /// The kid dragged a bead to the beam or away from it.
    pub fn push(&mut self, rod: usize, bead: Bead, to_beam: bool, now: i64) -> Feedback {
        if !self.beads_open() || !self.abacus.push(rod, bead, to_beam) {
            return Feedback::Nothing;
        }
        self.moved(now)
    }

    fn moved(&mut self, now: i64) -> Feedback {
        self.pushes += 1;
        // A hint's beads, once pushed, have done their job.
        if matches!(self.hint, Some(Hint::Push { .. }) | Some(Hint::Again)) {
            self.hint = None;
            if self.hinted {
                self.give_hint();
            }
        }
        if self.abacus.value() == self.question().answer() {
            self.right(now);
            Feedback::Right
        } else {
            Feedback::Moved
        }
    }

    /// The kid says they're done moving beads (the ✓).
    pub fn check(&mut self, now: i64) -> Feedback {
        if !self.beads_open() {
            return Feedback::Nothing;
        }
        if self.abacus.value() == self.question().answer() {
            self.right(now);
            Feedback::Right
        } else {
            self.miss(now)
        }
    }

    /// The kid chose an answer.
    pub fn choose(&mut self, value: u32, now: i64) -> Feedback {
        if !self.choosing() || self.hint == Some(Hint::Strike(value)) {
            return Feedback::Nothing;
        }
        let right = value == self.question().answer();
        self.chosen = Some((value, right));
        if right {
            // Show the beads that made it.
            if let Question::Ghost { .. } | Question::Flash { .. } = self.question() {
                self.abacus = Abacus::showing(self.abacus.len(), value);
            }
            self.right(now);
            Feedback::Right
        } else {
            self.miss(now)
        }
    }

    /// The kid asked for a hint.
    pub fn ask_hint(&mut self) {
        if self.phase != Phase::Asking {
            return;
        }
        self.hints += 1;
        self.hinted = true;
        self.give_hint();
    }

    /// Show the hint for where the beads are now.
    fn give_hint(&mut self) {
        let answer = self.question().answer();
        self.hint = match self.question().clone() {
            Question::Set { .. } | Question::PeekSet { .. } => {
                set_steps(&self.abacus, answer).first().map(|s| push_hint(&self.abacus, *s))
            }
            Question::Sum { .. } => {
                let plan = self.plan();
                // How far along the plan the beads are.
                let mut a = Abacus::showing(self.abacus.len(), self.question().start());
                let mut at = (a == self.abacus).then_some(0);
                for (i, s) in plan.iter().enumerate() {
                    method::apply(&mut a, *s);
                    if a == self.abacus {
                        at = Some(i + 1);
                    }
                }
                match at {
                    Some(i) if i < plan.len() => Some(push_hint(&self.abacus, plan[i])),
                    Some(_) => None,
                    None => {
                        self.abacus = Abacus::showing(self.abacus.len(), self.question().start());
                        Some(Hint::Again)
                    }
                }
            }
            Question::Ghost { .. } => Some(Hint::Look),
            q => {
                let c = q.choices().unwrap_or_default();
                let chosen = self.chosen.map(|(v, _)| v);
                c.iter().copied().find(|v| *v != answer && Some(*v) != chosen).or(chosen).map(Hint::Strike)
            }
        };
    }

    /// 0 to 3 stars: from how many came right the first time.
    pub fn stars(&self) -> u8 {
        let right = self.first_try();
        let n = self.questions.len() as u32;
        if right * 8 >= n * 7 {
            3
        } else if right * 8 >= n * 5 {
            2
        } else {
            1
        }
    }

    pub fn first_try(&self) -> u32 {
        self.results.iter().filter(|r| **r).count() as u32
    }

    /// The middle response time.
    pub fn response_ms(&self) -> Option<u32> {
        let mut v = self.responses.clone();
        v.sort_unstable();
        v.get(v.len() / 2).copied()
    }

    /// For the log.
    pub fn record(&self, app: &str, now: i64) -> sekka_kid::Round {
        let l = &LESSONS[self.lesson];
        sekka_kid::Round {
            app: app.to_owned(),
            activity: l.id.to_owned(),
            title: l.title.to_owned(),
            skills: l.skills.iter().map(|s| (*s).to_owned()).collect(),
            started: self.started,
            ended: now,
            items: if self.is_done() { self.questions.len() as u32 } else { self.results.len() as u32 },
            first_try: self.first_try(),
            hints: self.hints,
            stars: if self.is_done() { self.stars() } else { 0 },
            finished: self.is_done(),
            response_ms: self.response_ms(),
        }
    }
}

/// The pushes that set `to` from `from`, the tens first.
pub fn set_steps(from: &Abacus, to: u32) -> Vec<Step> {
    let target = Abacus::showing(from.len(), to);
    let mut out = Vec::new();
    for rod in (0..from.len()).rev() {
        let (a, b) = (from.rod(rod), target.rod(rod));
        let way = method::Way::Direct;
        if a.heaven != b.heaven {
            out.push(Step { rod, mv: if b.heaven { Move::HeavenDown } else { Move::HeavenUp }, way });
        }
        if a.earth < b.earth {
            out.push(Step { rod, mv: Move::EarthUp(b.earth - a.earth), way });
        } else if a.earth > b.earth {
            out.push(Step { rod, mv: Move::EarthDown(a.earth - b.earth), way });
        }
    }
    out
}

/// The hint for `step` on the beads as they are.
fn push_hint(a: &Abacus, step: Step) -> Hint {
    let r = a.rod(step.rod);
    let (beads, to_beam) = match step.mv {
        Move::EarthUp(n) => ((r.earth..r.earth + n).map(Bead::Earth).collect(), true),
        Move::EarthDown(n) => ((r.earth.saturating_sub(n)..r.earth).map(Bead::Earth).collect(), false),
        Move::HeavenDown => (vec![Bead::Heaven], true),
        Move::HeavenUp => (vec![Bead::Heaven], false),
    };
    Hint::Push { rod: step.rod, beads, to_beam, words: step.words(a.len()) }
}

/// A number in words, for Pon to say.
pub fn words(n: u32) -> String {
    const ONES: [&str; 20] = [
        "zero",
        "one",
        "two",
        "three",
        "four",
        "five",
        "six",
        "seven",
        "eight",
        "nine",
        "ten",
        "eleven",
        "twelve",
        "thirteen",
        "fourteen",
        "fifteen",
        "sixteen",
        "seventeen",
        "eighteen",
        "nineteen",
    ];
    const TENS: [&str; 10] = ["", "", "twenty", "thirty", "forty", "fifty", "sixty", "seventy", "eighty", "ninety"];
    match n {
        0..=19 => ONES[n as usize].to_owned(),
        20..=99 if n.is_multiple_of(10) => TENS[(n / 10) as usize].to_owned(),
        20..=99 => format!("{}-{}", TENS[(n / 10) as usize], ONES[(n % 10) as usize]),
        100..=999 if n.is_multiple_of(100) => format!("{} hundred", ONES[(n / 100) as usize]),
        100..=999 => format!("{} hundred {}", ONES[(n / 100) as usize], words(n % 100)),
        _ => n.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lessons::{Rng, index_of, questions};

    fn round(id: &str) -> Round {
        let l = index_of(id).unwrap();
        Round::new(l, questions(l, &mut Rng::new(7)), 0, PEEK_MS)
    }

    /// Push the beads the way the plan says, as a kid who knows how.
    fn solve(r: &mut Round, now: i64) -> Feedback {
        let mut last = Feedback::Nothing;
        for s in r.plan() {
            let a = r.abacus.clone();
            let mut b = a.clone();
            method::apply(&mut b, s);
            // A push of several earth beads is one flick of the farthest.
            let bead = match s.mv {
                Move::EarthUp(n) => Bead::Earth(a.rod(s.rod).earth + n - 1),
                Move::EarthDown(n) => Bead::Earth(a.rod(s.rod).earth - n),
                _ => Bead::Heaven,
            };
            last = r.flick(s.rod, bead, now);
            assert_eq!(r.abacus.rod(s.rod), b.rod(s.rod), "{s:?}");
        }
        last
    }

    #[test]
    fn a_round_played_well_gets_three_stars() {
        let mut r = round("little-friends-add");
        let mut now = 0;
        while !r.is_done() {
            now += 1000;
            assert_eq!(solve(&mut r, now), Feedback::Right);
            assert!(matches!(r.phase, Phase::Right { .. }));
            now += RIGHT_MS;
            assert!(r.tick(now));
        }
        assert_eq!((r.first_try(), r.stars()), (8, 3));
        let rec = r.record("app", now);
        assert!(rec.finished);
        assert_eq!((rec.items, rec.first_try, rec.hints), (8, 8, 0));
        assert_eq!(rec.response_ms, Some(1000));
    }

    #[test]
    fn two_misses_show_the_answer_by_themselves() {
        let mut r = round("read-numbers");
        let answer = r.question().answer();
        let wrong: Vec<u32> = r.question().choices().unwrap().into_iter().filter(|c| *c != answer).collect();
        assert_eq!(r.choose(wrong[0], 1), Feedback::Miss);
        assert_eq!(r.hint, Some(Hint::Strike(wrong[1])), "the other wrong one goes");
        assert_eq!(r.choose(wrong[1], 2), Feedback::Nothing, "struck out");
        // A second miss can only come from the first wrong choice again.
        assert_eq!(r.choose(wrong[0], 3), Feedback::Shown);
        let mut now = 3;
        let mut shown = None;
        while matches!(r.phase, Phase::Showing { .. }) {
            shown = Some(r.abacus.value());
            now += SHOW_STEP_MS;
            r.tick(now);
        }
        assert_eq!(shown, Some(answer), "the beads end on the answer");
        assert_eq!(r.index, 1);
        assert_eq!(r.results, [false]);
    }

    #[test]
    fn hints_follow_the_friends_method() {
        let mut r = round("little-friends-add");
        assert_eq!(r.question(), &Question::Sum { start: 4, op: Op::Add, n: 1 });
        r.ask_hint();
        let Some(Hint::Push { rod: 0, beads, to_beam: true, words }) = r.hint.clone() else { panic!("{:?}", r.hint) };
        assert_eq!(beads, [Bead::Heaven]);
        assert_eq!(words, "Bring the big bead down");
        r.flick(0, Bead::Heaven, 10);
        let Some(Hint::Push { beads, to_beam: false, .. }) = r.hint.clone() else { panic!("{:?}", r.hint) };
        assert_eq!(beads, (0..4).map(Bead::Earth).collect::<Vec<_>>(), "then send the four home");
        assert_eq!(r.flick(0, Bead::Earth(0), 20), Feedback::Right);
        assert_eq!(r.results, [false], "a hint was used");
        // Astray: back to the start.
        let mut r = round("little-friends-add");
        r.flick(0, Bead::Earth(0), 1);
        r.flick(0, Bead::Earth(0), 2);
        r.ask_hint();
        assert_eq!(r.hint, Some(Hint::Again));
        assert_eq!(r.abacus.value(), 4);
    }

    #[test]
    fn peeks_hide_the_beads_and_get_shorter() {
        let mut r = round("copy-cat");
        let v = r.question().answer();
        assert!(matches!(r.phase, Phase::Peeking { .. }));
        assert_eq!(r.abacus.value(), v);
        assert!(!r.beads_open());
        assert!(r.tick(i64::from(PEEK_MS)));
        assert_eq!(r.abacus.value(), 0, "copy it from memory");
        let target = Abacus::showing(1, v);
        if target.rod(0).heaven {
            r.flick(0, Bead::Heaven, 3000);
        }
        if target.rod(0).earth > 0 {
            r.flick(0, Bead::Earth(target.rod(0).earth - 1), 3000);
        }
        assert!(matches!(r.phase, Phase::Right { .. }));
        assert!(r.peek_ms < PEEK_MS);
    }

    #[test]
    fn flashes_show_each_number_then_ask() {
        let mut r = round("flash");
        let Question::Flash { numbers, .. } = r.question().clone() else { panic!() };
        let mut now = 0;
        let mut seen = Vec::new();
        while r.phase != Phase::Asking {
            if let Phase::Flashing { shown, gap: false, .. } = r.phase {
                seen.push(numbers[shown]);
            }
            now = r.deadline().unwrap();
            r.tick(now);
        }
        assert_eq!(seen, numbers);
        assert_eq!(r.choose(numbers.iter().sum(), now + 900), Feedback::Right);
        assert_eq!(r.abacus.value(), numbers.iter().sum::<u32>(), "the beads show the total");
    }

    #[test]
    fn a_round_left_part_way_is_logged_unfinished() {
        let mut r = round("wake-the-beads");
        r.check(5);
        let rec = r.record("app", 9);
        assert!(!rec.finished);
        assert_eq!((rec.items, rec.stars), (0, 0));
    }

    #[test]
    fn numbers_in_words() {
        assert_eq!(words(7), "seven");
        assert_eq!(words(13), "thirteen");
        assert_eq!(words(40), "forty");
        assert_eq!(words(42), "forty-two");
        assert_eq!(words(253), "two hundred fifty-three");
        assert_eq!(words(300), "three hundred");
    }
}
