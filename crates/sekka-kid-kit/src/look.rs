//! Sekka Edu's look for kids: warm paper by day, a lantern-lit indigo
//! evening by night (following Aizome's Night and Day), big round shapes,
//! and type set large in Archivo.

use aizome_kit::layout::Vec2;
use aizome_kit::layout::{Point, Rect, Size};
use aizome_kit::sidestep_ui::kurbo::{RoundedRect, Shape};
use aizome_kit::sidestep_ui::{Canvas, Color, LinearGradient, StrokeStyle};
use aizome_kit::theme::{FontSpec, Theme};

/// The colours kids' screens are drawn with.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Look {
    pub night: bool,
    pub sky_top: Color,
    pub sky: Color,
    pub ground: Color,
    pub ink: Color,
    pub ink_soft: Color,
    pub card: Color,
    pub card_edge: Color,
    pub accent: Color,
    pub accent_dark: Color,
    pub gold: Color,
    pub good: Color,
    pub good_dark: Color,
    pub calm: Color,
    pub calm_dark: Color,
    pub shadow: Color,
    pub wood: Color,
    pub wood_light: Color,
    pub wood_dark: Color,
    pub beam: Color,
    pub rod: Color,
    pub earth_bead: Color,
    pub heaven_bead: Color,
    pub face: Color,
    pub blush: Color,
}

impl Look {
    pub const DAY: Look = Look {
        night: false,
        sky_top: Color::hex(0xcfe9f7),
        sky: Color::hex(0xfff4e0),
        ground: Color::hex(0xf8e2bd),
        ink: Color::hex(0x3d2c24),
        ink_soft: Color::hex(0x8a7262),
        card: Color::hex(0xffffff),
        card_edge: Color::hex(0xead3ae),
        accent: Color::hex(0xe8553d),
        accent_dark: Color::hex(0xb73d2a),
        gold: Color::hex(0xf2b33d),
        good: Color::hex(0x43a85f),
        good_dark: Color::hex(0x2f7f45),
        calm: Color::hex(0x5d8fd6),
        calm_dark: Color::hex(0x3f6aab),
        shadow: Color::rgba(0.36, 0.22, 0.10, 0.22),
        wood: Color::hex(0xb9773e),
        wood_light: Color::hex(0xdca468),
        wood_dark: Color::hex(0x7d4a22),
        beam: Color::hex(0x3b2618),
        rod: Color::hex(0x8a6a4c),
        earth_bead: Color::hex(0xf2a93b),
        heaven_bead: Color::hex(0xe8553d),
        face: Color::hex(0x3d2c24),
        blush: Color::rgba(0.97, 0.45, 0.45, 0.45),
    };

    pub const NIGHT: Look = Look {
        night: true,
        sky_top: Color::hex(0x121534),
        sky: Color::hex(0x252a63),
        ground: Color::hex(0x2c3270),
        ink: Color::hex(0xfff2dc),
        ink_soft: Color::hex(0xb9b4dd),
        card: Color::hex(0x323978),
        card_edge: Color::hex(0x464ea0),
        accent: Color::hex(0xff7a59),
        accent_dark: Color::hex(0xc4523a),
        gold: Color::hex(0xffc75a),
        good: Color::hex(0x5cc77c),
        good_dark: Color::hex(0x3a9455),
        calm: Color::hex(0x7aa7f0),
        calm_dark: Color::hex(0x4f78c0),
        shadow: Color::rgba(0.02, 0.02, 0.10, 0.45),
        wood: Color::hex(0xa5672f),
        wood_light: Color::hex(0xc98d52),
        wood_dark: Color::hex(0x6a3c19),
        beam: Color::hex(0x2a1a10),
        rod: Color::hex(0x8f7256),
        earth_bead: Color::hex(0xffb547),
        heaven_bead: Color::hex(0xff6a4d),
        face: Color::hex(0x3d2c24),
        blush: Color::rgba(1.0, 0.45, 0.45, 0.45),
    };

    pub fn for_theme(theme: &Theme) -> Look {
        if theme.is_dark() { Look::NIGHT } else { Look::DAY }
    }
}

/// Archivo at `size`, `weight`.
pub fn font(size: f64, weight: f32) -> FontSpec {
    FontSpec { weight, ..FontSpec::ui(size) }
}

/// How wide `s` is set in `spec`.
pub fn measure(theme: &Theme, s: &str, spec: FontSpec) -> Size {
    aizome_kit::sidestep_ui::measure(s, &theme.style(spec, Color::BLACK))
}

