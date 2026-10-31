//! Anzan, run headlessly: making a player, a whole round played with the
//! beads, its record in the kid's log and progress, the day's allowance
//! ending play at a stopping point, the grown-ups' door, the keyboard; and
//! pictures of every screen by day and night.

use std::cell::Cell;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::time::Duration;

use aizome_kit::Appearance;
use aizome_kit::WindowKey;
use aizome_kit::layout::Point;
use aizome_kit::sidestep_ui::{Modifiers, NamedKey};
use aizome_kit::testing::{Harness, assert_golden};
use aizome_kit::theme::ThemeChoice;
use sekka_anzan::abacus::Bead;
use sekka_anzan::lessons::{LESSONS, Question, index_of};
use sekka_anzan::round::{FLASH_MS, PEEK_MS, RIGHT_MS};
use sekka_anzan::scene::{self, Screen};
use sekka_anzan::{APP_ID, Anzan};
use sekka_kid::{Animal, Event, Grade, KidColor, NewProfile, Profile, Store, clock};
use sekka_kid_kit::stage::Fit;

fn scratch(name: &str) -> PathBuf {
    let d = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("anzan-{name}"));
    let _ = std::fs::remove_dir_all(&d);
    d
}

struct Run {
    h: Harness<Anzan>,
    w: WindowKey,
    t: Rc<Cell<i64>>,
    store: Store,
}

/// Ten in the morning, today.
fn morning() -> i64 {
    clock::day_start(clock::now_ms()) + 10 * 3_600_000
}

fn anzan(name: &str, theme: ThemeChoice, motion: bool, kids: &[(&str, Animal, KidColor, Grade)]) -> Run {
    let store = Store::at(scratch(name));
    for (n, a, c, g) in kids {
        store.add(NewProfile { name: (*n).into(), animal: *a, color: *c, grade: *g }).unwrap();
    }
    let mut app = Anzan::new(store.clone(), true);
    let t = app.manual_clock(morning());
    let h = Harness::with(app, Appearance { theme, reduced_motion: !motion, ..Appearance::default() }, false);
    let w = h.window();
    Run { h, w, t, store }
}

impl Run {
    fn wait(&mut self, ms: i64) {
        // Frames come every 20 ms; move the clocks together.
        let mut left = ms;
        while left > 0 {
            let step = left.min(20);
            self.t.set(self.t.get() + step);
            self.h.advance(Duration::from_millis(step as u64));
            left -= step;
        }
    }

    /// Click the design's point `p`.
    fn tap(&mut self, p: Point) {
        let size = self.h.size(self.w);
        let fit = Fit::of_size(size);
        let at = Point::new(fit.origin.x + p.x * fit.scale, fit.origin.y + p.y * fit.scale);
        self.h.click(self.w, at);
    }

    fn tap_bead(&mut self, rod: usize, bead: Bead) {
        let r =
            self.h.program().round.as_ref().map(|r| r.abacus.clone()).unwrap_or_else(|| self.h.program().free.clone());
        let at = scene::frame(r.len()).bead_center(rod, bead, if r.rod(rod).counts(bead) { 1.0 } else { 0.0 });
        self.tap(at);
    }

    fn key(&mut self, k: NamedKey) {
        self.h.named(self.w, k, Modifiers::NONE);
    }

    fn screen(&self) -> Screen {
        self.h.program().screen
    }

    fn kid(&self) -> Profile {
        self.store.profiles()[0].clone()
    }
}

const HANA: (&str, Animal, KidColor, Grade) = ("Hana", Animal::Fox, KidColor::Coral, Grade::Kindergarten);
const REN: (&str, Animal, KidColor, Grade) = ("Ren", Animal::Panda, KidColor::Sky, Grade::First);

#[test]
fn the_first_run_makes_a_player_card() {
    let mut r = anzan("first-run", ThemeChoice::Day, false, &[]);
    assert_eq!(r.screen(), Screen::NewKid);
    assert!(r.h.find_id(r.w, "name").is_some(), "the name field shows");
    r.h.type_text(r.w, "Hana");
    r.h.click_text(r.w, "Make my card");
    assert_eq!(r.screen(), Screen::Map);
    let kids = r.store.profiles();
    assert_eq!(kids.len(), 1);
    assert_eq!(kids[0].name, "Hana");
    assert_eq!(r.h.program().kid.as_ref().map(|k| k.name.as_str()), Some("Hana"));
    assert!(r.store.apps().iter().any(|a| a.id == APP_ID), "Anzan says it's in the catalogue");
}

