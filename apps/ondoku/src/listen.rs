//! Fumi's ears: the microphone through PipeWire's `pw-record`, and Whisper
//! (the whisper.cpp Koe builds, [`crate::whisper`]), with Koe as a library
//! for its model catalogue and folder (`koe::models`) and its audio
//! decoding and gate (`koe::audio`).
//!
//! The microphone is open only while a kid is reading to Fumi (the big
//! button), and closes by itself after a long quiet. What it hears stays in
//! memory, is never written down, and goes nowhere: each stretch of reading
//! is recognised on this computer, then dropped.
//!
//! A stretch ends at a pause, and what was said in it goes to the page
//! ([`Heard::Said`]); while it goes on, a guess at it is made every half
//! second or so ([`Heard::Guess`]), so the page can follow along. Whisper
//! works on a thread of its own, so the microphone never waits for it, and
//! it starts on a stretch as soon as the reader goes quiet: by the time the
//! quiet is long enough to be a pause, what they said is usually ready.

use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use koe::audio::{self, PcmDecoder, RATE};
use tokio::io::AsyncReadExt;
use tokio::sync::mpsc;

use crate::whisper::{How, Whisper};

/// What the ears tell the app.
#[derive(Clone, Debug, PartialEq)]
pub enum Heard {
    /// Whisper's loaded and listening is possible.
    Ready,
    /// The microphone's loudness, a few times a second (RMS, 0 to about 0.3).
    Level(u64, f32),
    /// The reader has started a stretch (`true`), or every stretch they've
    /// read has been heard and told (`false`). Fumi doesn't talk over them,
    /// or before she knows what they said.
    Reading(u64, bool),
    /// A guess at what's been read of a stretch still going on.
    Guess(u64, String),
    /// A stretch of reading, ended at a pause.
    Said(u64, String),
    /// Nothing heard for a while since the last words.
    Quiet(u64),
    /// The microphone closed: asked to, or too long quiet.
    Closed(u64),
    Failed(String),
}

enum Ask {
    Open {
        turn: u64,
        prompt: String,
    },
    Close,
    /// Hear nothing for this long: Fumi is talking.
    Mute(std::time::Duration),
}

/// A pause this long ends a stretch.
const PAUSE: usize = RATE * 7 / 10;
/// Quiet this long, and Whisper starts on the stretch so far: if the quiet
/// lasts to a pause, that's what was said; if the reader goes on, it's
/// dropped.
const EARLY: usize = RATE * 3 / 10;
/// A stretch is cut here even without a pause.
const LONGEST: usize = RATE * 12;
/// Guess again once this much more has been heard.
const GUESS_EVERY: usize = RATE / 2;
/// Quiet this long after reading is "stuck", for Fumi to help.
const STUCK: usize = RATE * 4;
/// Quiet this long closes the microphone.
const GIVE_UP: usize = RATE * 40;
/// Kept from before speech starts, so its first sound isn't cut.
const PRE_ROLL: usize = RATE * 3 / 10;
/// Loud enough to be speech in a quiet room (Koe's own threshold).
const SPEECH: f32 = 0.008;
/// The room's own sound is taken as the quietest moment in this long, and
/// speech must be this many times louder: a fan's hum isn't reading, and
/// doesn't keep a stretch from ending.
const ROOM: usize = RATE * 10;
const OVER_ROOM: f32 = 2.0;

/// The Whisper model to listen with: `SEKKA_EDU_WHISPER`, else one Koe has
/// downloaded, English ones first.
pub fn model() -> Option<PathBuf> {
    if let Some(p) = std::env::var_os("SEKKA_EDU_WHISPER").filter(|v| !v.is_empty()).map(PathBuf::from) {
        return p.is_file().then_some(p);
    }
    let dir = koe::models::directory().ok()?;
    ["base.en", "small.en", "base", "small", "tiny.en", "tiny"].iter().find_map(|name| {
        let m = koe::models::find(name).ok()?;
        (koe::models::installed(&dir, &m) == koe::models::Installed::Present).then(|| dir.join(m.file_name()))
    })
}

/// Whether there's a microphone program.
pub fn can_record() -> bool {
    std::env::var_os("PATH")
        .is_some_and(|p| std::env::split_paths(&p).any(|d| Path::new(&d).join("pw-record").is_file()))
}

/// What Whisper is told to expect: a child reading these words.
pub fn prompt(page: &str) -> String {
    let mut seen = std::collections::HashSet::new();
    let words: Vec<String> = page
        .split_whitespace()
        .map(|w| w.trim_matches(|c: char| !c.is_alphanumeric()).to_owned())
        .filter(|w| !w.is_empty() && seen.insert(w.to_lowercase()))
        .collect();
    format!("A child reading aloud. {}", words.join(", "))
}

