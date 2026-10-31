//! Pictures every Sekka Edu app draws: a speech bubble for the app's
//! guide, stars, Lucide icons stroked boldly, and confetti. Drawn in the
//! design's points (`stage`).

use std::f64::consts::PI;

use aizome_kit::Icon;
use aizome_kit::layout::{Point, Rect, Size, Vec2};
use aizome_kit::sidestep_ui::kurbo::{Affine, BezPath, RoundedRect};
use aizome_kit::sidestep_ui::{Canvas, Color, LineCap, LineJoin, StrokeStyle};

use crate::look::Look;

/// A speech bubble over `r`, its tail toward `to`.
pub fn bubble(c: &mut Canvas, r: Rect, to: Point, look: &Look) {
    let body = RoundedRect::from_rect(r, 28.0);
    let mut tail = BezPath::new();
    let y = (to.y).clamp(r.y0 + 30.0, r.y1 - 30.0);
    tail.move_to(Point::new(r.x0 + 2.0, y - 18.0));
    tail.quad_to(Point::new(r.x0 - 14.0, y), to);
    tail.quad_to(Point::new(r.x0 - 4.0, y + 8.0), Point::new(r.x0 + 2.0, y + 16.0));
    tail.close_path();
    c.fill(&RoundedRect::from_rect(r + Vec2::new(0.0, 6.0), 28.0), look.shadow);
    c.fill(&body, look.card);
    c.fill(&tail, look.card);
    c.stroke(&body, &StrokeStyle::new(2.0), look.card_edge);
    // The tail over the body's edge, so they read as one.
    c.fill(&tail, look.card);
}

/// A five-pointed star centred at `at`, `r` to its points.
pub fn star(c: &mut Canvas, at: Point, r: f64, filled: bool, look: &Look) {
    let mut p = BezPath::new();
    for i in 0..10 {
        let a = -PI / 2.0 + PI * f64::from(i) / 5.0;
        let rr = if i % 2 == 0 { r } else { r * 0.46 };
        let q = Point::new(at.x + rr * a.cos(), at.y + rr * a.sin());
        if i == 0 {
            p.move_to(q);
        } else {
            p.line_to(q);
        }
    }
    p.close_path();
    if filled {
        c.fill(&p, look.gold);
        c.stroke(&p, &StrokeStyle::new(r * 0.08).with_join(LineJoin::Round), look.gold.mix(Color::BLACK, 0.25));
    } else {
        c.fill(&p, look.ink_soft.with_alpha(0.18));
        c.stroke(&p, &StrokeStyle::new(r * 0.07).with_join(LineJoin::Round), look.ink_soft.with_alpha(0.5));
    }
}

/// Lucide's `name` stroked boldly, `size` square, centred on `at`.
pub fn glyph(c: &mut Canvas, name: &str, at: Point, size: f64, color: Color) {
    let Some(icon) = Icon::from_name(name) else { return };
    let path = icon.path();
    c.saved(|c| {
        c.concat(Affine::translate(at.to_vec2() - Vec2::new(size / 2.0, size / 2.0)) * Affine::scale(size / 24.0));
        c.stroke(&*path, &StrokeStyle::new(2.6).with_cap(LineCap::Round).with_join(LineJoin::Round), color);
    });
}

/// Confetti: `n` pieces falling from the top of `r`, `t` (0 to 1) of the
/// way through.
pub fn confetti(c: &mut Canvas, r: Rect, t: f64, look: &Look) {
    if !(0.0..1.0).contains(&t) {
        return;
    }
    let colors = [look.accent, look.gold, look.good, look.calm, Color::hex(0xb05cc4)];
    for i in 0..70u32 {
        let k = f64::from(i);
        let x0 = r.x0 + r.width() * ((k * 0.618_034).fract());
        let speed = 0.6 + (k * 0.3819).fract() * 0.6;
        let y = r.y0 - 30.0 + (r.height() + 60.0) * (t * speed * 1.4).min(1.2);
        let sway = (t * 9.0 + k).sin() * 22.0;
        let spin = t * 12.0 + k;
        let alpha = (1.0 - t).clamp(0.0, 1.0) as f32 * 0.95;
        let piece = Rect::from_center_size(Point::ZERO, Size::new(10.0, 6.0));
        c.saved(|c| {
            c.concat(Affine::translate(Vec2::new(x0 + sway, y)) * Affine::rotate(spin));
            c.fill(&RoundedRect::from_rect(piece, 1.5), colors[i as usize % colors.len()].with_alpha(alpha));
        });
    }
}
