//! Whisper for Fumi's ears: the whisper.cpp Koe builds, through whisper-rs,
//! with the model kept loaded. Koe's own recognizer hears every utterance
//! as Whisper's whole window, half a minute of sound with the rest silent,
//! and tries again warmer when unsure. This one hears a sentence in a
//! window fitted to it, and a word or two (a kid trying a word again) in
//! the whole window, where Whisper hears a word alone best; it doesn't try
//! again, but cuts a transcript that loops. The hearing test
//! (`tests/hearing.rs`) measures each choice (DEVELOPERS.md).

use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, Once};

use koe::audio::RATE;
use whisper_rs::{FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters, WhisperState};

/// Whisper's window: 30 s, in its encoder's steps of 20 ms.
const WINDOW: usize = 1500;
/// Sound in one step of the encoder.
const STEP: usize = RATE / 50;

/// How Whisper is asked to hear (the hearing test compares them).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct How {
    pub threads: i32,
    /// Steps of the encoder past the end of the sound, to hear it whole;
    /// `None` for the whole window, as Koe does.
    pub margin: Option<usize>,
    /// Stretches shorter than this (samples) are heard in the whole window
    /// all the same: alone, a word in a short one can come out as another
    /// ("hens" as "pens", "rain" as "brain"), and a longer short one didn't
    /// help.
    pub whole_below: usize,
    /// Decode again, warmer, when the first try comes out unsure.
    pub fallback: bool,
    /// At most this many tokens a second of sound (and a few over), so a
    /// try that goes round in circles stops.
    pub tokens_a_second: Option<usize>,
    /// Keep the first time round of a loop ([`unloop`]).
    pub unloop: bool,
}

impl Default for How {
    fn default() -> How {
        // Four, or one fewer than the computer has, leaving one for the
        // app and the microphone: more spin waiting on the slowest (on a
        // laptop with four fast cores and four small ones, six took longer
        // than four, and eight far longer).
        let threads = std::thread::available_parallelism().map_or(4, |n| (n.get() as i32 - 1).clamp(1, 4));
        How {
            threads,
            margin: Some(MARGIN),
            whole_below: WHOLE_BELOW,
            fallback: false,
            tokens_a_second: Some(8),
            unloop: true,
        }
    }
}

/// See the hearing test (`tests/hearing.rs`, DEVELOPERS.md).
const MARGIN: usize = 128;
const WHOLE_BELOW: usize = RATE * 3;

/// The encoder steps for `samples` of sound, heard `how`.
pub fn steps(samples: usize, how: &How) -> usize {
    match how.margin {
        Some(m) if samples >= how.whole_below => (samples.div_ceil(STEP) + m).min(WINDOW),
        _ => WINDOW,
    }
}

pub struct Whisper {
    resident: Mutex<(WhisperContext, WhisperState)>,
}

unsafe extern "C" fn should_abort(data: *mut std::ffi::c_void) -> bool {
    // SAFETY: hear() passes a borrowed AtomicBool, alive until full()
    // returns; whisper.cpp calls this only during that call.
    unsafe { &*data.cast::<AtomicBool>() }.load(Ordering::Relaxed)
}

impl Whisper {
    /// Load `model` (on the processor: Ondoku's build has no GPU support).
    pub fn load(model: &Path) -> Result<Whisper, String> {
        // Whisper's own logs can hold what it heard: keep them out of the
        // journal, as Koe does.
        static QUIET: Once = Once::new();
        QUIET.call_once(whisper_rs::install_logging_hooks);
        let mut params = WhisperContextParameters::default();
        params.use_gpu(false);
        let model = WhisperContext::new_with_params(model, params).map_err(|e| format!("can't load Whisper: {e}"))?;
        let state = model.create_state().map_err(|e| format!("can't start Whisper: {e}"))?;
        Ok(Whisper { resident: Mutex::new((model, state)) })
    }

    /// The longest start of `prompt` (whole words) Whisper takes as a hint:
    /// half its text context, as Koe allows.
    pub fn fit_prompt(&self, prompt: &str) -> String {
        let Ok(r) = self.resident.lock() else { return String::new() };
        let limit = r.0.n_text_ctx() as usize / 2 - 8;
        let fits = |p: &str| r.0.tokenize(p, p.len() + 1).is_ok_and(|t| t.len() <= limit);
        let mut p = prompt.to_owned();
        while !fits(&p) {
            match p.rfind(", ") {
                Some(i) => p.truncate(i),
                None => return String::new(),
            }
        }
        p
    }

