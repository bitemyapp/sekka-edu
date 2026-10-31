//! The lessons, from waking the first bead to adding in your head, and the
//! questions each asks.
//!
//! Kindergarten meets the beads (ones, then the big bead that is a whole
//! hand), learns to read and set 0 to 9, to hold a picture of the beads in
//! mind (peeking), and to add and take away, calling little friends when
//! the earth beads run out. First grade adds a second rod: tens, and big
//! friends. The last chapter puts the abacus away: the beads fade while you
//! add, then numbers flash by and you add them in your head. That's anzan
//! (暗算), the mental abacus.

use sekka_kid::Grade;

use crate::method::{Way, add_way, take_way};

/// What a lesson asks.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Kind {
    /// Set a number with the beads.
    Set,
    /// Say what the beads show.
    Read,
    /// The beads show a number for a moment, then hide: say it.
    PeekRead,
    /// The beads show a number for a moment, then hide: set it again.
    PeekSet,
    /// Add with the beads.
    Add,
    /// Take away with the beads.
    Take,
    /// Add or take away while the beads fade away: say the answer.
    Ghost,
    /// Numbers flash by: say their total.
    Flash,
}

/// A chapter of the path.
pub struct Chapter {
    pub title: &'static str,
    pub grade: Grade,
}

pub const CHAPTERS: [Chapter; 5] = [
    Chapter { title: "Wake the beads", grade: Grade::Kindergarten },
    Chapter { title: "Peekaboo", grade: Grade::Kindergarten },
    Chapter { title: "More and less", grade: Grade::Kindergarten },
    Chapter { title: "Tens", grade: Grade::First },
    Chapter { title: "In your head", grade: Grade::First },
];

/// A lesson: one stop on the path.
pub struct Lesson {
    /// Stable: progress and the log name it.
    pub id: &'static str,
    pub title: &'static str,
    pub chapter: usize,
    pub kind: Kind,
    pub rods: usize,
    /// Whether the rods' numbers show under them as the beads move.
    pub numbers: bool,
    pub skills: &'static [&'static str],
    /// What Pon says before the first question, the first time.
    pub intro: &'static [&'static str],
}

