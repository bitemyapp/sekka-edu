//! Anzan, the program: who's playing, the lantern path, rounds, free play,
//! and the day's rest; the beads' slides and cheers, sounds and Pon's
//! voice; and the kid's time and rounds written to their Sekka Kid log.

use std::cell::Cell;
use std::os::unix::net::UnixListener;
use std::rc::Rc;
use std::time::Duration;

use aizome_kit::layout::{Point, Size};
use aizome_kit::prelude::*;
use aizome_kit::sidestep_ui::{Key, KeyEvent, NamedKey};
use aizome_kit::widget::{Event, FocusRing};
use aizome_kit::{row, stack};
use sekka_kid::{AppEntry, Event as LogEvent, Profile, Store, Tracker, clock};
use sekka_kid_kit::gate::{self, Gate, GateMsg};
use sekka_kid_kit::newkid::{self, NewKid, NewKidMsg};
use sekka_kid_kit::stage::{self, Fit};
use sekka_kid_kit::who::{Card, Who, WhoHit};
use sekka_kid_kit::{Look, instance};

use crate::abacus::{Abacus, Bead, Rod};
use crate::lessons::{self, LESSONS, Question, Rng};
use crate::progress::Progress;
use crate::round::{self, Feedback, Hint, Phase, Round, words};
use crate::scene::{self, Beads, Hit, PlayStage, PlayView, Scene, Screen, Stop, SummaryView};
use crate::sound::{Sound, Sounds};
use sekka_voice::{Kind, Voice};

pub const APP_ID: &str = "computer.sekka.Anzan";

/// A frame of the beads' slides and cheers.
pub const FRAME: Duration = Duration::from_millis(20);
const SLIDE_MS: i64 = 170;
const HOP_MS: i64 = 650;
const CURTAIN_MS: i64 = 300;
const PULSE_MS: i64 = 2400;
const STAR_GAP_MS: i64 = 450;
const STAR_POP_MS: i64 = 320;
const CONFETTI_MS: i64 = 2400;
const BLINK_EVERY: Duration = Duration::from_millis(3700);
const BLINK_MS: Duration = Duration::from_millis(160);
const FLUSH_EVERY: Duration = Duration::from_secs(30);
/// Digits typed this close together are one number.
const TYPING_MS: i64 = 1200;

const PRAISE: [&str; 6] = ["Yes!", "Great!", "You got it!", "Super!", "Hooray!", "Nice!"];

/// What Pon says to ask `q` (peeks and flashes say more as they go).
fn ask(q: &Question) -> String {
    match q {
        Question::Set { from: 0, target } => format!("Show me {}.", words(*target)),
        Question::Set { target, .. } => format!("Now make {}.", words(*target)),
        Question::Read { .. } => "What number is this?".to_owned(),
        Question::PeekRead { .. } => "What did you see?".to_owned(),
        Question::PeekSet { .. } => "Now make it!".to_owned(),
        Question::Sum { start, op, n } | Question::Ghost { start, op, n, .. } => {
            let op = if *op == lessons::Op::Add { "plus" } else { "take away" };
            format!("{} {op} {}.", words(*start), words(*n))
        }
        Question::Flash { .. } => "How many altogether?".to_owned(),
    }
}

#[derive(Clone, Debug)]
pub enum Msg {
    Down(Point, Size),
    Up(Point, Size),
    Key(KeyEvent),
    Tick,
    Blink(bool),
    Flush,
    /// Say the free-play number, if it's still what shows.
    SayFree(u32),
    NewKid(NewKidMsg),
    Gate(GateMsg),
    Grown(GrownMsg),
    Raise,
}

/// The grown-ups' sheet.
#[derive(Clone, Debug, PartialEq)]
pub enum GrownMsg {
    Sound(bool),
    Voice(bool),
    MoreTime,
    SwitchPlayer,
    OpenKids,
    Done,
}

#[derive(Clone, Debug, PartialEq)]
enum Sheet {
    Gate(Gate),
    Grown,
}

/// The summary's moment.
#[derive(Clone, Debug)]
struct Summary {
    lesson: usize,
    stars: u8,
    first_try: u32,
    items: u32,
    at: i64,
}

pub struct Anzan {
    pub store: Store,
    /// The time, in Unix milliseconds (a test's own clock in tests).
    pub clock: Rc<dyn Fn() -> i64>,
    /// No sounds, voice or other programs (tests).
    pub quiet: bool,
    /// Programs started (Sekka Kids), for tests.
    pub commands: Vec<Vec<String>>,
    pub instance: Option<UnixListener>,
    pub window: WindowKey,
    sheet: Option<(WindowKey, Sheet)>,
    pub screen: Screen,
    pub kids: Vec<Profile>,
    pub kid: Option<Profile>,
    pub progress: Progress,
    tracker: Option<Tracker>,
    who_focus: usize,
    pub newkid: NewKid,
    pub focus: usize,
    pub round: Option<Round>,
    /// Pon's introduction: the line showing.
    pub intro: Option<usize>,
    summary: Option<Summary>,
    pub free: Abacus,
    /// The rod the keyboard is on.
    rod: Option<usize>,
    /// The beads as last drawn, and each rod's slide: from what, since when.
    shown: Abacus,
    slides: Vec<Option<(Rod, i64)>>,
    cheer_at: Option<i64>,
    hop_at: Option<i64>,
    curtain_at: Option<i64>,
    hint_at: Option<i64>,
    phase_at: i64,
    /// The praise showing, and where this round's run of praises starts.
    praise: usize,
    praise_from: usize,
    down: Option<(Option<Hit>, Point)>,
    pressed: Option<Hit>,
    typed: (String, i64),
    ticker: Option<TimerHandle>,
    blink: bool,
    blinking: bool,
    motion: bool,
    /// Night (the theme the view can't ask for).
    dark: bool,
    sounds: Sounds,
    voice: Voice,
    rng: Rng,
}