    /// What Whisper hears in `samples` (16 kHz), told to expect `prompt`;
    /// `None` if `cancel` was set on the way, or it failed.
    pub fn hear(&self, samples: &[f32], prompt: &str, cancel: &AtomicBool, how: &How) -> Option<String> {
        if cancel.load(Ordering::Relaxed) {
            return None;
        }
        // whisper.cpp wants a second at least; silence after is no harm.
        let mut padded;
        let mut samples = samples;
        if samples.len() < RATE {
            padded = samples.to_vec();
            padded.resize(RATE, 0.0);
            samples = &padded;
        }
        let mut r = self.resident.lock().ok()?;
        let mut params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });
        params.set_n_threads(how.threads);
        params.set_language(Some("en"));
        params.set_no_context(true);
        params.set_no_timestamps(true);
        params.set_single_segment(true);
        params.set_print_special(false);
        params.set_print_progress(false);
        params.set_print_realtime(false);
        params.set_print_timestamps(false);
        params.set_initial_prompt(prompt);
        let steps = steps(samples.len(), how);
        if steps < WINDOW {
            params.set_audio_ctx(steps as i32);
        }
        if !how.fallback {
            params.set_temperature_inc(0.0);
        }
        if let Some(n) = how.tokens_a_second {
            params.set_max_tokens((samples.len() * n / RATE + 16) as i32);
        }
        // SAFETY: `cancel` is borrowed for this whole call, which is the
        // only time whisper.cpp calls back; the callback only reads it.
        unsafe {
            params.set_abort_callback(Some(should_abort));
            params.set_abort_callback_user_data(std::ptr::from_ref(cancel).cast_mut().cast());
        }
        if let Err(e) = r.1.full(params, samples) {
            log::debug!("whisper: {e}");
            return None;
        }
        if cancel.load(Ordering::Relaxed) {
            return None;
        }
        let text = r.1.as_iter().map(|s| s.to_string()).collect::<String>();
        let seconds = samples.len() as f32 / RATE as f32;
        Some(if how.unloop { unloop(text.trim(), seconds) } else { text.trim().to_owned() })
    }
}

/// More words a second than anyone reads aloud; and the quiet in every
/// stretch, before and after the reading (the ears' pre-roll and pause).
const TOO_MANY_A_SECOND: f32 = 4.0;
const QUIET_IN_A_STRETCH: f32 = 0.6;

/// Whisper going round in circles, as it can on a short stretch without a
/// warmer second try: more words than anyone reads in the time, and from
/// some word on the same few over and over, the last perhaps cut short
/// where Whisper was stopped. Keeps the first time round.
pub fn unloop(text: &str, seconds: f32) -> String {
    let words: Vec<&str> = text.split_whitespace().collect();
    if words.len() as f32 <= TOO_MANY_A_SECOND * (seconds - QUIET_IN_A_STRETCH).max(1.0) {
        return text.to_owned();
    }
    let keys: Vec<String> = words.iter().map(|w| crate::words::key(w)).collect();
    let n = keys.len();
    let again = |i: usize, j: usize| keys[i] == keys[j] || (j == n - 1 && keys[i].starts_with(keys[j].as_str()));
    for start in 0..n {
        for p in 1..=(n - start) / 2 {
            if (start..n - p).all(|i| again(i, i + p)) {
                return words[..start + p].join(" ");
            }
        }
    }
    text.to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_sentence_needs_far_less_than_the_window_and_a_word_has_it_all() {
        let how = How::default();
        assert_eq!(steps(RATE * 4, &how), 200 + MARGIN);
        assert_eq!(steps(RATE * 12, &how), 600 + MARGIN);
        assert_eq!(steps(RATE * 40, &how), WINDOW);
        assert_eq!(steps(RATE * 4, &How { margin: None, ..how }), WINDOW);
        assert_eq!(steps(RATE * 2, &how), WINDOW);
        assert_eq!(steps(RATE * 2, &How { whole_below: 0, ..how }), 100 + MARGIN);
    }

    #[test]
    fn a_loop_is_cut_to_its_first_time_round() {
        let dig = vec!["dig"; 29].join(" ");
        assert_eq!(unloop(&dig, 1.4), "dig");
        let sam = "Sam, has, a, cat Sam, has, cat Sam, has, cat Sam, has, cat Sam, has, cat Sam, has, cat Sam";
        assert_eq!(unloop(sam, 2.5), "Sam, has, a, cat Sam, has,");
        let chip = "Chip and Beth sit on a ship Chip and Beth sit on a ship Chip and Beth sit on a ship Chip and";
        assert_eq!(unloop(chip, 2.4), "Chip and Beth sit on a ship");
        let mira = "Mira loved, the stars. Mira loved, the stars. Mira loved, the stars. Mir";
        assert_eq!(unloop(mira, 3.0), "Mira loved, the stars.");
        assert_eq!(unloop("Sam, has, a, cat Sam, has, a, cat Sam, has,", 2.6), "Sam, has, a, cat");
        // Words read again, at a pace someone could read them, stay.
        assert_eq!(unloop("Hop, hop, hop, hop!", 1.5), "Hop, hop, hop, hop!");
        assert_eq!(unloop("dig dig dig", 0.5), "dig dig dig");
    }
}
