//! Ondoku's screens as one canvas: what's drawn ([`Scene`], made fresh by
//! each view), where things are, what a press lands on ([`hit`]), and the
//! drawing ([`paint`]), in the 1000 × 700 design of `sekka_kid_kit::stage`.
//! A page's words are laid out here ([`lay_out`]), so each can be lit as
//! it's read and pressed to be heard.

use std::hash::{Hash, Hasher};

use aizome_kit::layout::{Point, Rect, Size, Vec2};
use aizome_kit::sidestep_ui::kurbo::RoundedRect;
use aizome_kit::sidestep_ui::{Canvas, Color, StrokeStyle};
use aizome_kit::theme::Theme;
use sekka_kid_kit::art::{bubble, confetti, glyph, star};
use sekka_kid_kit::avatar::{self, Mood};
use sekka_kid_kit::look::{self, ButtonStyle, Look, font};
use sekka_kid_kit::stage::{self, Fit};
use sekka_kid_kit::who::{self, Card, Who, WhoHit};

use crate::art;
use crate::follow::Mark;
use crate::library::{self, Emblem, LEVELS};
use crate::phonics::Chunk;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Screen {
    Who,
    NewKid,
    Shelf,
    Cover,
    Read,
    Done,
    Rest,
}

/// Something to press.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Hit {
    Who(WhoHit),
    Kid,
    Home,
    Voice,
    GrownUps,
    /// A shelf's tab, by level.
    Level(u8),
    /// A book on the shelf: its place in the library.
    Book(usize),
    ShelfBack,
    ShelfOn,
    /// Read to Fumi: on the cover, from the bookmark; on a page, the
    /// microphone.
    Mic,
    /// Fumi reads the page (or the book, from the cover) aloud.
    Listen,
    /// From the first page (the cover, when there's a bookmark).
    FromStart,
    Word(usize),
    PrevPage,
    NextPage,
    PageAgain,
    BookAgain,
    Another,
    SwitchPlayer,
}