impl Anzan {
    pub fn new(store: Store, quiet: bool) -> Anzan {
        let clock: Rc<dyn Fn() -> i64> = Rc::new(clock::now_ms);
        let seed = clock() as u64 ^ u64::from(std::process::id());
        Anzan {
            store,
            clock,
            quiet,
            commands: Vec::new(),
            instance: None,
            window: WindowKey::NONE,
            sheet: None,
            screen: Screen::Who,
            kids: Vec::new(),
            kid: None,
            progress: Progress::default(),
            tracker: None,
            who_focus: 0,
            newkid: NewKid::new(0),
            focus: 0,
            round: None,
            intro: None,
            summary: None,
            free: Abacus::new(3),
            rod: None,
            shown: Abacus::new(1),
            slides: Vec::new(),
            cheer_at: None,
            hop_at: None,
            curtain_at: None,
            hint_at: None,
            phase_at: 0,
            praise: 0,
            praise_from: 0,
            down: None,
            pressed: None,
            typed: (String::new(), 0),
            ticker: None,
            blink: false,
            blinking: false,
            motion: true,
            dark: true,
            sounds: Sounds::new(true, quiet),
            voice: Voice::new(true, quiet),
            rng: Rng::new(seed),
        }
    }

    /// A clock a test moves by hand.
    pub fn manual_clock(&mut self, start: i64) -> Rc<Cell<i64>> {
        // And the same questions every run, whatever the day.
        self.rng = Rng::new(0x5e66_a0ed);
        let t = Rc::new(Cell::new(start));
        let read = t.clone();
        self.clock = Rc::new(move || read.get());
        t
    }

    fn now(&self) -> i64 {
        (self.clock)()
    }

    fn grade(&self) -> sekka_kid::Grade {
        self.kid.as_ref().map_or(sekka_kid::Grade::Kindergarten, |k| k.grade)
    }

    // The kid.

    fn reload_kids(&mut self) {
        self.kids = self.store.profiles();
        self.voice.prepare(self.kids.iter().map(|k| format!("Hi, {}!", k.name)));
        self.who_focus = self.who_focus.min(self.kids.len());
    }

    fn choose_kid(&mut self, i: usize, cx: &mut Cx<Msg>) {
        let Some(kid) = self.kids.get(i).cloned() else { return };
        self.leave_kid();
        self.progress = self.store.progress(&kid.id, APP_ID);
        self.sounds.on = self.progress.sound;
        self.voice.on = self.progress.voice;
        self.tracker = Some(Tracker::new(self.store.clone(), &kid.id, APP_ID));
        self.focus = self.progress.here(kid.grade);
        self.voice.say(format!("Hi, {}!", kid.name));
        self.kid = Some(kid);
        self.touch();
        if self.out_of_time() {
            self.go(Screen::Rest, cx);
        } else {
            self.go(Screen::Map, cx);
            self.hop_at = Some(self.now());
        }
    }

    /// Done with the kid playing: their time and any round left part way go
    /// to their log.
    fn leave_kid(&mut self) {
        self.leave_round();
        let now = self.now();
        if let Some(t) = self.tracker.as_mut() {
            t.finish(now);
        }
        self.tracker = None;
        self.kid = None;
    }

    fn touch(&mut self) {
        let now = self.now();
        if let Some(t) = self.tracker.as_mut() {
            t.touch(now);
        }
    }

    fn save(&self) {
        if let Some(kid) = &self.kid
            && let Err(e) = self.store.save_progress(&kid.id, APP_ID, &self.progress)
        {
            log::warn!("{}: {e}", self.store.root().display());
        }
    }

    fn log(&self, event: &LogEvent) {
        if let Some(kid) = &self.kid
            && let Err(e) = self.store.append(&kid.id, event)
        {
            log::warn!("{}: {e}", self.store.root().display());
        }
    }

    /// Minutes the kid may still play today, if a grown-up set a limit.
    pub fn minutes_left(&self) -> Option<f64> {
        let kid = self.kid.as_ref()?;
        let limit = kid.daily_minutes?;
        let now = self.now();
        let extra = self.progress.extra_minutes(clock::day_start(now));
        let played = self.store.played_today(&kid.id, now) + self.tracker.as_ref().map_or(0, |t| t.unwritten(now));
        Some(f64::from(limit + extra) - played as f64 / 60_000.0)
    }

    fn out_of_time(&self) -> bool {
        self.minutes_left().is_some_and(|m| m <= 0.0)
    }

    // Screens.

    fn go(&mut self, screen: Screen, cx: &mut Cx<Msg>) {
        let now = self.now();
        if let Some(t) = self.tracker.as_mut() {
            t.flush(now);
        }
        self.screen = screen;
        self.rod = None;
        self.pressed = None;
        self.down = None;
        match screen {
            Screen::Free => self.show(self.free.clone(), false),
            Screen::Play => {}
            _ => {}
        }
        if screen == Screen::NewKid {
            cx.focus("name");
        } else {
            cx.focus("stage");
        }
    }

    // Rounds.

    fn start_lesson(&mut self, lesson: usize, cx: &mut Cx<Msg>) {
        if self.out_of_time() {
            self.go(Screen::Rest, cx);
            self.voice.say("All done for today! The beads are sleepy.");
            return;
        }
        let now = self.now();
        let l = &LESSONS[lesson];
        let round = Round::new(lesson, lessons::questions(lesson, &mut self.rng), now, self.progress.peek_ms);
        self.shown = Abacus::new(l.rods);
        self.slides = vec![None; l.rods];
        self.round = Some(round);
        self.focus = lesson;
        self.summary = None;
        self.cheer_at = None;
        self.hint_at = None;
        self.curtain_at = None;
        self.intro = (!l.intro.is_empty() && !self.progress.intros.contains(l.id)).then_some(0);
        self.praise_from = (now / 1000) as usize % PRAISE.len();
        self.prepare_round();
        self.go(Screen::Play, cx);
        match self.intro {
            Some(_) => self.voice.say(l.intro[0]),
            None => {
                if let Some(r) = self.round.as_mut() {
                    r.begin(now);
                }
                self.announce();
            }
        }
        self.sync_beads();
    }