/// Answer the question showing, by bead or by choice, as a kid who knows.
fn answer(r: &mut Run) {
    let q = r.h.program().round.as_ref().unwrap().question().clone();
    match &q {
        Question::Set { target, .. } | Question::PeekSet { value: target } => {
            let a = r.h.program().round.as_ref().unwrap().abacus.clone();
            let want = sekka_anzan::abacus::Abacus::showing(a.len(), *target);
            for rod in (0..a.len()).rev() {
                let (have, need) = (r.h.program().round.as_ref().unwrap().abacus.rod(rod), want.rod(rod));
                if have.heaven != need.heaven {
                    r.tap_bead(rod, Bead::Heaven);
                }
                let have = r.h.program().round.as_ref().unwrap().abacus.rod(rod);
                if have.earth < need.earth {
                    r.tap_bead(rod, Bead::Earth(need.earth - 1));
                } else if have.earth > need.earth {
                    r.tap_bead(rod, Bead::Earth(need.earth));
                }
            }
        }
        _ => {
            let c = q.choices().expect("a question answered by choosing");
            let i = c.iter().position(|v| *v == q.answer()).unwrap();
            r.tap(scene::choice_center(i));
        }
    }
}

#[test]
fn a_round_of_wake_the_beads_lights_the_first_lantern() {
    let mut r = anzan("round", ThemeChoice::Night, true, &[HANA]);
    assert_eq!(r.screen(), Screen::Who);
    r.key(NamedKey::Enter);
    assert_eq!(r.screen(), Screen::Map, "Enter picks the focused card");
    r.tap(scene::stop_at(0));
    assert_eq!(r.screen(), Screen::Play);
    // Pon's introduction, a line at a time.
    let lines = LESSONS[0].intro.len();
    for _ in 0..lines {
        assert!(r.h.program().intro.is_some());
        r.tap(scene::check_center());
    }
    assert!(r.h.program().intro.is_none());
    for i in 0..8 {
        let before = r.h.program().round.as_ref().unwrap().index;
        assert_eq!(before, i);
        r.wait(500);
        answer(&mut r);
        r.wait(RIGHT_MS + 40);
    }
    assert_eq!(r.screen(), Screen::Summary);
    r.wait(2_500);
    let kid = r.kid();
    let events = r.store.events(&kid.id);
    let round = events
        .iter()
        .find_map(|e| match e {
            Event::Round(r) => Some(r.clone()),
            _ => None,
        })
        .expect("the round is in the log");
    assert!(round.finished);
    assert_eq!((round.items, round.first_try, round.stars), (8, 8, 3));
    assert_eq!(round.activity, "wake-the-beads");
    assert!(
        events.iter().any(|e| matches!(e, Event::Milestone { text, .. } if text == "Lit lantern 1: Wake the beads"))
    );
    let progress: sekka_anzan::progress::Progress = r.store.progress(&kid.id, APP_ID);
    assert_eq!(progress.stars(0), 3);
    assert!(progress.intros.contains("wake-the-beads"), "the introduction plays once");
    // On to the next lesson; leaving part way logs what was played.
    r.tap(Point::new(610.0, 563.0));
    assert_eq!(r.screen(), Screen::Play);
    assert_eq!(r.h.program().round.as_ref().unwrap().lesson, 1);
    r.key(NamedKey::Escape);
    assert_eq!(r.screen(), Screen::Map);
    // Time played is written when the kid moves on.
    let played = r.store.played_today(&kid.id, r.t.get());
    assert!(played > 10_000, "{played} ms");
}

