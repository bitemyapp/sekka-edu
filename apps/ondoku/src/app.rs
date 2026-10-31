//! Ondoku, the program: who's reading, the shelves, a book's cover, its
//! pages read aloud to Fumi (or by her), the end of a book, the day's
//! allowance, and the grown-ups' sheet.

use std::cell::Cell;
use std::os::unix::net::UnixListener;
use std::rc::Rc;
use std::time::Duration;

use aizome_kit::prelude::*;
use aizome_kit::sidestep_ui::{Key, KeyEvent, NamedKey};
use aizome_kit::widget::{Event, FocusRing};
use aizome_kit::{MsgSender, TimerHandle};
use aizome_kit::{row, stack};
use sekka_kid::{AppEntry, Event as LogEvent, Profile, Round, Store, Tracker, clock};
use sekka_kid_kit::gate::{self, Gate, GateMsg};
use sekka_kid_kit::newkid::{self, NewKid, NewKidMsg};
use sekka_kid_kit::stage::Fit;
use sekka_kid_kit::who::{Card, Who, WhoHit};
use sekka_kid_kit::{Look, instance, stage};
use sekka_voice::{Kind, Spoken, Voice};

use crate::follow::Page;
use crate::library::{self, Book};
use crate::listen::{self, Ears, Heard};
use crate::phonics;
use crate::progress::{self, Progress};
use crate::scene::{self, BookCard, CoverView, DoneView, Hit, Mic, ReadView, Scene, Screen, ShelfView};
use crate::words;

pub const APP_ID: &str = "computer.sekka.Ondoku";

/// Frames while something moves: the microphone's rings, a word lit as
/// Fumi says it, confetti.
const FRAME: Duration = Duration::from_millis(33);
const FLUSH_EVERY: Duration = Duration::from_secs(30);
const BLINK_EVERY: Duration = Duration::from_millis(3700);
const BLINK_MS: Duration = Duration::from_millis(160);
/// After "Let's sound it out", how long Fumi waits for a try before she
/// says the word herself; after a try that isn't the word, how long she
/// waits for another. She never talks while a kid is reading, or before
/// she's heard what they read.
const WAIT_FOR_TRY: i64 = 3_000;
const WAIT_AFTER_TRY: i64 = 2_500;
/// How long "Let's sound it out" is taken to be until the voice says.
const PROMPT_MS: i64 = 1_500;
/// How often Fumi looks again while a kid is reading.
const LOOK_AGAIN: Duration = Duration::from_millis(250);
const SOUND_IT_OUT: &str = "Let's sound it out.";
const CONFETTI_MS: i64 = 2600;

const PRAISE: [&str; 6] = [
    "Great reading!",
    "Wonderful! Turn the page.",
    "You read that so well!",
    "Hooray! On to the next page.",
    "Beautiful reading!",
    "Yes! You did it!",
];

#[derive(Clone, Debug)]
pub enum Msg {
    Down(Point, Size),
    Up(Point, Size),
    Key(KeyEvent),
    Tick,
    Blink(bool),
    Flush,
    Heard(Heard),
    Spoken(Spoken),
    /// Still stuck on word `w` of turn `t` after sounding it out: say it.
    Stuck(u64, usize),
    NewKid(NewKidMsg),
    Gate(GateMsg),
    Grown(GrownMsg),
    Raise,
}

#[derive(Clone, Debug, PartialEq)]
pub enum GrownMsg {
    Voice(bool),
    Help(bool),
    MoreTime,
    SwitchPlayer,
    OpenKids,
    Done,
}

#[derive(Clone, Debug)]
enum Sheet {
    Gate(Gate),
    Grown,
}

/// Fumi's ears.
enum EarState {
    /// Not started yet.
    Idle,
    /// Can't: no microphone program, or no Whisper model (why, for grown-ups).
    Missing(String),
    Loading(Ears),
    Ready(Ears),
    /// Tests: listening is pretended, and what's heard is sent in.
    Pretend,
}

/// A book being read, for the log.
struct Session {
    book: usize,
    started: i64,
    words: u32,
    alone: u32,
    helped: u32,
    reading_ms: i64,
    page: usize,
    current: Page,
    /// When the page showed, and when the reader started to speak on it.
    shown: i64,
    onset: Option<i64>,
    /// Whether anything was heard on this page, read right or not.
    tried: bool,
}

impl Session {
    /// Count the page being left, if it was read at all.
    fn fold(&mut self) {
        let p = &self.current;
        if p.started.is_none() && p.helped() == 0 {
            return;
        }
        self.words += p.spoken();
        self.alone += p.read();
        self.helped += p.helped();
        // From the first sound of reading (or the page showing) to the last
        // word heard: a page read in one breath still took that breath.
        if let (Some(a), Some(b)) = (p.started, p.ended) {
            let from = self.onset.unwrap_or(self.shown).min(a);
            self.reading_ms += b - from;
        }
    }
}

