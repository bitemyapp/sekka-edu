//! Sounds: made here (a wooden clack, chimes, a little fanfare), written
//! once as WAV files and played with PipeWire's `pw-play` (or `paplay`, or
//! `aplay`). Without a player there's quiet, and the app works the same.
//! Pon's voice is `sekka-voice`.

use std::collections::HashMap;
use std::f32::consts::TAU;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use sekka_voice::player;

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
    sekka_voice::wav(samples, RATE)
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
