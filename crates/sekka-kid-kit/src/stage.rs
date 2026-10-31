//! The stage kids' screens are drawn on: a 1000 × 700 design scaled to fit
//! the window and centred, over a backdrop that fills it all (sky and hills
//! by day, stars and a moon by night).

use aizome_kit::layout::{Point, Rect, Size, Vec2};
use aizome_kit::sidestep_ui::kurbo::{BezPath, Circle};
use aizome_kit::sidestep_ui::{Canvas, Color, RadialGradient};

use crate::look::Look;

/// The design's size, in its own points.
pub const W: f64 = 1000.0;
pub const H: f64 = 700.0;

/// How the design sits in a window.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Fit {
    pub scale: f64,
    pub origin: Point,
}

impl Fit {
    /// The design fitted into `b`, centred.
    pub fn new(b: Rect) -> Fit {
        let scale = (b.width() / W).min(b.height() / H).max(0.01);
        Fit { scale, origin: Point::new(b.x0 + (b.width() - W * scale) / 2.0, b.y0 + (b.height() - H * scale) / 2.0) }
    }

    pub fn of_size(size: Size) -> Fit {
        Fit::new(Rect::from_origin_size(Point::ZERO, size))
    }

    /// Draw in the design's points from here on (inside `saved`).
    pub fn apply(&self, c: &mut Canvas) {
        c.translate(self.origin.to_vec2());
        c.scale_by(self.scale, self.scale);
    }

    /// A window point in the design's points.
    pub fn to_design(&self, p: Point) -> Point {
        Point::new((p.x - self.origin.x) / self.scale, (p.y - self.origin.y) / self.scale)
    }

    /// A design rectangle in the window's points.
    pub fn to_window(&self, r: Rect) -> Rect {
        Rect::new(
            self.origin.x + r.x0 * self.scale,
            self.origin.y + r.y0 * self.scale,
            self.origin.x + r.x1 * self.scale,
            self.origin.y + r.y1 * self.scale,
        )
    }

    /// The window's whole rectangle `b`, in the design's points.
    pub fn visible(&self, b: Rect) -> Rect {
        let a = self.to_design(Point::new(b.x0, b.y0));
        let z = self.to_design(Point::new(b.x1, b.y1));
        Rect::new(a.x, a.y, z.x, z.y)
    }
}

/// A number from `i` that looks random and isn't.
fn noise(i: u32) -> f64 {
    let mut x = i.wrapping_mul(0x9e37_79b9) ^ 0x85eb_ca6b;
    x ^= x >> 15;
    x = x.wrapping_mul(0x2c1b_3c6d);
    x ^= x >> 12;
    f64::from(x % 10_000) / 10_000.0
}

/// The sky and the hills, over all of `b` (the window), drawn in the
/// design's points through `fit`. `hills` is how high the land comes (0
/// for none).
pub fn backdrop(c: &mut Canvas, b: Rect, fit: &Fit, look: &Look, hills: f64) {
    // The sky in flat bands too fine to see: a gradient over the whole
    // window costs several milliseconds a frame at twice the scale, flat
    // fills almost nothing.
    let bands = 48;
    for i in 0..bands {
        let t = f64::from(i) / f64::from(bands);
        let y0 = (b.y0 + b.height() * t).floor();
        let y1 = (b.y0 + b.height() * (t + 1.0 / f64::from(bands))).ceil();
        c.fill_rect(Rect::new(b.x0, y0, b.x1, y1), look.sky_top.mix(look.sky, (t + 0.5 / f64::from(bands)) as f32));
    }
    c.saved(|c| {
        fit.apply(c);
        let v = fit.visible(b);
        if look.night {
            for i in 0..90 {
                let x = v.x0 + noise(i * 2) * v.width();
                let y = v.y0 + noise(i * 2 + 1) * (H * 0.62 - v.y0);
                let r = 0.8 + noise(i * 7) * 1.6;
                c.fill(
                    &Circle::new(Point::new(x, y), r),
                    Color::rgba(1.0, 0.97, 0.88, 0.35 + noise(i * 3) as f32 * 0.5),
                );
            }
            // The moon, with its glow.
            let m = Point::new(W - 120.0, 92.0);
            c.fill(
                &Circle::new(m, 90.0),
                RadialGradient::new(m, 90.0, Color::rgba(1.0, 0.93, 0.75, 0.22), Color::rgba(1.0, 0.93, 0.75, 0.0)),
            );
            c.fill(&Circle::new(m, 34.0), Color::hex(0xfff1cf));
            c.fill(&Circle::new(m + Vec2::new(12.0, -8.0), 30.0), look.sky_top.mix(Color::hex(0xfff1cf), 0.12));
        } else {
            let s = Point::new(W - 120.0, 92.0);
            c.fill(
                &Circle::new(s, 110.0),
                RadialGradient::new(s, 110.0, Color::rgba(1.0, 0.86, 0.45, 0.45), Color::rgba(1.0, 0.86, 0.45, 0.0)),
            );
            c.fill(&Circle::new(s, 40.0), Color::hex(0xffd36b));
            for (cx, cy, k) in [(170.0, 95.0, 1.0), (430.0, 60.0, 0.7), (640.0, 130.0, 0.85)] {
                cloud(c, Point::new(cx, cy), k);
            }
        }
        if hills > 0.0 {
            let (far, near) = if look.night {
                (Color::hex(0x1e2356), Color::hex(0x171b45))
            } else {
                (Color::hex(0xcfe8b8), Color::hex(0xb3d99b))
            };
            hill(c, v, H - hills, 70.0, 0.9, far);
            hill(c, v, H - hills * 0.62, 50.0, 1.7, near);
        }
    });
}

fn cloud(c: &mut Canvas, at: Point, k: f64) {
    let white = Color::rgba(1.0, 1.0, 1.0, 0.92);
    for (dx, dy, r) in [(-38.0, 6.0, 24.0), (-8.0, -10.0, 32.0), (28.0, 0.0, 26.0), (52.0, 10.0, 18.0)] {
        c.fill(&Circle::new(at + Vec2::new(dx * k, dy * k), r * k), white);
    }
}

/// A rolling line of hills at height `y`, `amp` high, `waves` across the
/// design, filled down past the window's bottom.
fn hill(c: &mut Canvas, v: Rect, y: f64, amp: f64, waves: f64, color: Color) {
    let mut p = BezPath::new();
    let x0 = v.x0 - 10.0;
    let x1 = v.x1 + 10.0;
    p.move_to(Point::new(x0, v.y1 + 10.0));
    let steps = 48;
    for i in 0..=steps {
        let x = x0 + (x1 - x0) * f64::from(i) / f64::from(steps);
        let t = x / W * waves * std::f64::consts::TAU;
        p.line_to(Point::new(x, y - amp * 0.5 * (1.0 + t.sin())));
    }
    p.line_to(Point::new(x1, v.y1 + 10.0));
    p.close_path();
    c.fill(&p, color);
}