pub struct Ondoku {
    pub store: Store,
    clock: Rc<dyn Fn() -> i64>,
    quiet: bool,
    pub instance: Option<UnixListener>,
    window: WindowKey,
    sheet: Option<(WindowKey, Sheet)>,
    pub library: Vec<Book>,
    kids: Vec<Profile>,
    who_focus: usize,
    kid: Option<Profile>,
    pub progress: Progress,
    tracker: Option<Tracker>,
    pub screen: Screen,
    newkid: NewKid,
    shelf_level: u8,
    shelf_page: usize,
    book: Option<usize>,
    reading: Option<Session>,
    turn: u64,
    ears: EarState,
    mic: Mic,
    level: f32,
    /// The reader is reading, or what they read is still being heard; when
    /// it was last all heard; and until when Fumi is talking, as far as
    /// is known.
    hearing: bool,
    heard_at: i64,
    fumi_until: i64,
    say: String,
    /// Fumi reading the page aloud: since when, for how long.
    aloud: Option<(i64, f32)>,
    chunks: Option<(usize, Vec<phonics::Chunk>)>,
    done: Option<DoneView>,
    done_at: i64,
    pressed: Option<Hit>,
    down: Option<Option<Hit>>,
    blink: bool,
    motion: bool,
    dark: bool,
    voice: Voice,
    sender: Option<MsgSender<Msg>>,
    ticker: Option<TimerHandle>,
    seed: u64,
    /// Programs started (tests read them).
    pub commands: Vec<String>,
}

impl Ondoku {
    /// `quiet`: no voice and no microphone (tests).
    pub fn new(store: Store, quiet: bool) -> Ondoku {
        Ondoku {
            store,
            clock: Rc::new(clock::now_ms),
            quiet,
            instance: None,
            window: WindowKey::NONE,
            sheet: None,
            library: library::all(),
            kids: Vec::new(),
            who_focus: 0,
            kid: None,
            progress: Progress::default(),
            tracker: None,
            screen: Screen::Who,
            newkid: NewKid::new(0),
            shelf_level: 1,
            shelf_page: 0,
            book: None,
            reading: None,
            turn: 0,
            ears: EarState::Idle,
            mic: Mic::Off,
            level: 0.0,
            hearing: false,
            heard_at: 0,
            fumi_until: 0,
            say: String::new(),
            aloud: None,
            chunks: None,
            done: None,
            done_at: 0,
            pressed: None,
            down: None,
            blink: false,
            motion: true,
            dark: true,
            voice: Voice::new(true, quiet),
            sender: None,
            ticker: None,
            seed: clock::now_ms() as u64,
            commands: Vec::new(),
        }
    }

    /// A clock a test moves by hand.
    pub fn manual_clock(&mut self, start: i64) -> Rc<Cell<i64>> {
        let t = Rc::new(Cell::new(start));
        let read = t.clone();
        self.clock = Rc::new(move || read.get());
        t
    }

    /// Tests: pretend there are ears, and send in what they hear.
    pub fn pretend_ears(&mut self) {
        self.ears = EarState::Pretend;
    }

    /// The page being listened for: what's heard is matched to it.
    pub fn turn(&self) -> u64 {
        self.turn
    }

    /// The page being read, if one is.
    pub fn page(&self) -> Option<&Page> {
        self.reading.as_ref().map(|s| &s.current)
    }

    pub fn mic(&self) -> Mic {
        self.mic
    }

    /// The word whose sounds are showing.
    pub fn sounding_out(&self) -> Option<usize> {
        self.chunks.as_ref().map(|(w, _)| *w)
    }

    /// What Fumi is saying.
    pub fn says(&self) -> &str {
        &self.say
    }

    fn now(&self) -> i64 {
        (self.clock)()
    }