/// `s` with its top left at `at`.
pub fn text(c: &mut Canvas, theme: &Theme, s: &str, at: Point, spec: FontSpec, color: Color) {
    c.draw_label(s, &theme.style(spec, color), at);
}

/// `s` centred on `at`.
pub fn text_centred(c: &mut Canvas, theme: &Theme, s: &str, at: Point, spec: FontSpec, color: Color) {
    let size = measure(theme, s, spec);
    text(c, theme, s, Point::new(at.x - size.width / 2.0, at.y - size.height / 2.0), spec, color);
}

/// `s` wrapped to `width`, each line centred, the block centred on `at`.
pub fn paragraph_centred(c: &mut Canvas, theme: &Theme, s: &str, at: Point, width: f64, spec: FontSpec, color: Color) {
    let lines = wrap(theme, s, width, spec);
    let h = measure(theme, "Ag", spec).height * 1.08;
    let top = at.y - h * lines.len() as f64 / 2.0 + h / 2.0;
    for (i, line) in lines.iter().enumerate() {
        text_centred(c, theme, line, Point::new(at.x, top + h * i as f64), spec, color);
    }
}

/// `s` broken into lines no wider than `width`, at spaces.
pub fn wrap(theme: &Theme, s: &str, width: f64, spec: FontSpec) -> Vec<String> {
    let mut lines: Vec<String> = Vec::new();
    let mut line = String::new();
    for word in s.split_whitespace() {
        let candidate = if line.is_empty() { word.to_owned() } else { format!("{line} {word}") };
        if !line.is_empty() && measure(theme, &candidate, spec).width > width {
            lines.push(std::mem::replace(&mut line, word.to_owned()));
        } else {
            line = candidate;
        }
    }
    if !line.is_empty() {
        lines.push(line);
    }
    lines
}

/// A rounded card on a hard shadow (Aizome's, and cheap: a blurred one at
/// twice the scale costs a frame's budget on its own).
pub fn card(c: &mut Canvas, r: Rect, radius: f64, fill: Color, edge: Color, look: &Look) {
    let shape = RoundedRect::from_rect(r, radius);
    c.fill(&RoundedRect::from_rect(r + Vec2::new(0.0, 6.0), radius), look.shadow);
    c.fill(&shape, fill);
    c.stroke(&shape, &StrokeStyle::new(2.0), edge);
}

/// How a big button looks.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ButtonStyle {
    pub face: Color,
    pub lip: Color,
    pub ink: Color,
}

impl ButtonStyle {
    pub fn accent(look: &Look) -> ButtonStyle {
        ButtonStyle { face: look.accent, lip: look.accent_dark, ink: Color::WHITE }
    }
    pub fn good(look: &Look) -> ButtonStyle {
        ButtonStyle { face: look.good, lip: look.good_dark, ink: Color::WHITE }
    }
    pub fn calm(look: &Look) -> ButtonStyle {
        ButtonStyle { face: look.calm, lip: look.calm_dark, ink: Color::WHITE }
    }
    pub fn plain(look: &Look) -> ButtonStyle {
        ButtonStyle { face: look.card, lip: look.card_edge, ink: look.ink }
    }
}

/// A big, squashy button: a face on a darker lip, pressed down into it
/// while `pressed`. Returns the face's rectangle, for what goes on it.
pub fn button(c: &mut Canvas, r: Rect, radius: f64, style: ButtonStyle, pressed: bool, look: &Look) -> Rect {
    let lip = 6.0;
    let sink = if pressed { lip - 1.5 } else { 0.0 };
    let base = RoundedRect::from_rect(Rect::new(r.x0, r.y0 + lip, r.x1, r.y1), radius);
    c.fill(&RoundedRect::from_rect(Rect::new(r.x0, r.y0 + lip + 4.0, r.x1, r.y1 + 4.0), radius), look.shadow);
    c.fill(&base, style.lip);
    let face = Rect::new(r.x0, r.y0 + sink, r.x1, r.y1 - lip + sink);
    let shape = RoundedRect::from_rect(face, radius);
    let shine = LinearGradient::new(
        Point::new(face.x0, face.y0),
        Point::new(face.x0, face.y1),
        style.face.mix(Color::WHITE, 0.18),
        style.face,
    );
    c.fill(&shape, shine);
    face
}

/// Whether `p` is in `r`, grown by `slop` (fingers are wider than
/// pointers).
pub fn hits(r: Rect, p: Point, slop: f64) -> bool {
    r.inflate(slop, slop).contains(p)
}

/// A shape's bounding box.
pub fn bounds(s: &impl Shape) -> Rect {
    s.bounding_box()
}
