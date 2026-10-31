//! Kokoro through sherpa-onnx, for real: needs the model and the library
//! (`SEKKA_EDU_KOKORO`, `SEKKA_EDU_SHERPA_ONNX`), so it's run by hand:
//! `cargo test -p sekka-voice --test kokoro -- --ignored`. It writes what it
//! said under the target's temporary folder, to listen to (or to have
//! Whisper write down, as DEVELOPERS.md does).

use std::path::PathBuf;

use sekka_voice::kokoro::Kokoro;
use sekka_voice::{SPEAKER, SPEED, wav};

#[test]
#[ignore]
fn kokoro_says_a_line() {
    let dir = PathBuf::from(std::env::var_os("SEKKA_EDU_KOKORO").expect("SEKKA_EDU_KOKORO"));
    let lib = PathBuf::from(std::env::var_os("SEKKA_EDU_SHERPA_ONNX").expect("SEKKA_EDU_SHERPA_ONNX"));
    let k = Kokoro::open(&lib, &dir, SPEAKER, SPEED, 2).unwrap();
    let out = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("kokoro");
    std::fs::create_dir_all(&out).unwrap();
    for (i, line) in
        ["Show me seven.", "Each little bead is one.", "Hooray! Three stars! You lit lantern four!"].iter().enumerate()
    {
        let start = std::time::Instant::now();
        let (samples, rate) = k.speak(line).expect("speech");
        let secs = samples.len() as f64 / f64::from(rate);
        let rms = (samples.iter().map(|s| f64::from(*s).powi(2)).sum::<f64>() / samples.len() as f64).sqrt();
        eprintln!("{line:?}: {secs:.2} s of speech in {:.2} s, rms {rms:.3}", start.elapsed().as_secs_f64());
        assert_eq!(rate, 24_000);
        assert!((0.5..6.0).contains(&secs), "{secs} s");
        assert!(rms > 0.01, "audible: {rms}");
        std::fs::write(out.join(format!("{i}.wav")), wav(&samples, rate)).unwrap();
    }
}
