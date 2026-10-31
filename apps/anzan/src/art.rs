//! Anzan's pictures: the soroban (a wooden frame, rods, the beam, and beads
//! that sleep away from the beam and wake, smiling, when they touch it),
//! Pon the tanuki and his speech bubble, ten-frames of dots, paper
//! lanterns for the path, the curtain that closes on a peek, and icons.
//! Everything is drawn in the design's points (see `sekka_kid_kit::stage`).

use std::f64::consts::PI;

use aizome_kit::layout::{Point, Rect, Size, Vec2};
use aizome_kit::sidestep_ui::kurbo::{BezPath, Circle, Ellipse, Line, RoundedRect};
use aizome_kit::sidestep_ui::{Canvas, Color, LineCap, LineJoin, LinearGradient, RadialGradient, Shadow, StrokeStyle};
use aizome_kit::theme::Theme;
use sekka_kid_kit::avatar::arc;
use sekka_kid_kit::look::{self, Look, font};

pub use sekka_kid_kit::art::{bubble, confetti, glyph, star};

use crate::abacus::Bead;

// The soroban's measures.

pub const BEAD_W: f64 = 112.0;
pub const BEAD_H: f64 = 40.0;
pub const ROD_GAP: f64 = 132.0;
pub const PAD: f64 = 26.0;
pub const BEAM_H: f64 = 16.0;

/// Where a soroban of `rods` rods sits, and where its beads go.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Frame {
    pub rods: usize,
    pub outer: Rect,
    pub inner: Rect,
    pub beam: Rect,
}

impl Frame {
    /// Centred on `cx`, its top at `top`.
    pub fn new(rods: usize, cx: f64, top: f64) -> Frame {
        let inner_w = rods as f64 * ROD_GAP;
        let inner_h = 2.0 * BEAD_H + 4.0 + BEAM_H + 5.0 * BEAD_H + 4.0;
        let outer = Rect::new(cx - inner_w / 2.0 - PAD, top, cx + inner_w / 2.0 + PAD, top + inner_h + 2.0 * PAD);
        let inner = outer.inflate(-PAD, -PAD);
        let beam_top = inner.y0 + 2.0 * BEAD_H + 4.0;
        Frame { rods, outer, inner, beam: Rect::new(inner.x0, beam_top, inner.x1, beam_top + BEAM_H) }
    }

    /// Rod `i`'s x (rod 0, the ones, on the right).
    pub fn rod_x(&self, i: usize) -> f64 {
        self.inner.x1 - ROD_GAP * (i as f64 + 0.5)
    }

    /// A bead's centre, `at` of the way from home (0) to the beam (1).
    pub fn bead_center(&self, rod: usize, bead: Bead, at: f64) -> Point {
        let x = self.rod_x(rod);
        let y = match bead {
            Bead::Heaven => lerp(self.inner.y0 + BEAD_H / 2.0, self.beam.y0 - BEAD_H / 2.0, at),
            Bead::Earth(i) => {
                let i = f64::from(i);
                lerp(self.beam.y1 + BEAD_H * (i + 1.5) + 4.0, self.beam.y1 + BEAD_H * (i + 0.5), at)
            }
        };
        Point::new(x, y)
    }

    pub fn bead_rect(&self, rod: usize, bead: Bead, at: f64) -> Rect {
        Rect::from_center_size(self.bead_center(rod, bead, at), Size::new(BEAD_W, BEAD_H))
    }

    /// The bead a press at `p` is for: on a bead, or in its stretch of rod.
    pub fn bead_at(&self, p: Point, at: impl Fn(usize, Bead) -> f64) -> Option<(usize, Bead)> {
        for rod in 0..self.rods {
            let x = self.rod_x(rod);
            if (p.x - x).abs() > ROD_GAP / 2.0 {
                continue;
            }
            // Beads first, where they are.
            for bead in crate::abacus::Bead::ALL {
                if self.bead_rect(rod, bead, at(rod, bead)).inflate(4.0, 2.0).contains(p) {
                    return Some((rod, bead));
                }
            }
            // Above the beam is the heaven bead's.
            if p.y >= self.inner.y0 && p.y < self.beam.y0 {
                return Some((rod, Bead::Heaven));
            }
        }
        None
    }
}

