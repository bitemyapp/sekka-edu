//! Sounds and Pon's voice. Sounds are made here (a wooden clack, chimes, a
//! little fanfare), written once as WAV files and played with PipeWire's
//! `pw-play` (or `paplay`, or `aplay`); Pon speaks with `espeak-ng` when
//! it's installed. Without a player there's quiet, and the app works the
//! same: every prompt is also shown.

use std::collections::HashMap;
use std::f32::consts::TAU;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::mpsc::{Receiver, RecvTimeoutError, Sender, channel};
use std::time::{Duration, Instant};

const RATE: u32 = 44_100;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Sound {
    /// A bead against the beam.
    Clack,
    /// A bead going home.
    Soft,
    Right,
    /// Gentle: not a buzzer.
    Miss,
    Star,
    Lantern,
    Tap,
    Fanfare,
}

impl Sound {
    const ALL: [Sound; 8] =
        [Sound::Clack, Sound::Soft, Sound::Right, Sound::Miss, Sound::Star, Sound::Lantern, Sound::Tap, Sound::Fanfare];

    fn name(self) -> &'static str {
        match self {
            Sound::Clack => "clack",
            Sound::Soft => "soft",
            Sound::Right => "right",
            Sound::Miss => "miss",
            Sound::Star => "star",
            Sound::Lantern => "lantern",
            Sound::Tap => "tap",
            Sound::Fanfare => "fanfare",
        }
    }
}

/// A program on `PATH`.
fn which(name: &str) -> Option<PathBuf> {
    std::env::var_os("PATH")?.to_str()?.split(':').map(|d| Path::new(d).join(name)).find(|p| p.is_file())
}

/// The first player there is, with its arguments before the file.
fn player() -> Option<(PathBuf, Vec<&'static str>)> {
    if let Some(p) = which("pw-play") {
        return Some((p, vec![]));
    }
    if let Some(p) = which("paplay") {
        return Some((p, vec![]));
    }
    which("aplay").map(|p| (p, vec!["-q"]))
}

fn cache_dir(name: &str) -> PathBuf {
    let base = std::env::var_os("XDG_RUNTIME_DIR")
        .filter(|v| !v.is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir);
    base.join(name)
}

/// A tone that rings and fades: `freq` Hz, partials at `harmonics` of
/// it (multiple, level), dying away over `decay` seconds, from `at`.
fn ring(out: &mut [f32], at: f32, freq: f32, decay: f32, level: f32, harmonics: &[(f32, f32)]) {
    let start = (at * RATE as f32) as usize;
    for (i, s) in out.iter_mut().enumerate().skip(start) {
        let t = (i - start) as f32 / RATE as f32;
        let env = (-t / decay).exp() * (1.0 - (-t * 400.0).exp());
        let mut v = (TAU * freq * t).sin();
        for (m, l) in harmonics {
            v += l * (TAU * freq * m * t).sin();
        }
        *s += v * env * level;
    }
}

