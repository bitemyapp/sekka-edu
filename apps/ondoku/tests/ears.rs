//! Fumi's ears for real: Whisper, the microphone's stream cut at pauses,
//! and the page following along. A child's voice is stood in for by Kokoro
//! reading the page, played to the ears at a microphone's pace by a pretend
//! `pw-record`. Needs the voice and a Whisper model (`SEKKA_EDU_KOKORO`,
//! `SEKKA_EDU_SHERPA_ONNX`, `SEKKA_EDU_WHISPER`), so it's run by hand:
//! `cargo test -p sekka-ondoku --test ears -- --ignored --nocapture`.

use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::sync::Mutex;
use std::sync::mpsc::channel;
use std::time::{Duration, Instant};

use sekka_ondoku::follow::{Mark, Page};
use sekka_ondoku::listen::{Ears, Heard};
use sekka_voice::kokoro::Kokoro;

/// One pretend microphone at a time: it's found on `PATH`.
static MIC: Mutex<()> = Mutex::new(());

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

/// Kokoro reading `sentences`, a second's breath after each, with `hiss`
/// (RMS) of the room's own sound under it all; and when each sentence
/// ends, in seconds.
fn reading(sentences: &[&str], hiss: f32) -> (Vec<f32>, Vec<f64>) {
    let env = |k: &str| PathBuf::from(std::env::var_os(k).unwrap_or_else(|| panic!("{k}")));
    let voice = Kokoro::open(&env("SEKKA_EDU_SHERPA_ONNX"), &env("SEKKA_EDU_KOKORO"), 3, 0.85, 4).unwrap();
    let mut audio: Vec<f32> = vec![0.0; 8_000];
    let mut ends = Vec::new();
    for s in sentences {
        let (s, rate) = voice.speak(s).unwrap();
        audio.extend(to_16k(&s, rate));
        ends.push(audio.len() as f64 / 16_000.0);
        audio.extend(std::iter::repeat_n(0.0, 16_000));
    }
    // The room after: long enough for every stretch to be heard, and for
    // a stuck reader to be noticed.
    audio.extend(std::iter::repeat_n(0.0, 16_000 * 6));
    let mut seed = 0x5eed_u32;
    for x in &mut audio {
        seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        // Even in -1..1 has an RMS of 1/√3.
        *x += hiss * 3f32.sqrt() * ((seed >> 8) as f32 / (1u32 << 23) as f32 - 1.0);
    }
    (audio, ends)
}

/// A pretend `pw-record` in `name`'s folder that plays `audio` as a
/// microphone would, a tenth of a second at a time, then waits.
fn microphone(name: &str, audio: &[f32]) {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(name);
    std::fs::create_dir_all(&dir).unwrap();
    let raw = dir.join("reading.raw");
    let bytes: Vec<u8> = audio.iter().flat_map(|s| ((s.clamp(-1.0, 1.0) * 32_000.0) as i16).to_ne_bytes()).collect();
    std::fs::write(&raw, bytes).unwrap();
    let fake = dir.join("pw-record");
    let script = format!(
        r#"#!/bin/sh
raw='{}'
n=$(( $(stat -c %s "$raw") / 3200 ))
start=$(date +%s%N)
i=0
while [ $i -lt $n ]; do
  dd if="$raw" bs=3200 skip=$i count=1 status=none
  i=$((i + 1))
  d=$(( start + i * 100000000 - $(date +%s%N) ))
  [ $d -gt 0 ] && sleep "0.$(printf %09d $d)"
done
exec sleep 3600
"#,
        raw.display()
    );
    std::fs::write(&fake, script).unwrap();
    std::fs::set_permissions(&fake, std::fs::Permissions::from_mode(0o755)).unwrap();
    // SAFETY: under MIC, before the ears' thread starts; nothing else here
    // reads the environment concurrently.
    unsafe {
        std::env::set_var("PATH", format!("{}:{}", dir.display(), std::env::var("PATH").unwrap_or_default()));
    }
}

