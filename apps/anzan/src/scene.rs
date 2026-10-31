//! Anzan's screens as one canvas: what's drawn ([`Scene`], made fresh by
//! each view), where things are, what a press lands on ([`hit`]), and the
//! drawing ([`paint`]). Everything is laid out in the 1000 × 700 design of
//! `sekka_kid_kit::stage`, scaled to the window.

use std::f64::consts::PI;

use aizome_kit::layout::{Point, Rect, Size, Vec2};
use aizome_kit::sidestep_ui::kurbo::{Affine, BezPath, Circle, RoundedRect};
use aizome_kit::sidestep_ui::{Canvas, Color, LineCap, StrokeStyle};
use aizome_kit::theme::Theme;
use sekka_kid::Animal;
use sekka_kid_kit::avatar::{self, Mood};
use sekka_kid_kit::look::{self, ButtonStyle, Look, font};
use sekka_kid_kit::stage::{self, Fit};
use sekka_kid_kit::who::{self, Card, Who, WhoHit};

use crate::abacus::{Abacus, Bead};
use crate::art::{self, BeadLook, Frame};
use crate::lessons::{CHAPTERS, Kind, LESSONS, Question};
use crate::round::{Hint, words};

/// Which screen.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Screen {
    Who,
    NewKid,
    Map,
    Play,
    Summary,
    Free,
    Rest,
}

/// Something to press.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Hit {
    Bead(usize, Bead),
    Choice(u32),
    Check,
    Hint,
    Restart,
    /// On to Pon's next line, or the first question.
    Next,
    Home,
    Kid,
    Free,
    Sound,
    GrownUps,
    Stop(usize),
    Again,
    NextLesson,
    Clear,
    SwitchPlayer,
    Who(WhoHit),
}

/// A lesson's stop on the map.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Stop {
    pub stars: u8,
    pub done: bool,
    pub open: bool,
}

/// A bead's place: thousandths of the way from home to the beam.
pub type BeadPos = [u16; 5];

/// The beads as they show now (mid-slide included), and how they look.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Beads {
    pub abacus: Abacus,
    /// Each rod's beads, heaven first (see [`Bead::ALL`]).
    pub at: Vec<BeadPos>,
    /// The cheer's progress (thousandths), while the beads cheer.
    pub cheer: Option<u16>,
    /// Percent seen (ghost beads fade).
    pub ghost: u8,
    /// How far the curtain has closed (thousandths).
    pub curtain: u16,
    /// Rod numbers showing under the rods.
    pub numbers: bool,
    /// The rod the keyboard is on.
    pub rod: Option<usize>,
    pub blink: bool,
}

impl Beads {
    pub fn still(abacus: Abacus) -> Beads {
        let at = abacus.rods().iter().map(|r| Bead::ALL.map(|b| if r.counts(b) { 1000 } else { 0 })).collect();
        Beads { abacus, at, cheer: None, ghost: 100, curtain: 0, numbers: false, rod: None, blink: false }
    }

    fn pos(&self, rod: usize, bead: Bead) -> f64 {
        let i = Bead::ALL.iter().position(|b| *b == bead).unwrap_or(0);
        f64::from(self.at.get(rod).map_or(0, |p| p[i])) / 1000.0
    }
}

/// A round as drawn.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct PlayView {
    pub lesson: usize,
    pub results: Vec<bool>,
    pub count: usize,
    pub question: Question,
    pub stage: PlayStage,
    pub beads: Beads,
    pub hint: Option<Hint>,
    pub chosen: Option<(u32, bool)>,
    /// Thousandths of the peek left, while peeking.
    pub peek_left: u16,
    pub praise: &'static str,
    /// The hint's arrow, bobbing (thousandths of a bob), for a moment.
    pub pulse: u16,
}