    fn intro_next(&mut self) {
        let Some(i) = self.intro else { return };
        let Some(r) = &self.round else { return };
        let l = &LESSONS[r.lesson];
        if i + 1 < l.intro.len() {
            self.intro = Some(i + 1);
            self.voice.say(l.intro[i + 1]);
        } else {
            self.intro = None;
            self.progress.intros.insert(l.id.to_owned());
            self.save();
            let now = self.now();
            if let Some(r) = self.round.as_mut() {
                r.begin(now);
            }
            self.announce();
        }
        self.sounds.play(Sound::Tap);
    }

    /// Say the question as it now stands.
    fn announce(&mut self) {
        let Some(r) = &self.round else { return };
        let q = r.question();
        let text = match (&r.phase, q) {
            (Phase::Peeking { .. }, _) => "Peek!".to_owned(),
            (Phase::Flashing { shown, gap: false, .. }, Question::Flash { numbers, .. }) => words(numbers[*shown]),
            (Phase::Flashing { .. }, _) => return,
            (_, q) => ask(q),
        };
        self.voice.say(text);
    }

    /// Praise for the `i`th answer of the round: each round runs through
    /// them all from a place of its own, so it can be made ahead.
    fn praise_for(&self, i: usize) -> usize {
        (self.praise_from + i) % PRAISE.len()
    }

    /// Have Pon's lines for the round made while the kid listens and
    /// thinks: the introduction, each question, its praise, and the end.
    fn prepare_round(&self) {
        let Some(r) = &self.round else { return };
        let l = &LESSONS[r.lesson];
        let mut lines: Vec<String> = Vec::new();
        if self.intro.is_some() {
            lines.extend(l.intro.iter().map(|s| (*s).to_owned()));
        }
        for (i, q) in r.questions.iter().enumerate() {
            if let Question::Flash { numbers, .. } = q {
                lines.extend(numbers.iter().map(|n| words(*n)));
            }
            lines.push(ask(q));
            lines.push(format!("{} {}!", PRAISE[self.praise_for(i)], words(q.answer())));
        }
        let lantern = words(r.lesson as u32 + 1);
        lines.push(format!("Hooray! Three stars! You lit lantern {lantern}!"));
        lines.push(format!("You did it! You lit lantern {lantern}!"));
        self.voice.prepare(lines);
    }

    /// Do `f` to the round, then make the sounds, slides and words it
    /// calls for.
    fn act(&mut self, f: impl FnOnce(&mut Round, i64) -> Feedback, cx: &mut Cx<Msg>) {
        let now = self.now();
        let Some(r) = self.round.as_mut() else { return };
        let before = (r.index, std::mem::discriminant(&r.phase), r.phase.clone(), r.hint.clone());
        let fb = f(r, now);
        let after = (r.index, std::mem::discriminant(&r.phase));
        let hint = r.hint.clone();
        let phase = r.phase.clone();
        let done = r.is_done();
        match fb {
            Feedback::Right => {
                self.praise = self.praise_for(self.round.as_ref().map_or(0, |r| r.index));
                self.sounds.play(Sound::Right);
                let answer = self.round.as_ref().map_or(0, |r| r.question().answer());
                self.voice.say(format!("{} {}!", PRAISE[self.praise], words(answer)));
                self.cheer_at = Some(now);
            }
            Feedback::Miss => {
                self.sounds.play(Sound::Miss);
                match &hint {
                    Some(Hint::Push { words, .. }) => self.voice.say(format!("Almost! {words}.")),
                    _ => self.voice.say("Almost! Try again."),
                }
                self.hint_at = Some(now);
            }
            Feedback::Shown => {
                self.sounds.play(Sound::Miss);
                self.voice.say("Watch the beads.");
            }
            Feedback::Moved | Feedback::Nothing => {}
        }
        if hint != before.3 && hint.is_some() {
            self.hint_at = Some(now);
        }
        if (after.0 != before.0 || after.1 != before.1) && !done {
            self.phase_at = now;
            match (&before.2, &phase) {
                (Phase::Peeking { .. }, Phase::Asking) | (Phase::Flashing { .. }, Phase::Asking) => {
                    self.curtain_at = Some(now);
                    self.announce();
                }
                (_, Phase::Flashing { gap: false, .. }) => self.announce(),
                (
                    Phase::Right { .. } | Phase::Showing { .. },
                    Phase::Asking | Phase::Peeking { .. } | Phase::Flashing { .. },
                ) => {
                    self.cheer_at = None;
                    self.curtain_at = None;
                    self.announce();
                }
                _ => {}
            }
        }
        self.sync_beads();
        if done {
            self.finish_round(cx);
        }
    }

    /// The beads the screen shows changed: slide the rods that differ.
    fn sync_beads(&mut self) {
        let target = match self.screen {
            Screen::Free => self.free.clone(),
            _ => match &self.round {
                Some(r) => r.abacus.clone(),
                None => return,
            },
        };
        self.show(target, true);
    }

    fn show(&mut self, target: Abacus, sounds: bool) {
        let now = self.now();
        if self.shown.len() != target.len() {
            self.shown = target.clone();
            self.slides = vec![None; target.len()];
            return;
        }
        let mut up = false;
        let mut down = false;
        for i in 0..target.len() {
            let (a, b) = (self.shown.rod(i), target.rod(i));
            if a != b {
                self.slides[i] = self.motion.then_some((a, now));
                up |= b.value() > a.value() || (b.heaven && !a.heaven) || b.earth > a.earth;
                down |= b.earth < a.earth || (a.heaven && !b.heaven);
            }
        }
        if sounds {
            if up {
                self.sounds.play(Sound::Clack);
            } else if down {
                self.sounds.play(Sound::Soft);
            }
        }
        self.shown = target;
    }

