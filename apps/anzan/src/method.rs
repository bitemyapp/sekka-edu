//! How a soroban adds and takes away, one push at a time, the way it's
//! taught to children:
//!
//! - **Directly**, when there are beads to move: 2 + 1 pushes one more earth
//!   bead up.
//! - With a **little friend**, the number that makes five with it (1 and 4,
//!   2 and 3), when the earth beads run out: 4 + 1 brings the heaven bead
//!   down (five) and sends four earth beads home.
//! - With a **big friend**, the number that makes ten with it (1 and 9, 2
//!   and 8, …), when the rod runs out: 9 + 1 sends nine home and adds one
//!   to the tens.

use crate::abacus::{Abacus, Rod};

/// Which way a sum is done.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Way {
    Direct,
    LittleFriend,
    BigFriend,
    /// A big friend whose own step needs a little one (5 + 6: take four
    /// from five).
    BothFriends,
}

/// One push.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Move {
    /// `n` earth beads up to the beam.
    EarthUp(u8),
    /// `n` earth beads home, away from the beam.
    EarthDown(u8),
    /// The heaven bead down to the beam.
    HeavenDown,
    /// The heaven bead home.
    HeavenUp,
}

/// A push on a rod, and why.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Step {
    pub rod: usize,
    pub mv: Move,
    pub way: Way,
}

impl Step {
    /// What a grown-up would say for it.
    pub fn words(self, rods: usize) -> String {
        let place = if rods > 1 && self.rod == 1 {
            " on the tens"
        } else if rods > 1 {
            " on the ones"
        } else {
            ""
        };
        let beads = |n: u8| if n == 1 { "1 bead".to_owned() } else { format!("{n} beads") };
        match self.mv {
            Move::EarthUp(n) => format!("Push {} up{place}", beads(n)),
            Move::EarthDown(n) => format!("Send {} home{place}", beads(n)),
            Move::HeavenDown => format!("Bring the big bead down{place}"),
            Move::HeavenUp => format!("Send the big bead home{place}"),
        }
    }
}

/// Do `step` to `a`.
pub fn apply(a: &mut Abacus, step: Step) {
    let mut r = a.rod(step.rod);
    match step.mv {
        Move::EarthUp(n) => r.earth = (r.earth + n).min(4),
        Move::EarthDown(n) => r.earth = r.earth.saturating_sub(n),
        Move::HeavenDown => r.heaven = true,
        Move::HeavenUp => r.heaven = false,
    }
    a.set_rod(step.rod, r);
}

fn push(a: &mut Abacus, out: &mut Vec<Step>, rod: usize, mv: Move, way: Way) {
    let step = Step { rod, mv, way };
    apply(a, step);
    out.push(step);
}

/// Add `d` (1 to 9) on rod `rod`, carrying into the next.
fn add_on(a: &mut Abacus, rod: usize, d: u8, way: Option<Way>, out: &mut Vec<Step>) {
    let r: Rod = a.rod(rod);
    if r.value() + d >= 10 {
        // Send the big friend home, then one ten.
        let w = way.unwrap_or(Way::BigFriend);
        take_on(a, rod, 10 - d, Some(w), out);
        if rod + 1 < a.len() {
            add_on(a, rod + 1, 1, Some(w), out);
        }
    } else if d >= 5 {
        let w = way.unwrap_or(Way::Direct);
        push(a, out, rod, Move::HeavenDown, w);
        if d > 5 {
            push(a, out, rod, Move::EarthUp(d - 5), w);
        }
    } else if r.earth + d <= 4 {
        push(a, out, rod, Move::EarthUp(d), way.unwrap_or(Way::Direct));
    } else {
        // The big bead down, and the little friend home.
        let w = match way {
            Some(Way::BigFriend) | Some(Way::BothFriends) => Way::BothFriends,
            _ => Way::LittleFriend,
        };
        push(a, out, rod, Move::HeavenDown, w);
        push(a, out, rod, Move::EarthDown(5 - d), w);
    }
}

/// Take `d` (1 to 9) from rod `rod`, borrowing from the next.
fn take_on(a: &mut Abacus, rod: usize, d: u8, way: Option<Way>, out: &mut Vec<Step>) {
    let r = a.rod(rod);
    if r.value() < d {
        // One ten home, then the big friend up.
        let w = way.unwrap_or(Way::BigFriend);
        if rod + 1 < a.len() {
            take_on(a, rod + 1, 1, Some(w), out);
        }
        add_on(a, rod, 10 - d, Some(w), out);
    } else if d >= 5 {
        let w = way.unwrap_or(Way::Direct);
        push(a, out, rod, Move::HeavenUp, w);
        if d > 5 {
            push(a, out, rod, Move::EarthDown(d - 5), w);
        }
    } else if r.earth >= d {
        push(a, out, rod, Move::EarthDown(d), way.unwrap_or(Way::Direct));
    } else {
        // The little friend up, and the big bead home.
        let w = match way {
            Some(Way::BigFriend) | Some(Way::BothFriends) => Way::BothFriends,
            _ => Way::LittleFriend,
        };
        push(a, out, rod, Move::EarthUp(5 - d), w);
        push(a, out, rod, Move::HeavenUp, w);
    }
}