    fn reload_kids(&mut self) {
        self.kids = self.store.profiles();
        self.who_focus = self.who_focus.min(self.kids.len());
        self.voice.prepare(self.kids.iter().map(|k| format!("Hi, {}! Let's read a book.", k.name)));
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

    /// Minutes the kid may still read today, if a grown-up set a limit.
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

    fn speak(&mut self, line: impl Into<String>) {
        let line = line.into();
        self.say.clone_from(&line);
        self.voice.say(line);
    }

    // Who's reading.

    fn choose_kid(&mut self, i: usize, cx: &mut Cx<Msg>) {
        let Some(kid) = self.kids.get(i).cloned() else { return };
        // Their card as it is now: a grown-up may have changed it (their daily
        // time) in Sekka Kids since it was read.
        let kid = self.store.profile(&kid.id).unwrap_or(kid);
        self.leave_kid();
        self.progress = self.store.progress(&kid.id, APP_ID);
        self.voice.on = self.progress.voice;
        self.tracker = Some(Tracker::new(self.store.clone(), &kid.id, APP_ID));
        self.shelf_level = self.progress.shelf.unwrap_or_else(|| library::level_for(kid.grade));
        self.shelf_page = 0;
        let hi = format!("Hi, {}! Let's read a book.", kid.name);
        self.kid = Some(kid);
        self.touch();
        self.start_ears();
        if self.out_of_time() {
            self.rest(cx);
        } else {
            self.speak(hi);
            self.go(Screen::Shelf, cx);
        }
    }

    fn leave_kid(&mut self) {
        self.leave_book(false);
        let now = self.now();
        if let Some(t) = self.tracker.as_mut() {
            t.finish(now);
        }
        self.tracker = None;
        self.kid = None;
    }

    /// Load Whisper now, so it's ready by the first page.
    fn start_ears(&mut self) {
        if !matches!(self.ears, EarState::Idle) || self.quiet {
            if self.quiet && matches!(self.ears, EarState::Idle) {
                self.ears = EarState::Missing("Quiet (a test).".into());
            }
            return;
        }
        if !listen::can_record() {
            self.ears = EarState::Missing("PipeWire's pw-record isn't installed.".into());
            return;
        }
        let Some(model) = listen::model() else {
            self.ears = EarState::Missing(
                "There's no Whisper model: download one in Koe (Base, English is best), or install Sekka Edu's package, which brings one."
                    .into(),
            );
            return;
        };
        let Some(tx) = self.sender.clone() else { return };
        log::info!("ears: {}", model.display());
        self.ears = EarState::Loading(Ears::start(model, move |h| {
            tx.send(Msg::Heard(h));
        }));
    }

    // Screens.

    fn go(&mut self, screen: Screen, cx: &mut Cx<Msg>) {
        let now = self.now();
        if let Some(t) = self.tracker.as_mut() {
            t.flush(now);
        }
        self.screen = screen;
        self.pressed = None;
        self.down = None;
        if screen == Screen::NewKid {
            cx.focus("name");
        } else {
            cx.focus("stage");
        }
    }

    fn rest(&mut self, cx: &mut Cx<Msg>) {
        self.leave_book(false);
        self.go(Screen::Rest, cx);
        self.speak("Fumi is sleepy. Time to rest your eyes! Your page is saved.");
    }

    fn shelf_books(&self) -> Vec<usize> {
        (0..self.library.len()).filter(|i| self.library[*i].level == self.shelf_level).collect()
    }

    fn card(&self, i: usize) -> BookCard {
        let b = &self.library[i];
        let p = self.progress.book(&b.id);
        BookCard {
            index: i,
            title: b.title.clone(),
            emblem: b.emblem,
            color: b.color,
            classic: b.classic,
            bookmark: (p.page > 0).then_some(p.page),
            finished: p.finished,
        }
    }

    fn open_cover(&mut self, i: usize, cx: &mut Cx<Msg>) {
        self.book = Some(i);
        let b = &self.library[i];
        let line = format!("{}. {}", b.title, b.about);
        self.go(Screen::Cover, cx);
        self.voice.say(line);
    }

    /// Start reading book `book` at `page`, to Fumi (`mic`) or by her.
    fn start_book(&mut self, book: usize, page: usize, mic: bool, cx: &mut Cx<Msg>) {
        if self.out_of_time() {
            self.rest(cx);
            return;
        }
        self.leave_book(false);
        let now = self.now();
        let page = page.min(self.library[book].pages.len() - 1);
        self.reading = Some(Session {
            book,
            started: now,
            words: 0,
            alone: 0,
            helped: 0,
            reading_ms: 0,
            page,
            current: Page::new(&self.library[book].pages[page]),
            shown: now,
            onset: None,
            tried: false,
        });
        self.book = Some(book);
        self.go(Screen::Read, cx);
        self.turn_to(page);
        if mic {
            self.mic_on();
            if self.mic == Mic::On || self.mic == Mic::Waiting {
                self.speak("I'm listening! Read me this page.");
            }
        } else {
            self.read_aloud();
        }
    }

    /// Show `page` of the book being read.
    fn turn_to(&mut self, page: usize) {
        let Some(s) = self.reading.as_mut() else { return };
        let book = &self.library[s.book];
        s.fold();
        s.page = page;
        s.current = Page::new(&book.pages[page]);
        s.shown = (self.clock)();
        s.onset = None;
        s.tried = false;
        self.progress.at(&book.id, page);
        self.save();
        self.turn += 1;
        // What the ears said of the last turn is done with.
        self.hearing = false;
        self.aloud = None;
        self.chunks = None;
        self.voice.hush();
        if self.mic == Mic::On {
            self.open_ears();
        }
        // The page's words, ready for when they're tapped.
        self.prepare_page();
    }

    fn page_text(&self) -> Option<String> {
        let s = self.reading.as_ref()?;
        Some(self.library[s.book].pages[s.page].clone())
    }

    fn prepare_page(&self) {
        if let Some(text) = self.page_text() {
            let mut words: Vec<String> = text
                .split_whitespace()
                .map(|w| w.trim_matches(|c: char| !c.is_alphanumeric() && c != '\'').to_owned())
                .filter(|w| !w.is_empty())
                .collect();
            words.dedup();
            // And what Fumi says as it's read, the help first.
            let lines = std::iter::once(SOUND_IT_OUT).chain(PRAISE).map(String::from);
            self.voice.prepare(lines.chain(words));
        }
    }

    fn open_ears(&mut self) {
        let Some(text) = self.page_text() else { return };
        if let EarState::Ready(e) = &self.ears {
            e.open(self.turn, &text);
        }
    }

    fn mic_on(&mut self) {
        match &self.ears {
            EarState::Ready(_) | EarState::Pretend => {
                self.mic = Mic::On;
                self.aloud = None;
                self.voice.hush();
                self.open_ears();
            }
            EarState::Loading(_) => self.mic = Mic::Waiting,
            EarState::Idle | EarState::Missing(_) => {
                self.mic = Mic::None;
                self.say = "I can't hear right now. Read to me anyway, and turn the page when you're done!".into();
            }
        }
    }

    fn mic_off(&mut self) {
        if let EarState::Ready(e) = &self.ears {
            e.close();
        }
        if self.mic != Mic::None {
            self.mic = Mic::Off;
        }
        self.level = 0.0;
    }

    /// Fumi reads the page aloud; each word lights as she says it.
    fn read_aloud(&mut self) {
        let Some(text) = self.page_text() else { return };
        self.mic_off();
        self.chunks = None;
        self.say = "Listen, and follow along with your finger!".into();
        // The page in one line: the voice says when it starts, and for how
        // long, so the words can be lit in time.
        self.voice.say(text.split_whitespace().collect::<Vec<_>>().join(" "));
    }

    /// The word Fumi's at, reading aloud, by each word's share of the time.
    fn aloud_word(&self, now: i64) -> Option<usize> {
        let (from, secs) = self.aloud?;
        let text = self.page_text()?;
        let words: Vec<&str> = text.split_whitespace().collect();
        let weight = |w: &str| {
            let letters = w.chars().filter(|c| c.is_alphabetic()).count() as f64;
            let pause = match w.chars().last() {
                Some('.' | '!' | '?') => 4.0,
                Some(',' | ';' | ':' | '—') => 2.0,
                _ => 0.0,
            };
            1.5 + letters + pause
        };
        let total: f64 = words.iter().map(|w| weight(w)).sum();
        let t = (now - from) as f64 / 1000.0 / f64::from(secs.max(0.1)) * total;
        if t < 0.0 || t >= total {
            return None;
        }
        let mut acc = 0.0;
        for (i, w) in words.iter().enumerate() {
            acc += weight(w);
            if t < acc {
                return Some(i);
            }
        }
        None
    }

    /// Leave the book being read: what was read goes to the log, and the
    /// bookmark stays.
    fn leave_book(&mut self, finished: bool) {
        self.mic_off();
        self.aloud = None;
        self.chunks = None;
        let Some(mut s) = self.reading.take() else { return };
        s.fold();
        if s.words == 0 && !finished {
            return;
        }
        let now = self.now();
        let b = &self.library[s.book];
        let wpm = progress::wpm(s.alone + s.helped, 0, s.reading_ms);
        let round = Round {
            app: APP_ID.into(),
            activity: b.id.clone(),
            title: b.title.clone(),
            skills: b.skills.clone(),
            started: s.started,
            ended: now,
            items: s.words,
            first_try: s.alone,
            hints: s.helped,
            stars: u8::from(finished),
            finished,
            response_ms: None,
            words_per_minute: wpm,
        };
        self.log(&LogEvent::Round(round));
        if finished {
            let (id, title) = (b.id.clone(), b.title.clone());
            self.progress.finished(&id, wpm);
            let times = self.progress.book(&id).finished;
            self.save();
            let text = match times {
                1 => Some(format!("Read {title} to the end")),
                3 => Some(format!("Read {title} three times")),
                _ => None,
            };
            if let Some(text) = text {
                self.log(&LogEvent::Milestone { app: APP_ID.into(), at: now, text });
            }
        }
    }

    fn finish_book(&mut self, cx: &mut Cx<Msg>) {
        let Some(s) = &self.reading else { return };
        let book = s.book;
        let words = self.library[book].words() as u32;
        self.leave_book(true);
        let times = self.progress.book(&self.library[book].id).finished;
        self.done = Some(DoneView { card: self.card(book), words, times });
        self.done_at = self.now();
        self.go(Screen::Done, cx);
        let line = match times {
            1 => format!("You read {}! Hooray!", self.library[book].title),
            _ => "You read it again! Your reading gets smoother every time.".to_owned(),
        };
        self.speak(line);
    }

    // What's heard.

    fn heard(&mut self, h: Heard, cx: &mut Cx<Msg>) {
        match h {
            Heard::Ready => {
                if let EarState::Loading(e) = std::mem::replace(&mut self.ears, EarState::Idle) {
                    self.ears = EarState::Ready(e);
                }
                if self.mic == Mic::Waiting && self.screen == Screen::Read {
                    self.mic_on();
                }
            }
            Heard::Failed(why) => {
                log::warn!("ears: {why}");
                self.ears = EarState::Missing(why);
                self.mic = Mic::None;
                if self.screen == Screen::Read {
                    self.say = "I can't hear right now. Read to me anyway, and turn the page when you're done!".into();
                }
            }
            Heard::Level(turn, l) if turn == self.turn => {
                self.level = l;
                if l >= 0.02 {
                    self.touch();
                    let now = self.now();
                    if let Some(s) = self.reading.as_mut() {
                        s.onset.get_or_insert(now);
                    }
                }
            }
            Heard::Guess(turn, t) if turn == self.turn => {
                if let Some(s) = self.reading.as_mut() {
                    s.current.guess(&t);
                }
            }
            Heard::Reading(turn, on) if turn == self.turn => {
                self.hearing = on;
                if !on {
                    self.heard_at = self.now();
                }
            }
            Heard::Said(turn, t) if turn == self.turn => self.said(&t, cx),
            Heard::Quiet(turn) if turn == self.turn => self.stuck(cx),
            Heard::Closed(turn) if turn == self.turn => {
                self.hearing = false;
                if self.mic == Mic::On {
                    self.mic = Mic::Off;
                    self.say = "I'll wait. Press the green button when you want to read to me.".into();
                }
                self.level = 0.0;
            }
            _ => {}
        }
    }

    /// The reader said `t`: follow them along the page.
    fn said(&mut self, t: &str, cx: &mut Cx<Msg>) {
        let now = self.now();
        let level = self.reading.as_ref().map_or(1, |s| self.library[s.book].level);
        let Some(s) = self.reading.as_mut() else { return };
        if !words::heard(t).is_empty() {
            s.tried = true;
        }
        let was_done = s.current.done();
        if !s.current.hear(t, now) {
            return;
        }
        self.touch();
        self.chunks = None;
        let s = self.reading.as_ref().unwrap();
        if s.current.done() && !was_done {
            let i = (self.turn as usize + s.page) % PRAISE.len();
            self.speak(PRAISE[i]);
            let _ = level;
            let _ = cx;
        } else if !s.current.done() {
            self.say = "I'm listening…".into();
        }
    }

    /// The reader has gone quiet in the middle of a page, having tried it:
    /// help with the next word. In the first books, sound it out first, and
    /// give them the time to try it.
    fn stuck(&mut self, cx: &mut Cx<Msg>) {
        if !self.progress.help || self.mic != Mic::On || self.hearing {
            return;
        }
        let Some(s) = &self.reading else { return };
        if s.current.done() || !s.tried {
            return;
        }
        let w = s.current.next;
        if self.sounding_out() == Some(w) {
            return;
        }
        let word = s.current.words[w].text.clone();
        let level = self.library[s.book].level;
        let pieces = phonics::chunks(&word);
        if level <= 2 && pieces.len() >= 2 {
            self.chunks = Some((w, pieces));
            self.speak(SOUND_IT_OUT);
            self.fumi_until = self.now() + PROMPT_MS;
            cx.after(Duration::from_millis((PROMPT_MS + WAIT_FOR_TRY) as u64), Msg::Stuck(self.turn, w));
        } else {
            self.help_with(w);
        }
    }

    /// Fumi says word `w`, which counts as helped.
    fn help_with(&mut self, w: usize) {
        let now = self.now();
        let Some(s) = self.reading.as_mut() else { return };
        let Some(word) = s.current.words.get(w) else { return };
        let plain = word.text.trim_matches(|c: char| !c.is_alphanumeric() && c != '\'').to_owned();
        s.current.help(w, now);
        let done = s.current.done();
        self.say = format!("That word is “{plain}”.");
        self.voice.say(plain);
        if done {
            self.say = format!(
                "That word is “{}”. You finished the page!",
                self.say.trim_start_matches("That word is “").trim_end_matches("”.")
            );
        }
    }

    // Pressing.

    fn hit(&self, size: Size, p: Point, cx: &Cx<Msg>) -> Option<Hit> {
        scene::hit(size, &self.scene(), p, cx.theme())
    }

    fn activate(&mut self, h: Hit, cx: &mut Cx<Msg>) {
        match h {
            Hit::Who(WhoHit::Kid(i)) => self.choose_kid(i, cx),
            Hit::Who(WhoHit::New) => {
                self.newkid = NewKid::new(self.kids.len());
                self.go(Screen::NewKid, cx);
            }
            Hit::Kid | Hit::SwitchPlayer => {
                self.leave_kid();
                self.reload_kids();
                self.go(Screen::Who, cx);
            }
            Hit::Home => match self.screen {
                Screen::Read => {
                    let book = self.reading.as_ref().map(|s| s.book);
                    self.leave_book(false);
                    self.voice.hush();
                    match book {
                        Some(b) => self.open_cover(b, cx),
                        None => self.go(Screen::Shelf, cx),
                    }
                }
                _ => {
                    self.voice.hush();
                    self.say = "Pick a book!".into();
                    if self.out_of_time() {
                        self.rest(cx);
                    } else {
                        self.go(Screen::Shelf, cx);
                    }
                }
            },
            Hit::Voice => {
                self.progress.voice = !self.progress.voice;
                self.voice.on = self.progress.voice;
                if !self.progress.voice {
                    self.voice.hush();
                }
                self.save();
            }
            Hit::GrownUps => self.open_gate(cx),
            Hit::Level(l) => {
                self.shelf_level = l;
                self.shelf_page = 0;
                self.progress.shelf = Some(l);
                self.save();
            }
            Hit::ShelfBack => self.shelf_page = self.shelf_page.saturating_sub(1),
            Hit::ShelfOn => self.shelf_page += 1,
            Hit::Book(i) => self.open_cover(i, cx),
            Hit::Mic => match self.screen {
                Screen::Cover => {
                    if let Some(b) = self.book {
                        let page = self.progress.book(&self.library[b].id).page;
                        self.start_book(b, page, true, cx);
                    }
                }
                Screen::Read => {
                    if self.mic == Mic::On || self.mic == Mic::Waiting {
                        self.mic_off();
                        self.say = "Okay! Press the green button when you want to read to me.".into();
                    } else {
                        self.mic_on();
                        if self.mic == Mic::On || self.mic == Mic::Waiting {
                            self.speak("I'm listening! Read me this page.");
                        }
                    }
                }
                _ => {}
            },
            Hit::Listen => match self.screen {
                Screen::Cover => {
                    if let Some(b) = self.book {
                        let page = self.progress.book(&self.library[b].id).page;
                        self.start_book(b, page, false, cx);
                    }
                }
                Screen::Read => self.read_aloud(),
                _ => {}
            },
            Hit::FromStart => {
                if let Some(b) = self.book {
                    self.start_book(b, 0, true, cx);
                }
            }
            Hit::Word(i) => {
                let level = self.reading.as_ref().map_or(3, |s| self.library[s.book].level);
                let Some(s) = &self.reading else { return };
                let Some(word) = s.current.words.get(i) else { return };
                let pieces = phonics::chunks(&word.text);
                self.chunks = (level <= 2 && pieces.len() >= 2).then_some((i, pieces));
                self.help_with(i);
            }
            Hit::PageAgain => {
                let now = self.now();
                if let Some(s) = self.reading.as_mut() {
                    s.fold();
                    s.current.again();
                    s.shown = now;
                    s.onset = None;
                    s.tried = false;
                }
                self.turn += 1;
                // What the ears said of the last turn is done with.
                self.hearing = false;
                self.chunks = None;
                if self.mic == Mic::On {
                    self.open_ears();
                    self.speak("Let's read this page again!");
                }
            }
            Hit::PrevPage => {
                let Some(s) = &self.reading else { return };
                if s.page > 0 {
                    let p = s.page - 1;
                    self.turn_to(p);
                    self.say = "Back a page.".into();
                }
            }
            Hit::NextPage => {
                let Some(s) = &self.reading else { return };
                let pages = self.library[s.book].pages.len();
                if s.page + 1 >= pages {
                    self.finish_book(cx);
                    return;
                }
                let p = s.page + 1;
                // A page is a stopping point: the day may end here.
                self.turn_to(p);
                if self.out_of_time() {
                    self.rest(cx);
                    return;
                }
                if self.mic == Mic::On {
                    self.say = "I'm listening!".into();
                } else {
                    self.say = "Read me this page, or press the blue button and I'll read it.".into();
                }
            }
            Hit::BookAgain => {
                if let Some(b) = self.done.as_ref().map(|d| d.card.index) {
                    self.start_book(b, 0, true, cx);
                }
            }
            Hit::Another => {
                self.done = None;
                self.say = "Pick another book!".into();
                if self.out_of_time() {
                    self.rest(cx);
                } else {
                    self.go(Screen::Shelf, cx);
                }
            }
        }
    }

    fn key(&mut self, k: KeyEvent, cx: &mut Cx<Msg>) {
        let named = match &k.key {
            Key::Named(n) => Some(*n),
            _ => None,
        };
        let space = matches!(&k.key, Key::Character(s) if s == " ");
        match self.screen {
            Screen::Who => {
                let n = self.kids.len() + 1;
                match named {
                    Some(NamedKey::ArrowRight | NamedKey::ArrowDown) => self.who_focus = (self.who_focus + 1) % n,
                    Some(NamedKey::ArrowLeft | NamedKey::ArrowUp) => self.who_focus = (self.who_focus + n - 1) % n,
                    Some(NamedKey::Enter) => {
                        let h =
                            if self.who_focus < self.kids.len() { WhoHit::Kid(self.who_focus) } else { WhoHit::New };
                        self.activate(Hit::Who(h), cx);
                    }
                    _ => {}
                }
            }
            Screen::Read => match named {
                Some(NamedKey::ArrowRight | NamedKey::PageDown) => self.activate(Hit::NextPage, cx),
                Some(NamedKey::ArrowLeft | NamedKey::PageUp) => self.activate(Hit::PrevPage, cx),
                Some(NamedKey::Escape) => self.activate(Hit::Home, cx),
                Some(NamedKey::Enter) => self.activate(Hit::Mic, cx),
                _ if space => self.activate(Hit::Mic, cx),
                _ => {}
            },
            Screen::Cover => match named {
                Some(NamedKey::Enter) => self.activate(Hit::Mic, cx),
                Some(NamedKey::Escape) => self.activate(Hit::Home, cx),
                _ => {}
            },
            Screen::Done => match named {
                Some(NamedKey::Enter) => self.activate(Hit::Another, cx),
                Some(NamedKey::Escape) => self.activate(Hit::Home, cx),
                _ => {}
            },
            Screen::Shelf => {
                if named == Some(NamedKey::Escape) {
                    self.activate(Hit::Kid, cx);
                }
            }
            Screen::NewKid | Screen::Rest => {}
        }
    }

    // The grown-ups' door and sheet.

    fn open_gate(&mut self, cx: &mut Cx<Msg>) {
        if self.sheet.is_some() {
            return;
        }
        self.seed = self.seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        let key = cx.show_sheet(self.window);
        self.sheet = Some((key, Sheet::Gate(Gate::new(self.seed >> 33))));
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
            GrownMsg::Voice(on) => {
                self.progress.voice = on;
                self.voice.on = on;
                if !on {
                    self.voice.hush();
                }
                self.save();
            }
            GrownMsg::Help(on) => {
                self.progress.help = on;
                self.save();
            }
            GrownMsg::MoreTime => {
                let day = clock::day_start(self.now());
                let extra = self.progress.extra_minutes(day) + 10;
                self.progress.extra = Some((day, extra));
                self.save();
                if self.screen == Screen::Rest && !self.out_of_time() {
                    self.say = "Pick a book!".into();
                    self.go(Screen::Shelf, cx);
                }
            }
            GrownMsg::SwitchPlayer => {
                self.close_sheet(cx);
                self.activate(Hit::SwitchPlayer, cx);
            }
            GrownMsg::OpenKids => {
                if !self.quiet
                    && let Err(e) = std::process::Command::new("sekka-kids").spawn()
                {
                    log::warn!("sekka-kids: {e}");
                }
                self.commands.push("sekka-kids".into());
            }
            GrownMsg::Done => self.close_sheet(cx),
        }
    }