    fn finish_round(&mut self, cx: &mut Cx<Msg>) {
        let now = self.now();
        let Some(r) = self.round.take() else { return };
        self.log(&LogEvent::Round(r.record(APP_ID, now)));
        let stars = r.stars();
        let first = !self.progress.done(r.lesson);
        let best = self.progress.stars(r.lesson);
        self.progress.finished(r.lesson, stars, now);
        // Peeks that got shorter stay short.
        if matches!(LESSONS[r.lesson].kind, lessons::Kind::PeekRead | lessons::Kind::PeekSet) {
            self.progress.peek_ms = r.peek_ms;
        }
        self.save();
        let title = LESSONS[r.lesson].title;
        if first {
            self.log(&LogEvent::Milestone {
                app: APP_ID.into(),
                at: now,
                text: format!("Lit lantern {}: {title}", r.lesson + 1),
            });
        } else if stars == 3 && best < 3 {
            self.log(&LogEvent::Milestone { app: APP_ID.into(), at: now, text: format!("Three stars on {title}") });
        }
        self.summary = Some(Summary {
            lesson: r.lesson,
            stars,
            first_try: r.first_try(),
            items: r.questions.len() as u32,
            at: now,
        });
        self.sounds.play(Sound::Fanfare);
        self.voice.say(match stars {
            3 => format!("Hooray! Three stars! You lit lantern {}!", words(r.lesson as u32 + 1)),
            _ => format!("You did it! You lit lantern {}!", words(r.lesson as u32 + 1)),
        });
        self.go(Screen::Summary, cx);
    }

    /// Leave a round part way: what was played goes to the log.
    fn leave_round(&mut self) {
        let now = self.now();
        if let Some(r) = self.round.take()
            && (!r.results.is_empty() || r.pushes > 0 || r.chosen.is_some())
        {
            self.log(&LogEvent::Round(r.record(APP_ID, now)));
        }
        self.intro = None;
        self.voice.hush();
    }

    // Input.

    fn scene_size_hit(&self, size: Size, p: Point) -> Option<Hit> {
        scene::hit(size, &self.scene(), p)
    }

    fn pointer_down(&mut self, p: Point, size: Size) {
        let hit = self.scene_size_hit(size, p);
        self.pressed = hit.filter(|h| !matches!(h, Hit::Bead(..)));
        self.down = Some((hit, p));
        if let Some(Hit::Who(w)) = hit {
            self.who_focus = match w {
                WhoHit::Kid(i) => i,
                WhoHit::New => self.kids.len(),
            };
        }
    }

    fn pointer_up(&mut self, p: Point, size: Size, cx: &mut Cx<Msg>) {
        let Some((down, from)) = self.down.take() else { return };
        self.pressed = None;
        let hit = self.scene_size_hit(size, p);
        match down {
            Some(Hit::Bead(rod, bead)) => match scene::drag_direction(size, from, p) {
                // Dragged: toward the beam or home, as the finger went.
                Some(up) => {
                    let to_beam = match bead {
                        Bead::Heaven => !up,
                        Bead::Earth(_) => up,
                    };
                    self.bead(rod, bead, Some(to_beam), cx);
                }
                None if hit == down => self.bead(rod, bead, None, cx),
                None => {}
            },
            Some(h) if hit == Some(h) => self.activate(h, cx),
            _ => {}
        }
    }

    /// Push a bead: `to_beam`, or flicked (`None`).
    fn bead(&mut self, rod: usize, bead: Bead, to_beam: Option<bool>, cx: &mut Cx<Msg>) {
        match self.screen {
            Screen::Free => {
                let moved = match to_beam {
                    Some(t) => self.free.push(rod, bead, t),
                    None => self.free.flick(rod, bead),
                };
                if moved {
                    self.sync_beads();
                    cx.after(Duration::from_millis(650), Msg::SayFree(self.free.value()));
                }
            }
            Screen::Play if self.intro.is_none() => {
                self.act(
                    |r, now| match to_beam {
                        Some(t) => r.push(rod, bead, t, now),
                        None => r.flick(rod, bead, now),
                    },
                    cx,
                );
            }
            _ => {}
        }
    }

    fn activate(&mut self, h: Hit, cx: &mut Cx<Msg>) {
        match h {
            Hit::Who(WhoHit::Kid(i)) => {
                self.sounds.play(Sound::Tap);
                self.choose_kid(i, cx);
            }
            Hit::Who(WhoHit::New) => {
                self.newkid = NewKid::new(self.kids.len());
                self.go(Screen::NewKid, cx);
            }
            Hit::Kid | Hit::SwitchPlayer => {
                self.leave_kid();
                self.reload_kids();
                self.go(Screen::Who, cx);
            }
            Hit::Free => {
                self.sounds.play(Sound::Tap);
                self.go(Screen::Free, cx);
                self.voice.say("Play with the beads!");
                // What the beads come to, as they're pushed: the numbers a
                // kid makes most, made ahead.
                self.voice.prepare((0..=100).map(words));
            }
            Hit::Sound => {
                self.progress.sound = !self.progress.sound;
                self.sounds.on = self.progress.sound;
                self.sounds.play(Sound::Tap);
                self.save();
            }
            Hit::GrownUps => self.open_gate(cx),
            Hit::Stop(i) => {
                if self.progress.unlocked(i, self.grade()) {
                    self.sounds.play(Sound::Tap);
                    self.start_lesson(i, cx);
                } else {
                    self.focus = i;
                    self.sounds.play(Sound::Miss);
                    self.voice.say("Not yet! Light the lanterns before it first.");
                }
            }
            Hit::Home => {
                self.leave_round();
                self.go(Screen::Map, cx);
                if self.out_of_time() {
                    self.go(Screen::Rest, cx);
                }
            }
            Hit::Choice(v) => self.act(|r, now| r.choose(v, now), cx),
            Hit::Check => self.act(|r, now| r.check(now), cx),
            Hit::Hint => self.act(
                |r, _| {
                    r.ask_hint();
                    Feedback::Nothing
                },
                cx,
            ),
            Hit::Restart => self.act(
                |r, _| {
                    r.restart();
                    Feedback::Nothing
                },
                cx,
            ),
            Hit::Next => self.intro_next(),
            Hit::Again => {
                if let Some(s) = &self.summary {
                    let l = s.lesson;
                    self.start_lesson(l, cx);
                }
            }
            Hit::NextLesson => {
                let next = self.summary.as_ref().map(|s| s.lesson + 1).filter(|n| *n < LESSONS.len());
                match next {
                    Some(n) if self.progress.unlocked(n, self.grade()) => self.start_lesson(n, cx),
                    _ => {
                        self.go(Screen::Map, cx);
                        self.focus = self.progress.here(self.grade());
                        self.hop_at = Some(self.now());
                    }
                }
            }
            Hit::Clear => {
                self.free = Abacus::new(self.free.len());
                self.sync_beads();
            }
            Hit::Bead(rod, bead) => self.bead(rod, bead, None, cx),
        }
    }