pub const LESSONS: [Lesson; 16] = [
    Lesson {
        id: "wake-the-beads",
        title: "Wake the beads",
        chapter: 0,
        kind: Kind::Set,
        rods: 1,
        numbers: true,
        skills: &["numbers-to-4"],
        intro: &[
            "These are the beads. They're asleep!",
            "Push a bead up to the bar and it wakes up.",
            "Each little bead is one.",
        ],
    },
    Lesson {
        id: "the-big-bead",
        title: "The big bead",
        chapter: 0,
        kind: Kind::Set,
        rods: 1,
        numbers: true,
        skills: &["numbers-to-9"],
        intro: &["The big bead on top is five.", "Five is a whole hand!", "Six is the big bead and one more."],
    },
    Lesson {
        id: "make-numbers",
        title: "Make a number",
        chapter: 0,
        kind: Kind::Set,
        rods: 1,
        numbers: false,
        skills: &["numbers-to-9"],
        intro: &["Now change the beads to each new number."],
    },
    Lesson {
        id: "read-numbers",
        title: "What number?",
        chapter: 0,
        kind: Kind::Read,
        rods: 1,
        numbers: false,
        skills: &["numbers-to-9", "reading-beads"],
        intro: &["Count the beads that are awake."],
    },
    Lesson {
        id: "peek",
        title: "Peek!",
        chapter: 1,
        kind: Kind::PeekRead,
        rods: 1,
        numbers: false,
        skills: &["numbers-to-9", "picturing-beads"],
        intro: &["Look closely. The curtain will close!", "Keep the beads in your head."],
    },
    Lesson {
        id: "copy-cat",
        title: "Copy cat",
        chapter: 1,
        kind: Kind::PeekSet,
        rods: 1,
        numbers: false,
        skills: &["numbers-to-9", "picturing-beads"],
        intro: &["Peek at the beads, then make them again."],
    },
    Lesson {
        id: "add",
        title: "Add some more",
        chapter: 2,
        kind: Kind::Add,
        rods: 1,
        numbers: true,
        skills: &["adding-to-9"],
        intro: &["The beads show the first number.", "Push up more beads to add."],
    },
    Lesson {
        id: "take",
        title: "Take some away",
        chapter: 2,
        kind: Kind::Take,
        rods: 1,
        numbers: true,
        skills: &["taking-from-9"],
        intro: &["Send beads home to take them away."],
    },
    Lesson {
        id: "little-friends-add",
        title: "Little friends",
        chapter: 2,
        kind: Kind::Add,
        rods: 1,
        numbers: true,
        skills: &["little-friends", "adding-to-9"],
        intro: &[
            "Uh oh, there aren't enough little beads!",
            "Little friends make five: one and four, two and three.",
            "Bring the big bead down, and send the friend home.",
        ],
    },
    Lesson {
        id: "little-friends-take",
        title: "Friends go home",
        chapter: 2,
        kind: Kind::Take,
        rods: 1,
        numbers: true,
        skills: &["little-friends", "taking-from-9"],
        intro: &["Not enough little beads to send home?", "Push up the friend, and send the big bead home."],
    },
    Lesson {
        id: "tens",
        title: "Tens and ones",
        chapter: 3,
        kind: Kind::Set,
        rods: 2,
        numbers: true,
        skills: &["place-value", "numbers-to-99"],
        intro: &["A new rod! Each bead on it is ten.", "Tens on the left, ones on the right."],
    },
    Lesson {
        id: "read-tens",
        title: "Read the tens",
        chapter: 3,
        kind: Kind::Read,
        rods: 2,
        numbers: false,
        skills: &["place-value", "numbers-to-99", "reading-beads"],
        intro: &["Count the tens, then the ones."],
    },
    Lesson {
        id: "big-friends-add",
        title: "Big friends",
        chapter: 3,
        kind: Kind::Add,
        rods: 2,
        numbers: true,
        skills: &["big-friends", "adding-to-18"],
        intro: &[
            "When a rod is full, call a big friend.",
            "Big friends make ten: one and nine, two and eight, three and seven…",
            "Send the friend home, and push up a ten.",
        ],
    },
    Lesson {
        id: "big-friends-take",
        title: "Borrow a ten",
        chapter: 3,
        kind: Kind::Take,
        rods: 2,
        numbers: true,
        skills: &["big-friends", "taking-from-18"],
        intro: &["Not enough ones? Send a ten home,", "and push up its big friend."],
    },
    Lesson {
        id: "ghost-beads",
        title: "Ghost beads",
        chapter: 4,
        kind: Kind::Ghost,
        rods: 1,
        numbers: false,
        skills: &["mental-abacus", "adding-to-9"],
        intro: &["The beads are fading away!", "Move them in your head."],
    },
    Lesson {
        id: "flash",
        title: "Flash!",
        chapter: 4,
        kind: Kind::Flash,
        rods: 1,
        numbers: false,
        skills: &["mental-abacus", "adding-to-9"],
        intro: &["Numbers will flash, one at a time.", "Add them on the beads in your head!"],
    },
];

/// The lesson with id `id`.
pub fn index_of(id: &str) -> Option<usize> {
    LESSONS.iter().position(|l| l.id == id)
}

/// Add or take away.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Op {
    Add,
    Take,
}

impl Op {
    pub fn sign(self) -> &'static str {
        match self {
            Op::Add => "+",
            Op::Take => "−",
        }
    }

    pub fn apply(self, a: u32, b: u32) -> u32 {
        match self {
            Op::Add => a + b,
            Op::Take => a.saturating_sub(b),
        }
    }
}

/// One question.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Question {
    /// Set `target`, starting from `from` (zero, or the last answer).
    Set {
        from: u32,
        target: u32,
    },
    Read {
        value: u32,
        choices: [u32; 3],
    },
    PeekRead {
        value: u32,
        choices: [u32; 3],
    },
    PeekSet {
        value: u32,
    },
    /// `start op n`, with the beads.
    Sum {
        start: u32,
        op: Op,
        n: u32,
    },
    /// `start op n`, in your head as the beads fade to `ghost` percent.
    Ghost {
        start: u32,
        op: Op,
        n: u32,
        ghost: u8,
        choices: [u32; 3],
    },
    Flash {
        numbers: Vec<u32>,
        choices: [u32; 3],
    },
}