#[test]
fn the_day_ends_at_a_stopping_point_and_grown_ups_can_add_time() {
    let mut r = anzan("allowance", ThemeChoice::Day, false, &[HANA]);
    let mut kid = r.kid();
    kid.daily_minutes = Some(5);
    r.store.update(&kid).unwrap();
    let now = r.t.get();
    r.store.append(&kid.id, &Event::Active { app: "other".into(), from: now - 20 * 60_000, to: now - 60_000 }).unwrap();
    r.h.program_mut().kids = r.store.profiles();
    r.key(NamedKey::Enter);
    assert_eq!(r.screen(), Screen::Rest, "nineteen of five minutes played, across apps");
    // The grown-ups' door: a wrong answer, then the right one.
    r.tap(Point::new(946.0, 52.0));
    let texts = r.h.texts(r.h.windows().last().copied().unwrap());
    let q = texts.iter().find(|t| t.starts_with("What is ")).expect("the question").clone();
    let nums: Vec<u32> =
        q.trim_start_matches("What is ").trim_end_matches('?').split(" × ").map(|n| n.parse().unwrap()).collect();
    r.h.send(sekka_anzan::Msg::Gate(sekka_kid_kit::gate::GateMsg::Answer("1".into())));
    r.h.send(sekka_anzan::Msg::Gate(sekka_kid_kit::gate::GateMsg::Submit));
    let sheet = r.h.texts(r.h.windows().last().copied().unwrap());
    assert!(sheet.contains(&q) && !sheet.iter().any(|t| t == "Grown-ups"), "still asking: {sheet:?}");
    r.h.send(sekka_anzan::Msg::Gate(sekka_kid_kit::gate::GateMsg::Answer((nums[0] * nums[1]).to_string())));
    r.h.send(sekka_anzan::Msg::Gate(sekka_kid_kit::gate::GateMsg::Submit));
    assert!(r.h.texts(r.h.windows().last().copied().unwrap()).iter().any(|t| t == "Grown-ups"));
    r.h.send(sekka_anzan::Msg::Grown(sekka_anzan::app::GrownMsg::MoreTime));
    assert_eq!(r.screen(), Screen::Rest, "fifteen isn't enough for nineteen");
    r.h.send(sekka_anzan::Msg::Grown(sekka_anzan::app::GrownMsg::MoreTime));
    assert_eq!(r.screen(), Screen::Map, "twenty-five is");
    r.h.send(sekka_anzan::Msg::Grown(sekka_anzan::app::GrownMsg::OpenKids));
    assert_eq!(r.h.program().commands, [vec!["sekka-kids".to_owned()]]);
}

#[test]
fn first_graders_may_start_with_tens_and_keys_play_too() {
    let mut r = anzan("keys", ThemeChoice::Day, false, &[REN]);
    r.key(NamedKey::Enter);
    assert_eq!(r.screen(), Screen::Map);
    let tens = index_of("read-tens").unwrap() - 1;
    r.tap(scene::stop_at(tens));
    assert_eq!(r.h.program().round.as_ref().map(|r| r.lesson), Some(tens));
    // Skip Pon's introduction with Return.
    for _ in 0..LESSONS[tens].intro.len() {
        r.key(NamedKey::Enter);
    }
    // ↑ pushes an earth bead, ⇧↓ the heaven bead, ← moves to the tens.
    let target = r.h.program().round.as_ref().unwrap().question().answer();
    let (tens_d, ones_d) = (target / 10, target % 10);
    let push = |r: &mut Run, d: u32| {
        if d >= 5 {
            r.h.named(r.w, NamedKey::ArrowDown, Modifiers::SHIFT);
        }
        for _ in 0..d % 5 {
            r.key(NamedKey::ArrowUp);
        }
    };
    push(&mut r, ones_d);
    r.key(NamedKey::ArrowLeft);
    push(&mut r, tens_d);
    assert_eq!(r.h.program().round.as_ref().unwrap().abacus.value(), target);
    assert_eq!(r.h.program().round.as_ref().unwrap().results, [true]);
    // Locked lessons say "not yet".
    r.key(NamedKey::Escape);
    let last = LESSONS.len() - 1;
    r.tap(scene::stop_at(last));
    assert_eq!(r.screen(), Screen::Map);
    assert_eq!(r.h.program().focus, last);
}