/// The pushes that add `d` (1 to 9) to what `start` shows.
pub fn add_steps(start: &Abacus, d: u8) -> Vec<Step> {
    let mut a = start.clone();
    let mut out = Vec::new();
    add_on(&mut a, 0, d, None, &mut out);
    out
}

/// The pushes that take `d` (1 to 9) from what `start` shows.
pub fn take_steps(start: &Abacus, d: u8) -> Vec<Step> {
    let mut a = start.clone();
    let mut out = Vec::new();
    take_on(&mut a, 0, d, None, &mut out);
    out
}

/// The hardest way any of `steps` takes.
pub fn way_of(steps: &[Step]) -> Way {
    let rank = |w: Way| match w {
        Way::Direct => 0,
        Way::LittleFriend => 1,
        Way::BigFriend => 2,
        Way::BothFriends => 3,
    };
    steps.iter().map(|s| s.way).max_by_key(|w| rank(*w)).unwrap_or(Way::Direct)
}

/// How `a + d` is done on an abacus of two rods.
pub fn add_way(a: u32, d: u8) -> Way {
    way_of(&add_steps(&Abacus::showing(2, a), d))
}

/// How `a − d` is done on an abacus of two rods.
pub fn take_way(a: u32, d: u8) -> Way {
    way_of(&take_steps(&Abacus::showing(2, a), d))
}

/// The friend that makes five with `d` (1 to 4).
pub fn little_friend(d: u8) -> u8 {
    5 - d
}

/// The friend that makes ten with `d` (1 to 9).
pub fn big_friend(d: u8) -> u8 {
    10 - d
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(start: u32, steps: &[Step]) -> u32 {
        let mut a = Abacus::showing(2, start);
        for s in steps {
            apply(&mut a, *s);
        }
        a.value()
    }

    #[test]
    fn every_sum_lands_on_its_answer() {
        for a in 0..=89u32 {
            for d in 1..=9u8 {
                let steps = add_steps(&Abacus::showing(2, a), d);
                assert_eq!(run(a, &steps), a + u32::from(d), "{a} + {d}: {steps:?}");
                if a >= u32::from(d) {
                    let steps = take_steps(&Abacus::showing(2, a), d);
                    assert_eq!(run(a, &steps), a - u32::from(d), "{a} − {d}: {steps:?}");
                }
            }
        }
    }

    #[test]
    fn friends_are_called_when_beads_run_out() {
        assert_eq!(add_way(2, 1), Way::Direct);
        assert_eq!(add_way(6, 3), Way::Direct);
        assert_eq!(add_way(3, 5), Way::Direct);
        assert_eq!(add_way(4, 1), Way::LittleFriend);
        assert_eq!(add_way(3, 4), Way::LittleFriend);
        assert_eq!(add_way(9, 1), Way::BigFriend);
        assert_eq!(add_way(8, 4), Way::BigFriend);
        assert_eq!(add_way(5, 6), Way::BothFriends);
        assert_eq!(take_way(5, 1), Way::LittleFriend);
        assert_eq!(take_way(7, 2), Way::Direct);
        assert_eq!(take_way(10, 1), Way::BigFriend);
        assert_eq!(take_way(14, 6), Way::BothFriends);
    }

    #[test]
    fn four_and_one_calls_the_big_bead_and_sends_four_home() {
        let steps = add_steps(&Abacus::showing(1, 4), 1);
        assert_eq!(steps.iter().map(|s| s.mv).collect::<Vec<_>>(), [Move::HeavenDown, Move::EarthDown(4)]);
        assert_eq!(steps[0].words(1), "Bring the big bead down");
        assert_eq!(steps[1].words(1), "Send 4 beads home");
        let steps = add_steps(&Abacus::showing(2, 9), 1);
        assert_eq!(
            steps.iter().map(|s| (s.rod, s.mv)).collect::<Vec<_>>(),
            [(0, Move::HeavenUp), (0, Move::EarthDown(4)), (1, Move::EarthUp(1))]
        );
        assert_eq!(steps[2].words(2), "Push 1 bead up on the tens");
    }
}