impl Question {
    /// The answer.
    pub fn answer(&self) -> u32 {
        match self {
            Question::Set { target, .. } => *target,
            Question::Read { value, .. } | Question::PeekRead { value, .. } | Question::PeekSet { value } => *value,
            Question::Sum { start, op, n } | Question::Ghost { start, op, n, .. } => op.apply(*start, *n),
            Question::Flash { numbers, .. } => numbers.iter().sum(),
        }
    }

    /// What the beads show when it's asked.
    pub fn start(&self) -> u32 {
        match self {
            Question::Set { from, .. } => *from,
            Question::Read { value, .. } | Question::PeekRead { value, .. } => *value,
            Question::PeekSet { .. } | Question::Flash { .. } => 0,
            Question::Sum { start, .. } | Question::Ghost { start, .. } => *start,
        }
    }

    /// Answer choices, for questions answered by choosing.
    pub fn choices(&self) -> Option<[u32; 3]> {
        match self {
            Question::Read { choices, .. }
            | Question::PeekRead { choices, .. }
            | Question::Ghost { choices, .. }
            | Question::Flash { choices, .. } => Some(*choices),
            _ => None,
        }
    }
}

/// A small, seedable random number generator (xorshift64*), so a round can
/// be played again exactly in a test.
#[derive(Clone, Debug)]
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Rng {
        Rng(seed.max(1))
    }

    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_f491_4f6c_dd1d)
    }

    /// A number in `lo..=hi`.
    pub fn range(&mut self, lo: u32, hi: u32) -> u32 {
        lo + (self.next_u64() % u64::from(hi - lo + 1)) as u32
    }

    pub fn pick<T: Clone>(&mut self, from: &[T]) -> T {
        from[(self.next_u64() % from.len() as u64) as usize].clone()
    }

    pub fn shuffle<T>(&mut self, v: &mut [T]) {
        for i in (1..v.len()).rev() {
            let j = (self.next_u64() % (i as u64 + 1)) as usize;
            v.swap(i, j);
        }
    }
}

/// Questions in a round.
pub const ROUND: usize = 8;

/// Three choices: the answer and two near misses a child might make (one
/// bead off, the big bead forgotten, the tens and ones swapped), shuffled.
pub fn choices(rng: &mut Rng, answer: u32, max: u32) -> [u32; 3] {
    let mut near: Vec<u32> = Vec::new();
    let mut add = |v: i64| {
        if v >= 0 && v as u32 <= max && v as u32 != answer && !near.contains(&(v as u32)) {
            near.push(v as u32);
        }
    };
    let a = i64::from(answer);
    for d in [1, -1, 5, -5] {
        add(a + d);
    }
    if answer >= 10 {
        add(i64::from((answer % 10) * 10 + answer / 10));
        for d in [10, -10] {
            add(a + d);
        }
    }
    for d in [2, -2, 3, -3] {
        add(a + d);
    }
    rng.shuffle(&mut near);
    let mut c = [answer, near[0], near[1]];
    rng.shuffle(&mut c);
    c
}

/// Pairs `(a, d)` with `lo <= a <= hi`, `1 <= d <= 9`, done `way`.
fn sums(op: Op, lo: u32, hi: u32, dmax: u8, ways: &[Way]) -> Vec<(u32, u32)> {
    let mut v = Vec::new();
    for a in lo..=hi {
        for d in 1..=dmax {
            let ok = match op {
                Op::Add => ways.contains(&add_way(a, d)),
                Op::Take => a >= u32::from(d) && ways.contains(&take_way(a, d)),
            };
            if ok {
                v.push((a, u32::from(d)));
            }
        }
    }
    v
}

/// `n` picks from `pool`, no two alike in a row, each used before any is
/// used again.
fn deal<T: Clone + PartialEq>(rng: &mut Rng, pool: &[T], n: usize) -> Vec<T> {
    let mut out: Vec<T> = Vec::new();
    while out.len() < n {
        let mut deck = pool.to_vec();
        rng.shuffle(&mut deck);
        if deck.len() > 1 && out.last() == deck.first() {
            deck.swap(0, 1);
        }
        out.extend(deck);
    }
    out.truncate(n);
    out
}

