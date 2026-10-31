//! "Who's playing?": a card for each kid, and one to make a new player.

use aizome_kit::layout::{Point, Rect, Size};
use aizome_kit::sidestep_ui::kurbo::RoundedRect;
use aizome_kit::sidestep_ui::{Canvas, Color, StrokeStyle};
use aizome_kit::theme::Theme;
use sekka_kid::{Animal, Grade, KidColor, Profile};

use crate::avatar::{self, Mood, kid_color};
use crate::look::{self, Look, font};
use crate::stage::{self, Fit};

/// A kid's card.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Card {
    pub name: String,
    pub animal: Animal,
    pub color: KidColor,
    pub grade: Grade,
}

impl From<&Profile> for Card {
    fn from(p: &Profile) -> Card {
        Card { name: p.name.clone(), animal: p.animal, color: p.color, grade: p.grade }
    }
}

/// The screen.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct Who {
    pub cards: Vec<Card>,
    /// The card the keyboard is on (the last is New).
    pub focus: usize,
    /// The card under a press.
    pub pressed: Option<usize>,
    /// Whether the animals are blinking.
    pub blink: bool,
}

/// What a press landed on.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum WhoHit {
    Kid(usize),
    New,
}

const CARD: Size = Size::new(190.0, 230.0);
const GAP: f64 = 28.0;
/// Cards to a row.
const ROW: usize = 4;

/// Each card's rectangle in the design (kids', then New).
pub fn cards(n: usize) -> Vec<Rect> {
    let total = n + 1;
    let rows = total.div_ceil(ROW);
    let top = if rows > 1 { 176.0 } else { 236.0 };
    (0..total)
        .map(|i| {
            let row = i / ROW;
            let in_row = (total - row * ROW).min(ROW);
            let col = i % ROW;
            let width = in_row as f64 * CARD.width + (in_row - 1) as f64 * GAP;
            let x = (stage::W - width) / 2.0 + col as f64 * (CARD.width + GAP);
            let y = top + row as f64 * (CARD.height + GAP);
            Rect::from_origin_size(Point::new(x, y), CARD)
        })
        .collect()
}

/// What's at `p` (the canvas's points).
pub fn hit(size: Size, who: &Who, p: Point) -> Option<WhoHit> {
    let d = Fit::of_size(size).to_design(p);
    let n = who.cards.len();
    cards(n).iter().position(|r| look::hits(*r, d, 8.0)).map(|i| if i < n { WhoHit::Kid(i) } else { WhoHit::New })
}

pub fn paint(c: &mut Canvas, b: Rect, who: &Who, theme: &Theme) {
    let look = Look::for_theme(theme);
    let fit = Fit::new(b);
    stage::backdrop(c, b, &fit, &look, 150.0);
    c.saved(|c| {
        fit.apply(c);
        look::text_centred(c, theme, "Who's playing?", Point::new(stage::W / 2.0, 110.0), font(54.0, 800.0), look.ink);
        let rects = cards(who.cards.len());
        for (i, r) in rects.iter().enumerate() {
            let pressed = who.pressed == Some(i);
            let r = if pressed { r.inflate(-4.0, -4.0) } else { *r };
            match who.cards.get(i) {
                Some(card) => kid_card(c, theme, &look, r, card, who.focus == i, who.blink),
                None => new_card(c, theme, &look, r, who.focus == i),
            }
        }
    });
}

fn kid_card(c: &mut Canvas, theme: &Theme, look: &Look, r: Rect, card: &Card, focused: bool, blink: bool) {
    let col = kid_color(card.color);
    look::card(c, r, 26.0, look.card, if focused { col } else { look.card_edge }, look);
    if focused {
        c.stroke(&RoundedRect::from_rect(r.inflate(5.0, 5.0), 30.0), &StrokeStyle::new(4.0), col);
    }
    let mood = if blink { Mood::Blink } else { Mood::Happy };
    avatar::paint(c, Point::new(r.center().x, r.y0 + 92.0), 52.0, card.animal, Some(card.color), mood);
    let size = fitted(theme, &card.name, r.width() - 24.0, 30.0);
    look::text_centred(c, theme, &card.name, Point::new(r.center().x, r.y0 + 184.0), font(size, 800.0), look.ink);
    look::text_centred(
        c,
        theme,
        card.grade.name(),
        Point::new(r.center().x, r.y0 + 212.0),
        font(16.0, 600.0),
        look.ink_soft,
    );
}

fn new_card(c: &mut Canvas, theme: &Theme, look: &Look, r: Rect, focused: bool) {
    let shape = RoundedRect::from_rect(r, 26.0);
    c.fill(&shape, look.card.with_alpha(0.55));
    let edge = if focused { look.accent } else { look.ink_soft };
    c.stroke(&shape, &StrokeStyle::new(3.0).with_dash(vec![10.0, 8.0], 0.0), edge);
    let at = Point::new(r.center().x, r.y0 + 92.0);
    c.fill(&aizome_kit::sidestep_ui::kurbo::Circle::new(at, 46.0), look.accent);
    let plus = StrokeStyle::new(9.0).with_cap(aizome_kit::sidestep_ui::LineCap::Round);
    c.stroke(&aizome_kit::sidestep_ui::kurbo::Line::new((at.x - 20.0, at.y), (at.x + 20.0, at.y)), &plus, Color::WHITE);
    c.stroke(&aizome_kit::sidestep_ui::kurbo::Line::new((at.x, at.y - 20.0), (at.x, at.y + 20.0)), &plus, Color::WHITE);
    look::text_centred(c, theme, "New player", Point::new(r.center().x, r.y0 + 184.0), font(26.0, 800.0), look.ink);
}

/// The largest size up to `max` at which `s` fits `width`.
fn fitted(theme: &Theme, s: &str, width: f64, max: f64) -> f64 {
    let w = look::measure(theme, s, font(max, 800.0)).width;
    if w <= width { max } else { (max * width / w).max(14.0) }
}
