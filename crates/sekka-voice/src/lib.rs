//! The voice Sekka Edu's apps speak with: Kokoro-82M ([`kokoro`]), a small
//! neural voice that runs on this computer, through sherpa-onnx; or, where
//! those aren't installed, espeak-ng. It speaks on a thread of its own,
//! one thing at a time, the latest cutting in. What it has said is kept
//! as WAV files under `$XDG_CACHE_HOME/sekka-edu/voice`, so a line is made
//! only once; between lines it makes the ones an app expects to need
//! ([`Voice::prepare`]), so they start at once.
//!
//! Where Kokoro is:
//! - `SEKKA_EDU_KOKORO`: the model's folder (`nix/kokoro.nix`).
//! - `SEKKA_EDU_SHERPA_ONNX`: `libsherpa-onnx-c-api.so`, else the library
//!   search path's.

pub mod kokoro;
mod wav;

use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::mpsc::{Receiver, RecvTimeoutError, Sender, channel};
use std::time::Duration;

pub use wav::wav;

/// Kokoro's speaker: af_heart, a warm, clear American voice (sherpa-onnx's
/// `kokoro-multi-lang-v1_0` numbers its 53 speakers; 3 is af_heart).
pub const SPEAKER: i32 = 3;
/// A little slower than Kokoro's own pace, for young listeners.
pub const SPEED: f32 = 0.92;

/// The lines kept at most; the oldest go first.
const KEEP: usize = 4000;

/// A program on `PATH`.
pub fn which(name: &str) -> Option<PathBuf> {
    std::env::var_os("PATH")?.to_str()?.split(':').map(|d| Path::new(d).join(name)).find(|p| p.is_file())
}

/// The first player there is, with its arguments before the file.
pub fn player() -> Option<(PathBuf, Vec<&'static str>)> {
    if let Some(p) = which("pw-play") {
        return Some((p, vec![]));
    }
    if let Some(p) = which("paplay") {
        return Some((p, vec![]));
    }
    which("aplay").map(|p| (p, vec!["-q"]))
}

/// Which voice is speaking.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    /// Kokoro: natural.
    Kokoro,
    /// espeak-ng: robotic, but there.
    Espeak,
}

impl Kind {
    pub fn name(self) -> &'static str {
        match self {
            Kind::Kokoro => "Kokoro",
            Kind::Espeak => "eSpeak NG",
        }
    }
}

/// Something that turns text into a WAV file.
pub trait Engine {
    /// What's in the name of the files it makes: another engine, voice or
    /// pace makes other files.
    fn tag(&self) -> String;
    /// Write `text`, spoken, to `out`.
    fn make(&mut self, text: &str, out: &Path) -> bool;
}

struct KokoroEngine(kokoro::Kokoro);

impl Engine for KokoroEngine {
    fn tag(&self) -> String {
        format!("kokoro-1.0-{SPEAKER}-{SPEED}")
    }
    fn make(&mut self, text: &str, out: &Path) -> bool {
        match self.0.speak(text) {
            Some((samples, rate)) => write_atomically(out, &wav(&samples, rate)),
            None => false,
        }
    }
}

struct EspeakEngine(PathBuf);

impl Engine for EspeakEngine {
    fn tag(&self) -> String {
        "espeak-ng-f3-135-62".into()
    }
    fn make(&mut self, text: &str, out: &Path) -> bool {
        let part = out.with_extension("part");
        let made = Command::new(&self.0)
            .args(["-v", "en-us+f3", "-s", "135", "-p", "62", "-w"])
            .arg(&part)
            .arg(text)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .is_ok_and(|s| s.success());
        made && std::fs::rename(&part, out).is_ok()
    }
}

fn write_atomically(out: &Path, bytes: &[u8]) -> bool {
    let part = out.with_extension("part");
    std::fs::write(&part, bytes).is_ok() && std::fs::rename(&part, out).is_ok()
}

/// Where Kokoro's model and sherpa-onnx are, if they're given.
fn kokoro_paths() -> Option<(PathBuf, PathBuf)> {
    let dir = PathBuf::from(std::env::var_os("SEKKA_EDU_KOKORO").filter(|v| !v.is_empty())?);
    let lib = std::env::var_os("SEKKA_EDU_SHERPA_ONNX")
        .filter(|v| !v.is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("libsherpa-onnx-c-api.so"));
    Some((dir, lib))
}

/// Kokoro if it loads, else espeak-ng if it's installed.
fn engine() -> Option<(Box<dyn Engine>, Kind)> {
    if let Some((dir, lib)) = kokoro_paths() {
        // Two threads: the app's own drawing comes first.
        match kokoro::Kokoro::open(&lib, &dir, SPEAKER, SPEED, 2) {
            Ok(k) => return Some((Box::new(KokoroEngine(k)), Kind::Kokoro)),
            Err(e) => log::warn!("voice: {e}; speaking with espeak-ng instead"),
        }
    }
    let e = which("espeak-ng").or_else(|| which("espeak"))?;
    Some((Box::new(EspeakEngine(e)), Kind::Espeak))
}

