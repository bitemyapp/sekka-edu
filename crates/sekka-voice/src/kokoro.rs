//! Kokoro through sherpa-onnx's C API (`libsherpa-onnx-c-api.so`), loaded
//! when the voice starts rather than linked: a build needs neither, and
//! without them the voice falls back to espeak-ng.
//!
//! The structures are sherpa-onnx 1.13's (`sherpa-onnx/c-api/c-api.h`).
//! sherpa-onnx adds new models' settings at the ends of them, so each is
//! handed over with zeroed room after it, which a later version reads as
//! "not used"; a version other than 1.13 isn't used at all.

use std::ffi::{CStr, CString, c_char, c_void};
use std::path::Path;

type Str = *const c_char;

#[repr(C)]
struct Vits {
    model: Str,
    lexicon: Str,
    tokens: Str,
    data_dir: Str,
    noise_scale: f32,
    noise_scale_w: f32,
    length_scale: f32,
    dict_dir: Str,
}

#[repr(C)]
struct Matcha {
    acoustic_model: Str,
    vocoder: Str,
    lexicon: Str,
    tokens: Str,
    data_dir: Str,
    noise_scale: f32,
    length_scale: f32,
    dict_dir: Str,
}

#[repr(C)]
struct KokoroConfig {
    model: Str,
    voices: Str,
    tokens: Str,
    data_dir: Str,
    length_scale: f32,
    dict_dir: Str,
    lexicon: Str,
    lang: Str,
}

#[repr(C)]
struct Kitten {
    model: Str,
    voices: Str,
    tokens: Str,
    data_dir: Str,
    length_scale: f32,
}

#[repr(C)]
struct Zipvoice {
    tokens: Str,
    encoder: Str,
    decoder: Str,
    vocoder: Str,
    data_dir: Str,
    lexicon: Str,
    feat_scale: f32,
    t_shift: f32,
    target_rms: f32,
    guidance_scale: f32,
}

#[repr(C)]
struct Pocket {
    lm_flow: Str,
    lm_main: Str,
    encoder: Str,
    decoder: Str,
    text_conditioner: Str,
    vocab_json: Str,
    token_scores_json: Str,
    voice_embedding_cache_capacity: i32,
}

#[repr(C)]
struct Supertonic {
    duration_predictor: Str,
    text_encoder: Str,
    vector_estimator: Str,
    vocoder: Str,
    tts_json: Str,
    unicode_indexer: Str,
    voice_style: Str,
}

#[repr(C)]
struct ModelConfig {
    vits: Vits,
    num_threads: i32,
    debug: i32,
    provider: Str,
    matcha: Matcha,
    kokoro: KokoroConfig,
    kitten: Kitten,
    zipvoice: Zipvoice,
    pocket: Pocket,
    supertonic: Supertonic,
}

#[repr(C)]
struct Config {
    model: ModelConfig,
    rule_fsts: Str,
    max_num_sentences: i32,
    rule_fars: Str,
    silence_scale: f32,
}

#[repr(C)]
struct GenerationConfig {
    silence_scale: f32,
    speed: f32,
    sid: i32,
    reference_audio: *const f32,
    reference_audio_len: i32,
    reference_sample_rate: i32,
    reference_text: Str,
    num_steps: i32,
    extra: Str,
}

#[repr(C)]
struct Generated {
    samples: *const f32,
    n: i32,
    sample_rate: i32,
}

/// A structure with zeroed room after it, for fields a later sherpa-onnx
/// adds at its end.
#[repr(C)]
struct Roomy<T> {
    inner: T,
    room: [u64; 64],
}

impl<T> Roomy<T> {
    fn zeroed() -> Box<Roomy<T>> {
        // SAFETY: every field of the configurations is a pointer (null is
        // "not given"), an integer or a float, for all of which zero is a
        // valid value.
        unsafe { Box::new(std::mem::zeroed()) }
    }
}