pub fn lerp(a: f64, b: f64, t: f64) -> f64 {
    a + (b - a) * t
}

/// Smoothly from 0 to 1, overshooting a little at the end: a bead
/// settling against the beam.
pub fn ease_back(t: f64) -> f64 {
    let t = t.clamp(0.0, 1.0);
    let s = 1.4;
    let u = t - 1.0;
    1.0 + u * u * ((s + 1.0) * u + s)
}

pub fn ease(t: f64) -> f64 {
    let t = t.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// The wooden frame, the rods and the beam (beads go on after).
pub fn soroban_frame(c: &mut Canvas, f: &Frame, look: &Look) {
    let outer = RoundedRect::from_rect(f.outer, 22.0);
    c.fill(&RoundedRect::from_rect(f.outer + Vec2::new(0.0, 9.0), 22.0), look.shadow);
    c.fill(&outer, look.wood_dark);
    // Flat fills, not gradients: the soroban is drawn again with every new
    // question, and big gradients cost milliseconds at twice the scale.
    c.fill(&RoundedRect::from_rect(f.outer.inflate(-3.0, -3.0), 19.0), look.wood_light.mix(look.wood, 0.45));
    c.fill(
        &RoundedRect::from_rect(
            Rect::new(f.outer.x0 + 3.0, f.outer.y0 + 3.0, f.outer.x1 - 3.0, f.outer.y0 + 12.0),
            8.0,
        ),
        look.wood_light.with_alpha(0.7),
    );
    // The well the beads move in, shaded at the top.
    let well = RoundedRect::from_rect(f.inner.inflate(4.0, 4.0), 10.0);
    c.fill(&well, look.wood_dark.mix(look.sky, 0.25).mix(Color::BLACK, 0.12));
    c.fill_rect(
        Rect::new(f.inner.x0 - 2.0, f.inner.y0 - 4.0, f.inner.x1 + 2.0, f.inner.y0 + 4.0),
        Color::rgba(0.0, 0.0, 0.0, 0.14),
    );
    // A few lines of grain on the wood.
    let line = StrokeStyle::new(1.2).with_cap(LineCap::Round);
    for i in 0..3 {
        let y = f.outer.y0 + 8.0 + f64::from(i) * 5.0;
        c.stroke(&Line::new((f.outer.x0 + 30.0, y), (f.outer.x1 - 30.0, y)), &line, look.wood_dark.with_alpha(0.18));
        let y = f.outer.y1 - 8.0 - f64::from(i) * 5.0;
        c.stroke(&Line::new((f.outer.x0 + 30.0, y), (f.outer.x1 - 30.0, y)), &line, look.wood_dark.with_alpha(0.18));
    }
    // Rods.
    for r in 0..f.rods {
        let x = f.rod_x(r);
        let rod = Rect::new(x - 3.5, f.inner.y0 - 4.0, x + 3.5, f.inner.y1 + 4.0);
        c.fill(
            &RoundedRect::from_rect(rod, 3.5),
            LinearGradient::new(
                Point::new(rod.x0, 0.0),
                Point::new(rod.x1, 0.0),
                look.rod.mix(Color::WHITE, 0.35),
                look.rod.mix(Color::BLACK, 0.25),
            ),
        );
    }
    // The beam, with the ones' dot.
    let beam = f.beam.inflate(PAD - 6.0, 0.0);
    c.fill(&RoundedRect::from_rect(beam, 4.0), look.beam);
    c.fill_rect(
        Rect::new(beam.x0 + 4.0, beam.y0 + 2.0, beam.x1 - 4.0, beam.y0 + 4.0),
        Color::rgba(1.0, 1.0, 1.0, 0.12),
    );
    for r in 0..f.rods {
        let x = f.rod_x(r);
        if r == 0 {
            c.fill(&Circle::new(Point::new(x, f.beam.center().y), 3.6), Color::hex(0xfff1cf));
        }
    }
}

/// How a bead looks.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct BeadLook {
    /// Awake: touching the beam.
    pub awake: bool,
    pub blink: bool,
    pub cheer: bool,
    /// Pointed out by a hint.
    pub glow: bool,
}

