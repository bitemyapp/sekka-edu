//! How fast, and how well, Whisper hears reading, asked different ways:
//! the time each stretch takes to recognise, and how many of its words the
//! page then counts as read. Kokoro stands in for readers: three voices, at
//! a grown-up's pace and a slower one, reading sentences from each shelf
//! one at a time (a stretch apiece, as the ears cut them), a word on its
//! own (a correction), a word misread, and words read one by one.
//!
//! Needs `SEKKA_EDU_KOKORO`, `SEKKA_EDU_SHERPA_ONNX` and `SEKKA_EDU_WHISPER`;
//! `HEARING_MODELS` may name more models (colon-separated) to compare
//! with it. Run by hand, on the computer that matters:
//! `cargo test --release -p sekka-ondoku --test hearing -- --ignored --nocapture`.

use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;
use std::time::Instant;

use sekka_ondoku::follow::{Mark, Page};
use sekka_ondoku::library;
use sekka_ondoku::listen::prompt;
use sekka_ondoku::whisper::{How, Whisper, steps};
use sekka_voice::kokoro::Kokoro;

/// `samples` at `from` Hz, at 16 kHz.
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

/// A stretch as the ears would have it: a little quiet before (their
/// pre-roll), the reading, and the pause that ended it.
fn stretch(voice: &Kokoro, said: &str) -> Vec<f32> {
    let (s, rate) = voice.speak(said).unwrap();
    let mut out = vec![0.0; 4_800];
    out.extend(to_16k(&s, rate));
    out.extend(std::iter::repeat_n(0.0, 11_200));
    out
}

struct Clip {
    /// The page the reader is on (the prompt), what they read of it, and
    /// what they said.
    page: String,
    text: String,
    samples: Vec<f32>,
    /// A word that was misread, which mustn't be counted as read.
    wrong: Option<&'static str>,
}

fn sentences(page: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    for w in page.split_whitespace() {
        if !cur.is_empty() {
            cur.push(' ');
        }
        cur.push_str(w);
        if w.ends_with(['.', '!', '?']) || w.ends_with(".\"") || w.ends_with("!\"") || w.ends_with("?\"") {
            out.push(std::mem::take(&mut cur));
        }
    }
    if !cur.is_empty() {
        out.push(cur);
    }
    out
}

fn clips(voices: &[(i32, f32)], kokoro: &Path, sherpa: &Path) -> Vec<Clip> {
    let books = library::all();
    let page = |id: &str| books.iter().find(|b| b.id == id).unwrap().pages[0].clone();
    let mut out = Vec::new();
    for &(sid, speed) in voices {
        // One voice loaded at a time: each is a third of a gigabyte.
        let voice = &Kokoro::open(sherpa, kokoro, sid, speed, 4).unwrap();
        // Sentences from each shelf, short to long.
        for id in ["sam-and-the-cat", "the-ship-and-the-fish", "the-star-garden", "the-lighthouse-cat", "peter-rabbit"]
        {
            let p = page(id);
            for s in sentences(&p).into_iter().take(2) {
                out.push(Clip { page: p.clone(), samples: stretch(voice, &s), text: s, wrong: None });
            }
        }
        // Words on their own, as a kid tries one again.
        for (page, word) in [
            ("The pig can dig.", "dig"),
            ("Ten red hens sat in a pen.", "hens"),
            ("Chip and Beth sit on a ship.", "ship"),
            ("The frog sat on a log.", "frog"),
            ("Kate can ride a bike.", "bike"),
            ("The rain made a pond.", "rain"),
        ] {
            out.push(Clip {
                page: page.into(),
                text: word.into(),
                samples: stretch(voice, &format!("{word}.")),
                wrong: None,
            });
        }
        let pig = "The pig can dig.";
        out.push(Clip {
            page: pig.into(),
            text: "The pig can dig.".into(),
            samples: stretch(voice, "The pig can dog."),
            wrong: Some("dig"),
        });
        let hens = "Ten red hens sat in a pen.";
        out.push(Clip {
            page: hens.into(),
            text: hens.into(),
            samples: stretch(voice, "Ten. Red. Hens. Sat. In. A. Pen."),
            wrong: None,
        });
    }
    out
}

#[derive(Default)]
struct Score {
    ms: Vec<f64>,
    short_ms: Vec<f64>,
    words: u32,
    read: u32,
    wrong_caught: u32,
    wrong: u32,
    /// Words on their own, heard as themselves.
    alone: u32,
    alones: u32,
    /// Heard as more words than anyone reads in the time: Whisper looping.
    loops: u32,
}

