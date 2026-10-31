//! The soroban: rods of one heaven bead (five) over the beam and four earth
//! beads (one each) under it. A bead counts when it touches the beam.

/// Earth beads on a rod.
pub const EARTH: u8 = 4;

/// One rod's beads: whether the heaven bead is down at the beam, and how
/// many earth beads are up at it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Rod {
    pub heaven: bool,
    pub earth: u8,
}

impl Rod {
    /// The rod showing digit `d` (0 to 9).
    pub fn of(d: u8) -> Rod {
        Rod { heaven: d >= 5, earth: d % 5 }
    }

    pub fn value(self) -> u8 {
        if self.heaven { 5 + self.earth } else { self.earth }
    }

    /// Whether `bead` touches the beam.
    pub fn counts(self, bead: Bead) -> bool {
        match bead {
            Bead::Heaven => self.heaven,
            Bead::Earth(i) => i < self.earth,
        }
    }
}

/// A bead of a rod: the heaven bead, or an earth bead counted from the
/// beam (0 is the one nearest it).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Bead {
    Heaven,
    Earth(u8),
}

impl Bead {
    /// A rod's five beads, the heaven bead first.
    pub const ALL: [Bead; 5] = [Bead::Heaven, Bead::Earth(0), Bead::Earth(1), Bead::Earth(2), Bead::Earth(3)];
}

/// An abacus of a few rods; rod 0 is the ones, on the right.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Abacus {
    rods: Vec<Rod>,
}

impl Abacus {
    /// `n` rods, all beads away from the beam: zero.
    pub fn new(n: usize) -> Abacus {
        Abacus { rods: vec![Rod::default(); n.max(1)] }
    }

    /// `n` rods showing `value`.
    pub fn showing(n: usize, value: u32) -> Abacus {
        let mut a = Abacus::new(n);
        a.set(value);
        a
    }

    pub fn len(&self) -> usize {
        self.rods.len()
    }

    pub fn is_empty(&self) -> bool {
        self.rods.is_empty()
    }

    pub fn rods(&self) -> &[Rod] {
        &self.rods
    }

    pub fn rod(&self, i: usize) -> Rod {
        self.rods.get(i).copied().unwrap_or_default()
    }

    pub fn set_rod(&mut self, i: usize, rod: Rod) {
        if let Some(r) = self.rods.get_mut(i) {
            *r = rod;
        }
    }

    /// The largest number it holds.
    pub fn max(&self) -> u32 {
        10u32.pow(self.rods.len() as u32) - 1
    }

    pub fn value(&self) -> u32 {
        self.rods.iter().rev().fold(0, |v, r| v * 10 + u32::from(r.value()))
    }

    /// Show `value` (what doesn't fit is dropped from the left).
    pub fn set(&mut self, value: u32) {
        let mut v = value;
        for r in &mut self.rods {
            *r = Rod::of((v % 10) as u8);
            v /= 10;
        }
    }

    /// Push `bead` the way a finger would: a bead away from the beam goes to
    /// it, carrying the earth beads between; one at the beam goes away,
    /// carrying the ones beyond it. Whether anything moved.
    pub fn flick(&mut self, rod: usize, bead: Bead) -> bool {
        let Some(r) = self.rods.get_mut(rod) else { return false };
        let to = !r.counts(bead);
        Abacus::push_rod(r, bead, to)
    }

    /// Push `bead` to the beam (`to_beam`) or away from it, as far as it
    /// goes. Whether anything moved.
    pub fn push(&mut self, rod: usize, bead: Bead, to_beam: bool) -> bool {
        let Some(r) = self.rods.get_mut(rod) else { return false };
        Abacus::push_rod(r, bead, to_beam)
    }

    fn push_rod(r: &mut Rod, bead: Bead, to_beam: bool) -> bool {
        let before = *r;
        match bead {
            Bead::Heaven => r.heaven = to_beam,
            Bead::Earth(i) if to_beam => r.earth = r.earth.max(i + 1).min(EARTH),
            Bead::Earth(i) => r.earth = r.earth.min(i),
        }
        *r != before
    }

    /// The fewest pushes that turn this abacus into one showing `value`:
    /// on each rod, one for the heaven bead and one for the earth beads,
    /// where they differ.
    pub fn pushes_to(&self, value: u32) -> u32 {
        let target = Abacus::showing(self.len(), value);
        self.rods
            .iter()
            .zip(&target.rods)
            .map(|(a, b)| u32::from(a.heaven != b.heaven) + u32::from(a.earth != b.earth))
            .sum()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rods_show_digits() {
        for d in 0..10 {
            assert_eq!(Rod::of(d).value(), d);
        }
        assert_eq!(Abacus::showing(2, 47).value(), 47);
        assert_eq!(Abacus::showing(2, 47).rod(1), Rod::of(4));
        assert_eq!(Abacus::showing(1, 47).value(), 7, "the tens don't fit");
        assert_eq!(Abacus::new(3).max(), 999);
    }

    #[test]
    fn a_flick_carries_the_beads_between() {
        let mut a = Abacus::new(1);
        assert!(a.flick(0, Bead::Earth(2)));
        assert_eq!(a.value(), 3, "the third bead up carries the two above it");
        assert!(a.flick(0, Bead::Earth(1)));
        assert_eq!(a.value(), 1, "the second bead down carries the one below it");
        assert!(a.flick(0, Bead::Heaven));
        assert_eq!(a.value(), 6);
        assert!(!a.push(0, Bead::Heaven, true), "already at the beam");
        assert!(a.push(0, Bead::Earth(3), true));
        assert_eq!(a.value(), 9);
    }

    #[test]
    fn pushes_count_the_heaven_bead_and_the_earth_beads() {
        let a = Abacus::showing(2, 3);
        assert_eq!(a.pushes_to(3), 0);
        assert_eq!(a.pushes_to(4), 1);
        assert_eq!(a.pushes_to(5), 2, "the big bead down and the earth beads away");
        assert_eq!(a.pushes_to(13), 1);
    }
}