/// What the ears heard of `audio` read from `page`: what each stretch was
/// heard as and how long after its sentence ended, and the page followed.
fn listen(page: &str, audio: &[f32], ends: &[f64], name: &str) -> (Vec<(String, f64)>, Page, bool) {
    microphone(name, audio);
    let whisper = PathBuf::from(std::env::var_os("SEKKA_EDU_WHISPER").expect("SEKKA_EDU_WHISPER"));
    let (tx, rx) = channel();
    let ears = Ears::start(whisper, move |h| {
        let _ = tx.send(h);
    });
    let mut p = Page::new(page);
    let mut said = Vec::new();
    let mut quiet = false;
    let mut opened = None;
    let ready = Instant::now();
    while let Ok(h) = rx.recv_timeout(Duration::from_secs(60)) {
        match h {
            Heard::Ready => {
                eprintln!("ready in {:.1} s", ready.elapsed().as_secs_f64());
                ears.open(1, page);
                opened = Some(Instant::now());
            }
            Heard::Said(1, t) => {
                let at = opened.unwrap().elapsed().as_secs_f64();
                // After the last sentence to end before it was heard.
                let late = at - ends.iter().copied().filter(|e| *e <= at).fold(0.0, f64::max);
                eprintln!("said {late:.2} s after the sentence ended: {t:?}");
                p.hear(&t, 0);
                said.push((t, late));
            }
            Heard::Guess(1, t) => eprintln!("guess: {t:?}"),
            Heard::Quiet(1) => {
                quiet = true;
                break;
            }
            Heard::Failed(e) => panic!("{e}"),
            Heard::Closed(_) => break,
            _ => {}
        }
    }
    ears.close();
    (said, p, quiet)
}

#[test]
#[ignore]
fn fumi_follows_a_page_read_aloud() {
    let _mic = MIC.lock().unwrap_or_else(|e| e.into_inner());
    let page = "Sam has a cat. The cat is fat and tan. Sam has a hat, and the cat naps in it.";
    // Read with a pause between the sentences, as a child takes a breath.
    let (audio, ends) =
        reading(&["Sam has a cat. The cat is fat and tan.", "Sam has a hat, and the cat naps in it."], 0.0);
    let (said, p, quiet) = listen(page, &audio, &ends, "ears");
    eprintln!("read {} of {} words; marks {:?}", p.read(), p.spoken(), p.marks);
    assert!(said.len() >= 2, "a stretch for each sentence at least: {said:?}");
    assert!(p.done(), "the page was followed to the end");
    assert!(p.read() * 10 >= p.spoken() * 9, "nine words in ten heard as read");
    // A pause is 0.7 s; Whisper's work is mostly done by then.
    assert!(said.iter().all(|s| s.1 < 2.0), "heard soon after: {said:?}");
    assert!(quiet, "the quiet after is noticed");
}

/// What Whisper is told to expect mustn't hide a word read wrongly: "red"
/// read for "tan" is noticed, and the right words around it still count.
#[test]
#[ignore]
fn a_misread_word_is_noticed() {
    let _mic = MIC.lock().unwrap_or_else(|e| e.into_inner());
    let page = "The cat is fat and tan. The cat naps on a mat.";
    let (audio, ends) = reading(&["The cat is fat and red.", "The cat naps on a mat."], 0.0);
    let (_, p, _) = listen(page, &audio, &ends, "ears-misread");
    eprintln!("marks {:?}", p.marks);
    let tan = p.words.iter().position(|w| w.key == "tan").unwrap();
    assert_ne!(p.marks[tan], Mark::Read, "\"red\" isn't \"tan\"");
    assert!(p.done(), "the reader goes on past it");
}

/// A fan's hum under the reading (louder than Koe's speech threshold by
/// itself) doesn't run the sentences together, or hide the quiet after.
#[test]
#[ignore]
fn a_humming_room_still_has_pauses() {
    let _mic = MIC.lock().unwrap_or_else(|e| e.into_inner());
    let page = "Sam has a cat. The cat is fat and tan. Sam has a hat, and the cat naps in it.";
    let (audio, ends) =
        reading(&["Sam has a cat. The cat is fat and tan.", "Sam has a hat, and the cat naps in it."], 0.012);
    let (said, p, quiet) = listen(page, &audio, &ends, "ears-hum");
    eprintln!("read {} of {} words", p.read(), p.spoken());
    assert!(said.len() >= 2, "a stretch for each sentence at least: {said:?}");
    assert!(p.done());
    assert!(quiet, "the quiet after is noticed");
}