/// A round of lesson `lesson`.
pub fn questions(lesson: usize, rng: &mut Rng) -> Vec<Question> {
    let l = &LESSONS[lesson];
    let n = ROUND;
    match l.id {
        "wake-the-beads" => {
            deal(rng, &[1, 2, 3, 4], n).into_iter().map(|t| Question::Set { from: 0, target: t }).collect()
        }
        "the-big-bead" => {
            // Five first, the big bead's own number, then the rest.
            let mut t = vec![5];
            t.extend(deal(rng, &[5, 6, 7, 8, 9, 6, 7], n - 1));
            t.into_iter().map(|t| Question::Set { from: 0, target: t }).collect()
        }
        "make-numbers" => {
            let targets = deal(rng, &(0..=9).collect::<Vec<_>>(), n);
            let mut from = 0;
            targets
                .into_iter()
                .map(|t| {
                    // Never "make" what's already there.
                    let t = if t == from { (t + 3) % 10 } else { t };
                    let q = Question::Set { from, target: t };
                    from = t;
                    q
                })
                .collect()
        }
        "read-numbers" => deal(rng, &(0..=9).collect::<Vec<_>>(), n)
            .into_iter()
            .map(|v| Question::Read { value: v, choices: choices(rng, v, 9) })
            .collect(),
        "peek" => deal(rng, &(1..=9).collect::<Vec<_>>(), n)
            .into_iter()
            .map(|v| Question::PeekRead { value: v, choices: choices(rng, v, 9) })
            .collect(),
        "copy-cat" => {
            deal(rng, &(1..=9).collect::<Vec<_>>(), n).into_iter().map(|v| Question::PeekSet { value: v }).collect()
        }
        // Direct sums never pass nine: those need a big friend.
        "add" => deal(rng, &sums(Op::Add, 1, 8, 8, &[Way::Direct]), n)
            .into_iter()
            .map(|(a, d)| Question::Sum { start: a, op: Op::Add, n: d })
            .collect(),
        "take" => deal(rng, &sums(Op::Take, 1, 9, 9, &[Way::Direct]), n)
            .into_iter()
            .map(|(a, d)| Question::Sum { start: a, op: Op::Take, n: d })
            .collect(),
        "little-friends-add" => {
            let pool: Vec<(u32, u32)> =
                sums(Op::Add, 1, 4, 4, &[Way::LittleFriend]).into_iter().filter(|(a, d)| a + d <= 9).collect();
            // Four and one first: the plainest case.
            let mut q = vec![(4, 1)];
            q.extend(deal(rng, &pool, n - 1));
            q.into_iter().map(|(a, d)| Question::Sum { start: a, op: Op::Add, n: d }).collect()
        }
        "little-friends-take" => {
            let pool = sums(Op::Take, 5, 8, 4, &[Way::LittleFriend]);
            let mut q = vec![(5, 1)];
            q.extend(deal(rng, &pool, n - 1));
            q.into_iter().map(|(a, d)| Question::Sum { start: a, op: Op::Take, n: d }).collect()
        }
        "tens" => {
            let mut t: Vec<u32> = (0..n).map(|_| rng.range(10, 99)).collect();
            t[0] = 10 * rng.range(1, 4);
            t.into_iter().map(|t| Question::Set { from: 0, target: t }).collect()
        }
        "read-tens" => (0..n)
            .map(|_| {
                let v = rng.range(10, 99);
                Question::Read { value: v, choices: choices(rng, v, 99) }
            })
            .collect(),
        "big-friends-add" => {
            let pool = sums(Op::Add, 1, 9, 9, &[Way::BigFriend]);
            let mut q = vec![(9, 1)];
            q.extend(deal(rng, &pool, n - 1));
            q.into_iter().map(|(a, d)| Question::Sum { start: a, op: Op::Add, n: d }).collect()
        }
        "big-friends-take" => {
            let pool = sums(Op::Take, 10, 18, 9, &[Way::BigFriend]);
            let mut q = vec![(10, 1)];
            q.extend(deal(rng, &pool, n - 1));
            q.into_iter().map(|(a, d)| Question::Sum { start: a, op: Op::Take, n: d }).collect()
        }
        "ghost-beads" => {
            let mut pool: Vec<(Op, u32, u32)> = sums(Op::Add, 1, 8, 4, &[Way::Direct, Way::LittleFriend])
                .into_iter()
                .filter(|(a, d)| a + d <= 9)
                .map(|(a, d)| (Op::Add, a, d))
                .collect();
            pool.extend(
                sums(Op::Take, 2, 9, 4, &[Way::Direct, Way::LittleFriend]).into_iter().map(|(a, d)| (Op::Take, a, d)),
            );
            deal(rng, &pool, n)
                .into_iter()
                .enumerate()
                .map(|(i, (op, a, d))| {
                    // The beads fade as the round goes: half, a quarter, gone.
                    let ghost = [60, 60, 60, 30, 30, 30, 0, 0][i.min(7)];
                    let answer = op.apply(a, d);
                    Question::Ghost { start: a, op, n: d, ghost, choices: choices(rng, answer, 9) }
                })
                .collect()
        }
        "flash" => (0..n)
            .map(|i| {
                let count = if i < 4 { 2 } else { 3 };
                let mut numbers = Vec::new();
                while numbers.len() < count {
                    let left = 9 - numbers.iter().sum::<u32>();
                    let room = left - (count - numbers.len() - 1) as u32;
                    numbers.push(rng.range(1, room.clamp(1, 4)));
                }
                let total = numbers.iter().sum();
                Question::Flash { choices: choices(rng, total, 9), numbers }
            })
            .collect(),
        _ => unreachable!("every lesson has questions"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::abacus::Abacus;
    use crate::method::{add_steps, take_steps, way_of};

    #[test]
    fn every_lesson_asks_a_full_round_of_fair_questions() {
        for seed in 1..60 {
            let mut rng = Rng::new(seed);
            for (i, l) in LESSONS.iter().enumerate() {
                let qs = questions(i, &mut rng);
                assert_eq!(qs.len(), ROUND, "{}", l.id);
                let max = Abacus::new(l.rods).max();
                for q in &qs {
                    assert!(q.answer() <= max, "{}: {q:?} doesn't fit", l.id);
                    if let Some(c) = q.choices() {
                        assert!(c.contains(&q.answer()), "{}: {q:?}", l.id);
                        assert!(c[0] != c[1] && c[1] != c[2] && c[0] != c[2], "{}: {q:?}", l.id);
                        assert!(c.iter().all(|v| *v <= max));
                    }
                    if let Question::Set { from, target } = q {
                        assert_ne!(from, target, "{}: nothing to do", l.id);
                    }
                }
            }
        }
    }

    #[test]
    fn friends_lessons_need_their_friends_and_no_others() {
        for seed in 1..40 {
            let mut rng = Rng::new(seed);
            for (id, way) in [
                ("add", Way::Direct),
                ("take", Way::Direct),
                ("little-friends-add", Way::LittleFriend),
                ("little-friends-take", Way::LittleFriend),
                ("big-friends-add", Way::BigFriend),
                ("big-friends-take", Way::BigFriend),
            ] {
                let l = index_of(id).unwrap();
                for q in questions(l, &mut rng) {
                    let Question::Sum { start, op, n } = q else { panic!("{id}: {q:?}") };
                    let a = Abacus::showing(LESSONS[l].rods, start);
                    let steps = match op {
                        Op::Add => add_steps(&a, n as u8),
                        Op::Take => take_steps(&a, n as u8),
                    };
                    assert_eq!(way_of(&steps), way, "{id}: {start} {} {n}", op.sign());
                }
            }
        }
    }

    #[test]
    fn flashed_numbers_stay_under_ten() {
        for seed in 1..200 {
            let mut rng = Rng::new(seed);
            for q in questions(index_of("flash").unwrap(), &mut rng) {
                let Question::Flash { numbers, .. } = &q else { unreachable!() };
                assert!(numbers.iter().all(|n| (1..=4).contains(n)), "{numbers:?}");
                assert!(q.answer() <= 9, "{numbers:?}");
            }
        }
    }
}