/// The ears: a thread with the microphone when asked, and one with Whisper.
pub struct Ears {
    tx: mpsc::UnboundedSender<Ask>,
}

impl Ears {
    /// Load `model` on a thread of its own; `tell` hears what happens.
    pub fn start(model: PathBuf, tell: impl Fn(Heard) + Send + 'static) -> Ears {
        let (tx, rx) = mpsc::unbounded_channel();
        let started = std::thread::Builder::new().name("ears".into()).spawn(move || {
            let rt = match tokio::runtime::Builder::new_current_thread().enable_all().build() {
                Ok(rt) => rt,
                Err(e) => return tell(Heard::Failed(e.to_string())),
            };
            rt.block_on(run(model, rx, tell));
        });
        if let Err(e) = started {
            log::warn!("ears: {e}");
        }
        Ears { tx }
    }

    /// Listen for `turn` (a page), expecting its words.
    pub fn open(&self, turn: u64, page: &str) {
        let _ = self.tx.send(Ask::Open { turn, prompt: prompt(page) });
    }

    pub fn close(&self) {
        let _ = self.tx.send(Ask::Close);
    }

    /// Don't take what's heard for `seconds` as reading: Fumi's own voice.
    pub fn mute(&self, seconds: f32) {
        let _ = self.tx.send(Ask::Mute(std::time::Duration::from_secs_f32(seconds.max(0.0))));
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Kind {
    Guess,
    Final,
}

/// Something for Whisper to hear.
struct Job {
    n: u64,
    turn: u64,
    kind: Kind,
    samples: Vec<f32>,
    prompt: Arc<str>,
    /// How loud speech must be: quieter than this throughout, it's not heard.
    gate: f32,
    cancel: Arc<AtomicBool>,
}

/// What Whisper heard (`None`: nothing, or it was cancelled).
struct Done {
    n: u64,
    turn: u64,
    kind: Kind,
    text: Option<String>,
    cancel: Arc<AtomicBool>,
}

impl Done {
    fn said(&self) -> Option<String> {
        self.text.clone().filter(|t| !t.is_empty() && !self.cancel.load(Ordering::Relaxed))
    }
}

/// Whisper's thread: each job in the order sent, a job cancelled before its
/// turn skipped.
fn work(whisper: Whisper, jobs: std::sync::mpsc::Receiver<Job>, done: mpsc::UnboundedSender<Done>) {
    let how = How::default();
    // A guess only shows how far the reader has got: the quicker way does.
    let guessing = How { whole_below: 0, ..how };
    let mut hint: (Arc<str>, String) = (Arc::from(""), String::new());
    for j in jobs {
        if !Arc::ptr_eq(&hint.0, &j.prompt) {
            hint = (j.prompt.clone(), whisper.fit_prompt(&j.prompt));
        }
        let text = if j.cancel.load(Ordering::Relaxed) || !audio::has_signal(&j.samples, j.gate) {
            None
        } else {
            whisper.hear(&j.samples, &hint.1, &j.cancel, if j.kind == Kind::Guess { &guessing } else { &how })
        };
        if done.send(Done { n: j.n, turn: j.turn, kind: j.kind, text, cancel: j.cancel }).is_err() {
            break;
        }
    }
}

async fn run(model: PathBuf, mut rx: mpsc::UnboundedReceiver<Ask>, tell: impl Fn(Heard)) {
    let whisper = match Whisper::load(&model) {
        Ok(w) => w,
        Err(e) => return tell(Heard::Failed(e)),
    };
    // The first time through sets Whisper's buffers up: do it now.
    let _ = whisper.hear(&[0.0; RATE], "", &AtomicBool::new(false), &How::default());
    let (jobs, queue) = std::sync::mpsc::channel();
    let (heard, mut done) = mpsc::unbounded_channel();
    if let Err(e) = std::thread::Builder::new().name("whisper".into()).spawn(move || work(whisper, queue, heard)) {
        return tell(Heard::Failed(e.to_string()));
    }
    tell(Heard::Ready);
    let mut jobs = Jobs { tx: jobs, n: 0 };
    let mut next = None;
    loop {
        let ask = match next.take() {
            Some(a) => a,
            None => tokio::select! {
                a = rx.recv() => match a {
                    Some(a) => a,
                    None => break,
                },
                Some(d) = done.recv() => {
                    // The last words before the microphone closed.
                    if d.kind == Kind::Final && let Some(t) = d.said() {
                        tell(Heard::Said(d.turn, t));
                    }
                    continue;
                }
            },
        };
        if let Ask::Open { turn, prompt } = ask {
            next = listen(turn, Arc::from(prompt), &mut jobs, &mut rx, &mut done, &tell).await;
        }
    }
}

struct Jobs {
    tx: std::sync::mpsc::Sender<Job>,
    n: u64,
}

impl Jobs {
    /// Give Whisper `samples` to hear; the job's number, and how to cancel it.
    fn send(
        &mut self,
        turn: u64,
        kind: Kind,
        samples: Vec<f32>,
        prompt: &Arc<str>,
        gate: f32,
    ) -> (u64, Arc<AtomicBool>) {
        self.n += 1;
        let cancel = Arc::new(AtomicBool::new(false));
        let _ =
            self.tx.send(Job { n: self.n, turn, kind, samples, prompt: prompt.clone(), gate, cancel: cancel.clone() });
        (self.n, cancel)
    }
}

/// How loud the room is by itself, so speech can be told from it.
#[derive(Default)]
struct Room {
    levels: VecDeque<(usize, f32)>,
    span: usize,
}

impl Room {
    fn hear(&mut self, samples: usize, level: f32) {
        self.levels.push_back((samples, level));
        self.span += samples;
        while let Some(&(n, _)) = self.levels.front()
            && self.span - n >= ROOM
        {
            self.levels.pop_front();
            self.span -= n;
        }
    }

    /// How loud speech must be, here.
    fn gate(&self) -> f32 {
        let floor = self.levels.iter().map(|l| l.1).fold(f32::INFINITY, f32::min);
        if floor.is_finite() { SPEECH.max(floor * OVER_ROOM) } else { SPEECH }
    }
}

/// Whisper's start on a stretch, at the first of a quiet.
struct Early {
    n: u64,
    cancel: Arc<AtomicBool>,
    /// What it heard, if it's done.
    text: Option<Option<String>>,
}

/// Listen until asked to stop, or too long quiet. Returns what was asked
/// next, if anything was.
async fn listen(
    turn: u64,
    prompt: Arc<str>,
    jobs: &mut Jobs,
    rx: &mut mpsc::UnboundedReceiver<Ask>,
    done: &mut mpsc::UnboundedReceiver<Done>,
    tell: &impl Fn(Heard),
) -> Option<Ask> {
    let mut child = match tokio::process::Command::new("pw-record")
        .args(["--raw", "--rate=16000", "--channels=1", "--format=s16", "--media-role=Communication"])
        .arg("--properties=application.name=Ondoku node.description=Ondoku")
        .arg("-")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .kill_on_drop(true)
        .spawn()
    {
        Ok(c) => c,
        Err(e) => {
            tell(Heard::Failed(format!("can't start pw-record: {e}")));
            tell(Heard::Closed(turn));
            return None;
        }
    };
    let Some(mut out) = child.stdout.take() else {
        tell(Heard::Closed(turn));
        return None;
    };
    let mut decoder = PcmDecoder::default();
    let mut buf = [0u8; 3200];
    let mut room = Room::default();
    let mut stretch: Vec<f32> = Vec::new();
    let mut speaking = false;
    let mut quiet = 0usize;
    let mut stuck_told = false;
    let mut guessed_at = 0usize;
    // The guess Whisper's working on, Whisper's start on this stretch, and
    // stretches ended whose words haven't been told yet.
    let mut guess: Option<(u64, Arc<AtomicBool>)> = None;
    let mut early: Option<Early> = None;
    let mut waiting: VecDeque<u64> = VecDeque::new();
    let mut reading = false;
    let mut muted_until: Option<std::time::Instant> = None;
    let cancel = |c: Option<Arc<AtomicBool>>| {
        if let Some(c) = c {
            c.store(true, Ordering::Relaxed);
        }
    };
    let asked = loop {
        tokio::select! {
            ask = rx.recv() => match ask {
                Some(Ask::Mute(d)) => {
                    muted_until = Some(std::time::Instant::now() + d);
                    continue;
                }
                other => break other,
            },
            Some(d) = done.recv() => {
                if d.turn != turn {
                    if d.kind == Kind::Final && let Some(t) = d.said() {
                        tell(Heard::Said(d.turn, t));
                    }
                    continue;
                }
                match d.kind {
                    Kind::Guess => {
                        if guess.as_ref().is_some_and(|g| g.0 == d.n) {
                            guess = None;
                            if speaking && let Some(t) = d.said() {
                                tell(Heard::Guess(turn, t));
                            }
                        }
                    }
                    Kind::Final => {
                        if waiting.front() == Some(&d.n) {
                            waiting.pop_front();
                            if let Some(t) = d.said() {
                                tell(Heard::Said(turn, t));
                            }
                        } else if let Some(e) = early.as_mut().filter(|e| e.n == d.n) {
                            e.text = Some(d.said());
                        }
                    }
                }
            }
            n = out.read(&mut buf) => {
                let n = match n { Ok(0) | Err(_) => { tell(Heard::Failed("the microphone stopped".into())); break None }, Ok(n) => n };
                let chunk = decoder.decode(&buf[..n]);
                if muted_until.is_some_and(|t| std::time::Instant::now() < t) {
                    // Fumi talking: none of it is reading, and it ends no stretch.
                    cancel(guess.take().map(|g| g.1));
                    cancel(early.take().map(|e| e.cancel));
                    stretch.clear();
                    speaking = false;
                    quiet = 0;
                    guessed_at = 0;
                } else {
                    let level = audio::rms(&chunk);
                    room.hear(chunk.len(), level);
                    let gate = room.gate();
                    tell(Heard::Level(turn, level));
                    if level >= gate {
                        if quiet > 0 {
                            // They went on: Whisper's start was too soon.
                            cancel(early.take().map(|e| e.cancel));
                        }
                        speaking = true;
                        quiet = 0;
                        stuck_told = false;
                    } else {
                        quiet += chunk.len();
                    }
                    stretch.extend_from_slice(&chunk);
                    if !speaking && stretch.len() > PRE_ROLL {
                        stretch.drain(..stretch.len() - PRE_ROLL);
                    }
                    let paused = speaking && quiet >= PAUSE;
                    if speaking && !paused && quiet >= EARLY && early.is_none() {
                        cancel(guess.take().map(|g| g.1));
                        let (n, c) = jobs.send(turn, Kind::Final, stretch.clone(), &prompt, gate);
                        early = Some(Early { n, cancel: c, text: None });
                    }
                    if paused || stretch.len() >= LONGEST {
                        cancel(guess.take().map(|g| g.1));
                        let said = std::mem::take(&mut stretch);
                        speaking = false;
                        guessed_at = 0;
                        match early.take() {
                            Some(Early { text: Some(t), .. }) if paused => {
                                if let Some(t) = t {
                                    tell(Heard::Said(turn, t));
                                }
                            }
                            Some(e) if paused => waiting.push_back(e.n),
                            e => {
                                cancel(e.map(|e| e.cancel));
                                waiting.push_back(jobs.send(turn, Kind::Final, said, &prompt, gate).0);
                            }
                        }
                    } else if speaking && guess.is_none() && early.is_none() && stretch.len() >= guessed_at + GUESS_EVERY {
                        guessed_at = stretch.len();
                        guess = Some(jobs.send(turn, Kind::Guess, stretch.clone(), &prompt, gate));
                    }
                    if !speaking && waiting.is_empty() && quiet >= STUCK && !stuck_told {
                        stuck_told = true;
                        tell(Heard::Quiet(turn));
                    }
                    if quiet >= GIVE_UP {
                        break None;
                    }
                }
            }
        }
        let now = speaking || !waiting.is_empty();
        if now != reading {
            reading = now;
            tell(Heard::Reading(turn, now));
        }
    };
    let _ = child.kill().await;
    cancel(guess.take().map(|g| g.1));
    cancel(early.take().map(|e| e.cancel));
    // The last words before the stop: told when heard, as are any still
    // waiting.
    if speaking {
        jobs.send(turn, Kind::Final, stretch, &prompt, room.gate());
    }
    tell(Heard::Closed(turn));
    asked
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_prompt_names_the_pages_words_once() {
        assert_eq!(prompt("Sam has a cat. The cat is fat."), "A child reading aloud. Sam, has, a, cat, The, is, fat");
    }

    #[test]
    fn a_hum_raises_the_gate_and_quiet_leaves_it() {
        let mut room = Room::default();
        for _ in 0..50 {
            room.hear(1600, 0.002);
        }
        assert_eq!(room.gate(), SPEECH);
        // A fan comes on: once the quiet before it has passed, it's the room.
        for _ in 0..120 {
            room.hear(1600, 0.012);
        }
        assert!((room.gate() - 0.024).abs() < 1e-6);
        // Reading over it, with the gaps between words, keeps it the room.
        for i in 0..100 {
            room.hear(1600, if i % 4 == 0 { 0.012 } else { 0.08 });
        }
        assert!((room.gate() - 0.024).abs() < 1e-6);
    }
}