/// A sound's samples, -1 to 1.
fn synth(sound: Sound) -> Vec<f32> {
    let len = |secs: f32| vec![0.0f32; (secs * RATE as f32) as usize];
    let mut noise = 0x1234_5678u32;
    let mut rand = move || {
        noise ^= noise << 13;
        noise ^= noise >> 17;
        noise ^= noise << 5;
        (noise as f32 / u32::MAX as f32) * 2.0 - 1.0
    };
    match sound {
        Sound::Clack | Sound::Soft | Sound::Tap => {
            // Wood on wood: a click and a few short, high resonances.
            let (level, pitch) = match sound {
                Sound::Clack => (0.5, 1.0),
                Sound::Soft => (0.25, 0.8),
                _ => (0.2, 1.3),
            };
            let mut v = len(0.09);
            for (i, s) in v.iter_mut().enumerate().take(90) {
                *s += rand() * (1.0 - i as f32 / 90.0) * 0.6 * level;
            }
            for (f, d, l) in [(1850.0, 0.018, 0.6), (2930.0, 0.012, 0.35), (4400.0, 0.008, 0.2)] {
                ring(&mut v, 0.0, f * pitch, d, l * level, &[]);
            }
            v
        }
        Sound::Right => {
            // A marimba's rising arpeggio: C, E, G, C.
            let mut v = len(0.9);
            for (i, f) in [523.25, 659.25, 783.99, 1046.5].iter().enumerate() {
                ring(&mut v, i as f32 * 0.085, *f, 0.28, 0.22, &[(4.0, 0.12), (10.0, 0.03)]);
            }
            v
        }
        Sound::Miss => {
            // "Hmm?": two soft notes, down a third.
            let mut v = len(0.6);
            ring(&mut v, 0.0, 392.0, 0.2, 0.18, &[(2.0, 0.1)]);
            ring(&mut v, 0.14, 329.63, 0.25, 0.16, &[(2.0, 0.1)]);
            v
        }
        Sound::Star => {
            let mut v = len(0.6);
            ring(&mut v, 0.0, 1318.5, 0.18, 0.2, &[(2.0, 0.15), (3.0, 0.05)]);
            ring(&mut v, 0.06, 1975.5, 0.2, 0.14, &[(2.0, 0.1)]);
            v
        }
        Sound::Lantern => {
            // A shimmering chord that swells and fades.
            let mut v = len(1.4);
            for (i, f) in [1046.5f32, 1318.5, 1568.0, 2093.0].iter().enumerate() {
                let start = (i as f32 * 0.05 * RATE as f32) as usize;
                for (n, s) in v.iter_mut().enumerate().skip(start) {
                    let t = (n - start) as f32 / RATE as f32;
                    let env = (t * 6.0).min(1.0) * (-t / 0.45).exp();
                    let vib = 1.0 + 0.004 * (TAU * 6.0 * t).sin();
                    *s += (TAU * f * vib * t).sin() * env * 0.09;
                }
            }
            v
        }
        Sound::Fanfare => {
            // Ta-da: G, C, E, then a held C chord.
            let mut v = len(1.6);
            for (at, f) in [(0.0, 392.0), (0.11, 523.25), (0.22, 659.25)] {
                ring(&mut v, at, f, 0.16, 0.2, &[(2.0, 0.2), (3.0, 0.08)]);
            }
            for f in [523.25, 659.25, 783.99, 1046.5] {
                ring(&mut v, 0.36, f, 0.55, 0.12, &[(2.0, 0.15), (4.0, 0.05)]);
            }
            v
        }
    }
}

/// 16-bit mono WAV.
fn wav(samples: &[f32]) -> Vec<u8> {
    let data = (samples.len() * 2) as u32;
    let mut b = Vec::with_capacity(44 + data as usize);
    b.extend_from_slice(b"RIFF");
    b.extend_from_slice(&(36 + data).to_le_bytes());
    b.extend_from_slice(b"WAVEfmt ");
    b.extend_from_slice(&16u32.to_le_bytes());
    b.extend_from_slice(&1u16.to_le_bytes());
    b.extend_from_slice(&1u16.to_le_bytes());
    b.extend_from_slice(&RATE.to_le_bytes());
    b.extend_from_slice(&(RATE * 2).to_le_bytes());
    b.extend_from_slice(&2u16.to_le_bytes());
    b.extend_from_slice(&16u16.to_le_bytes());
    b.extend_from_slice(b"data");
    b.extend_from_slice(&data.to_le_bytes());
    for s in samples {
        b.extend_from_slice(&((s.clamp(-1.0, 1.0) * 32_000.0) as i16).to_le_bytes());
    }
    b
}

/// Play `file` with `player`, on a thread that waits for it (no zombies).
fn play_file(player: &(PathBuf, Vec<&'static str>), file: PathBuf) {
    let mut cmd = Command::new(&player.0);
    cmd.args(&player.1).arg(file).stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null());
    let _ = std::thread::Builder::new().name("sound".into()).spawn(move || {
        if let Ok(mut child) = cmd.spawn() {
            let _ = child.wait();
        }
    });
}

/// The app's sounds.
pub struct Sounds {
    pub on: bool,
    player: Option<(PathBuf, Vec<&'static str>)>,
    files: HashMap<Sound, PathBuf>,
    last: HashMap<Sound, Instant>,
}

impl Sounds {
    /// `quiet`: never play anything (tests).
    pub fn new(on: bool, quiet: bool) -> Sounds {
        Sounds { on, player: if quiet { None } else { player() }, files: HashMap::new(), last: HashMap::new() }
    }

    fn file(&mut self, sound: Sound) -> Option<PathBuf> {
        if let Some(f) = self.files.get(&sound) {
            return Some(f.clone());
        }
        let dir = cache_dir("sekka-anzan-sounds");
        std::fs::create_dir_all(&dir).ok()?;
        for s in Sound::ALL {
            let path = dir.join(format!("{}.wav", s.name()));
            if !path.exists() {
                std::fs::write(&path, wav(&synth(s))).ok()?;
            }
            self.files.insert(s, path);
        }
        self.files.get(&sound).cloned()
    }