    fn key(&mut self, k: KeyEvent, cx: &mut Cx<Msg>) {
        let ch = match &k.key {
            Key::Character(s) => s.chars().next(),
            _ => None,
        };
        let named = match &k.key {
            Key::Named(n) => Some(*n),
            _ => None,
        };
        let enter = named == Some(NamedKey::Enter) || ch == Some(' ');
        let shift = k.modifiers.shift();
        match self.screen {
            Screen::Who => {
                let n = self.kids.len() + 1;
                match named {
                    Some(NamedKey::ArrowRight | NamedKey::ArrowDown) => self.who_focus = (self.who_focus + 1) % n,
                    Some(NamedKey::ArrowLeft | NamedKey::ArrowUp) => self.who_focus = (self.who_focus + n - 1) % n,
                    _ if enter => {
                        let h =
                            if self.who_focus < self.kids.len() { WhoHit::Kid(self.who_focus) } else { WhoHit::New };
                        self.activate(Hit::Who(h), cx);
                    }
                    _ => {}
                }
            }
            Screen::Map => {
                let open: Vec<usize> =
                    (0..LESSONS.len()).filter(|i| self.progress.unlocked(*i, self.grade())).collect();
                let at = open.iter().position(|i| *i == self.focus).unwrap_or(0);
                match named {
                    Some(NamedKey::ArrowRight | NamedKey::ArrowDown) => self.focus = open[(at + 1).min(open.len() - 1)],
                    Some(NamedKey::ArrowLeft | NamedKey::ArrowUp) => self.focus = open[at.saturating_sub(1)],
                    Some(NamedKey::Escape) => self.activate(Hit::Kid, cx),
                    _ if enter => self.activate(Hit::Stop(self.focus), cx),
                    _ if ch == Some('f') => self.activate(Hit::Free, cx),
                    _ => {}
                }
            }
            Screen::Play => {
                if named == Some(NamedKey::Escape) {
                    self.activate(Hit::Home, cx);
                } else if self.intro.is_some() {
                    if enter || named == Some(NamedKey::ArrowRight) {
                        self.intro_next();
                    }
                } else if let Some(d) = ch.filter(char::is_ascii_digit) {
                    self.typed_digit(d, cx);
                } else if ch == Some('h') || ch == Some('?') {
                    self.activate(Hit::Hint, cx);
                } else if enter {
                    self.activate(Hit::Check, cx);
                } else {
                    self.bead_key(named, shift, cx);
                }
            }
            Screen::Free => match named {
                Some(NamedKey::Escape) => self.activate(Hit::Home, cx),
                Some(NamedKey::Backspace | NamedKey::Delete) => self.activate(Hit::Clear, cx),
                _ => self.bead_key(named, shift, cx),
            },
            Screen::Summary => match named {
                Some(NamedKey::Escape) => self.activate(Hit::Home, cx),
                _ if enter => self.activate(Hit::NextLesson, cx),
                _ if ch == Some('r') => self.activate(Hit::Again, cx),
                _ => {}
            },
            Screen::Rest | Screen::NewKid => {}
        }
    }

    /// ← → choose a rod; ↑ ↓ push its earth beads; ⇧↓ ⇧↑ its heaven bead.
    fn bead_key(&mut self, named: Option<NamedKey>, shift: bool, cx: &mut Cx<Msg>) {
        let abacus = if self.screen == Screen::Free { self.free.clone() } else { self.shown.clone() };
        let rods = abacus.len();
        let rod = self.rod.unwrap_or(0).min(rods - 1);
        match named {
            Some(NamedKey::ArrowLeft) => self.rod = Some((rod + 1).min(rods - 1)),
            Some(NamedKey::ArrowRight) => self.rod = Some(rod.saturating_sub(1)),
            Some(NamedKey::ArrowUp) | Some(NamedKey::ArrowDown) => {
                self.rod = Some(rod);
                let up = named == Some(NamedKey::ArrowUp);
                let r = abacus.rod(rod);
                let (bead, to_beam) = if shift {
                    (Bead::Heaven, !up)
                } else if up {
                    (Bead::Earth(r.earth.min(3)), true)
                } else {
                    (Bead::Earth(r.earth.saturating_sub(1)), false)
                };
                self.bead(rod, bead, Some(to_beam), cx);
            }
            _ => {}
        }
    }

    fn typed_digit(&mut self, d: char, cx: &mut Cx<Msg>) {
        let Some(choices) = self.round.as_ref().filter(|r| r.choosing()).and_then(|r| r.question().choices()) else {
            return;
        };
        let now = self.now();
        if now - self.typed.1 > TYPING_MS {
            self.typed.0.clear();
        }
        self.typed.0.push(d);
        self.typed.1 = now;
        let Ok(v) = self.typed.0.parse::<u32>() else { return };
        // A whole choice, or the only one it could still become.
        let could: Vec<u32> = choices.iter().copied().filter(|c| c.to_string().starts_with(&self.typed.0)).collect();
        if choices.contains(&v) && could.len() == 1 {
            self.typed.0.clear();
            self.activate(Hit::Choice(v), cx);
        } else if could.is_empty() {
            self.typed.0.clear();
        }
    }

    // The grown-ups.

    fn open_gate(&mut self, cx: &mut Cx<Msg>) {
        if self.sheet.is_some() {
            return;
        }
        let key = cx.show_sheet(self.window);
        self.sheet = Some((key, Sheet::Gate(Gate::new(self.rng.next_u64()))));
        cx.focus("gate-answer");
    }

    fn close_sheet(&mut self, cx: &mut Cx<Msg>) {
        if let Some((k, _)) = self.sheet.take() {
            cx.close_window(k);
        }
        cx.focus("stage");
    }

