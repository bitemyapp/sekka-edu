//! Ondoku, run headlessly with a clock the test moves and ears that are
//! pretended (what's "heard" is sent in): a first reader, a whole book read
//! to Fumi and its log, a word tapped, help when stuck, the day ending at a
//! page, and pictures of every screen by Day and Night.

use std::cell::Cell;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::time::Duration;

use aizome_kit::Appearance;
use aizome_kit::layout::Point;
use aizome_kit::sidestep_ui::{Modifiers, NamedKey};
use aizome_kit::testing::{Harness, assert_golden};
use aizome_kit::theme::ThemeChoice;
use sekka_kid::{Animal, Event, Grade, KidColor, NewProfile, Store, clock};
use sekka_kid_kit::stage::Fit;
use sekka_ondoku::follow::Mark;
use sekka_ondoku::listen::Heard;
use sekka_ondoku::scene::{self, Mic, Screen};
use sekka_ondoku::{APP_ID, Msg, Ondoku};

fn scratch(name: &str) -> PathBuf {
    let d = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("ondoku-{name}"));
    let _ = std::fs::remove_dir_all(&d);
    d
}

/// Ten in the morning, today.
fn morning() -> i64 {
    clock::day_start(clock::now_ms()) + 10 * 3_600_000
}

struct Run {
    h: Harness<Ondoku>,
    w: aizome_kit::WindowKey,
    t: Rc<Cell<i64>>,
    store: Store,
}

const HANA: (&str, Animal, KidColor, Grade) = ("Hana", Animal::Fox, KidColor::Coral, Grade::Kindergarten);
const SORA: (&str, Animal, KidColor, Grade) = ("Sora", Animal::Owl, KidColor::Indigo, Grade::Fourth);

fn ondoku(name: &str, theme: ThemeChoice, kids: &[(&str, Animal, KidColor, Grade)]) -> Run {
    let store = Store::at(scratch(name));
    for (n, a, c, g) in kids {
        store.add(NewProfile { name: (*n).into(), animal: *a, color: *c, grade: *g }).unwrap();
    }
    let mut app = Ondoku::new(store.clone(), true);
    let t = app.manual_clock(morning());
    app.pretend_ears();
    // Still pictures: no confetti or rings moving.
    let h = Harness::with(app, Appearance { theme, reduced_motion: true, ..Appearance::default() }, false);
    let w = h.window();
    Run { h, w, t, store }
}

impl Run {
    fn key(&mut self, k: NamedKey) {
        self.h.named(self.w, k, Modifiers::NONE);
    }

    /// Press at `p` in the design's points.
    fn tap(&mut self, p: Point) {
        let fit = Fit::of_size(self.h.size(self.w));
        let at = Point::new(fit.origin.x + p.x * fit.scale, fit.origin.y + p.y * fit.scale);
        self.h.click(self.w, at);
    }

    fn wait(&mut self, ms: i64) {
        self.t.set(self.t.get() + ms);
        self.h.advance(Duration::from_millis(ms as u64));
    }

    /// Open the book called `title` from the shelf showing.
    fn open(&mut self, title: &str) {
        let p = self.h.program();
        let level = p.library.iter().find(|b| b.title == title).unwrap().level;
        let on_shelf: Vec<&str> = p.library.iter().filter(|b| b.level == level).map(|b| b.title.as_str()).collect();
        let i = on_shelf.iter().position(|t| *t == title).unwrap();
        self.tap(scene::cover_rect(i % scene::PER_SHELF).center());
        assert_eq!(self.h.program().screen, Screen::Cover, "{title}'s cover");
    }

    /// Hear the reader say `text` at a pause.
    fn say(&mut self, text: &str) {
        let turn = self.h.program().turn();
        self.h.send(Msg::Heard(Heard::Said(turn, text.into())));
    }

    fn page_text(&self) -> String {
        let p = self.h.program().page().unwrap();
        p.words.iter().map(|w| w.text.clone()).collect::<Vec<_>>().join(" ")
    }

    fn log(&self) -> Vec<Event> {
        let kid = self.store.profiles().into_iter().next().unwrap();
        self.store.events(&kid.id)
    }
}

#[test]
fn a_first_reader_makes_a_card_and_meets_their_shelf() {
    let mut r = ondoku("first", ThemeChoice::Day, &[]);
    assert_eq!(r.h.program().screen, Screen::NewKid);
    r.h.type_text(r.w, "Mei");
    r.h.send(Msg::NewKid(sekka_kid_kit::newkid::NewKidMsg::Make));
    assert_eq!(r.h.program().screen, Screen::Shelf);
    assert!(r.h.program().says().contains("Mei"));
    assert_eq!(r.store.profiles()[0].name, "Mei");
    // Ondoku is in the catalogue for Sekka Kids.
    assert!(r.store.apps().iter().any(|a| a.id == APP_ID));
}