/// The shape of a soroban bead seen from the side: two cones base to
/// base, softened.
fn bead_shape(r: Rect) -> BezPath {
    let (cx, cy, w, h) = (r.center().x, r.center().y, r.width(), r.height());
    let (x0, x1, y0, y1) = (r.x0, r.x1, r.y0, r.y1);
    let mut p = BezPath::new();
    p.move_to(Point::new(x0, cy));
    p.curve_to(Point::new(x0 + w * 0.06, cy - h * 0.42), Point::new(cx - w * 0.34, y0), Point::new(cx - w * 0.16, y0));
    p.line_to(Point::new(cx + w * 0.16, y0));
    p.curve_to(Point::new(cx + w * 0.34, y0), Point::new(x1 - w * 0.06, cy - h * 0.42), Point::new(x1, cy));
    p.curve_to(Point::new(x1 - w * 0.06, cy + h * 0.42), Point::new(cx + w * 0.34, y1), Point::new(cx + w * 0.16, y1));
    p.line_to(Point::new(cx - w * 0.16, y1));
    p.curve_to(Point::new(cx - w * 0.34, y1), Point::new(x0 + w * 0.06, cy + h * 0.42), Point::new(x0, cy));
    p.close_path();
    p
}

/// A bead in `r`, of colour `base`.
pub fn bead(c: &mut Canvas, r: Rect, base: Color, l: BeadLook, look: &Look) {
    let shape = bead_shape(r);
    if l.glow {
        c.saved(|c| {
            c.set_shadow(Some(Shadow { offset: Vec2::ZERO, blur: 14.0, color: look.gold }));
            c.fill(&shape, look.gold);
        });
    }
    let base = if l.awake { base } else { base.mix(look.wood_dark, 0.22) };
    c.fill(
        &shape,
        LinearGradient {
            start: Point::new(0.0, r.y0),
            end: Point::new(0.0, r.y1),
            stops: vec![(0.0, base.mix(Color::WHITE, 0.38)), (0.45, base), (1.0, base.mix(Color::BLACK, 0.22))],
        },
    );
    c.fill(
        &Ellipse::new(
            Point::new(r.center().x - r.width() * 0.16, r.y0 + r.height() * 0.28),
            (r.width() * 0.17, r.height() * 0.11),
            -0.12,
        ),
        Color::rgba(1.0, 1.0, 1.0, 0.38),
    );
    c.stroke(&shape, &StrokeStyle::new(1.5), base.mix(Color::BLACK, 0.4));
    if l.glow {
        c.stroke(&shape, &StrokeStyle::new(3.0), look.gold);
    }
    // The face.
    let (cx, cy) = (r.center().x, r.center().y + 1.0);
    let ink = look.face;
    let eye = r.height() * 0.085;
    for s in [-1.0, 1.0] {
        let e = Point::new(cx + s * r.width() * 0.12, cy - r.height() * 0.06);
        if !l.awake {
            arc(c, e, eye * 1.2, 0.15 * PI, 0.85 * PI, ink.with_alpha(0.7), 2.0);
        } else if l.cheer {
            arc(c, e, eye * 1.25, PI, 2.0 * PI, ink, 2.4);
        } else if l.blink {
            arc(c, e, eye * 1.2, 0.15 * PI, 0.85 * PI, ink, 2.2);
        } else {
            c.fill(&Circle::new(e, eye), ink);
            c.fill(&Circle::new(e + Vec2::new(-eye * 0.35, -eye * 0.4), eye * 0.38), Color::WHITE);
        }
    }
    if l.awake {
        let m = Point::new(cx, cy + r.height() * 0.1);
        if l.cheer {
            let mut mouth = BezPath::new();
            mouth.move_to(Point::new(m.x - 6.0, m.y - 1.0));
            mouth.quad_to(Point::new(m.x, m.y + 9.0), Point::new(m.x + 6.0, m.y - 1.0));
            mouth.close_path();
            c.fill(&mouth, Color::hex(0x9c3a2e));
        } else {
            arc(c, Point::new(m.x, m.y - 3.0), 5.0, 0.2 * PI, 0.8 * PI, ink, 2.0);
        }
        for s in [-1.0, 1.0] {
            c.fill(
                &Ellipse::new(Point::new(cx + s * r.width() * 0.24, cy + r.height() * 0.08), (5.5, 3.2), 0.0),
                look.blush,
            );
        }
    }
}