type Progress = Option<unsafe extern "C" fn(*const f32, i32, f32, *mut c_void) -> i32>;

struct Api {
    create: unsafe extern "C" fn(*const Config) -> *const c_void,
    destroy: unsafe extern "C" fn(*const c_void),
    generate:
        unsafe extern "C" fn(*const c_void, Str, *const GenerationConfig, Progress, *mut c_void) -> *const Generated,
    free: unsafe extern "C" fn(*const Generated),
}

/// `name` in `lib`, as a function of type `F`.
///
/// # Safety
/// `F` must be the function's true type.
unsafe fn symbol<F: Copy>(lib: *mut c_void, name: &CStr) -> Result<F, String> {
    // SAFETY: lib is a handle from dlopen; name is NUL-terminated.
    let p = unsafe { libc::dlsym(lib, name.as_ptr()) };
    if p.is_null() {
        return Err(format!("no {} in sherpa-onnx", name.to_string_lossy()));
    }
    // SAFETY: F is a function pointer type of pointer size (the caller's
    // promise), and p is the function's address.
    Ok(unsafe { std::mem::transmute_copy::<*mut c_void, F>(&p) })
}

/// Where Kokoro's files are: `model.onnx`, `voices.bin`, `tokens.txt`,
/// `lexicon-us-en.txt` and `espeak-ng-data`.
pub fn model_ready(dir: &Path) -> bool {
    ["model.onnx", "voices.bin", "tokens.txt", "lexicon-us-en.txt", "espeak-ng-data"]
        .iter()
        .all(|f| dir.join(f).exists())
}

/// Kokoro, loaded and ready to speak. It belongs to the thread that made
/// it (sherpa-onnx's handle isn't shared).
pub struct Kokoro {
    api: Api,
    tts: *const c_void,
    speaker: i32,
    speed: f32,
}

impl Kokoro {
    /// Load `lib` (sherpa-onnx's C API) and the model in `dir`, to speak as
    /// `speaker` at `speed` (1 is Kokoro's own pace) on `threads` threads.
    pub fn open(lib: &Path, dir: &Path, speaker: i32, speed: f32, threads: i32) -> Result<Kokoro, String> {
        if !model_ready(dir) {
            return Err(format!("no Kokoro model in {}", dir.display()));
        }
        let c = |p: &Path| CString::new(p.as_os_str().as_encoded_bytes()).map_err(|_| "a path with a NUL".to_owned());
        let lib_name = c(lib)?;
        // SAFETY: lib_name is NUL-terminated. The library stays loaded for
        // the life of the process (onnxruntime's threads may outlive it).
        let handle = unsafe { libc::dlopen(lib_name.as_ptr(), libc::RTLD_NOW | libc::RTLD_LOCAL) };
        if handle.is_null() {
            // SAFETY: dlerror returns null or a NUL-terminated message.
            let why = unsafe { libc::dlerror() };
            let why = if why.is_null() {
                String::new()
            } else {
                unsafe { CStr::from_ptr(why) }.to_string_lossy().into_owned()
            };
            return Err(format!("can't load {}: {why}", lib.display()));
        }
        // SAFETY: the types are those of c-api.h for each name.
        let version: unsafe extern "C" fn() -> Str = unsafe { symbol(handle, c"SherpaOnnxGetVersionStr")? };
        let api = unsafe {
            Api {
                create: symbol(handle, c"SherpaOnnxCreateOfflineTts")?,
                destroy: symbol(handle, c"SherpaOnnxDestroyOfflineTts")?,
                generate: symbol(handle, c"SherpaOnnxOfflineTtsGenerateWithConfig")?,
                free: symbol(handle, c"SherpaOnnxDestroyOfflineTtsGeneratedAudio")?,
            }
        };
        // SAFETY: the version is a static NUL-terminated string.
        let v = unsafe { CStr::from_ptr(version()) }.to_string_lossy().into_owned();
        if !v.starts_with("1.13.") {
            return Err(format!("sherpa-onnx {v}, and the voice knows 1.13's interface"));
        }
        let files = ["model.onnx", "voices.bin", "tokens.txt", "espeak-ng-data", "lexicon-us-en.txt"]
            .map(|f| c(&dir.join(f)))
            .into_iter()
            .collect::<Result<Vec<_>, _>>()?;
        let provider = c"cpu";
        let lang = c"en-us";
        let mut config = Roomy::<Config>::zeroed();
        let m = &mut config.inner.model;
        m.num_threads = threads;
        m.provider = provider.as_ptr();
        m.kokoro.model = files[0].as_ptr();
        m.kokoro.voices = files[1].as_ptr();
        m.kokoro.tokens = files[2].as_ptr();
        m.kokoro.data_dir = files[3].as_ptr();
        m.kokoro.lexicon = files[4].as_ptr();
        m.kokoro.lang = lang.as_ptr();
        m.kokoro.length_scale = 1.0;
        config.inner.max_num_sentences = 1;
        config.inner.silence_scale = 0.2;
        // SAFETY: config and the strings it points to live until create
        // returns; sherpa-onnx copies what it keeps.
        let tts = unsafe { (api.create)(&config.inner) };
        if tts.is_null() {
            return Err(format!("sherpa-onnx couldn't load Kokoro from {}", dir.display()));
        }
        Ok(Kokoro { api, tts, speaker, speed })
    }