#[test]
fn reading_a_whole_book_to_fumi() {
    let mut r = ondoku("book", ThemeChoice::Day, &[HANA]);
    r.key(NamedKey::Enter);
    assert_eq!(r.h.program().screen, Screen::Shelf);
    r.open("Sam and the Cat");
    // Enter on the cover: read to Fumi.
    r.key(NamedKey::Enter);
    assert_eq!(r.h.program().screen, Screen::Read);
    assert_eq!(r.h.program().mic(), Mic::On);
    let pages = r.h.program().library.iter().find(|b| b.title == "Sam and the Cat").unwrap().pages.len();
    for page in 0..pages {
        r.wait(4_000);
        let text = r.page_text();
        r.say(&text);
        assert!(r.h.program().page().unwrap().done(), "page {page} followed to its end");
        r.key(NamedKey::ArrowRight);
    }
    assert_eq!(r.h.program().screen, Screen::Done);
    let log = r.log();
    let round = log
        .iter()
        .find_map(|e| match e {
            Event::Round(r) if r.app == APP_ID => Some(r.clone()),
            _ => None,
        })
        .expect("the book in the log");
    assert!(round.finished);
    assert_eq!(round.activity, "sam-and-the-cat");
    assert_eq!((round.items, round.first_try, round.hints), (54, 54, 0));
    assert!(round.words_per_minute.is_some(), "{round:?}");
    assert!(round.skills.contains(&"short-a".to_owned()));
    assert!(
        log.iter().any(|e| matches!(e, Event::Milestone { text, .. } if text == "Read Sam and the Cat to the end"))
    );
    assert_eq!(r.h.program().progress.book("sam-and-the-cat").finished, 1);
    // Again from the end: a second time through.
    r.tap(Point::new(380.0, 580.0));
    assert_eq!(r.h.program().screen, Screen::Read);
    assert_eq!(r.h.program().page().unwrap().next, 0);
}

#[test]
fn a_tapped_word_is_said_and_a_stuck_reader_is_helped() {
    let mut r = ondoku("help", ThemeChoice::Day, &[HANA]);
    r.key(NamedKey::Enter);
    r.open("Tim Can Dig");
    r.key(NamedKey::Enter);
    // "Kim has a big bag. Kim can dig, and Tim can fill the bag."
    r.key(NamedKey::ArrowRight);
    r.key(NamedKey::ArrowRight);
    let text = r.page_text();
    assert!(text.starts_with("Kim has a big bag."), "{text}");
    r.say("Kim has a big bag");
    assert_eq!(r.h.program().page().unwrap().next, 5);
    // Quiet on "Kim": its sounds show, then Fumi says it.
    let turn = r.h.program().turn();
    r.h.send(Msg::Heard(Heard::Quiet(turn)));
    assert_eq!(r.h.program().sounding_out(), Some(5));
    r.h.send(Msg::Stuck(turn, 5));
    let page = r.h.program().page().unwrap();
    assert_eq!((page.marks[5], page.next), (Mark::Helped, 6));
    // A word tapped where it's drawn: "fill".
    let laid =
        scene::lay_out(r.h.theme(), &r.h.program().library.iter().find(|b| b.id == "tim-can-dig").unwrap().pages[2], 1);
    let fill = r.h.program().page().unwrap().words.iter().position(|w| w.key == "fill").unwrap();
    r.tap(laid.boxes[fill].center());
    let page = r.h.program().page().unwrap();
    assert_eq!(page.marks[fill], Mark::Helped);
    assert_eq!(r.h.program().sounding_out(), Some(fill));
    assert!(r.h.program().says().contains("fill"), "{}", r.h.program().says());
}

