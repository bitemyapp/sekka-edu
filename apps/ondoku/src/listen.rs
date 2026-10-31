//! Fumi's ears: the microphone through PipeWire's `pw-record`, and Whisper
//! through Koe, Aizome's dictation, as a library: its recognizer
//! (`koe::engine`), its model catalogue and folder (`koe::models`), and its
//! audio decoding and gate (`koe::audio`).
//!
//! The microphone is open only while a kid is reading to Fumi (the big
//! button), and closes by itself after a long quiet. What it hears stays in
//! memory, is never written down, and goes nowhere: each stretch of reading
//! is recognised on this computer, then dropped.
//!
//! A stretch ends at a pause, and what was said in it goes to the page
//! ([`Heard::Said`]); while it goes on, a guess at it is made every second
//! or so ([`Heard::Guess`]), so the page can follow along.

use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use koe::audio::{self, PcmDecoder, RATE};
use koe::config::Engine;
use koe::engine::Recognizer;
use tokio::io::AsyncReadExt;
use tokio::sync::mpsc;

/// What the ears tell the app.
#[derive(Clone, Debug, PartialEq)]
pub enum Heard {
    /// Whisper's loaded and listening is possible.
    Ready,
    /// The microphone's loudness, a few times a second (RMS, 0 to about 0.3).
    Level(u64, f32),
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
/// A stretch is cut here even without a pause.
const LONGEST: usize = RATE * 12;
/// Guess again once this much more has been heard.
const GUESS_EVERY: usize = RATE;
/// Quiet this long after reading is "stuck", for Fumi to help.
const STUCK: usize = RATE * 4;
/// Quiet this long closes the microphone.
const GIVE_UP: usize = RATE * 40;
/// Kept from before speech starts, so its first sound isn't cut.
const PRE_ROLL: usize = RATE * 3 / 10;
/// Loud enough to be speech (Koe's own threshold).
const SPEECH: f32 = 0.008;

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

/// The ears: a thread with Whisper loaded, and the microphone when asked.
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

async fn run(model: PathBuf, mut rx: mpsc::UnboundedReceiver<Ask>, tell: impl Fn(Heard)) {
    let recognizer = Recognizer::new(Engine {
        model,
        language: "en".into(),
        threads: 4,
        prompt: String::new(),
        ..Engine::default()
    });
    match recognizer.warmup().await {
        Ok(()) => tell(Heard::Ready),
        Err(e) => return tell(Heard::Failed(e.to_string())),
    }
    let mut next = rx.recv().await;
    while let Some(ask) = next {
        next = match ask {
            Ask::Open { turn, prompt } => listen(&recognizer.with_prompt(prompt), turn, &mut rx, &tell).await,
            Ask::Close | Ask::Mute(_) => rx.recv().await,
        };
    }
}

/// Whisper's words for `samples`, or nothing (cancelled, or no speech).
async fn recognise(r: &Recognizer, samples: Vec<f32>, cancel: Arc<AtomicBool>) -> Option<String> {
    if !audio::has_signal(&samples, SPEECH) {
        return None;
    }
    match r.transcribe(samples, cancel).await {
        Ok(t) => Some(t.trim().to_owned()),
        Err(e) => {
            log::debug!("ears: {e}");
            None
        }
    }
}

/// Listen until asked to stop, or too long quiet. Returns what was asked
/// next, if anything was.
async fn listen(
    r: &Recognizer,
    turn: u64,
    rx: &mut mpsc::UnboundedReceiver<Ask>,
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
            return rx.recv().await;
        }
    };
    let Some(mut out) = child.stdout.take() else {
        tell(Heard::Closed(turn));
        return rx.recv().await;
    };
    let mut decoder = PcmDecoder::default();
    let mut buf = [0u8; 3200];
    let mut stretch: Vec<f32> = Vec::new();
    let mut speaking = false;
    let mut quiet = 0usize;
    let mut stuck_told = false;
    let mut guessed_at = 0usize;
    let mut guess: Option<(tokio::task::JoinHandle<Option<String>>, Arc<AtomicBool>)> = None;
    let mut muted_until: Option<std::time::Instant> = None;
    let asked = loop {
        tokio::select! {
            ask = rx.recv() => match ask {
                Some(Ask::Mute(d)) => {
                    muted_until = Some(std::time::Instant::now() + d);
                    continue;
                }
                other => break other,
            },
            done = async { (&mut guess.as_mut().unwrap().0).await }, if guess.is_some() => {
                guess = None;
                if let Ok(Some(t)) = done && !t.is_empty() {
                    tell(Heard::Guess(turn, t));
                }
            }
            n = out.read(&mut buf) => {
                let n = match n { Ok(0) | Err(_) => { tell(Heard::Failed("the microphone stopped".into())); break None }, Ok(n) => n };
                let chunk = decoder.decode(&buf[..n]);
                if muted_until.is_some_and(|t| std::time::Instant::now() < t) {
                    // Fumi talking: none of it is reading, and it ends no stretch.
                    if let Some((_, cancel)) = guess.take() {
                        cancel.store(true, Ordering::Relaxed);
                    }
                    stretch.clear();
                    speaking = false;
                    quiet = 0;
                    guessed_at = 0;
                    continue;
                }
                let level = audio::rms(&chunk);
                tell(Heard::Level(turn, level));
                if level >= SPEECH {
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
                if paused || stretch.len() >= LONGEST {
                    if let Some((_, cancel)) = guess.take() {
                        cancel.store(true, Ordering::Relaxed);
                    }
                    let said = std::mem::take(&mut stretch);
                    speaking = false;
                    guessed_at = 0;
                    if let Some(t) = recognise(r, said, Arc::new(AtomicBool::new(false))).await && !t.is_empty() {
                        tell(Heard::Said(turn, t));
                    }
                } else if speaking && guess.is_none() && stretch.len() >= guessed_at + GUESS_EVERY {
                    guessed_at = stretch.len();
                    let (r, s, cancel) = (r.clone(), stretch.clone(), Arc::new(AtomicBool::new(false)));
                    let c = cancel.clone();
                    guess = Some((tokio::spawn(async move { recognise(&r, s, c).await }), cancel));
                }
                if !speaking && quiet >= STUCK && !stuck_told {
                    stuck_told = true;
                    tell(Heard::Quiet(turn));
                }
                if quiet >= GIVE_UP {
                    break None;
                }
            }
        }
    };
    let _ = child.kill().await;
    if let Some((_, cancel)) = guess.take() {
        cancel.store(true, Ordering::Relaxed);
    }
    // The last words before the stop.
    if speaking
        && let Some(t) = recognise(r, stretch, Arc::new(AtomicBool::new(false))).await
        && !t.is_empty()
    {
        tell(Heard::Said(turn, t));
    }
    tell(Heard::Closed(turn));
    match asked {
        Some(a) => Some(a),
        None => rx.recv().await,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_prompt_names_the_pages_words_once() {
        assert_eq!(prompt("Sam has a cat. The cat is fat."), "A child reading aloud. Sam, has, a, cat, The, is, fat");
    }
}
