//! Fumi's ears for real: Whisper through Koe, the microphone's stream cut at
//! pauses, and the page following along. A child's voice is stood in for by
//! Kokoro reading the page, played to the ears by a pretend `pw-record`.
//! Needs the voice and a Whisper model (`SEKKA_EDU_KOKORO`,
//! `SEKKA_EDU_SHERPA_ONNX`, `SEKKA_EDU_WHISPER`), so it's run by hand:
//! `cargo test -p sekka-ondoku --test ears -- --ignored --nocapture`.

use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::sync::mpsc::channel;
use std::time::{Duration, Instant};

use sekka_ondoku::follow::Page;
use sekka_ondoku::listen::{Ears, Heard};
use sekka_voice::kokoro::Kokoro;

/// `samples` at `from` Hz, at 16 kHz (linear: plenty for speech).
fn to_16k(samples: &[f32], from: u32) -> Vec<f32> {
    let n = (samples.len() as f64 * 16_000.0 / f64::from(from)) as usize;
    (0..n)
        .map(|i| {
            let x = i as f64 * f64::from(from) / 16_000.0;
            let (j, f) = (x as usize, (x.fract()) as f32);
            let a = samples.get(j).copied().unwrap_or(0.0);
            let b = samples.get(j + 1).copied().unwrap_or(a);
            a + (b - a) * f
        })
        .collect()
}

#[test]
#[ignore]
fn fumi_follows_a_page_read_aloud() {
    let env = |k: &str| PathBuf::from(std::env::var_os(k).unwrap_or_else(|| panic!("{k}")));
    let (kokoro, sherpa, whisper) = (env("SEKKA_EDU_KOKORO"), env("SEKKA_EDU_SHERPA_ONNX"), env("SEKKA_EDU_WHISPER"));
    let page = "Sam has a cat. The cat is fat and tan. Sam has a hat, and the cat naps in it.";
    // Read with a pause between the sentences, as a child takes a breath.
    let voice = Kokoro::open(&sherpa, &kokoro, 3, 0.85, 4).unwrap();
    let mut audio: Vec<f32> = vec![0.0; 8_000];
    for sentence in ["Sam has a cat. The cat is fat and tan.", "Sam has a hat, and the cat naps in it."] {
        let (s, rate) = voice.speak(sentence).unwrap();
        audio.extend(to_16k(&s, rate));
        audio.extend(std::iter::repeat_n(0.0, 16_000));
    }
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("ears");
    std::fs::create_dir_all(&dir).unwrap();
    let raw = dir.join("reading.raw");
    let bytes: Vec<u8> = audio.iter().flat_map(|s| ((s * 32_000.0) as i16).to_ne_bytes()).collect();
    std::fs::write(&raw, bytes).unwrap();
    // pw-record as the ears run it: raw 16 kHz to stdout, then the room's
    // quiet until it's stopped.
    let fake = dir.join("pw-record");
    std::fs::write(&fake, format!("#!/bin/sh\ncat '{}'\nexec cat /dev/zero\n", raw.display())).unwrap();
    std::fs::set_permissions(&fake, std::fs::Permissions::from_mode(0o755)).unwrap();
    // SAFETY: set before the ears' thread starts, and nothing else here
    // reads the environment concurrently.
    unsafe {
        std::env::set_var("PATH", format!("{}:{}", dir.display(), std::env::var("PATH").unwrap_or_default()));
    }

    let (tx, rx) = channel();
    let start = Instant::now();
    let ears = Ears::start(whisper, move |h| {
        let _ = tx.send(h);
    });
    let mut p = Page::new(page);
    let mut said = Vec::new();
    while let Ok(h) = rx.recv_timeout(Duration::from_secs(120)) {
        match h {
            Heard::Ready => {
                eprintln!("ready in {:.1} s", start.elapsed().as_secs_f64());
                ears.open(1, page);
            }
            Heard::Said(1, t) => {
                eprintln!("said: {t:?}");
                p.hear(&t, 0);
                said.push(t);
                if p.done() {
                    break;
                }
            }
            Heard::Guess(1, t) => eprintln!("guess: {t:?}"),
            Heard::Failed(e) => panic!("{e}"),
            Heard::Closed(_) => break,
            _ => {}
        }
    }
    ears.close();
    eprintln!("read {} of {} words; marks {:?}", p.read(), p.spoken(), p.marks);
    assert!(said.len() >= 2, "a stretch for each sentence: {said:?}");
    assert!(p.done(), "the page was followed to the end");
    assert!(p.read() * 10 >= p.spoken() * 9, "nine words in ten heard as read");
}

/// What Whisper is told to expect mustn't hide a word read wrongly: "red"
/// read for "tan" is noticed, and the right words around it still count.
#[test]
#[ignore]
fn a_misread_word_is_noticed() {
    let env = |k: &str| PathBuf::from(std::env::var_os(k).unwrap_or_else(|| panic!("{k}")));
    let (kokoro, sherpa, whisper) = (env("SEKKA_EDU_KOKORO"), env("SEKKA_EDU_SHERPA_ONNX"), env("SEKKA_EDU_WHISPER"));
    let page = "The cat is fat and tan. The cat naps on a mat.";
    let voice = Kokoro::open(&sherpa, &kokoro, 3, 0.85, 4).unwrap();
    let mut audio: Vec<f32> = vec![0.0; 8_000];
    for sentence in ["The cat is fat and red.", "The cat naps on a mat."] {
        let (s, rate) = voice.speak(sentence).unwrap();
        audio.extend(to_16k(&s, rate));
        audio.extend(std::iter::repeat_n(0.0, 16_000));
    }
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("ears-misread");
    std::fs::create_dir_all(&dir).unwrap();
    let raw = dir.join("reading.raw");
    let bytes: Vec<u8> = audio.iter().flat_map(|s| ((s * 32_000.0) as i16).to_ne_bytes()).collect();
    std::fs::write(&raw, bytes).unwrap();
    let fake = dir.join("pw-record");
    std::fs::write(&fake, format!("#!/bin/sh\ncat '{}'\nexec cat /dev/zero\n", raw.display())).unwrap();
    std::fs::set_permissions(&fake, std::fs::Permissions::from_mode(0o755)).unwrap();
    // SAFETY: as above.
    unsafe {
        std::env::set_var("PATH", format!("{}:{}", dir.display(), std::env::var("PATH").unwrap_or_default()));
    }
    let (tx, rx) = channel();
    let ears = Ears::start(whisper, move |h| {
        let _ = tx.send(h);
    });
    let mut p = Page::new(page);
    let mut stretches = 0;
    while let Ok(h) = rx.recv_timeout(Duration::from_secs(120)) {
        match h {
            Heard::Ready => ears.open(1, page),
            Heard::Said(1, t) => {
                eprintln!("said: {t:?}");
                p.hear(&t, 0);
                stretches += 1;
                if stretches == 2 {
                    break;
                }
            }
            Heard::Failed(e) => panic!("{e}"),
            Heard::Closed(_) => break,
            _ => {}
        }
    }
    ears.close();
    eprintln!("marks {:?}", p.marks);
    let tan = p.words.iter().position(|w| w.key == "tan").unwrap();
    assert_ne!(p.marks[tan], sekka_ondoku::follow::Mark::Read, "\"red\" isn't \"tan\"");
    assert!(p.done(), "the reader goes on past it");
}