/// Where the round is, as drawn.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum PlayStage {
    /// Pon's introduction: the line showing.
    Intro(usize),
    Peeking,
    /// A flashed number, or the blank after it.
    Flash(Option<u32>),
    Asking,
    Right,
    /// The beads showing how: the push being made.
    Showing(String),
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct SummaryView {
    pub lesson: usize,
    pub stars: u8,
    pub first_try: u32,
    pub items: u32,
    /// Stars shown so far, and the last one's pop (thousandths).
    pub shown: u8,
    pub pop: u16,
    pub confetti: Option<u16>,
    pub has_next: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Scene {
    pub screen: Screen,
    pub who: Who,
    pub kid: Option<Card>,
    pub stops: Vec<Stop>,
    pub focus: usize,
    pub here: usize,
    /// The kid's marker hopping onto its stop (thousandths).
    pub hop: Option<u16>,
    pub play: Option<PlayView>,
    pub summary: Option<SummaryView>,
    pub free: Option<Beads>,
    pub pressed: Option<Hit>,
    pub blink: bool,
    pub sound: bool,
    /// For the map's top bar: the chapter of the focused stop.
    pub chapter: usize,
}

// Where things are, in the design's points.

const HOME: (Point, f64) = (Point::new(58.0, 52.0), 32.0);
const SOUND: (Point, f64) = (Point::new(878.0, 52.0), 28.0);
const GROWN: (Point, f64) = (Point::new(946.0, 52.0), 28.0);
const FREE: (Point, f64) = (Point::new(806.0, 52.0), 28.0);
const KID: Rect = Rect::new(24.0, 22.0, 250.0, 82.0);
const PON: (Point, f64) = (Point::new(126.0, 196.0), 56.0);
const BUBBLE: Rect = Rect::new(222.0, 116.0, 872.0, 268.0);
pub const ABACUS_X: f64 = 400.0;
pub const ABACUS_TOP: f64 = 288.0;
const HINT: (Point, f64) = (Point::new(126.0, 566.0), 44.0);
const RESTART: (Point, f64) = (Point::new(126.0, 452.0), 38.0);
const CHECK: Rect = Rect::new(690.0, 528.0, 900.0, 628.0);

fn choice_rect(i: usize) -> Rect {
    Rect::new(700.0, 300.0 + i as f64 * 112.0, 890.0, 392.0 + i as f64 * 112.0)
}

/// Where lesson `i`'s lantern hangs: four rows, snaking.
pub fn stop_at(i: usize) -> Point {
    let row = i / 4;
    let col = if row.is_multiple_of(2) { i % 4 } else { 3 - i % 4 };
    Point::new(200.0 + col as f64 * 200.0, 214.0 + row as f64 * 126.0)
}

const SUMMARY: Rect = Rect::new(250.0, 116.0, 750.0, 640.0);
const AGAIN: Rect = Rect::new(290.0, 520.0, 490.0, 606.0);
const NEXT_LESSON: Rect = Rect::new(510.0, 520.0, 710.0, 606.0);
const CLEAR: Rect = Rect::new(700.0, 528.0, 890.0, 618.0);
const SWITCH: Rect = Rect::new(380.0, 590.0, 620.0, 660.0);

fn round_hit(c: (Point, f64), p: Point) -> bool {
    (p - c.0).hypot() <= c.1 + 8.0
}

/// What a press at `p` (the canvas's points) is on.
pub fn hit(size: Size, s: &Scene, p: Point) -> Option<Hit> {
    let d = Fit::of_size(size).to_design(p);
    match s.screen {
        Screen::Who => who::hit(size, &s.who, p).map(Hit::Who),
        Screen::NewKid => None,
        Screen::Map => {
            if round_hit(SOUND, d) {
                return Some(Hit::Sound);
            }
            if round_hit(GROWN, d) {
                return Some(Hit::GrownUps);
            }
            if round_hit(FREE, d) {
                return Some(Hit::Free);
            }
            if look::hits(KID, d, 4.0) {
                return Some(Hit::Kid);
            }
            (0..s.stops.len()).find(|i| (d - stop_at(*i)).hypot() < 56.0).map(Hit::Stop)
        }
        Screen::Play => {
            if round_hit(HOME, d) {
                return Some(Hit::Home);
            }
            if round_hit(SOUND, d) {
                return Some(Hit::Sound);
            }
            if round_hit(GROWN, d) {
                return Some(Hit::GrownUps);
            }
            let play = s.play.as_ref()?;
            match &play.stage {
                PlayStage::Intro(_) => (look::hits(CHECK, d, 6.0) || BUBBLE.contains(d)).then_some(Hit::Next),
                PlayStage::Asking => {
                    if let Some(c) = play.question.choices() {
                        if let Some(i) = (0..3).find(|i| look::hits(choice_rect(*i), d, 6.0)) {
                            return Some(Hit::Choice(c[i]));
                        }
                    } else if look::hits(CHECK, d, 6.0) {
                        return Some(Hit::Check);
                    }
                    if round_hit(HINT, d) {
                        return Some(Hit::Hint);
                    }
                    if matches!(play.question, Question::Sum { .. }) && round_hit(RESTART, d) {
                        return Some(Hit::Restart);
                    }
                    let f = Frame::new(play.beads.abacus.len(), ABACUS_X, ABACUS_TOP);
                    f.bead_at(d, |r, b| play.beads.pos(r, b)).map(|(r, b)| Hit::Bead(r, b))
                }
                _ => None,
            }
        }
        Screen::Summary => {
            if round_hit(HOME, d) {
                return Some(Hit::Home);
            }
            if look::hits(AGAIN, d, 6.0) {
                return Some(Hit::Again);
            }
            look::hits(NEXT_LESSON, d, 6.0).then_some(Hit::NextLesson)
        }
        Screen::Free => {
            if round_hit(HOME, d) {
                return Some(Hit::Home);
            }
            if round_hit(SOUND, d) {
                return Some(Hit::Sound);
            }
            if look::hits(CLEAR, d, 6.0) {
                return Some(Hit::Clear);
            }
            let beads = s.free.as_ref()?;
            let f = Frame::new(beads.abacus.len(), ABACUS_X, ABACUS_TOP);
            f.bead_at(d, |r, b| beads.pos(r, b)).map(|(r, b)| Hit::Bead(r, b))
        }
        Screen::Rest => {
            if round_hit(GROWN, d) {
                return Some(Hit::GrownUps);
            }
            look::hits(SWITCH, d, 6.0).then_some(Hit::SwitchPlayer)
        }
    }
}

/// Whether a press at `p` (the canvas's points), released at `q`, dragged
/// a bead: up (true) or down (false) by more than a little.
pub fn drag_direction(size: Size, p: Point, q: Point) -> Option<bool> {
    let fit = Fit::of_size(size);
    let dy = (q.y - p.y) / fit.scale;
    (dy.abs() > 14.0).then_some(dy < 0.0)
}

// Drawing.

pub fn paint(c: &mut Canvas, b: Rect, s: &Scene, theme: &Theme) {
    if s.screen == Screen::Who {
        who::paint(c, b, &s.who, theme);
        return;
    }
    let look = Look::for_theme(theme);
    let fit = Fit::new(b);
    let hills = match s.screen {
        Screen::Map => 0.0,
        Screen::Rest => 170.0,
        _ => 110.0,
    };
    stage::backdrop(c, b, &fit, &look, hills);
    c.saved(|c| {
        fit.apply(c);
        match s.screen {
            Screen::Map => map(c, s, theme, &look),
            Screen::Play => play(c, s, theme, &look),
            Screen::Summary => summary(c, s, theme, &look),
            Screen::Free => free(c, s, theme, &look),
            Screen::Rest => rest(c, s, theme, &look),
            Screen::Who | Screen::NewKid => {}
        }
    });
}

fn pressed(s: &Scene, h: Hit) -> bool {
    s.pressed == Some(h)
}

/// A round button with an icon.
fn round_button(
    c: &mut Canvas,
    at: (Point, f64),
    style: ButtonStyle,
    down: bool,
    look: &Look,
    icon: impl FnOnce(&mut Canvas, Point),
) {
    let (p, r) = at;
    let face = look::button(c, Rect::new(p.x - r, p.y - r, p.x + r, p.y + r + 5.0), r, style, down, look);
    icon(c, face.center());
}

fn top_buttons(c: &mut Canvas, s: &Scene, look: &Look, home: bool, grown: bool) {
    if home {
        round_button(c, HOME, ButtonStyle::plain(look), pressed(s, Hit::Home), look, |c, at| {
            art::glyph(c, "house", at, 30.0, look.ink)
        });
    }
    round_button(c, SOUND, ButtonStyle::plain(look), pressed(s, Hit::Sound), look, |c, at| {
        art::glyph(c, if s.sound { "volume-2" } else { "volume-x" }, at, 26.0, look.ink)
    });
    if grown {
        round_button(c, GROWN, ButtonStyle::plain(look), pressed(s, Hit::GrownUps), look, |c, at| {
            art::glyph(c, "lock", at, 24.0, look.ink_soft)
        });
    }
}

/// Pon, the tanuki who asks the questions, with his leaf.
fn pon(c: &mut Canvas, at: Point, r: f64, mood: Mood, bob: f64) {
    let at = at + Vec2::new(0.0, -bob);
    avatar::paint(c, at, r, Animal::Tanuki, None, mood);
    // The leaf a tanuki wears to change shape.
    let base = Point::new(at.x + r * 0.08, at.y - r * 0.86);
    c.saved(|c| {
        c.concat(Affine::rotate_about(-0.5, base));
        let mut leaf = BezPath::new();
        leaf.move_to(base);
        leaf.quad_to(base + Vec2::new(-r * 0.32, -r * 0.3), base + Vec2::new(0.0, -r * 0.6));
        leaf.quad_to(base + Vec2::new(r * 0.32, -r * 0.3), base);
        c.fill(&leaf, Color::hex(0x5fae4e));
        c.stroke(
            &aizome_kit::sidestep_ui::kurbo::Line::new(base, base + Vec2::new(0.0, -r * 0.5)),
            &StrokeStyle::new(2.0),
            Color::hex(0x3d7f31),
        );
    });
}

fn kid_chip(c: &mut Canvas, s: &Scene, theme: &Theme, look: &Look) {
    let Some(kid) = &s.kid else { return };
    let down = pressed(s, Hit::Kid);
    let r = if down { KID.inflate(-2.0, -2.0) } else { KID };
    look::card(c, r, 30.0, look.card, look.card_edge, look);
    let mood = if s.blink { Mood::Blink } else { Mood::Happy };
    avatar::paint(c, Point::new(r.x0 + 32.0, r.center().y + 2.0), 19.0, kid.animal, Some(kid.color), mood);
    look::text(c, theme, &kid.name, Point::new(r.x0 + 66.0, r.center().y - 13.0), font(22.0, 800.0), look.ink);
}

fn map(c: &mut Canvas, s: &Scene, theme: &Theme, look: &Look) {
    kid_chip(c, s, theme, look);
    look::text_centred(c, theme, "Anzan", Point::new(stage::W / 2.0, 42.0), font(40.0, 900.0), look.ink);
    look::text_centred(
        c,
        theme,
        CHAPTERS[s.chapter].title,
        Point::new(stage::W / 2.0, 80.0),
        font(18.0, 700.0),
        look.ink_soft,
    );
    round_button(c, FREE, ButtonStyle::plain(look), pressed(s, Hit::Free), look, |c, at| mini_abacus(c, at, look));
    top_buttons(c, s, look, false, true);

    // The path.
    let mut path = BezPath::new();
    for i in 0..s.stops.len() {
        let p = stop_at(i);
        if i == 0 {
            path.move_to(p - Vec2::new(70.0, 0.0));
            path.line_to(p);
            continue;
        }
        let q = stop_at(i - 1);
        if (q.y - p.y).abs() < 1.0 {
            path.line_to(p);
        } else {
            let out = if q.x > 500.0 { 120.0 } else { -120.0 };
            path.curve_to(q + Vec2::new(out, 0.0), p + Vec2::new(out, 0.0), p);
        }
    }
    let road = if look.night { Color::hex(0x343b84) } else { Color::hex(0xf1d6a2) };
    c.stroke(&path, &StrokeStyle::new(30.0).with_cap(LineCap::Round), road);
    c.stroke(
        &path,
        &StrokeStyle::new(4.0).with_cap(LineCap::Round).with_dash(vec![2.0, 16.0], 0.0),
        look.ink_soft.with_alpha(0.45),
    );

    for (i, stop) in s.stops.iter().enumerate() {
        let at = stop_at(i);
        let down = pressed(s, Hit::Stop(i));
        let size = if down { 28.0 } else { 30.0 };
        let lantern = match (stop.done, stop.open) {
            (true, _) => art::Lantern::Lit,
            (false, true) => art::Lantern::Waiting,
            _ => art::Lantern::Dark,
        };
        if s.focus == i {
            c.stroke(&Circle::new(at, 52.0), &StrokeStyle::new(4.0).with_dash(vec![8.0, 6.0], 0.0), look.accent);
        }
        art::lantern(c, theme, at, size, i + 1, lantern, look);
        if stop.done {
            for k in 0..3u8 {
                let x = at.x - 22.0 + f64::from(k) * 22.0;
                art::star(c, Point::new(x, at.y + 46.0), 9.0, k < stop.stars, look);
            }
        }
        let title_ink = if stop.open { look.ink } else { look.ink_soft.with_alpha(0.7) };
        look::text_centred(c, theme, LESSONS[i].title, Point::new(at.x, at.y + 68.0), font(15.0, 700.0), title_ink);
    }

    // The kid, by where they're up to.
    if let Some(kid) = &s.kid {
        let hop = s.hop.map_or(0.0, |h| (f64::from(h) / 1000.0 * PI).sin() * 26.0);
        let at = stop_at(s.here) + Vec2::new(-48.0, -12.0 - hop);
        avatar::paint(c, at, 20.0, kid.animal, Some(kid.color), if s.blink { Mood::Blink } else { Mood::Happy });
    }
}

/// A tiny soroban, for the free play button.
fn mini_abacus(c: &mut Canvas, at: Point, look: &Look) {
    let r = Rect::from_center_size(at, Size::new(34.0, 26.0));
    c.fill(&RoundedRect::from_rect(r, 4.0), look.wood);
    c.fill_rect(Rect::new(r.x0 + 3.0, r.y0 + 9.0, r.x1 - 3.0, r.y0 + 11.0), look.beam);
    for (k, x) in [r.x0 + 9.0, r.center().x, r.x1 - 9.0].into_iter().enumerate() {
        c.fill(
            &RoundedRect::from_rect(
                Rect::from_center_size(Point::new(x, r.y0 + 6.0 + if k == 1 { 3.0 } else { 0.0 }), Size::new(8.0, 4.5)),
                2.0,
            ),
            look.heaven_bead,
        );
        for j in 0..2 {
            let y = r.y0 + 15.0 + f64::from(j) * 5.0;
            c.fill(
                &RoundedRect::from_rect(Rect::from_center_size(Point::new(x, y), Size::new(8.0, 4.5)), 2.0),
                look.earth_bead,
            );
        }
    }
}

/// The soroban as `beads` says.
fn soroban(c: &mut Canvas, beads: &Beads, hint: Option<&Hint>, pulse: f64, theme: &Theme, look: &Look) {
    let f = Frame::new(beads.abacus.len(), ABACUS_X, ABACUS_TOP);
    let draw = |c: &mut Canvas| {
        art::soroban_frame(c, &f, look);
        for rod in 0..f.rods {
            let r = beads.abacus.rod(rod);
            for (k, bead) in Bead::ALL.iter().enumerate() {
                let pos = beads.pos(rod, *bead);
                let mut rect = f.bead_rect(rod, *bead, pos);
                let awake = r.counts(*bead) && pos > 0.5;
                if let Some(t) = beads.cheer.filter(|_| awake) {
                    let t = f64::from(t) / 1000.0 - k as f64 * 0.06;
                    let hop = if (0.0..0.5).contains(&t) { (t / 0.5 * PI).sin() * 10.0 } else { 0.0 };
                    rect = rect + Vec2::new(0.0, -hop);
                }
                let glow =
                    matches!(hint, Some(Hint::Push { rod: hr, beads: bs, .. }) if *hr == rod && bs.contains(bead));
                art::bead(
                    c,
                    rect,
                    art::bead_color(*bead, look),
                    BeadLook { awake, blink: beads.blink && awake, cheer: beads.cheer.is_some() && awake, glow },
                    look,
                );
            }
        }
        if let Some(Hint::Push { rod, beads: bs, to_beam, .. }) = hint
            && let Some(first) = bs.first()
        {
            let up = match first {
                Bead::Heaven => !*to_beam,
                Bead::Earth(_) => *to_beam,
            };
            let at = f.bead_center(*rod, *first, beads.pos(*rod, *first));
            art::push_arrow(c, at, up, look, pulse);
        }
        if let Some(rod) = beads.rod {
            let x = f.rod_x(rod);
            c.stroke(
                &RoundedRect::from_rect(
                    Rect::new(
                        x - art::ROD_GAP / 2.0 + 4.0,
                        f.inner.y0 - 6.0,
                        x + art::ROD_GAP / 2.0 - 4.0,
                        f.inner.y1 + 6.0,
                    ),
                    12.0,
                ),
                &StrokeStyle::new(3.0).with_dash(vec![8.0, 6.0], 0.0),
                look.gold,
            );
        }
    };
    if beads.ghost >= 100 {
        draw(c);
    } else {
        c.group(f32::from(beads.ghost) / 100.0, draw);
    }
    if beads.numbers {
        for rod in 0..f.rods {
            let x = f.rod_x(rod);
            let chip = Rect::from_center_size(Point::new(x, f.outer.y1 + 22.0), Size::new(46.0, 34.0));
            c.fill(&RoundedRect::from_rect(chip, 17.0), look.card);
            c.stroke(&RoundedRect::from_rect(chip, 17.0), &StrokeStyle::new(2.0), look.card_edge);
            let d = beads.abacus.rod(rod).value();
            look::text_centred(c, theme, &d.to_string(), chip.center(), font(24.0, 900.0), look.ink);
        }
    }
    art::curtain(c, f.outer.inflate(6.0, 6.0), f64::from(beads.curtain) / 1000.0, theme, look);
}

/// The question in Pon's bubble: the big line, and what it says in words.
fn ask(q: &Question, kind: Kind) -> (String, String, Option<u32>) {
    match q {
        Question::Set { target, from } => {
            let say = if *from == 0 { format!("Make {target}") } else { format!("Change it to {target}") };
            (target.to_string(), say, (*target <= 10).then_some(*target))
        }
        Question::Read { .. } => ("?".into(), "What number is this?".into(), None),
        Question::PeekRead { .. } => ("?".into(), "What did you see?".into(), None),
        Question::PeekSet { .. } => ("?".into(), "Make it again!".into(), None),
        Question::Sum { start, op, n } => {
            let say = if kind == Kind::Take { "Take away with the beads" } else { "Add with the beads" };
            (format!("{start} {} {n}", op.sign()), say.into(), None)
        }
        Question::Ghost { start, op, n, .. } => {
            (format!("{start} {} {n} = ?", op.sign()), "In your head!".into(), None)
        }
        Question::Flash { .. } => ("?".into(), "How many altogether?".into(), None),
    }
}

fn play(c: &mut Canvas, s: &Scene, theme: &Theme, look: &Look) {
    let Some(p) = &s.play else { return };
    top_buttons(c, s, look, true, true);
    let lesson = &LESSONS[p.lesson];
    look::text_centred(c, theme, lesson.title, Point::new(stage::W / 2.0, 38.0), font(30.0, 900.0), look.ink);
    // Progress: a dot for each question, a star for each right first time.
    let n = p.count.max(1);
    for i in 0..n {
        let at = Point::new(stage::W / 2.0 + (i as f64 - (n as f64 - 1.0) / 2.0) * 28.0, 78.0);
        match p.results.get(i) {
            Some(true) => art::star(c, at, 10.0, true, look),
            Some(false) => {
                c.fill(&Circle::new(at, 7.0), look.calm);
            }
            None if i == p.results.len() && !matches!(p.stage, PlayStage::Intro(_)) => {
                c.stroke(&Circle::new(at, 8.0), &StrokeStyle::new(3.0), look.accent);
            }
            None => {
                c.fill(&Circle::new(at, 6.0), look.ink_soft.with_alpha(0.3));
            }
        }
    }

    let cheering = matches!(p.stage, PlayStage::Right);
    let bob = p.beads.cheer.map_or(0.0, |t| (f64::from(t) / 1000.0 * PI * 2.0).sin().abs() * 10.0);
    let mood = if cheering {
        Mood::Cheer
    } else if s.blink {
        Mood::Blink
    } else {
        Mood::Happy
    };
    pon(c, PON.0, PON.1, mood, bob);
    art::bubble(c, BUBBLE, PON.0 + Vec2::new(PON.1 * 0.9, 0.0), look);

    let mid = Point::new(BUBBLE.center().x, BUBBLE.center().y - 14.0);
    let below = Point::new(BUBBLE.center().x, BUBBLE.y1 - 30.0);
    match &p.stage {
        PlayStage::Intro(i) => {
            let line = lesson.intro.get(*i).copied().unwrap_or("");
            look::paragraph_centred(
                c,
                theme,
                line,
                BUBBLE.center(),
                BUBBLE.width() - 70.0,
                font(32.0, 800.0),
                look.ink,
            );
        }
        PlayStage::Peeking => {
            look::text_centred(c, theme, "Peek!", mid, font(64.0, 900.0), look.accent);
            look::text_centred(c, theme, "Remember the beads", below, font(22.0, 700.0), look.ink_soft);
            let bar = Rect::new(BUBBLE.x0 + 60.0, BUBBLE.y1 - 12.0, BUBBLE.x1 - 60.0, BUBBLE.y1 - 6.0);
            let left = Rect::new(bar.x0, bar.y0, bar.x0 + bar.width() * f64::from(p.peek_left) / 1000.0, bar.y1);
            c.fill(&RoundedRect::from_rect(bar, 3.0), look.card_edge);
            c.fill(&RoundedRect::from_rect(left, 3.0), look.accent);
        }
        PlayStage::Flash(n) => {
            if let Some(n) = n {
                look::text_centred(c, theme, &n.to_string(), BUBBLE.center(), font(120.0, 900.0), look.accent);
            }
        }
        PlayStage::Right => {
            let answer = p.question.answer();
            look::text_centred(c, theme, &format!("{} {answer}", p.praise), mid, font(60.0, 900.0), look.good);
            look::text_centred(c, theme, &words(answer), below, font(24.0, 700.0), look.ink_soft);
        }
        PlayStage::Showing(step) => {
            look::text_centred(c, theme, "Watch!", mid, font(54.0, 900.0), look.calm);
            look::text_centred(c, theme, step, below, font(24.0, 700.0), look.ink_soft);
        }
        PlayStage::Asking => {
            let (big, say, dots) = ask(&p.question, lesson.kind);
            let hint_words = match &p.hint {
                Some(Hint::Push { words, .. }) => Some(words.clone()),
                Some(Hint::Again) => Some("Let's start again".to_owned()),
                Some(Hint::Look) => Some("Peek at the beads!".to_owned()),
                _ => None,
            };
            match dots {
                Some(n) => {
                    look::text_centred(
                        c,
                        theme,
                        &big,
                        Point::new(mid.x - 110.0, mid.y),
                        font(96.0, 900.0),
                        look.accent,
                    );
                    art::ten_frame(c, Point::new(mid.x - 10.0, mid.y - 36.0), 30.0, n, look);
                }
                None => look::text_centred(c, theme, &big, mid, font(84.0, 900.0), look.accent),
            }
            match hint_words {
                Some(w) => {
                    art::bulb(
                        c,
                        Point::new(below.x - look::measure(theme, &w, font(24.0, 800.0)).width / 2.0 - 22.0, below.y),
                        26.0,
                        look.calm,
                    );
                    look::text_centred(c, theme, &w, below, font(24.0, 800.0), look.calm);
                }
                None => look::text_centred(c, theme, &say, below, font(24.0, 700.0), look.ink_soft),
            }
        }
    }

    // The beads.
    let hint = if matches!(p.stage, PlayStage::Asking) { p.hint.as_ref() } else { None };
    let pulse = (f64::from(p.pulse) / 1000.0 * PI * 2.0).sin();
    soroban(c, &p.beads, hint, pulse, theme, look);

    // Answers and buttons.
    match &p.stage {
        PlayStage::Intro(i) => {
            let last = *i + 1 >= lesson.intro.len();
            let face = look::button(c, CHECK, 26.0, ButtonStyle::accent(look), pressed(s, Hit::Next), look);
            look::text_centred(
                c,
                theme,
                if last { "Let's go!" } else { "Next" },
                face.center(),
                font(30.0, 900.0),
                Color::WHITE,
            );
        }
        PlayStage::Asking | PlayStage::Right | PlayStage::Showing(_) => {
            if let Some(choices) = p.question.choices() {
                for (i, v) in choices.iter().enumerate() {
                    let struck = p.hint == Some(Hint::Strike(*v));
                    let style = match p.chosen {
                        Some((c, true)) if c == *v => ButtonStyle::good(look),
                        Some((c, false)) if c == *v => ButtonStyle::calm(look),
                        _ => ButtonStyle::plain(look),
                    };
                    let r = choice_rect(i);
                    let face = c.saved(|c| {
                        if struck {
                            c.group(0.35, |c| {
                                look::button(c, r, 28.0, style, false, look);
                            });
                            r
                        } else {
                            look::button(c, r, 28.0, style, pressed(s, Hit::Choice(*v)), look)
                        }
                    });
                    let ink = if struck { look.ink_soft.with_alpha(0.4) } else { style.ink };
                    look::text_centred(
                        c,
                        theme,
                        &v.to_string(),
                        Point::new(face.center().x, face.center().y - 2.0),
                        font(54.0, 900.0),
                        ink,
                    );
                }
            } else if matches!(p.stage, PlayStage::Asking) {
                let face = look::button(c, CHECK, 26.0, ButtonStyle::good(look), pressed(s, Hit::Check), look);
                art::glyph(c, "check", Point::new(face.center().x - 46.0, face.center().y), 36.0, Color::WHITE);
                look::text_centred(
                    c,
                    theme,
                    "Done",
                    Point::new(face.center().x + 18.0, face.center().y),
                    font(32.0, 900.0),
                    Color::WHITE,
                );
            }
            if matches!(p.stage, PlayStage::Asking) {
                round_button(c, HINT, ButtonStyle::calm(look), pressed(s, Hit::Hint), look, |c, at| {
                    art::bulb(c, at, 40.0, Color::WHITE)
                });
                if matches!(p.question, Question::Sum { .. }) {
                    round_button(c, RESTART, ButtonStyle::plain(look), pressed(s, Hit::Restart), look, |c, at| {
                        art::glyph(c, "rotate-ccw", at, 30.0, look.ink)
                    });
                }
            }
        }
        PlayStage::Peeking | PlayStage::Flash(_) => {}
    }
}

fn summary(c: &mut Canvas, s: &Scene, theme: &Theme, look: &Look) {
    let Some(v) = &s.summary else { return };
    top_buttons(c, s, look, true, false);
    look::card(c, SUMMARY, 36.0, look.card, look.card_edge, look);
    art::lantern(c, theme, Point::new(stage::W / 2.0, 210.0), 46.0, v.lesson + 1, art::Lantern::Lit, look);
    let title = match v.stars {
        3 => "Hooray!",
        2 => "Well done!",
        _ => "You did it!",
    };
    look::text_centred(c, theme, title, Point::new(stage::W / 2.0, 306.0), font(56.0, 900.0), look.ink);
    for k in 0..3u8 {
        let at = Point::new(stage::W / 2.0 + (f64::from(k) - 1.0) * 100.0, 388.0);
        let won = k < v.stars;
        let shown = k < v.shown;
        let r = if won && shown && k + 1 == v.shown {
            let t = f64::from(v.pop) / 1000.0;
            40.0 * (1.0 + 0.35 * (t * PI).sin())
        } else {
            40.0
        };
        art::star(c, at, r, won && shown, look);
    }
    look::text_centred(
        c,
        theme,
        &format!("{} of {} right the first time", v.first_try, v.items),
        Point::new(stage::W / 2.0, 462.0),
        font(22.0, 700.0),
        look.ink_soft,
    );
    let face = look::button(c, AGAIN, 26.0, ButtonStyle::calm(look), pressed(s, Hit::Again), look);
    art::glyph(c, "rotate-ccw", Point::new(face.x0 + 40.0, face.center().y), 28.0, Color::WHITE);
    look::text_centred(
        c,
        theme,
        "Again",
        Point::new(face.center().x + 16.0, face.center().y),
        font(28.0, 900.0),
        Color::WHITE,
    );
    let face = look::button(c, NEXT_LESSON, 26.0, ButtonStyle::accent(look), pressed(s, Hit::NextLesson), look);
    let next = if v.has_next { "Next" } else { "Map" };
    look::text_centred(
        c,
        theme,
        next,
        Point::new(face.center().x - 16.0, face.center().y),
        font(28.0, 900.0),
        Color::WHITE,
    );
    art::glyph(c, "arrow-right", Point::new(face.x1 - 40.0, face.center().y), 28.0, Color::WHITE);
    if let Some(t) = v.confetti {
        art::confetti(c, Rect::new(0.0, 0.0, stage::W, stage::H), f64::from(t) / 1000.0, look);
    }
}

fn free(c: &mut Canvas, s: &Scene, theme: &Theme, look: &Look) {
    let Some(beads) = &s.free else { return };
    top_buttons(c, s, look, true, false);
    look::text_centred(c, theme, "Play with the beads", Point::new(stage::W / 2.0, 42.0), font(30.0, 900.0), look.ink);
    let value = beads.abacus.value();
    pon(c, PON.0, PON.1, if s.blink { Mood::Blink } else { Mood::Happy }, 0.0);
    art::bubble(c, BUBBLE, PON.0 + Vec2::new(PON.1 * 0.9, 0.0), look);
    look::text_centred(
        c,
        theme,
        &value.to_string(),
        Point::new(BUBBLE.center().x, BUBBLE.center().y - 16.0),
        font(88.0, 900.0),
        look.accent,
    );
    look::text_centred(
        c,
        theme,
        &words(value),
        Point::new(BUBBLE.center().x, BUBBLE.y1 - 30.0),
        font(24.0, 700.0),
        look.ink_soft,
    );
    soroban(c, beads, None, 0.0, theme, look);
    let face = look::button(c, CLEAR, 26.0, ButtonStyle::plain(look), pressed(s, Hit::Clear), look);
    look::text_centred(c, theme, "Clear", face.center(), font(30.0, 900.0), look.ink);
}

fn rest(c: &mut Canvas, s: &Scene, theme: &Theme, look: &Look) {
    top_buttons(c, s, look, false, true);
    pon(c, Point::new(stage::W / 2.0, 260.0), 100.0, Mood::Sleep, 0.0);
    // Zzz.
    for (i, (dx, dy, size)) in [(110.0, -90.0, 26.0), (140.0, -130.0, 34.0), (176.0, -176.0, 42.0)].iter().enumerate() {
        let _ = i;
        look::text_centred(
            c,
            theme,
            "z",
            Point::new(stage::W / 2.0 + dx, 260.0 + dy),
            font(*size, 900.0),
            look.ink_soft,
        );
    }
    look::text_centred(c, theme, "All done for today!", Point::new(stage::W / 2.0, 440.0), font(50.0, 900.0), look.ink);
    let name = s.kid.as_ref().map(|k| k.name.as_str()).unwrap_or("friend");
    look::text_centred(
        c,
        theme,
        &format!("The beads are sleepy. See you tomorrow, {name}!"),
        Point::new(stage::W / 2.0, 500.0),
        font(24.0, 700.0),
        look.ink_soft,
    );
    let face = look::button(c, SWITCH, 26.0, ButtonStyle::plain(look), pressed(s, Hit::SwitchPlayer), look);
    look::text_centred(c, theme, "Switch player", face.center(), font(24.0, 800.0), look.ink);
}

/// The soroban of `rods` rods, where play and free play put it.
pub fn frame(rods: usize) -> Frame {
    Frame::new(rods, ABACUS_X, ABACUS_TOP)
}

/// Where the choice `i` (0 to 2) is, in the design's points.
pub fn choice_center(i: usize) -> Point {
    choice_rect(i).center()
}

/// Where the Done button is, in the design's points.
pub fn check_center() -> Point {
    CHECK.center()
}