    fn gate(&mut self, m: GateMsg, cx: &mut Cx<Msg>) {
        let Some((key, Sheet::Gate(g))) = self.sheet.as_mut() else { return };
        match m {
            GateMsg::Answer(a) => {
                g.answer = a;
                g.wrong = false;
            }
            GateMsg::Submit if g.right() => {
                let key = *key;
                self.sheet = Some((key, Sheet::Grown));
            }
            GateMsg::Submit => {
                g.wrong = true;
                g.answer.clear();
            }
            GateMsg::Cancel => self.close_sheet(cx),
        }
    }

    fn grown(&mut self, m: GrownMsg, cx: &mut Cx<Msg>) {
        match m {
            GrownMsg::Sound(on) => {
                self.progress.sound = on;
                self.sounds.on = on;
                self.save();
            }
            GrownMsg::Voice(on) => {
                self.progress.voice = on;
                self.voice.on = on;
                if !on {
                    self.voice.hush();
                }
                self.save();
            }
            GrownMsg::MoreTime => {
                let day = clock::day_start(self.now());
                let extra = self.progress.extra_minutes(day) + 10;
                self.progress.extra = Some((day, extra));
                self.save();
                if self.screen == Screen::Rest && !self.out_of_time() {
                    self.go(Screen::Map, cx);
                }
            }
            GrownMsg::SwitchPlayer => {
                self.close_sheet(cx);
                self.activate(Hit::SwitchPlayer, cx);
            }
            GrownMsg::OpenKids => {
                let cmd = vec!["sekka-kids".to_owned()];
                if !self.quiet
                    && let Err(e) = std::process::Command::new(&cmd[0]).spawn()
                {
                    log::warn!("sekka-kids: {e}");
                }
                self.commands.push(cmd);
            }
            GrownMsg::Done => self.close_sheet(cx),
        }
    }

    fn grown_view(&self) -> Element<Msg> {
        let kid = self.kid.as_ref();
        let who = kid.map_or("No one's playing".to_owned(), |k| format!("{} is playing ({})", k.name, k.grade.name()));
        let time = match (kid.and_then(|k| k.daily_minutes), self.minutes_left()) {
            (Some(limit), Some(left)) => format!(
                "Daily time: {limit} min{}, {} min left today.",
                match self.progress.extra_minutes(clock::day_start(self.now())) {
                    0 => String::new(),
                    e => format!(" and {e} more today"),
                },
                left.max(0.0).round()
            ),
            _ => "No daily limit. Set one in Sekka Kids.".to_owned(),
        };
        let voice_note = match self.voice.kind() {
            Some(Kind::Kokoro) => "Pon reads each question aloud in Kokoro, a natural voice made on this computer.",
            Some(Kind::Espeak) => {
                "Pon reads each question aloud with espeak-ng. Kokoro, Sekka Edu's voice, is much clearer."
            }
            None if self.quiet => "Pon reads each question aloud.",
            None => "Pon needs Kokoro or espeak-ng to speak.",
        };
        let mut items: Vec<Element<Msg>> = vec![
            text("Grown-ups").size(20.0).bold().into(),
            text(who).role(Role::Text2).into(),
            form_section(
                "Anzan",
                vec![
                    form_row("Sounds", toggle(self.progress.sound).on_toggle(|on| Msg::Grown(GrownMsg::Sound(on))))
                        .into(),
                    form_row(
                        "Pon's voice",
                        toggle(self.progress.voice).on_toggle(|on| Msg::Grown(GrownMsg::Voice(on))),
                    )
                    .description(voice_note)
                    .into(),
                ],
            ),
            text(time).role(Role::Text2).wrap().into(),
        ];
        let mut buttons: Vec<Element<Msg>> = Vec::new();
        if kid.is_some_and(|k| k.daily_minutes.is_some()) {
            buttons.push(button("10 more minutes today").on_press(Msg::Grown(GrownMsg::MoreTime)).into());
        }
        buttons.push(button("Reports in Sekka Kids").on_press(Msg::Grown(GrownMsg::OpenKids)).into());
        items.push(aizome_kit::widgets::layout::row(buttons).spacing(10.0).into());
        items.push(
            row![
                button("Switch player").on_press(Msg::Grown(GrownMsg::SwitchPlayer)),
                spacer(),
                button("Done").default().on_press(Msg::Grown(GrownMsg::Done)),
            ]
            .spacing(10.0)
            .into(),
        );
        aizome_kit::widgets::layout::column(items).spacing(14.0).padding(24.0).width(460.0).into()
    }

    // Time and motion.

    /// Each bead's place now, sliding.
    fn beads(&self, abacus: &Abacus, now: i64) -> Beads {
        let mut b = Beads::still(abacus.clone());
        for (i, slide) in self.slides.iter().enumerate().take(abacus.len()) {
            let Some((from, at)) = slide else { continue };
            let t = ((now - at) as f64 / SLIDE_MS as f64).clamp(0.0, 1.0);
            if t >= 1.0 {
                continue;
            }
            let k = 1.0 - (1.0 - t).powi(3);
            let to = abacus.rod(i);
            b.at[i] = Bead::ALL.map(|bead| {
                let (a, z) = (f64::from(u8::from(from.counts(bead))), f64::from(u8::from(to.counts(bead))));
                ((a + (z - a) * k) * 1000.0).round() as u16
            });
        }
        b.blink = self.blink;
        b.rod = self.rod;
        b
    }

    fn thousandths(&self, since: Option<i64>, len: i64, now: i64) -> Option<u16> {
        let t = now - since?;
        (self.motion && (0..len).contains(&t)).then(|| (t * 1000 / len) as u16)
    }