#[test]
fn the_day_ends_at_a_page_and_the_place_is_kept() {
    let mut r = ondoku("allowance", ThemeChoice::Day, &[HANA]);
    let mut kid = r.store.profiles().remove(0);
    kid.daily_minutes = Some(5);
    r.store.update(&kid).unwrap();
    // Four and a half minutes read already today, in another app.
    let from = morning() - 30 * 60_000;
    r.store.append(&kid.id, &Event::Active { app: "computer.sekka.Anzan".into(), from, to: from + 270_000 }).unwrap();
    r.key(NamedKey::Enter);
    r.open("Hop, Fox!");
    r.key(NamedKey::Enter);
    // A minute's reading (the microphone hears it), then the page turns:
    // the day ends there.
    r.say("Fox can hop");
    for _ in 0..6 {
        r.wait(10_000);
        let turn = r.h.program().turn();
        r.h.send(Msg::Heard(Heard::Level(turn, 0.05)));
    }
    r.key(NamedKey::ArrowRight);
    assert_eq!(r.h.program().screen, Screen::Rest);
    assert_eq!(r.h.program().progress.book("hop-fox").page, 1, "the next page is kept");
    let round = r.log().into_iter().find_map(|e| match e {
        Event::Round(r) => Some(r),
        _ => None,
    });
    assert!(round.is_some_and(|r| !r.finished), "the part read is in the log");
}

fn picture(name: &str, theme: ThemeChoice, kids: &[(&str, Animal, KidColor, Grade)], set: impl FnOnce(&mut Run)) {
    let mut r = ondoku(&format!("golden-{name}"), theme, kids);
    set(&mut r);
    let suffix = if theme == ThemeChoice::Day { "day" } else { "night" };
    assert_golden(&mut r.h, r.w, format!("tests/golden/{name}-{suffix}.png"), 1.0);
}

#[test]
fn pictures_of_every_screen() {
    for theme in [ThemeChoice::Day, ThemeChoice::Night] {
        picture("who", theme, &[HANA, SORA], |_| {});
        picture("shelf", theme, &[HANA], |r| r.key(NamedKey::Enter));
        picture("chapter-books", theme, &[SORA], |r| r.key(NamedKey::Enter));
        picture("cover", theme, &[SORA], |r| {
            r.key(NamedKey::Enter);
            // The 2nd and 3rd grade shelf.
            r.tap(Point::new(604.0, 127.0));
            r.open("The Tale of Peter Rabbit");
        });
        picture("reading", theme, &[HANA], |r| {
            r.key(NamedKey::Enter);
            r.open("Tim Can Dig");
            r.key(NamedKey::Enter);
            r.key(NamedKey::ArrowRight);
            r.key(NamedKey::ArrowRight);
            // "big" left out, then going on; a guess runs ahead of the words
            // heard so far.
            r.say("Kim has a bag. Kim can");
            let turn = r.h.program().turn();
            r.h.send(Msg::Heard(Heard::Guess(turn, "dig and Tim".into())));
        });
        picture("sound-it-out", theme, &[HANA], |r| {
            r.key(NamedKey::Enter);
            r.open("Ten Red Hens");
            r.key(NamedKey::Enter);
            r.say("Ten red");
            let turn = r.h.program().turn();
            r.h.send(Msg::Heard(Heard::Quiet(turn)));
        });
        picture("long-page", theme, &[SORA], |r| {
            r.key(NamedKey::Enter);
            r.open("The Lighthouse Cat");
            r.key(NamedKey::Enter);
        });
        picture("done", theme, &[HANA], |r| {
            r.key(NamedKey::Enter);
            r.open("Gus the Pup");
            r.key(NamedKey::Enter);
            let pages = r.h.program().library.iter().find(|b| b.title == "Gus the Pup").unwrap().pages.len();
            for _ in 0..pages {
                let text = r.page_text();
                r.say(&text);
                r.key(NamedKey::ArrowRight);
            }
        });
    }
}

/// Every page of every book fits on the paper, in type no smaller than its
/// readers can manage: large for the first books, smaller for chapters.
#[test]
fn every_page_fits_on_the_paper() {
    let r = ondoku("fit", ThemeChoice::Day, &[HANA]);
    let theme = r.h.theme();
    let bottom = scene::PAGE.y1 - 46.0;
    let mut small = Vec::new();
    for b in &r.h.program().library {
        let least = [0.0, 36.0, 32.0, 24.0, 22.0][usize::from(b.level)];
        let mut smallest = f64::MAX;
        for (i, page) in b.pages.iter().enumerate() {
            let laid = scene::lay_out(theme, page, b.level);
            let low = laid.boxes.iter().map(|r| r.y1).fold(0.0, f64::max);
            if low > bottom + 0.5 {
                small.push(format!("{} page {}: runs off the paper", b.id, i + 1));
            } else if laid.size < least {
                small.push(format!("{} page {}: {} pt", b.id, i + 1, laid.size));
            }
            smallest = smallest.min(laid.size);
        }
        eprintln!("{:32} level {} smallest {smallest} pt", b.id, b.level);
    }
    assert!(small.is_empty(), "too small for their level: {small:#?}");
}