fn score(w: &Whisper, clips: &[Clip], how: &How, said: &mut Vec<String>) -> Score {
    let mut s = Score::default();
    let cancel = AtomicBool::new(false);
    for c in clips {
        let hint = w.fit_prompt(&prompt(&c.page));
        let t = Instant::now();
        let heard = w.hear(&c.samples, &hint, &cancel, how).unwrap_or_default();
        let ms = t.elapsed().as_secs_f64() * 1000.0;
        s.ms.push(ms);
        // Each stretch has 0.3 s of quiet before and 0.7 s after.
        let reading = (c.samples.len() as f64 / 16_000.0 - 1.0).max(1.0);
        if heard.split_whitespace().count() as f64 > 4.0 * reading {
            s.loops += 1;
        }
        if c.samples.len() < 16_000 * 2 {
            s.short_ms.push(ms);
        }
        let mut p = Page::new(&c.text);
        p.hear(&heard, 0);
        match c.wrong {
            Some(word) => {
                s.wrong += 1;
                let i = p.words.iter().position(|x| x.key == word).unwrap();
                if p.marks[i] != Mark::Read {
                    s.wrong_caught += 1;
                }
                s.words += p.spoken() - 1;
                s.read += p.read().min(p.spoken() - 1);
            }
            None => {
                s.words += p.spoken();
                s.read += p.read();
                if p.spoken() == 1 {
                    s.alones += 1;
                    s.alone += p.read();
                }
            }
        }
        said.push(heard);
    }
    s
}

fn summary(name: &str, s: &Score) {
    let mut ms = s.ms.clone();
    ms.sort_by(f64::total_cmp);
    let mean = |v: &[f64]| v.iter().sum::<f64>() / v.len().max(1) as f64;
    eprintln!(
        "{name:<40} {:>6.0} {:>6.0} {:>6.0} {:>6.0}   {:>5.1}%   {:>2}/{}   {:>2}/{}   {:>3}",
        mean(&s.ms),
        mean(&s.short_ms),
        ms[ms.len() * 9 / 10],
        ms[ms.len() - 1],
        f64::from(s.read) * 100.0 / f64::from(s.words),
        s.alone,
        s.alones,
        s.wrong_caught,
        s.wrong,
        s.loops,
    );
}

#[test]
#[ignore]
fn how_fast_and_how_well_fumi_hears() {
    let env = |k: &str| PathBuf::from(std::env::var_os(k).unwrap_or_else(|| panic!("{k}")));
    let (kokoro, sherpa, model) = (env("SEKKA_EDU_KOKORO"), env("SEKKA_EDU_SHERPA_ONNX"), env("SEKKA_EDU_WHISPER"));
    let t = Instant::now();
    // af_heart, af_sky and am_puck; the second time through, slower.
    let voices = [(3, 1.0), (10, 1.0), (18, 1.0), (3, 0.75), (10, 0.75), (18, 0.75)];
    let clips = clips(&voices, &kokoro, &sherpa);
    let secs: f64 = clips.iter().map(|c| c.samples.len() as f64 / 16_000.0).sum();
    eprintln!("{} stretches, {secs:.0} s of reading, made in {:.0} s", clips.len(), t.elapsed().as_secs_f64());
    let longest = clips.iter().map(|c| c.samples.len()).max().unwrap();
    eprintln!("encoder steps for the longest: {} of 1500", steps(longest, &How::default()));

    // Koe's way, and the way Ondoku hears (How::default()).
    let koe = How { threads: 4, margin: None, whole_below: 0, fallback: true, tokens_a_second: None, unloop: false };
    let base = How::default();
    // Measured on an 8-core laptop (DEVELOPERS.md): Koe's way took four
    // times as long, fallback most of that; without fallback Whisper loops
    // on short stretches; in a short window a word on its own can come out
    // as another, and 10 to 20 s at the least didn't help; 8 threads on 8
    // cores spun so long the run was stopped.
    let ways = [
        ("whole window, as Koe", koe),
        ("whole window, no fallback", How { fallback: false, ..koe }),
        ("fitted", How { whole_below: 0, ..base }),
        ("Ondoku's way", base),
    ];
    let mut models = vec![model];
    if let Some(more) = std::env::var_os("HEARING_MODELS") {
        models.extend(std::env::split_paths(&more));
    }
    eprintln!(
        "\n{:<40} {:>6} {:>6} {:>6} {:>6}   {:>6}   {:>5}   {:>5}   loops",
        "", "mean", "short", "p90", "max", "read", "alone", "wrong"
    );
    let mut alone: Vec<(String, Vec<String>)> = Vec::new();
    for m in &models {
        let w = Whisper::load(m).unwrap();
        // Warm: the first time through sets up Whisper's buffers.
        let _ = w.hear(&clips[0].samples, "", &AtomicBool::new(false), &base);
        let file =
            m.file_name().unwrap().to_string_lossy().trim_start_matches("ggml-").trim_end_matches(".bin").to_owned();
        for (name, how) in &ways {
            // Koe's way is the slowest by far: once is enough.
            if how.fallback && m != &models[0] {
                continue;
            }
            let mut said = Vec::new();
            let name = format!("{file}, {name}");
            summary(&name, &score(&w, &clips, how, &mut said));
            let words: Vec<String> = clips
                .iter()
                .zip(said)
                .filter(|(c, _)| c.text.split_whitespace().count() == 1)
                .map(|(_, s)| s)
                .collect();
            alone.push((name, words));
        }
    }
    // Each word on its own, as each way heard it.
    eprintln!();
    let wanted: Vec<&str> =
        clips.iter().filter(|c| c.text.split_whitespace().count() == 1).map(|c| c.text.as_str()).collect();
    eprintln!("{:<40} {}", "", wanted.iter().map(|w| format!("{w:<8}")).collect::<String>());
    for (name, words) in &alone {
        let cells: String = words.iter().map(|w| format!("{:<8}", w.chars().take(7).collect::<String>())).collect();
        eprintln!("{name:<40} {cells}");
    }
}