/// Where the voice keeps what it has said.
pub fn cache_root() -> PathBuf {
    let base = std::env::var_os("XDG_CACHE_HOME")
        .filter(|v| !v.is_empty())
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".cache")))
        .unwrap_or_else(std::env::temp_dir);
    base.join("sekka-edu").join("voice")
}

/// The file `text` is kept in, under `dir`.
pub fn file_for(dir: &Path, text: &str) -> PathBuf {
    let key = text.bytes().fold(0xcbf2_9ce4_8422_2325u64, |h, b| (h ^ u64::from(b)).wrapping_mul(0x100_0000_01b3));
    dir.join(format!("{key:016x}.wav"))
}

/// Keep at most `keep` lines in `dir`, the most recently made.
pub fn prune(dir: &Path, keep: usize) {
    let Ok(entries) = std::fs::read_dir(dir) else { return };
    let mut files: Vec<(std::time::SystemTime, PathBuf)> = entries
        .flatten()
        .filter(|e| e.path().extension().is_some_and(|x| x == "wav" || x == "part"))
        .filter_map(|e| Some((e.metadata().ok()?.modified().ok()?, e.path())))
        .collect();
    if files.len() <= keep {
        return;
    }
    files.sort();
    for (_, p) in &files[..files.len() - keep] {
        let _ = std::fs::remove_file(p);
    }
}

enum Ask {
    Say(String),
    Hush,
    Prepare(Vec<String>),
}

/// A voice: Kokoro, espeak-ng, or none, speaking on its own thread.
pub struct Voice {
    pub on: bool,
    tx: Option<Sender<Ask>>,
    kind: Option<Kind>,
}

impl Voice {
    /// A voice that speaks when `on`; `quiet` (tests) never makes a sound.
    /// Kokoro loads on the voice's thread, so this returns at once; until
    /// it's loaded (a second or two), what's said waits.
    pub fn new(on: bool, quiet: bool) -> Voice {
        if quiet {
            return Voice { on, tx: None, kind: None };
        }
        let Some(player) = player() else {
            return Voice { on, tx: None, kind: None };
        };
        // Which it will be, without loading it here: Kokoro's model given
        // and its library found, or espeak-ng.
        let kind = match kokoro_paths() {
            Some((dir, _)) if kokoro::model_ready(&dir) => Some(Kind::Kokoro),
            _ => (which("espeak-ng").is_some() || which("espeak").is_some()).then_some(Kind::Espeak),
        };
        if kind.is_none() {
            return Voice { on, tx: None, kind };
        }
        let (tx, rx) = channel();
        let started = std::thread::Builder::new().name("voice".into()).spawn(move || {
            // Below the app's own threads (onnxruntime's take this too).
            // SAFETY: setpriority on this thread's id changes only it.
            unsafe {
                libc::setpriority(libc::PRIO_PROCESS, libc::gettid() as libc::id_t, 5);
            }
            let Some((mut engine, _)) = engine() else { return };
            let dir = cache_root().join(engine.tag());
            let _ = std::fs::create_dir_all(&dir);
            prune(&dir, KEEP);
            speaker(rx, engine.as_mut(), &player, &dir);
        });
        Voice { on, tx: started.ok().map(|_| tx), kind }
    }

    /// Whether anything can speak here.
    pub fn available(&self) -> bool {
        self.tx.is_some()
    }

    /// Which voice speaks (as far as can be told before it has loaded:
    /// Kokoro that fails to load gives way to espeak-ng).
    pub fn kind(&self) -> Option<Kind> {
        self.kind.filter(|_| self.tx.is_some())
    }

    /// Say `text`, stopping whatever's being said.
    pub fn say(&self, text: impl Into<String>) {
        if self.on
            && let Some(tx) = &self.tx
        {
            let _ = tx.send(Ask::Say(text.into()));
        }
    }

    /// Stop talking.
    pub fn hush(&self) {
        if let Some(tx) = &self.tx {
            let _ = tx.send(Ask::Hush);
        }
    }

    /// Make these lines ready while nothing's being said, so they start at
    /// once when they're wanted.
    pub fn prepare<S: Into<String>>(&self, lines: impl IntoIterator<Item = S>) {
        if self.on
            && let Some(tx) = &self.tx
        {
            let _ = tx.send(Ask::Prepare(lines.into_iter().map(Into::into).collect()));
        }
    }
}

/// The file `text` is in, made now if it wasn't already.
fn made(engine: &mut dyn Engine, dir: &Path, text: &str) -> Option<PathBuf> {
    let file = file_for(dir, text);
    if file.exists() || engine.make(text, &file) { Some(file) } else { None }
}