/// A book as a shelf shows it.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct BookCard {
    pub index: usize,
    pub title: String,
    pub emblem: Emblem,
    pub color: u32,
    pub classic: bool,
    pub bookmark: Option<usize>,
    pub finished: u32,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ShelfView {
    pub level: u8,
    /// The shelf for the kid's grade, marked.
    pub kid_level: u8,
    pub books: Vec<BookCard>,
    /// Which twelve of the shelf's books show, and how many twelves.
    pub page: usize,
    pub pages: usize,
    pub say: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct CoverView {
    pub card: BookCard,
    pub byline: Option<String>,
    pub about: String,
    pub pages: usize,
    pub words: usize,
    pub level: u8,
}

/// The microphone.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Mic {
    Off,
    /// Asked for, Whisper still loading.
    Waiting,
    On,
    /// No microphone program, or no Whisper model.
    None,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ReadView {
    pub title: String,
    pub level: u8,
    pub page: usize,
    pub pages: usize,
    pub text: String,
    pub marks: Vec<Mark>,
    pub next: usize,
    pub ahead: usize,
    pub mic: Mic,
    /// The word Fumi is saying, reading the page aloud.
    pub aloud: Option<usize>,
    /// The pieces of a word shown over it: (word, pieces).
    pub chunks: Option<(usize, Vec<Chunk>)>,
    pub done: bool,
    pub say: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct DoneView {
    pub card: BookCard,
    pub words: u32,
    pub times: u32,
}

/// What the canvas draws, now. Its `Hash` leaves out what moves by itself
/// (the microphone's rings, blinks, confetti), which `parts` draws alone.
#[derive(Clone, Debug)]
pub struct Scene {
    pub screen: Screen,
    pub who: Who,
    pub kid: Option<Card>,
    pub pressed: Option<Hit>,
    pub voice: bool,
    pub shelf: Option<ShelfView>,
    pub cover: Option<CoverView>,
    pub read: Option<ReadView>,
    pub done: Option<DoneView>,
    pub rest_say: String,
    // Moving.
    pub blink: bool,
    pub level: f32,
    pub phase: f64,
    pub confetti: f64,
}

impl Hash for Scene {
    fn hash<H: Hasher>(&self, h: &mut H) {
        (&self.screen, &self.who, &self.kid, &self.pressed, self.voice).hash(h);
        (&self.shelf, &self.cover, &self.read, &self.done, &self.rest_say).hash(h);
    }
}

// Where things are.

const HOME: (Point, f64) = (Point::new(58.0, 52.0), 32.0);
const VOICE: (Point, f64) = (Point::new(878.0, 52.0), 28.0);
const GROWN: (Point, f64) = (Point::new(946.0, 52.0), 28.0);
const KID: Rect = Rect::new(24.0, 22.0, 250.0, 82.0);

const TAB_W: f64 = 196.0;
const TAB_TOP: f64 = 104.0;
const COVER_W: f64 = 120.0;
const COVER_H: f64 = 160.0;
const ROW_TOP: [f64; 2] = [176.0, 380.0];
pub const PER_SHELF: usize = 12;
const SHELF_BACK: (Point, f64) = (Point::new(44.0, 360.0), 26.0);
const SHELF_ON: (Point, f64) = (Point::new(956.0, 360.0), 26.0);
const SHELF_FUMI: (Point, f64) = (Point::new(104.0, 626.0), 46.0);
const SHELF_BUBBLE: Rect = Rect::new(176.0, 590.0, 940.0, 664.0);

const BIG_COVER: Rect = Rect::new(90.0, 128.0, 370.0, 500.0);
const TEXT_X: f64 = 420.0;
const READ_BUTTON: Rect = Rect::new(420.0, 520.0, 700.0, 610.0);
const LISTEN_BUTTON: Rect = Rect::new(720.0, 520.0, 920.0, 610.0);
const FROM_START: Rect = Rect::new(420.0, 626.0, 700.0, 676.0);

const FUMI: (Point, f64) = (Point::new(148.0, 112.0), 44.0);
const READ_BUBBLE: Rect = Rect::new(214.0, 70.0, 846.0, 152.0);
pub const PAGE: Rect = Rect::new(60.0, 172.0, 940.0, 588.0);
const PREV: (Point, f64) = (Point::new(230.0, 642.0), 30.0);
const LISTEN: (Point, f64) = (Point::new(366.0, 642.0), 34.0);
pub const MIC: (Point, f64) = (Point::new(500.0, 638.0), 44.0);
const AGAIN: (Point, f64) = (Point::new(634.0, 642.0), 30.0);
const NEXT: (Point, f64) = (Point::new(770.0, 642.0), 34.0);

const CARD: Rect = Rect::new(230.0, 96.0, 770.0, 650.0);
const BOOK_AGAIN: Rect = Rect::new(270.0, 540.0, 490.0, 620.0);
const ANOTHER: Rect = Rect::new(510.0, 540.0, 730.0, 620.0);
const SWITCH: Rect = Rect::new(380.0, 590.0, 620.0, 660.0);

fn tab_rect(i: usize) -> Rect {
    let x = 90.0 + i as f64 * (TAB_W + 12.0);
    Rect::new(x, TAB_TOP, x + TAB_W, TAB_TOP + 46.0)
}

/// Where the `i`th book on a shelf's page stands.
pub fn cover_rect(i: usize) -> Rect {
    let (row, col) = (i / 6, i % 6);
    let x = 90.0 + col as f64 * (COVER_W + 20.0);
    let y = ROW_TOP[row.min(1)];
    Rect::new(x, y, x + COVER_W, y + COVER_H)
}

fn round_rect(c: (Point, f64)) -> Rect {
    Rect::new(c.0.x - c.1, c.0.y - c.1, c.0.x + c.1, c.0.y + c.1 + 5.0)
}

fn round_hit(c: (Point, f64), p: Point) -> bool {
    (p - c.0).hypot() <= c.1 + 8.0
}

// A page's words, laid out.

/// A page set in type: each word's box (an index per word of the page).
#[derive(Clone, Debug, PartialEq)]
pub struct Laid {
    pub boxes: Vec<Rect>,
    pub size: f64,
    pub line: f64,
}

/// The size a level's pages start at; long pages come down from it.
fn base_size(level: u8) -> f64 {
    [54.0, 46.0, 36.0, 29.0][usize::from(level.clamp(1, 4)) - 1]
}

/// The smallest type each level's pages may be set in: big for the first
/// books, smaller for chapters. A page that needs smaller is split
/// (`library::paginate`).
pub const SMALLEST: [f64; 4] = [36.0, 32.0, 24.0, 22.0];

/// Whether `text` fits on the paper in its level's type, no smaller than
/// [`SMALLEST`].
pub fn fits(theme: &Theme, text: &str, level: u8) -> bool {
    let least = SMALLEST[usize::from(level.clamp(1, 4)) - 1];
    let laid = lay_out_down_to(theme, text, level, least);
    laid.size >= least && laid.boxes.iter().all(|b| b.y1 <= PAGE.y1 - 46.0 + 0.5)
}

/// `text` laid out on [`PAGE`]: paragraphs apart, lines of verse on lines
/// of their own, the type as large as fits (from the level's size down),
/// and the whole block centred.
pub fn lay_out(theme: &Theme, text: &str, level: u8) -> Laid {
    lay_out_down_to(theme, text, level, 18.0)
}

fn lay_out_down_to(theme: &Theme, text: &str, level: u8, least: f64) -> Laid {
    // Room at the foot for the page number.
    let inner = Rect::new(PAGE.x0 + 48.0, PAGE.y0 + 34.0, PAGE.x1 - 48.0, PAGE.y1 - 46.0);
    let paragraphs = library::paragraphs(text);
    let mut size = base_size(level);
    loop {
        let style = art::reading(theme, size, Color::BLACK);
        let width = |s: &str| aizome_kit::sidestep_ui::measure(s, &style).width;
        let space = width("x x") - width("xx");
        let line = size * 1.5;
        let mut boxes = Vec::new();
        let mut x;
        let mut y = inner.y0;
        let mut widest: f64 = 0.0;
        for (pi, para) in paragraphs.iter().enumerate() {
            if pi > 0 {
                y += line + size * 0.55;
            }
            for (li, verse) in para.iter().enumerate() {
                if li > 0 {
                    y += line;
                }
                x = inner.x0;
                for word in verse.split_whitespace() {
                    let w = width(word);
                    if x > inner.x0 && x + w > inner.x1 {
                        y += line;
                        // A verse line that runs over is indented.
                        x = inner.x0 + if para.len() > 1 { size } else { 0.0 };
                    }
                    boxes.push(Rect::new(x, y, x + w, y + line));
                    widest = widest.max(x + w - inner.x0);
                    x += w + space;
                }
            }
        }
        let height = y + line - inner.y0;
        if height <= inner.height() || size <= least {
            let dx = ((inner.width() - widest) / 2.0).max(0.0);
            let dy = ((inner.height() - height) / 2.0).max(0.0);
            for b in &mut boxes {
                *b = *b + Vec2::new(dx, dy);
            }
            return Laid { boxes, size, line };
        }
        size -= 2.0;
    }
}

// Pressing.

pub fn hit(size: Size, s: &Scene, p: Point, theme: &Theme) -> Option<Hit> {
    if s.screen == Screen::Who {
        return who::hit(size, &s.who, p).map(Hit::Who);
    }
    let p = Fit::of_size(size).to_design(p);
    let top = |p: Point, home: bool| {
        if home && round_hit(HOME, p) {
            Some(Hit::Home)
        } else if round_hit(VOICE, p) {
            Some(Hit::Voice)
        } else if round_hit(GROWN, p) {
            Some(Hit::GrownUps)
        } else {
            None
        }
    };
    match s.screen {
        Screen::Who | Screen::NewKid => None,
        Screen::Shelf => {
            let v = s.shelf.as_ref()?;
            if look::hits(KID, p, 4.0) {
                return Some(Hit::Kid);
            }
            if let Some(h) = top(p, false) {
                return Some(h);
            }
            if let Some(i) = (0..4).find(|i| look::hits(tab_rect(*i), p, 4.0)) {
                return Some(Hit::Level(LEVELS[i].n));
            }
            if v.page > 0 && round_hit(SHELF_BACK, p) {
                return Some(Hit::ShelfBack);
            }
            if v.page + 1 < v.pages && round_hit(SHELF_ON, p) {
                return Some(Hit::ShelfOn);
            }
            v.books
                .iter()
                .enumerate()
                .find(|(i, _)| look::hits(cover_rect(*i), p, 6.0))
                .map(|(_, b)| Hit::Book(b.index))
        }
        Screen::Cover => {
            let v = s.cover.as_ref()?;
            if let Some(h) = top(p, true) {
                return Some(h);
            }
            if look::hits(READ_BUTTON, p, 4.0) {
                Some(Hit::Mic)
            } else if look::hits(LISTEN_BUTTON, p, 4.0) {
                Some(Hit::Listen)
            } else if v.card.bookmark.is_some() && look::hits(FROM_START, p, 4.0) {
                Some(Hit::FromStart)
            } else if look::hits(BIG_COVER, p, 0.0) {
                Some(Hit::Mic)
            } else {
                None
            }
        }
        Screen::Read => {
            let v = s.read.as_ref()?;
            if let Some(h) = top(p, true) {
                return Some(h);
            }
            for (c, h) in [
                (PREV, Hit::PrevPage),
                (LISTEN, Hit::Listen),
                (MIC, Hit::Mic),
                (AGAIN, Hit::PageAgain),
                (NEXT, Hit::NextPage),
            ] {
                if round_hit(c, p) && (h != Hit::PrevPage || v.page > 0) {
                    return Some(h);
                }
            }
            let laid = lay_out(theme, &v.text, v.level);
            laid.boxes.iter().position(|b| b.inflate(6.0, 0.0).contains(p)).map(Hit::Word)
        }
        Screen::Done => {
            if look::hits(BOOK_AGAIN, p, 4.0) {
                Some(Hit::BookAgain)
            } else if look::hits(ANOTHER, p, 4.0) {
                Some(Hit::Another)
            } else {
                top(p, true)
            }
        }
        Screen::Rest => {
            if look::hits(SWITCH, p, 4.0) {
                Some(Hit::SwitchPlayer)
            } else {
                top(p, false)
            }
        }
    }
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
        Screen::Shelf | Screen::Read => 0.0,
        Screen::Rest => 170.0,
        _ => 110.0,
    };
    stage::backdrop(c, b, &fit, &look, hills);
    c.saved(|c| {
        fit.apply(c);
        match s.screen {
            Screen::Shelf => shelf(c, s, theme, &look),
            Screen::Cover => cover(c, s, theme, &look),
            Screen::Read => read(c, s, theme, &look),
            Screen::Done => done(c, s, theme, &look),
            Screen::Rest => rest(c, s, theme, &look),
            Screen::Who | Screen::NewKid => {}
        }
    });
}

fn pressed(s: &Scene, h: Hit) -> bool {
    s.pressed == Some(h)
}

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

/// A wide button with an icon and words.
#[allow(clippy::too_many_arguments)]
fn wide_button(
    c: &mut Canvas,
    theme: &Theme,
    r: Rect,
    style: ButtonStyle,
    down: bool,
    icon: &str,
    label: &str,
    look: &Look,
) {
    let face = look::button(c, r, 22.0, style, down, look);
    // As large as fits, icon and all.
    let mut spec = font(if face.height() > 60.0 { 26.0 } else { 20.0 }, 800.0);
    let fits = |spec: aizome_kit::theme::FontSpec| {
        let iw = if icon.is_empty() { 0.0 } else { spec.size * 1.2 + 10.0 };
        look::measure(theme, label, spec).width + iw <= face.width() - 28.0
    };
    while spec.size > 14.0 && !fits(spec) {
        spec.size -= 1.0;
    }
    let w = look::measure(theme, label, spec).width;
    let iw = if icon.is_empty() { 0.0 } else { spec.size * 1.2 + 10.0 };
    let x = face.center().x - (w + iw) / 2.0;
    if !icon.is_empty() {
        glyph(c, icon, Point::new(x + spec.size * 0.6, face.center().y), spec.size * 1.2, style.ink);
    }
    look::text(c, theme, label, Point::new(x + iw, face.center().y - spec.size * 0.62), spec, style.ink);
}

fn top_buttons(c: &mut Canvas, s: &Scene, look: &Look, home: bool) {
    if home {
        round_button(c, HOME, ButtonStyle::plain(look), pressed(s, Hit::Home), look, |c, at| {
            glyph(c, "library", at, 30.0, look.ink)
        });
    }
    round_button(c, VOICE, ButtonStyle::plain(look), pressed(s, Hit::Voice), look, |c, at| {
        glyph(c, if s.voice { "volume-2" } else { "volume-x" }, at, 26.0, look.ink)
    });
    round_button(c, GROWN, ButtonStyle::plain(look), pressed(s, Hit::GrownUps), look, |c, at| {
        glyph(c, "lock", at, 24.0, look.ink_soft)
    });
}

fn kid_chip(c: &mut Canvas, s: &Scene, theme: &Theme, look: &Look) {
    let Some(kid) = &s.kid else { return };
    let r = if pressed(s, Hit::Kid) { KID.inflate(-2.0, -2.0) } else { KID };
    look::card(c, r, 30.0, look.card, look.card_edge, look);
    let mood = if s.blink { Mood::Blink } else { Mood::Happy };
    avatar::paint(c, Point::new(r.x0 + 32.0, r.center().y + 2.0), 19.0, kid.animal, Some(kid.color), mood);
    look::text(c, theme, &kid.name, Point::new(r.x0 + 66.0, r.center().y - 13.0), font(22.0, 800.0), look.ink);
}

/// Fumi, and what she's saying in a bubble beside her.
#[allow(clippy::too_many_arguments)]
fn fumi_says(
    c: &mut Canvas,
    theme: &Theme,
    at: (Point, f64),
    bubble_r: Rect,
    say: &str,
    mood: Mood,
    size: f64,
    look: &Look,
) {
    art::fumi(c, at.0, at.1, mood);
    if say.is_empty() {
        return;
    }
    bubble(c, bubble_r, Point::new(at.0.x + at.1 * 0.9, at.0.y), look);
    let spec = font(size, 700.0);
    look::paragraph_centred(c, theme, say, bubble_r.center(), bubble_r.width() - 40.0, spec, look.ink);
}

fn shelf(c: &mut Canvas, s: &Scene, theme: &Theme, look: &Look) {
    let Some(v) = &s.shelf else { return };
    kid_chip(c, s, theme, look);
    look::text_centred(c, theme, "Ondoku", Point::new(560.0, 48.0), font(34.0, 900.0), look.ink);
    top_buttons(c, s, look, false);
    for (i, l) in LEVELS.iter().enumerate() {
        let r = tab_rect(i);
        let on = l.n == v.level;
        let style = if on { ButtonStyle::calm(look) } else { ButtonStyle::plain(look) };
        let face = look::button(c, r, 23.0, style, pressed(s, Hit::Level(l.n)), look);
        let mut label_at = face.center();
        if l.n == v.kid_level
            && let Some(kid) = &s.kid
        {
            avatar::paint(c, Point::new(face.x0 + 24.0, face.center().y), 13.0, kid.animal, None, Mood::Happy);
            label_at.x += 12.0;
        }
        look::text_centred(c, theme, l.name, label_at, font(19.0, 800.0), style.ink);
    }
    for (row, top) in ROW_TOP.iter().enumerate() {
        if v.books.len() > row * 6 {
            art::plank(c, 70.0, 930.0, top + COVER_H + 2.0, look);
        }
    }
    for (i, b) in v.books.iter().enumerate() {
        let mut r = cover_rect(i);
        if pressed(s, Hit::Book(b.index)) {
            r = r + Vec2::new(0.0, 3.0);
        }
        art::cover(c, theme, r, &b.title, b.emblem, b.color, b.classic, b.bookmark.is_some(), b.finished, look);
    }
    if v.books.is_empty() {
        look::text_centred(c, theme, "No books here yet.", Point::new(500.0, 330.0), font(26.0, 700.0), look.ink_soft);
    }
    if v.page > 0 {
        round_button(c, SHELF_BACK, ButtonStyle::plain(look), pressed(s, Hit::ShelfBack), look, |c, at| {
            glyph(c, "chevron-left", at, 28.0, look.ink)
        });
    }
    if v.page + 1 < v.pages {
        round_button(c, SHELF_ON, ButtonStyle::plain(look), pressed(s, Hit::ShelfOn), look, |c, at| {
            glyph(c, "chevron-right", at, 28.0, look.ink)
        });
    }
    let mood = if s.blink { Mood::Blink } else { Mood::Happy };
    fumi_says(c, theme, SHELF_FUMI, SHELF_BUBBLE, &v.say, mood, 21.0, look);
}

fn cover(c: &mut Canvas, s: &Scene, theme: &Theme, look: &Look) {
    let Some(v) = &s.cover else { return };
    top_buttons(c, s, look, true);
    let b = &v.card;
    art::cover(c, theme, BIG_COVER, &b.title, b.emblem, b.color, b.classic, b.bookmark.is_some(), b.finished, look);
    let width = 920.0 - TEXT_X;
    let title = font(40.0, 900.0);
    let mut y = 120.0;
    for line in look::wrap(theme, &b.title, width, title) {
        look::text(c, theme, &line, Point::new(TEXT_X, y), title, look.ink);
        y += 48.0;
    }
    if let Some(by) = &v.byline {
        look::text(c, theme, by, Point::new(TEXT_X, y + 2.0), font(20.0, 600.0), look.ink_soft);
        y += 32.0;
    }
    let level = LEVELS[usize::from(v.level.clamp(1, 4)) - 1];
    let mut chip = format!("{} · {}", level.name, level.grades);
    if b.classic {
        chip = format!("A classic · {chip}");
    }
    look::text(c, theme, &chip, Point::new(TEXT_X, y + 6.0), font(17.0, 700.0), look.calm);
    y += 48.0;
    let about = font(23.0, 600.0);
    for line in look::wrap(theme, &v.about, width, about) {
        look::text(c, theme, &line, Point::new(TEXT_X, y), about, look.ink);
        y += 32.0;
    }
    let pages = if v.pages == 1 { "1 page".to_owned() } else { format!("{} pages", v.pages) };
    let mut facts = format!("{pages} · {} words", v.words);
    match b.finished {
        0 => {}
        1 => facts.push_str(" · read once"),
        2 => facts.push_str(" · read twice"),
        n => facts.push_str(&format!(" · read {n} times")),
    }
    look::text(c, theme, &facts, Point::new(TEXT_X, y + 12.0), font(18.0, 600.0), look.ink_soft);
    let read = match b.bookmark {
        Some(p) => format!("Go on: page {}", p + 1),
        None => "Read to Fumi".to_owned(),
    };
    wide_button(c, theme, READ_BUTTON, ButtonStyle::good(look), pressed(s, Hit::Mic), "mic", &read, look);
    wide_button(
        c,
        theme,
        LISTEN_BUTTON,
        ButtonStyle::calm(look),
        pressed(s, Hit::Listen),
        "volume-2",
        "Fumi reads",
        look,
    );
    if b.bookmark.is_some() {
        wide_button(
            c,
            theme,
            FROM_START,
            ButtonStyle::plain(look),
            pressed(s, Hit::FromStart),
            "rotate-ccw",
            "From the start",
            look,
        );
    }
}

/// Where a word's type is drawn in its box `b`, `th` tall: centred on its line.
fn type_top(b: Rect, laid: &Laid, th: f64) -> f64 {
    b.y0 + (laid.line - th) / 2.0
}

/// The rounded rectangle a word is lit in.
fn word_rect(b: Rect, laid: &Laid, th: f64) -> Rect {
    let pad = laid.size * 0.14;
    let top = type_top(b, laid, th);
    Rect::new(b.x0 - pad, top, b.x1 + pad, top + th)
}

/// How a word looks for its mark: a fill behind it, or dots under it.
fn word_marks(c: &mut Canvas, b: Rect, laid: &Laid, th: f64, mark: Mark, look: &Look) {
    let pad = laid.size * 0.14;
    let r = word_rect(b, laid, th);
    match mark {
        Mark::Read => c.fill(&RoundedRect::from_rect(r, pad * 2.0), look.good.with_alpha(0.2)),
        Mark::Helped => c.fill(&RoundedRect::from_rect(r, pad * 2.0), look.gold.with_alpha(0.32)),
        Mark::Missed => {
            let y = r.y1 - 2.0;
            let mut x = r.x0 + 2.0;
            while x < r.x1 - 2.0 {
                c.fill(
                    &aizome_kit::sidestep_ui::kurbo::Circle::new(Point::new(x, y), 2.4),
                    look.accent.with_alpha(0.75),
                );
                x += 9.0;
            }
        }
        Mark::Unread => {}
    }
}

fn read(c: &mut Canvas, s: &Scene, theme: &Theme, look: &Look) {
    let Some(v) = &s.read else { return };
    top_buttons(c, s, look, true);
    let mood = if v.done {
        Mood::Cheer
    } else if s.blink {
        Mood::Blink
    } else {
        Mood::Happy
    };
    fumi_says(c, theme, FUMI, READ_BUBBLE, &v.say, mood, 22.0, look);
    look::card(c, PAGE, 26.0, look.card, look.card_edge, look);
    let laid = lay_out(theme, &v.text, v.level);
    let style = art::reading(theme, laid.size, look.ink);
    let th = aizome_kit::sidestep_ui::measure("Ag", &style).height;
    let listening = v.mic == Mic::On && !v.done;
    for (i, b) in laid.boxes.iter().enumerate() {
        let mark = v.marks.get(i).copied().unwrap_or(Mark::Unread);
        word_marks(c, *b, &laid, th, mark, look);
        let pad = laid.size * 0.14;
        let r = word_rect(*b, &laid, th);
        if listening && i >= v.next && i < v.ahead {
            c.fill(&RoundedRect::from_rect(r, pad * 2.0), look.calm.with_alpha(0.12));
        }
        if (listening && i == v.next) || v.aloud == Some(i) || v.chunks.as_ref().is_some_and(|(w, _)| *w == i) {
            c.stroke(&RoundedRect::from_rect(r, pad * 2.0), &StrokeStyle::new(3.0), look.calm);
        }
    }
    let words: Vec<&str> = v.text.split_whitespace().collect();
    for (i, b) in laid.boxes.iter().enumerate() {
        c.draw_label(words[i], &style, Point::new(b.x0, type_top(*b, &laid, th)));
    }
    // Where the reader is in the book.
    let foot = format!("{} · page {} of {}", v.title, v.page + 1, v.pages);
    look::text_centred(c, theme, &foot, Point::new(PAGE.center().x, PAGE.y1 - 24.0), font(15.0, 700.0), look.ink_soft);
    if let Some((i, pieces)) = &v.chunks
        && let Some(b) = laid.boxes.get(*i)
    {
        sound_it_out(c, theme, word_rect(*b, &laid, th), pieces, look);
    }
    // The buttons.
    if v.page > 0 {
        round_button(c, PREV, ButtonStyle::plain(look), pressed(s, Hit::PrevPage), look, |c, at| {
            glyph(c, "chevron-left", at, 28.0, look.ink)
        });
    }
    round_button(c, LISTEN, ButtonStyle::calm(look), pressed(s, Hit::Listen), look, |c, at| {
        glyph(c, "volume-2", at, 30.0, Color::WHITE)
    });
    // What the microphone is doing, Fumi says in her bubble.
    let (style, icon) = match v.mic {
        Mic::Off => (ButtonStyle::good(look), "mic"),
        Mic::Waiting => (ButtonStyle::calm(look), "mic"),
        Mic::On => (ButtonStyle::accent(look), "mic"),
        Mic::None => (ButtonStyle::plain(look), "mic-off"),
    };
    round_button(c, MIC, style, pressed(s, Hit::Mic), look, |c, at| glyph(c, icon, at, 38.0, style.ink));
    round_button(c, AGAIN, ButtonStyle::plain(look), pressed(s, Hit::PageAgain), look, |c, at| {
        glyph(c, "rotate-ccw", at, 26.0, look.ink)
    });
    let next_style = if v.done { ButtonStyle::good(look) } else { ButtonStyle::plain(look) };
    let last = v.page + 1 == v.pages;
    round_button(c, NEXT, next_style, pressed(s, Hit::NextPage), look, |c, at| {
        glyph(c, if last { "check" } else { "chevron-right" }, at, 30.0, next_style.ink)
    });
    if v.mic == Mic::On {
        art::rings(c, MIC.0, MIC.1, s.level, s.phase, look);
    }
}

/// A word in the pieces it's sounded out in, on a card over it, in the
/// page's own type (the letters a reader knows), a silent e faded.
fn sound_it_out(c: &mut Canvas, theme: &Theme, word: Rect, pieces: &[Chunk], look: &Look) {
    let style = |color| art::reading(theme, 44.0, color);
    let width = |s: &str| aizome_kit::sidestep_ui::measure(s, &style(look.ink)).width;
    let th = aizome_kit::sidestep_ui::measure("Ag", &style(look.ink)).height;
    let dot = " · ";
    let widths: Vec<f64> = pieces.iter().map(|p| width(&p.text)).collect();
    let dot_w = width(dot);
    let w = widths.iter().sum::<f64>() + dot_w * (pieces.len().saturating_sub(1)) as f64 + 44.0;
    let h = th + 16.0;
    let x0 = (word.center().x - w / 2.0).clamp(PAGE.x0 + 8.0, PAGE.x1 - w - 8.0);
    let y0 = if word.y0 - h - 8.0 > PAGE.y0 { word.y0 - h - 8.0 } else { word.y1 + 8.0 };
    let r = Rect::new(x0, y0, x0 + w, y0 + h);
    look::card(c, r, 20.0, look.card, look.calm, look);
    let mut x = r.x0 + 22.0;
    let top = r.y0 + 8.0;
    for (i, (p, pw)) in pieces.iter().zip(&widths).enumerate() {
        if i > 0 {
            c.draw_label(dot, &style(look.calm), Point::new(x, top));
            x += dot_w;
        }
        let ink = if p.silent { look.ink_soft.with_alpha(0.5) } else { look.ink };
        c.draw_label(&p.text, &style(ink), Point::new(x, top));
        x += pw;
    }
}

fn done(c: &mut Canvas, s: &Scene, theme: &Theme, look: &Look) {
    let Some(v) = &s.done else { return };
    top_buttons(c, s, look, true);
    look::card(c, CARD, 32.0, look.card, look.card_edge, look);
    art::fumi(c, Point::new(CARD.center().x, 196.0), 62.0, Mood::Cheer);
    look::text_centred(c, theme, "You read it!", Point::new(CARD.center().x, 300.0), font(46.0, 900.0), look.accent);
    look::paragraph_centred(
        c,
        theme,
        &v.card.title,
        Point::new(CARD.center().x, 356.0),
        460.0,
        font(26.0, 800.0),
        look.ink,
    );
    let words = format!("{} words, all the way to the end.", v.words);
    look::text_centred(c, theme, &words, Point::new(CARD.center().x, 412.0), font(21.0, 700.0), look.ink_soft);
    let times = match v.times {
        1 => "Your first time through!".to_owned(),
        2 => "Twice now! Reading again makes it smoother.".to_owned(),
        n => format!("{n} times now! You know it well."),
    };
    look::text_centred(c, theme, &times, Point::new(CARD.center().x, 448.0), font(19.0, 700.0), look.ink_soft);
    for i in 0..3u32 {
        star(c, Point::new(CARD.center().x + (f64::from(i) - 1.0) * 50.0, 494.0), 20.0, i < v.times, look);
    }
    wide_button(
        c,
        theme,
        BOOK_AGAIN,
        ButtonStyle::calm(look),
        pressed(s, Hit::BookAgain),
        "repeat",
        "Read again",
        look,
    );
    wide_button(c, theme, ANOTHER, ButtonStyle::good(look), pressed(s, Hit::Another), "library", "Another book", look);
    confetti(c, Rect::new(0.0, 0.0, stage::W, stage::H), s.confetti, look);
}

fn rest(c: &mut Canvas, s: &Scene, theme: &Theme, look: &Look) {
    top_buttons(c, s, look, false);
    art::fumi(c, Point::new(500.0, 260.0), 90.0, Mood::Sleep);
    look::paragraph_centred(c, theme, &s.rest_say, Point::new(500.0, 440.0), 620.0, font(30.0, 800.0), look.ink);
    wide_button(c, theme, SWITCH, ButtonStyle::plain(look), pressed(s, Hit::SwitchPlayer), "", "Someone else", look);
}

// Drawing only what moves.

fn hash_of(v: impl Hash) -> u64 {
    let mut h = std::collections::hash_map::DefaultHasher::new();
    v.hash(&mut h);
    h.finish()
}

/// The rectangles that move by themselves, each with a hash of what it
/// shows: the microphone's rings, Fumi's and the kid's blinks.
pub fn parts(s: &Scene, size: Size) -> Vec<(Rect, u64)> {
    let fit = Fit::of_size(size);
    let mut out: Vec<(Rect, u64)> = Vec::new();
    let mut part = |r: Rect, hash: u64| out.push((fit.to_window(r), hash));
    match s.screen {
        Screen::Read => {
            if let Some(v) = &s.read {
                let rings = round_rect(MIC).inflate(52.0, 50.0);
                let level = (s.level * 400.0) as u32;
                let phase = (s.phase * 30.0) as u32;
                part(rings, hash_of((v.mic == Mic::On, level, phase)));
                part(Rect::new(FUMI.0.x - 50.0, FUMI.0.y - 50.0, FUMI.0.x + 50.0, FUMI.0.y + 50.0), hash_of(s.blink));
            }
        }
        Screen::Shelf => {
            part(Rect::new(KID.x0, KID.y0, KID.x0 + 70.0, KID.y1), hash_of(s.blink));
            let (at, r) = SHELF_FUMI;
            part(Rect::new(at.x - r - 6.0, at.y - r - 6.0, at.x + r + 6.0, at.y + r + 6.0), hash_of(s.blink));
        }
        Screen::Who => {
            for r in who::cards(s.who.cards.len()).iter().take(s.who.cards.len()) {
                part(Rect::new(r.x0, r.y0, r.x1, r.y0 + 160.0), hash_of(s.who.blink));
            }
        }
        Screen::Done => part(Rect::new(0.0, 0.0, stage::W, stage::H), hash_of((s.confetti * 50.0) as u32)),
        _ => {}
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shelf_places_dont_overlap() {
        for i in 0..PER_SHELF {
            for j in i + 1..PER_SHELF {
                assert!(cover_rect(i).intersect(cover_rect(j)).area() <= 0.0, "{i} and {j}");
            }
        }
        assert!(tab_rect(3).x1 <= 920.0);
    }
}