    /// `text` spoken: mono samples from -1 to 1, and their rate.
    pub fn speak(&self, text: &str) -> Option<(Vec<f32>, u32)> {
        let text = CString::new(text.replace('\0', " ")).ok()?;
        let mut g = Roomy::<GenerationConfig>::zeroed();
        g.inner.silence_scale = 0.2;
        g.inner.speed = self.speed;
        g.inner.sid = self.speaker;
        // SAFETY: tts is live (dropped only with self), text and g outlive
        // the call, and no progress callback is given.
        let audio = unsafe { (self.api.generate)(self.tts, text.as_ptr(), &g.inner, None, std::ptr::null_mut()) };
        if audio.is_null() {
            return None;
        }
        // SAFETY: audio is sherpa-onnx's, holding n samples at `samples`,
        // until it's freed below.
        let out = unsafe {
            let a = &*audio;
            let samples = if a.samples.is_null() || a.n <= 0 {
                Vec::new()
            } else {
                std::slice::from_raw_parts(a.samples, a.n as usize).to_vec()
            };
            let rate = a.sample_rate.max(1) as u32;
            (self.api.free)(audio);
            (samples, rate)
        };
        (!out.0.is_empty()).then_some(out)
    }
}

impl Drop for Kokoro {
    fn drop(&mut self) {
        // SAFETY: tts came from create and is destroyed once.
        unsafe { (self.api.destroy)(self.tts) };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The sizes c-api.h's structures have on x86-64 and aarch64 Linux,
    /// worked out from their fields: a mismatch means a field is wrong.
    #[test]
    fn structures_are_the_headers() {
        assert_eq!(std::mem::size_of::<Vits>(), 56);
        assert_eq!(std::mem::size_of::<KokoroConfig>(), 64);
        assert_eq!(std::mem::size_of::<Kitten>(), 40);
        assert_eq!(std::mem::size_of::<Zipvoice>(), 64);
        assert_eq!(std::mem::size_of::<Pocket>(), 64);
        assert_eq!(std::mem::size_of::<Supertonic>(), 56);
        assert_eq!(std::mem::offset_of!(ModelConfig, matcha), 72);
        assert_eq!(std::mem::offset_of!(ModelConfig, kokoro), 72 + 56);
        assert_eq!(std::mem::offset_of!(GenerationConfig, reference_audio), 16);
        assert_eq!(std::mem::size_of::<Generated>(), 16);
    }
}