fn start(player: &(PathBuf, Vec<&'static str>), file: &Path) -> Option<Child> {
    Command::new(&player.0)
        .args(&player.1)
        .arg(file)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .ok()
}

fn stop(playing: &mut Option<Child>) {
    if let Some(mut c) = playing.take() {
        let _ = c.kill();
        let _ = c.wait();
    }
}

/// The voice's thread: say what's asked, the latest winning; between
/// lines, make the ones prepared.
fn speaker(rx: Receiver<Ask>, engine: &mut dyn Engine, player: &(PathBuf, Vec<&'static str>), dir: &Path) {
    let mut playing: Option<Child> = None;
    let mut ahead: VecDeque<String> = VecDeque::new();
    loop {
        // Idle: wait. Speaking: look in now and then to reap it. Lines to
        // make: only look.
        let first = if playing.is_some() || !ahead.is_empty() {
            let wait = if playing.is_some() { Duration::from_millis(50) } else { Duration::ZERO };
            match rx.recv_timeout(wait) {
                Ok(a) => Some(a),
                Err(RecvTimeoutError::Timeout) => None,
                Err(RecvTimeoutError::Disconnected) => break,
            }
        } else {
            match rx.recv() {
                Ok(a) => Some(a),
                Err(_) => break,
            }
        };
        let mut say: Option<String> = None;
        for ask in first.into_iter().chain(std::iter::from_fn(|| rx.try_recv().ok())) {
            match ask {
                Ask::Say(t) => say = Some(t),
                Ask::Hush => say = Some(String::new()),
                Ask::Prepare(lines) => ahead.extend(lines),
            }
        }
        if let Some(text) = say {
            stop(&mut playing);
            ahead.retain(|t| *t != text);
            if !text.is_empty()
                && let Some(file) = made(engine, dir, &text)
            {
                playing = start(player, &file);
            }
            continue;
        }
        if let Some(c) = playing.as_mut() {
            if let Ok(Some(_)) = c.try_wait() {
                playing = None;
            }
            continue;
        }
        // Quiet: make one prepared line, then look for asks again.
        if let Some(text) = ahead.pop_front() {
            let _ = made(engine, dir, &text);
        }
    }
    stop(&mut playing);
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};
    use std::time::Instant;

    /// Writes a tenth of a second of quiet, noting what it was asked.
    struct Fake(Arc<Mutex<Vec<String>>>);

    impl Engine for Fake {
        fn tag(&self) -> String {
            "fake".into()
        }
        fn make(&mut self, text: &str, out: &Path) -> bool {
            self.0.lock().unwrap().push(text.to_owned());
            write_atomically(out, &wav(&[0.0; 2400], 24_000))
        }
    }

    fn scratch(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("sekka-voice-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    fn until(what: impl Fn() -> bool) -> bool {
        let start = Instant::now();
        while start.elapsed() < Duration::from_secs(5) {
            if what() {
                return true;
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        false
    }

    #[test]
    fn the_latest_line_wins_and_prepared_lines_are_made_between() {
        let dir = scratch("speaker");
        let asked = Arc::new(Mutex::new(Vec::new()));
        let (tx, rx) = channel();
        // Everything asked before the thread looks: one Say wins.
        tx.send(Ask::Say("Show me seven.".into())).unwrap();
        tx.send(Ask::Say("Show me eight.".into())).unwrap();
        tx.send(Ask::Prepare(vec!["Great! Eight!".into(), "Show me eight.".into(), "Now make nine.".into()])).unwrap();
        let (a, d) = (asked.clone(), dir.clone());
        // A player that finishes at once.
        let player = (PathBuf::from("/bin/sh"), vec!["-c", "exit 0"]);
        let t = std::thread::spawn(move || speaker(rx, &mut Fake(a), &player, &d));
        assert!(until(|| asked.lock().unwrap().len() >= 3), "{:?}", asked.lock().unwrap());
        // Said first; then what was prepared, less what was just said.
        assert_eq!(*asked.lock().unwrap(), ["Show me eight.", "Great! Eight!", "Now make nine."]);
        // A prepared line is said from what was made, not made again.
        tx.send(Ask::Say("Great! Eight!".into())).unwrap();
        std::thread::sleep(Duration::from_millis(150));
        assert_eq!(asked.lock().unwrap().len(), 3);
        assert!(file_for(&dir, "Now make nine.").exists());
        drop(tx);
        t.join().unwrap();
    }

    #[test]
    fn the_oldest_lines_go_first() {
        let dir = scratch("prune");
        for i in 0..5 {
            let f = dir.join(format!("{i}.wav"));
            std::fs::write(&f, b"x").unwrap();
            let t = std::time::SystemTime::now() - Duration::from_secs(100 - i);
            std::fs::File::options().write(true).open(&f).unwrap().set_modified(t).unwrap();
        }
        prune(&dir, 3);
        let mut left: Vec<String> =
            std::fs::read_dir(&dir).unwrap().map(|e| e.unwrap().file_name().to_string_lossy().into()).collect();
        left.sort();
        assert_eq!(left, ["2.wav", "3.wav", "4.wav"]);
    }

    #[test]
    fn a_quiet_voice_says_nothing() {
        let v = Voice::new(true, true);
        assert!(!v.available());
        assert_eq!(v.kind(), None);
        v.say("Hello");
        v.prepare(["Hello"]);
    }
}