    /// Whether anything is moving (frames are wanted).
    fn animating(&self, now: i64) -> bool {
        let sliding = self.slides.iter().flatten().any(|(_, at)| now - at < SLIDE_MS);
        let round_moving = self.round.as_ref().is_some_and(|r| {
            r.deadline().is_some() && self.intro.is_none()
                || (matches!(r.phase, Phase::Asking) && self.hint_at.is_some_and(|h| now - h < PULSE_MS) && self.motion)
        });
        let summary = self.summary.as_ref().is_some_and(|s| {
            self.screen == Screen::Summary && now - s.at < CONFETTI_MS.max(STAR_GAP_MS * 4 + STAR_POP_MS)
        });
        let hop = self.hop_at.is_some_and(|h| now - h < HOP_MS) && self.screen == Screen::Map;
        let curtain = self.curtain_at.is_some_and(|c| now - c < CURTAIN_MS);
        sliding || round_moving || summary || hop || curtain
    }

    fn schedule(&mut self, cx: &mut Cx<Msg>) {
        let want = self.animating(self.now());
        match (want, self.ticker) {
            (true, None) => self.ticker = Some(cx.every(FRAME, Msg::Tick)),
            (false, Some(h)) => {
                cx.cancel(h);
                self.ticker = None;
            }
            _ => {}
        }
    }

    /// What the canvas draws, now.
    pub fn scene(&self) -> Scene {
        let now = self.now();
        let kid = self.kid.as_ref().map(Card::from);
        let grade = self.grade();
        let stops = (0..LESSONS.len())
            .map(|i| Stop {
                stars: self.progress.stars(i),
                done: self.progress.done(i),
                open: self.progress.unlocked(i, grade),
            })
            .collect();
        let here = self.progress.here(grade);
        let play = self.round.as_ref().filter(|_| self.screen == Screen::Play).map(|r| {
            let l = &LESSONS[r.lesson];
            let stage = if let Some(i) = self.intro {
                PlayStage::Intro(i)
            } else {
                match (&r.phase, r.question()) {
                    (Phase::Peeking { .. }, _) => PlayStage::Peeking,
                    (Phase::Flashing { shown, gap, .. }, Question::Flash { numbers, .. }) => {
                        PlayStage::Flash((!gap).then(|| numbers[*shown]))
                    }
                    (Phase::Right { .. }, _) => PlayStage::Right,
                    (Phase::Showing { steps, done, .. }, _) => PlayStage::Showing(
                        steps.get(done.saturating_sub(1)).map(|s| s.words(r.abacus.len())).unwrap_or_default(),
                    ),
                    _ => PlayStage::Asking,
                }
            };
            let mut beads = self.beads(&r.abacus, now);
            beads.numbers = l.numbers && self.intro.is_none();
            beads.cheer = matches!(r.phase, Phase::Right { .. })
                .then(|| self.thousandths(self.cheer_at, round::RIGHT_MS, now))
                .flatten();
            if let Question::Ghost { ghost, .. } = r.question() {
                let seen = matches!(r.phase, Phase::Asking) && r.hint != Some(Hint::Look);
                beads.ghost = if seen { *ghost } else { 100 };
            }
            if self.intro.is_none() && r.curtained() {
                beads.curtain = self.thousandths(self.curtain_at, CURTAIN_MS, now).unwrap_or(1000);
            }
            let peek_left = match r.phase {
                Phase::Peeking { until } => {
                    let left = (until - now).max(0) as f64 / f64::from(r.peek_ms.max(1));
                    // In twentieths: a few redraws, not one a frame.
                    ((left * 20.0).ceil() * 50.0).min(1000.0) as u16
                }
                _ => 0,
            };
            let pulse = self.thousandths(self.hint_at, PULSE_MS, now).map_or(0, |t| (u32::from(t) * 6 % 1000) as u16);
            PlayView {
                lesson: r.lesson,
                results: r.results.clone(),
                count: r.questions.len(),
                question: r.question().clone(),
                stage,
                beads,
                hint: r.hint.clone(),
                chosen: r.chosen,
                peek_left,
                praise: PRAISE[self.praise],
                pulse,
            }
        });
        let summary = self.summary.as_ref().filter(|_| self.screen == Screen::Summary).map(|s| {
            let t = now - s.at;
            let (shown, pop) = if self.motion {
                let shown = ((t - 300).max(0) / STAR_GAP_MS + 1).min(i64::from(s.stars)) as u8;
                let since = t - 300 - (i64::from(shown) - 1) * STAR_GAP_MS;
                (
                    if t < 300 { 0 } else { shown },
                    if (0..STAR_POP_MS).contains(&since) { (since * 1000 / STAR_POP_MS) as u16 } else { 0 },
                )
            } else {
                (s.stars, 0)
            };
            SummaryView {
                lesson: s.lesson,
                stars: s.stars,
                first_try: s.first_try,
                items: s.items,
                shown,
                pop,
                confetti: if s.stars == 3 { self.thousandths(Some(s.at), CONFETTI_MS, now) } else { None },
                has_next: s.lesson + 1 < LESSONS.len() && self.progress.unlocked(s.lesson + 1, grade),
            }
        });
        let free = (self.screen == Screen::Free).then(|| {
            let mut b = self.beads(&self.free, now);
            b.numbers = true;
            b
        });
        Scene {
            screen: self.screen,
            who: Who {
                cards: self.kids.iter().map(Card::from).collect(),
                focus: self.who_focus,
                pressed: match self.pressed {
                    Some(Hit::Who(WhoHit::Kid(i))) => Some(i),
                    Some(Hit::Who(WhoHit::New)) => Some(self.kids.len()),
                    _ => None,
                },
                blink: self.blink,
            },
            kid,
            stops,
            focus: self.focus,
            here,
            hop: self.thousandths(self.hop_at, HOP_MS, now).filter(|_| self.screen == Screen::Map),
            play,
            summary,
            free,
            pressed: self.pressed,
            blink: self.blink,
            sound: self.progress.sound,
            chapter: LESSONS[self.focus.min(LESSONS.len() - 1)].chapter,
        }
    }

    fn label(&self) -> String {
        match self.screen {
            Screen::Who => "Who's playing?".into(),
            Screen::NewKid => "New player".into(),
            Screen::Map => "Lantern path".into(),
            Screen::Play => self.round.as_ref().map_or("Play".into(), |r| LESSONS[r.lesson].title.to_owned()),
            Screen::Summary => "Round finished".into(),
            Screen::Free => "Play with the beads".into(),
            Screen::Rest => "All done for today".into(),
        }
    }
}