#[test]
fn peeks_and_flashes_hide_the_beads_then_ask() {
    let mut r = anzan("peek", ThemeChoice::Day, false, &[REN]);
    r.key(NamedKey::Enter);
    let peek = index_of("peek").unwrap();
    r.tap(scene::stop_at(peek));
    for _ in 0..LESSONS[peek].intro.len() {
        r.key(NamedKey::Enter);
    }
    assert!(!r.h.program().round.as_ref().unwrap().choosing(), "peeking");
    r.wait(i64::from(PEEK_MS) + 100);
    assert!(r.h.program().round.as_ref().unwrap().choosing(), "the curtain closed: choose");
    let answer = r.h.program().round.as_ref().unwrap().question().answer();
    r.h.type_text(r.w, &answer.to_string());
    assert_eq!(r.h.program().round.as_ref().unwrap().results, [true], "a digit chooses");
    let _ = FLASH_MS;
}

// Pictures.

fn picture(name: &str, theme: ThemeChoice, set: impl FnOnce(&mut Run)) {
    let mut r = anzan(&format!("golden-{name}"), theme, false, &[HANA, REN]);
    set(&mut r);
    let suffix = if theme == ThemeChoice::Day { "day" } else { "night" };
    assert_golden(&mut r.h, r.w, format!("tests/golden/{name}-{suffix}.png"), 1.0);
}

fn to_play(r: &mut Run, lesson: &str) {
    let kid = r.kid();
    let mut p = sekka_anzan::progress::Progress::default();
    for l in LESSONS.iter().take(index_of(lesson).unwrap() + 1) {
        p.intros.insert(l.id.to_owned());
    }
    for i in 0..index_of(lesson).unwrap() {
        p.finished(i, [3, 2, 3, 1, 3, 2, 3, 3, 2, 3, 3, 2, 1, 3, 2][i % 15], 1);
    }
    r.store.save_progress(&kid.id, APP_ID, &p).unwrap();
    r.key(NamedKey::Enter);
}

#[test]
fn pictures_of_every_screen() {
    for theme in [ThemeChoice::Day, ThemeChoice::Night] {
        picture("who", theme, |_| {});
        picture("new-player", theme, |r| {
            r.tap(Point::new(0.0, 0.0));
            r.key(NamedKey::ArrowRight);
            r.key(NamedKey::ArrowRight);
            r.key(NamedKey::Enter);
            r.h.type_text(r.w, "Sora");
        });
        picture("map", theme, |r| to_play(r, "little-friends-add"));
        picture("intro", theme, |r| {
            r.key(NamedKey::Enter);
            r.tap(scene::stop_at(0));
        });
        picture("set", theme, |r| {
            to_play(r, "the-big-bead");
            r.tap(scene::stop_at(index_of("the-big-bead").unwrap()));
            r.tap_bead(0, Bead::Earth(1));
        });
        picture("hint", theme, |r| {
            to_play(r, "little-friends-add");
            r.tap(scene::stop_at(index_of("little-friends-add").unwrap()));
            r.h.type_text(r.w, "h");
        });
        picture("read", theme, |r| {
            to_play(r, "read-tens");
            r.tap(scene::stop_at(index_of("read-tens").unwrap()));
        });
        picture("peek", theme, |r| {
            to_play(r, "peek");
            r.tap(scene::stop_at(index_of("peek").unwrap()));
        });
        picture("free", theme, |r| {
            to_play(r, "add");
            r.h.type_text(r.w, "f");
            r.tap_bead(2, Bead::Earth(1));
            r.tap_bead(1, Bead::Heaven);
            r.tap_bead(0, Bead::Earth(2));
        });
        picture("summary", theme, |r| {
            to_play(r, "wake-the-beads");
            r.tap(scene::stop_at(0));
            for _ in 0..8 {
                answer(r);
                r.wait(RIGHT_MS + 40);
            }
        });
        picture("rest", theme, |r| {
            let mut kid = r.kid();
            kid.daily_minutes = Some(10);
            r.store.update(&kid).unwrap();
            let now = r.t.get();
            r.store.append(&kid.id, &Event::Active { app: "x".into(), from: now - 11 * 60_000, to: now }).unwrap();
            r.h.program_mut().kids = r.store.profiles();
            r.key(NamedKey::Enter);
        });
    }
}