    fn grown_view(&self) -> Element<Msg> {
        let kid = self.kid.as_ref();
        let who = kid.map_or("No one's reading".to_owned(), |k| format!("{} is reading ({})", k.name, k.grade.name()));
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
            Some(Kind::Kokoro) => {
                "Fumi reads pages aloud and helps with words in Kokoro, a natural voice made on this computer."
            }
            Some(Kind::Espeak) => "Fumi speaks with espeak-ng. Kokoro, Sekka Edu's voice, is much clearer.",
            None if self.quiet => "Fumi reads pages aloud and helps with words.",
            None => "Fumi needs Kokoro or espeak-ng to speak.",
        };
        let ears_note = match &self.ears {
            EarState::Ready(_) | EarState::Pretend => {
                "Fumi listens with Whisper, the model Koe uses, on this computer. What she hears is never saved or sent anywhere."
                    .to_owned()
            }
            EarState::Loading(_) | EarState::Idle => "Fumi's ears are getting ready.".to_owned(),
            EarState::Missing(why) => format!("Fumi can't listen: {why}"),
        };
        let mut items: Vec<Element<Msg>> = vec![
            text("Grown-ups").size(20.0).bold().into(),
            text(who).role(Role::Text2).into(),
            form_section(
                "Ondoku",
                vec![
                    form_row("Fumi's voice", toggle(self.progress.voice).on_toggle(|on| Msg::Grown(GrownMsg::Voice(on))))
                        .description(voice_note)
                        .into(),
                    form_row(
                        "Help after a pause",
                        toggle(self.progress.help).on_toggle(|on| Msg::Grown(GrownMsg::Help(on))),
                    )
                    .description(
                        "When a reader stops on a word, Fumi helps: in the first books she shows its sounds, then says it.",
                    )
                    .into(),
                ],
            ),
            text(ears_note).role(Role::Text2).wrap().into(),
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
                button("Switch reader").on_press(Msg::Grown(GrownMsg::SwitchPlayer)),
                spacer(),
                button("Done").default().on_press(Msg::Grown(GrownMsg::Done)),
            ]
            .spacing(10.0)
            .into(),
        );
        aizome_kit::widgets::layout::column(items).spacing(14.0).padding(24.0).width(480.0).into()
    }

    // Drawing.

    fn animating(&self, now: i64) -> bool {
        let listening = self.screen == Screen::Read && self.mic == Mic::On;
        let aloud = self.screen == Screen::Read && self.aloud.is_some();
        let confetti = self.screen == Screen::Done && now - self.done_at < CONFETTI_MS;
        self.motion && (listening || aloud || confetti)
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
        let who = Who {
            cards: self.kids.iter().map(Card::from).collect(),
            focus: self.who_focus,
            pressed: match self.pressed {
                Some(Hit::Who(WhoHit::Kid(i))) => Some(i),
                Some(Hit::Who(WhoHit::New)) => Some(self.kids.len()),
                _ => None,
            },
            blink: self.blink,
        };
        let shelf = (self.screen == Screen::Shelf).then(|| {
            let books = self.shelf_books();
            let pages = books.len().div_ceil(scene::PER_SHELF).max(1);
            let page = self.shelf_page.min(pages - 1);
            ShelfView {
                level: self.shelf_level,
                kid_level: self.kid.as_ref().map_or(0, |k| library::level_for(k.grade)),
                books: books
                    .iter()
                    .skip(page * scene::PER_SHELF)
                    .take(scene::PER_SHELF)
                    .map(|i| self.card(*i))
                    .collect(),
                page,
                pages,
                say: if self.say.is_empty() { "Pick a book!".into() } else { self.say.clone() },
            }
        });
        let cover = (self.screen == Screen::Cover).then_some(self.book).flatten().map(|i| {
            let b = &self.library[i];
            CoverView {
                card: self.card(i),
                byline: b.byline(),
                about: b.about.clone(),
                pages: b.pages.len(),
                words: b.words(),
                level: b.level,
            }
        });
        let read = (self.screen == Screen::Read).then_some(self.reading.as_ref()).flatten().map(|s| {
            let b = &self.library[s.book];
            ReadView {
                title: b.title.clone(),
                level: b.level,
                page: s.page,
                pages: b.pages.len(),
                text: b.pages[s.page].clone(),
                marks: s.current.marks.clone(),
                next: s.current.next,
                ahead: s.current.ahead,
                mic: self.mic,
                aloud: self.aloud_word(now),
                chunks: self.chunks.clone(),
                done: s.current.done(),
                say: self.say.clone(),
            }
        });
        Scene {
            screen: self.screen,
            who,
            kid,
            pressed: self.pressed,
            voice: self.progress.voice,
            shelf,
            cover,
            read,
            done: (self.screen == Screen::Done).then(|| self.done.clone()).flatten(),
            rest_say: self.say.clone(),
            blink: self.blink,
            level: self.level,
            phase: now as f64 / 1000.0,
            confetti: if self.motion { (now - self.done_at) as f64 / CONFETTI_MS as f64 } else { 1.0 },
        }
    }

    fn label(&self) -> String {
        match self.screen {
            Screen::Who => "Who's reading?".into(),
            Screen::NewKid => "New reader".into(),
            Screen::Shelf => "Bookshelf".into(),
            Screen::Cover | Screen::Read => self.book.map_or("Book".into(), |b| self.library[b].title.clone()),
            Screen::Done => "Finished".into(),
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

impl Program for Ondoku {
    type Msg = Msg;

    fn info(&self) -> AppInfo {
        AppInfo::new(APP_ID, "Ondoku", "Reading Buddy").icon(Icon::BookOpen).version(env!("CARGO_PKG_VERSION"))
    }

    fn launched(&mut self, cx: &mut Cx<Msg>) {
        crate::art::install_reading_font();
        // Pages too long for their level's type, in this type, split in two.
        let started = std::time::Instant::now();
        let theme = cx.theme();
        for b in &mut self.library {
            let level = b.level;
            library::paginate(b, |t| scene::fits(theme, t, level));
        }
        log::info!("paginated {} books in {:?}", self.library.len(), started.elapsed());
        self.window = cx.open_window(WindowSpec::new("Ondoku").size(1000.0, 700.0).min_size(720.0, 504.0));
        self.motion = cx.theme().motion();
        self.dark = cx.theme().is_dark();
        self.sender = Some(cx.stream());
        if let Some(tx) = self.sender.clone() {
            self.voice.on_speaking(move |s| {
                tx.send(Msg::Spoken(s));
            });
        }
        if let Err(e) = self.store.register_app(&AppEntry {
            id: APP_ID.into(),
            name: "Ondoku".into(),
            blurb: "A reading buddy who listens".into(),
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
                let hit = self.hit(size, p, cx);
                self.pressed = hit;
                self.down = Some(hit);
                if let Some(Hit::Who(w)) = hit {
                    self.who_focus = match w {
                        WhoHit::Kid(i) => i,
                        WhoHit::New => self.kids.len(),
                    };
                }
            }
            Msg::Up(p, size) => {
                self.touch();
                self.pressed = None;
                let hit = self.hit(size, p, cx);
                if let Some(Some(h)) = self.down.take()
                    && hit == Some(h)
                {
                    self.activate(h, cx);
                }
            }
            Msg::Key(k) => {
                self.touch();
                self.key(k, cx);
            }
            Msg::Tick => {
                // Fumi's done reading the page aloud.
                let now = self.now();
                if let Some((from, secs)) = self.aloud
                    && now - from > (f64::from(secs) * 1000.0) as i64 + 300
                {
                    self.aloud = None;
                    self.say = "Now you read it to me! Press the green button.".into();
                }
            }
            Msg::Blink(on) => {
                self.blink = on;
                if self.motion {
                    cx.after(if on { BLINK_MS } else { BLINK_EVERY }, Msg::Blink(!on));
                } else {
                    self.blink = false;
                }
            }
            Msg::Flush => {
                let now = self.now();
                if let Some(t) = self.tracker.as_mut() {
                    t.flush(now);
                }
                if matches!(self.screen, Screen::Shelf | Screen::Cover | Screen::Done) && self.out_of_time() {
                    self.rest(cx);
                }
            }
            Msg::Heard(h) => self.heard(h, cx),
            Msg::Spoken(s) => {
                let now = self.now();
                self.fumi_until = now + (f64::from(s.seconds) * 1000.0) as i64;
                // Fumi's own voice isn't reading.
                if let EarState::Ready(e) = &self.ears {
                    e.mute(s.seconds + 0.3);
                }
                let page = self.page_text().map(|t| t.split_whitespace().collect::<Vec<_>>().join(" "));
                if self.screen == Screen::Read && page.as_deref() == Some(s.text.as_str()) {
                    self.aloud = Some((now, s.seconds));
                }
            }
            Msg::Stuck(turn, w) => {
                // Still on the word being sounded out: say it, once the kid's
                // had time to try it and isn't trying it now.
                if turn == self.turn
                    && self.reading.as_ref().is_some_and(|s| s.current.next == w)
                    && self.sounding_out() == Some(w)
                {
                    let now = self.now();
                    let at = (self.fumi_until + WAIT_FOR_TRY).max(self.heard_at + WAIT_AFTER_TRY);
                    if self.hearing {
                        cx.after(LOOK_AGAIN, Msg::Stuck(turn, w));
                    } else if now < at {
                        cx.after(Duration::from_millis((at - now) as u64), Msg::Stuck(turn, w));
                    } else {
                        self.help_with(w);
                    }
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