/// An arrow by a hinted bead: which way to push it.
pub fn push_arrow(c: &mut Canvas, from: Point, up: bool, look: &Look, pulse: f64) {
    let d = if up { -1.0 } else { 1.0 };
    let x = from.x + BEAD_W / 2.0 + 26.0;
    let y = from.y + d * 6.0 * pulse;
    let mut p = BezPath::new();
    p.move_to(Point::new(x, y - d * 22.0));
    p.line_to(Point::new(x, y + d * 6.0));
    let s = StrokeStyle::new(7.0).with_cap(LineCap::Round).with_join(LineJoin::Round);
    c.stroke(&p, &s, look.gold);
    let mut head = BezPath::new();
    head.move_to(Point::new(x - 13.0, y + d * 2.0));
    head.line_to(Point::new(x, y + d * 18.0));
    head.line_to(Point::new(x + 13.0, y + d * 2.0));
    head.close_path();
    c.fill(&head, look.gold);
}

/// The colour of a rod's beads (the heaven bead stands out).
pub fn bead_color(bead: Bead, look: &Look) -> Color {
    match bead {
        Bead::Heaven => look.heaven_bead,
        Bead::Earth(_) => look.earth_bead,
    }
}

/// The curtain (a noren) drawn across `r`, `closed` of the way (0 open, 1
/// shut): two indigo panels with the crest, meeting in the middle.
pub fn curtain(c: &mut Canvas, r: Rect, closed: f64, theme: &Theme, look: &Look) {
    if closed <= 0.0 {
        return;
    }
    let half = r.width() / 2.0 * closed;
    let cloth = Color::hex(0x2c3a8c);
    for (x0, x1) in [(r.x0, r.x0 + half), (r.x1 - half, r.x1)] {
        let panel = Rect::new(x0, r.y0, x1, r.y1);
        c.fill(
            &RoundedRect::from_rect(panel, 14.0),
            LinearGradient::new(
                Point::new(0.0, r.y0),
                Point::new(0.0, r.y1),
                cloth.mix(Color::WHITE, 0.08),
                cloth.mix(Color::BLACK, 0.18),
            ),
        );
        // Folds.
        let fold = StrokeStyle::new(2.0);
        let mut x = x0 + 22.0;
        while x < x1 - 10.0 {
            c.stroke(&Line::new((x, r.y0 + 10.0), (x, r.y1 - 10.0)), &fold, Color::rgba(0.0, 0.0, 0.0, 0.12));
            x += 36.0;
        }
    }
    if closed > 0.95 {
        let m = r.center();
        c.fill(&Circle::new(m, 44.0), Color::hex(0xfff1cf));
        c.fill(&Circle::new(m, 38.0), cloth);
        look::text_centred(c, theme, "?", m, font(54.0, 900.0), Color::hex(0xfff1cf));
    }
    let _ = look;
}

/// A ten-frame: two rows of five, `n` (up to ten) filled, its top left at
/// `at`, cells `cell` points.
pub fn ten_frame(c: &mut Canvas, at: Point, cell: f64, n: u32, look: &Look) {
    let frame = Rect::from_origin_size(at, Size::new(cell * 5.0, cell * 2.0));
    c.fill(&RoundedRect::from_rect(frame.inflate(3.0, 3.0), 6.0), look.card);
    let s = StrokeStyle::new(2.0);
    c.stroke(&RoundedRect::from_rect(frame.inflate(3.0, 3.0), 6.0), &s, look.ink_soft);
    for i in 1..5 {
        let x = at.x + cell * f64::from(i);
        c.stroke(&Line::new((x, frame.y0), (x, frame.y1)), &s, look.ink_soft.with_alpha(0.5));
    }
    c.stroke(&Line::new((frame.x0, frame.center().y), (frame.x1, frame.center().y)), &s, look.ink_soft.with_alpha(0.5));
    for i in 0..n.min(10) {
        let (row, col) = (i / 5, i % 5);
        let p = Point::new(at.x + cell * (f64::from(col) + 0.5), at.y + cell * (f64::from(row) + 0.5));
        c.fill(&Circle::new(p, cell * 0.33), if row == 0 { look.accent } else { look.calm });
    }
}

