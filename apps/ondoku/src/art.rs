//! Ondoku's pictures: book covers, Fumi the owl, the wooden shelf, the
//! microphone's rings, and the type a page is set in.

use std::sync::OnceLock;

use aizome_kit::layout::{Point, Rect, Vec2};
use aizome_kit::sidestep_ui::kurbo::{Circle, RoundedRect};
use aizome_kit::sidestep_ui::{Canvas, Color, Font, StrokeStyle, TextStyle};
use aizome_kit::theme::{FontSpec, Theme};
use sekka_kid::Animal;
use sekka_kid_kit::art::{glyph, star};
use sekka_kid_kit::avatar::{self, Mood};
use sekka_kid_kit::look::{self, Look, font};

use crate::library::Emblem;

static READING_FONT: OnceLock<bool> = OnceLock::new();

/// Install the type pages are set in: Andika (SIL), made for children
/// learning to read, with an "a" and a "g" shaped as they're written.
/// `SEKKA_EDU_READING_FONT` names its regular file; without it pages are
/// set in Archivo, Aizome's own.
pub fn install_reading_font() {
    READING_FONT.get_or_init(|| {
        let Some(path) = std::env::var_os("SEKKA_EDU_READING_FONT").filter(|v| !v.is_empty()) else { return false };
        match Font::install_file(&path) {
            Ok(families) => families.iter().any(|f| f == "Andika"),
            Err(e) => {
                log::warn!("{}: {e}", std::path::Path::new(&path).display());
                false
            }
        }
    });
}

/// The style a page's words are set in, at `size`.
pub fn reading(theme: &Theme, size: f64, color: Color) -> TextStyle {
    if READING_FONT.get() == Some(&true)
        && let Some(f) = Font::named("Andika", size)
    {
        return TextStyle::new(f, color);
    }
    theme.style(FontSpec { weight: 500.0, ..FontSpec::ui(size) }, color)
}

/// Fumi the owl, Ondoku's reading buddy.
pub fn fumi(c: &mut Canvas, at: Point, r: f64, mood: Mood) {
    avatar::paint(c, at, r, Animal::Owl, None, mood);
}

/// A book's cover in `r`: its colour, a spine, its emblem, its title, a
/// ribbon if it's half read, a star for each time it's been read through
/// (three at most), and a gold corner on classics.
#[allow(clippy::too_many_arguments)]
pub fn cover(
    c: &mut Canvas,
    theme: &Theme,
    r: Rect,
    title: &str,
    emblem: Emblem,
    color: u32,
    classic: bool,
    bookmark: bool,
    finished: u32,
    look: &Look,
) {
    let base = Color::hex(color);
    let radius = r.width() * 0.06;
    c.fill(&RoundedRect::from_rect(r + Vec2::new(4.0, 6.0), radius), look.shadow);
    c.fill(&RoundedRect::from_rect(r, radius), base);
    let spine = Rect::new(r.x0, r.y0, r.x0 + r.width() * 0.09, r.y1);
    c.fill(&RoundedRect::from_rect(spine, radius), base.mix(Color::BLACK, 0.22));
    c.fill(
        &RoundedRect::from_rect(Rect::new(r.x0 + r.width() * 0.09, r.y0, r.x1, r.y0 + 3.0), 1.5),
        base.mix(Color::WHITE, 0.25),
    );
    let face = Rect::new(spine.x1, r.y0, r.x1, r.y1);
    // The emblem in a pale circle, in the upper part.
    let at = Point::new(face.center().x, r.y0 + r.height() * 0.38);
    let er = face.width() * 0.3;
    c.fill(&Circle::new(at, er * 1.12), base.mix(Color::WHITE, 0.82));
    match emblem {
        Emblem::Animal(a) => avatar::paint(c, at + Vec2::new(0.0, er * 0.08), er * 0.92, a, None, Mood::Happy),
        Emblem::Icon(name) => glyph(c, name, at, er * 1.25, base.mix(Color::BLACK, 0.15)),
    }
    // The title, under it, as many lines as it takes.
    let size = (r.width() * 0.105).clamp(11.0, 30.0);
    let spec = font(size, 800.0);
    let lines = look::wrap(theme, title, face.width() - size * 0.9, spec);
    let lh = size * 1.12;
    let top = r.y0 + r.height() * 0.66;
    for (i, line) in lines.iter().take(3).enumerate() {
        look::text_centred(c, theme, line, Point::new(face.center().x, top + lh * i as f64), spec, Color::WHITE);
    }
    if classic {
        let s = r.width() * 0.2;
        let mut corner = aizome_kit::sidestep_ui::kurbo::BezPath::new();
        corner.move_to(Point::new(r.x1 - s, r.y0));
        corner.line_to(Point::new(r.x1 - radius, r.y0));
        corner.quad_to(Point::new(r.x1, r.y0), Point::new(r.x1, r.y0 + radius));
        corner.line_to(Point::new(r.x1, r.y0 + s));
        corner.close_path();
        c.fill(&corner, look.gold);
    }
    if bookmark {
        let x = r.x1 - r.width() * 0.3;
        let w = r.width() * 0.1;
        let mut ribbon = aizome_kit::sidestep_ui::kurbo::BezPath::new();
        ribbon.move_to(Point::new(x, r.y0 - 4.0));
        ribbon.line_to(Point::new(x + w, r.y0 - 4.0));
        ribbon.line_to(Point::new(x + w, r.y0 + r.height() * 0.24));
        ribbon.line_to(Point::new(x + w / 2.0, r.y0 + r.height() * 0.2));
        ribbon.line_to(Point::new(x, r.y0 + r.height() * 0.24));
        ribbon.close_path();
        c.fill(&ribbon, look.accent);
    }
    let n = finished.min(3);
    let sr = r.width() * 0.065;
    for i in 0..n {
        let x = face.center().x + (f64::from(i) - f64::from(n - 1) / 2.0) * sr * 2.6;
        star(c, Point::new(x, r.y1 - sr * 1.8), sr, true, look);
    }
}

/// A plank for books to stand on, under `x0..x1` at `y`.
pub fn plank(c: &mut Canvas, x0: f64, x1: f64, y: f64, look: &Look) {
    let r = Rect::new(x0, y, x1, y + 14.0);
    c.fill(&RoundedRect::from_rect(r + Vec2::new(0.0, 6.0), 4.0), look.shadow);
    c.fill(&RoundedRect::from_rect(r, 4.0), look.wood);
    c.fill(&RoundedRect::from_rect(Rect::new(x0, y, x1, y + 4.0), 2.0), look.wood_light);
}

/// Rings around the microphone as it hears: `level` its loudness (RMS),
/// `t` a running phase so they breathe even in quiet.
pub fn rings(c: &mut Canvas, at: Point, r: f64, level: f32, t: f64, look: &Look) {
    let loud = (f64::from(level) * 14.0).clamp(0.0, 1.0);
    for k in 0..3 {
        let phase = ((t * 0.9 + f64::from(k) / 3.0) % 1.0).abs();
        let rr = r + 6.0 + phase * (18.0 + 26.0 * loud);
        let alpha = ((1.0 - phase) * (0.18 + 0.5 * loud)) as f32;
        c.stroke(&Circle::new(at, rr), &StrokeStyle::new(3.0), look.accent.with_alpha(alpha));
    }
}