    pub fn play(&mut self, sound: Sound) {
        if !self.on || self.player.is_none() {
            return;
        }
        // A burst of beads is one clack.
        let now = Instant::now();
        if self.last.get(&sound).is_some_and(|t| now.duration_since(*t) < Duration::from_millis(60)) {
            return;
        }
        self.last.insert(sound, now);
        if let (Some(file), Some(player)) = (self.file(sound), self.player.as_ref()) {
            play_file(player, file);
        }
    }
}

/// Pon's voice: speaks on a thread of its own, one thing at a time, the
/// latest winning.
pub struct Voice {
    pub on: bool,
    tx: Option<Sender<String>>,
}

impl Voice {
    pub fn new(on: bool, quiet: bool) -> Voice {
        let engine = if quiet { None } else { which("espeak-ng").or_else(|| which("espeak")) };
        let tx = match (engine, player()) {
            (Some(engine), Some(player)) => {
                let (tx, rx) = channel();
                let _ = std::thread::Builder::new().name("voice".into()).spawn(move || speaker(rx, engine, player));
                Some(tx)
            }
            _ => None,
        };
        Voice { on, tx }
    }

    /// Whether Pon can speak here at all.
    pub fn available(&self) -> bool {
        self.tx.is_some()
    }

    pub fn say(&self, text: impl Into<String>) {
        if self.on
            && let Some(tx) = &self.tx
        {
            let _ = tx.send(text.into());
        }
    }

    /// Stop talking.
    pub fn hush(&self) {
        if let Some(tx) = &self.tx {
            let _ = tx.send(String::new());
        }
    }
}

fn speaker(rx: Receiver<String>, engine: PathBuf, player: (PathBuf, Vec<&'static str>)) {
    let dir = cache_dir("sekka-anzan-voice");
    let _ = std::fs::create_dir_all(&dir);
    let mut playing: Option<Child> = None;
    loop {
        // Idle, wait; while speaking, look in now and then to reap it.
        let next = match playing.as_mut() {
            None => match rx.recv() {
                Ok(t) => t,
                Err(_) => break,
            },
            Some(child) => match rx.recv_timeout(Duration::from_millis(50)) {
                Ok(t) => t,
                Err(RecvTimeoutError::Timeout) => {
                    if let Ok(Some(_)) = child.try_wait() {
                        playing = None;
                    }
                    continue;
                }
                Err(RecvTimeoutError::Disconnected) => break,
            },
        };
        // Only the latest of what's waiting; it cuts in.
        let mut text = next;
        while let Ok(t) = rx.try_recv() {
            text = t;
        }
        if let Some(mut c) = playing.take() {
            let _ = c.kill();
            let _ = c.wait();
        }
        if !text.is_empty() {
            playing = speak(&engine, &player, &dir, &text);
        }
    }
    if let Some(mut c) = playing {
        let _ = c.kill();
        let _ = c.wait();
    }
}

/// Make `text` into speech (once: it's kept by its hash) and start it.
fn speak(engine: &Path, player: &(PathBuf, Vec<&'static str>), dir: &Path, text: &str) -> Option<Child> {
    let key = text.bytes().fold(0xcbf2_9ce4_8422_2325u64, |h, b| (h ^ u64::from(b)).wrapping_mul(0x100_0000_01b3));
    let file = dir.join(format!("{key:016x}.wav"));
    if !file.exists() {
        let made = Command::new(engine)
            .args(["-v", "en-us+f3", "-s", "135", "-p", "62", "-w"])
            .arg(&file)
            .arg(text)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
        if !made.is_ok_and(|s| s.success()) {
            return None;
        }
    }
    Command::new(&player.0)
        .args(&player.1)
        .arg(&file)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sounds_are_short_and_never_clip() {
        for s in Sound::ALL {
            let v = synth(s);
            assert!(!v.is_empty() && v.len() < RATE as usize * 2, "{s:?}");
            let peak = v.iter().fold(0.0f32, |m, x| m.max(x.abs()));
            assert!(peak > 0.02 && peak <= 1.0, "{s:?} peaks at {peak}");
        }
        let w = wav(&[0.0, 0.5, -0.5]);
        assert_eq!(&w[..4], b"RIFF");
        assert_eq!(w.len(), 44 + 6);
    }
}