/// How a lantern on the path is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Lantern {
    /// Its lesson done: lit.
    Lit,
    /// Open and not yet done: dark, with a warm glow round it.
    Waiting,
    Dark,
}

/// A paper lantern (chōchin) hanging at `at` (its middle), with `n` written
/// on it.
pub fn lantern(c: &mut Canvas, theme: &Theme, at: Point, size: f64, n: usize, state: Lantern, look: &Look) {
    let lit = state == Lantern::Lit;
    let glow = if state == Lantern::Waiting { 0.75 } else { 0.0 };
    let (rx, ry) = (size * 0.78, size);
    let red = Color::hex(0xd8432e);
    let paper = if lit {
        red
    } else if look.night {
        Color::hex(0x5a5f8c)
    } else {
        Color::hex(0xd9cbb4)
    };
    if lit || glow > 0.0 {
        let halo = if lit { 1.0 } else { glow };
        c.fill(
            &Circle::new(at, size * 2.2),
            RadialGradient::new(at, size * 2.2, look.gold.with_alpha((0.55 * halo) as f32), look.gold.with_alpha(0.0)),
        );
    }
    // The string, and the caps.
    let cap = Color::hex(0x2a1d18);
    c.stroke(&Line::new((at.x, at.y - ry - 16.0), (at.x, at.y - ry)), &StrokeStyle::new(2.0), cap);
    let body = Ellipse::new(at, (rx, ry), 0.0);
    let inside: aizome_kit::sidestep_ui::Paint =
        if lit { RadialGradient::new(at, ry, Color::hex(0xffd27a), paper).into() } else { paper.into() };
    c.fill(&body, inside);
    // Ribs.
    let rib = StrokeStyle::new(1.4);
    for i in 1..6 {
        let t = f64::from(i) / 6.0;
        let y = at.y - ry + 2.0 * ry * t;
        let half = rx * (1.0 - ((t - 0.5) * 2.0).powi(2)).sqrt();
        c.stroke(&Line::new((at.x - half, y), (at.x + half, y)), &rib, Color::rgba(0.0, 0.0, 0.0, 0.16));
    }
    c.stroke(&body, &StrokeStyle::new(1.5), paper.mix(Color::BLACK, 0.3));
    for y in [at.y - ry, at.y + ry] {
        c.fill(
            &RoundedRect::from_rect(Rect::from_center_size(Point::new(at.x, y), Size::new(rx * 1.1, size * 0.22)), 3.0),
            cap,
        );
    }
    let ink = if lit { Color::hex(0xfff3dc) } else { look.ink_soft };
    look::text_centred(c, theme, &n.to_string(), at, font(size * 0.78, 900.0), ink);
}

/// A light bulb: the hint button's.
pub fn bulb(c: &mut Canvas, at: Point, size: f64, color: Color) {
    let r = size * 0.3;
    let s = StrokeStyle::new(size * 0.09).with_cap(LineCap::Round).with_join(LineJoin::Round);
    let top = Point::new(at.x, at.y - size * 0.1);
    let mut p = BezPath::new();
    p.move_to(Point::new(at.x - r * 0.55, at.y + size * 0.16));
    p.curve_to(
        Point::new(at.x - r * 1.2, top.y + r * 0.4),
        Point::new(at.x - r, top.y - r * 1.1),
        top + Vec2::new(0.0, -r),
    );
    p.curve_to(
        Point::new(at.x + r, top.y - r * 1.1),
        Point::new(at.x + r * 1.2, top.y + r * 0.4),
        Point::new(at.x + r * 0.55, at.y + size * 0.16),
    );
    c.fill(&p, color.with_alpha(0.25));
    c.stroke(&p, &s, color);
    for dy in [0.24, 0.34] {
        c.stroke(&Line::new((at.x - r * 0.5, at.y + size * dy), (at.x + r * 0.5, at.y + size * dy)), &s, color);
    }
}