fn stage_event(e: &Event, size: Size, _: &Scene) -> Option<Msg> {
    match e {
        Event::PointerDown { pos, button: aizome_kit::sidestep_ui::PointerButton::Left, .. } => {
            Some(Msg::Down(*pos, size))
        }
        Event::PointerUp { pos, button: aizome_kit::sidestep_ui::PointerButton::Left, .. } => Some(Msg::Up(*pos, size)),
        Event::Key(k) if k.pressed && !k.modifiers.super_key() && !k.modifiers.control() => match &k.key {
            Key::Named(NamedKey::Tab) => None,
            _ => Some(Msg::Key(k.clone())),
        },
        _ => None,
    }
}

impl Program for Anzan {
    type Msg = Msg;

    fn info(&self) -> AppInfo {
        AppInfo::new(APP_ID, "Anzan", "Mental Abacus").icon(Icon::Calculator).version(env!("CARGO_PKG_VERSION"))
    }

    fn launched(&mut self, cx: &mut Cx<Msg>) {
        self.window = cx.open_window(WindowSpec::new("Anzan").size(1000.0, 700.0).min_size(720.0, 504.0));
        self.motion = cx.theme().motion();
        self.dark = cx.theme().is_dark();
        if let Err(e) = self.store.register_app(&AppEntry {
            id: APP_ID.into(),
            name: "Anzan".into(),
            blurb: "The mental abacus".into(),
        }) {
            log::warn!("{}: {e}", self.store.root().display());
        }
        self.reload_kids();
        if self.kids.is_empty() {
            self.newkid = NewKid::new(0);
            self.go(Screen::NewKid, cx);
        } else {
            self.go(Screen::Who, cx);
        }
        cx.every(FLUSH_EVERY, Msg::Flush);
        if self.motion {
            cx.after(BLINK_EVERY, Msg::Blink(true));
            self.blinking = true;
        }
        if let Some(listener) = self.instance.take() {
            instance::serve(listener, cx.stream(), || Msg::Raise);
        }
    }

    fn update(&mut self, msg: Msg, cx: &mut Cx<Msg>) {
        self.motion = cx.theme().motion();
        self.dark = cx.theme().is_dark();
        match msg {
            Msg::Down(p, size) => {
                self.touch();
                self.pointer_down(p, size);
            }
            Msg::Up(p, size) => {
                self.touch();
                self.pointer_up(p, size, cx);
            }
            Msg::Key(k) => {
                self.touch();
                self.key(k, cx);
            }
            Msg::Tick => {
                let now = self.now();
                if self.intro.is_none() && self.round.as_ref().and_then(Round::deadline).is_some_and(|d| now >= d) {
                    self.act(
                        |r, now| {
                            r.tick(now);
                            Feedback::Nothing
                        },
                        cx,
                    );
                }
            }
            Msg::Blink(on) => {
                self.blink = on;
                if self.motion {
                    cx.after(if on { BLINK_MS } else { BLINK_EVERY }, Msg::Blink(!on));
                } else {
                    self.blink = false;
                    self.blinking = false;
                }
            }
            Msg::Flush => {
                let now = self.now();
                if let Some(t) = self.tracker.as_mut() {
                    t.flush(now);
                }
                if matches!(self.screen, Screen::Map | Screen::Free) && self.out_of_time() {
                    self.go(Screen::Rest, cx);
                    self.voice.say("All done for today! The beads are sleepy.");
                }
            }
            Msg::SayFree(v) => {
                if self.screen == Screen::Free && self.free.value() == v {
                    self.voice.say(words(v));
                }
            }
            Msg::NewKid(m) => match m {
                NewKidMsg::Name(n) => self.newkid.name = n,
                NewKidMsg::Animal(a) => self.newkid.animal = a,
                NewKidMsg::Color(c) => self.newkid.color = c,
                NewKidMsg::Grade(g) => self.newkid.grade = g,
                NewKidMsg::Cancel => {
                    if !self.kids.is_empty() {
                        self.go(Screen::Who, cx);
                    }
                }
                NewKidMsg::Make => {
                    if !self.newkid.ready() {
                        return;
                    }
                    match self.store.add(self.newkid.profile()) {
                        Ok(p) => {
                            self.reload_kids();
                            if let Some(i) = self.kids.iter().position(|k| k.id == p.id) {
                                self.who_focus = i;
                                self.sounds.play(Sound::Lantern);
                                self.choose_kid(i, cx);
                            }
                        }
                        Err(e) => log::warn!("{}: {e}", self.store.root().display()),
                    }
                }
            },
            Msg::Gate(m) => self.gate(m, cx),
            Msg::Grown(m) => self.grown(m, cx),
            Msg::Raise => cx.activate(self.window),
        }
        self.schedule(cx);
    }

    fn view(&self, window: WindowKey) -> Element<Msg> {
        if let Some((key, sheet)) = &self.sheet
            && *key == window
        {
            return match sheet {
                Sheet::Gate(g) => gate::view(g, "Settings, time and reports are for grown-ups.").map(Msg::Gate),
                Sheet::Grown => self.grown_view(),
            };
        }
        if self.screen == Screen::NewKid {
            let backdrop = canvas((), |c, b, _, theme| {
                let look = Look::for_theme(theme);
                stage::backdrop(c, b, &Fit::new(b), &look, 150.0);
            });
            let look = if self.dark { Look::NIGHT } else { Look::DAY };
            return stack![backdrop, newkid::view(&self.newkid, !self.kids.is_empty(), &look).map(Msg::NewKid)].into();
        }
        canvas(self.scene(), scene::paint)
            .parts(scene::parts)
            .on_event(stage_event)
            .focusable()
            .focus_ring(FocusRing::Never)
            .id("stage")
            .label(self.label())
            .into()
    }

    fn close_requested(&mut self, window: WindowKey, _cx: &mut Cx<Msg>) -> bool {
        if window == self.window {
            self.leave_kid();
        }
        true
    }
}
