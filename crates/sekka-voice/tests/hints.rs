//! The hints, heard as Kokoro hears them: the sounds espeak-ng gives for a
//! line, read with Kokoro's own espeak-ng data, are the sounds sherpa-onnx
//! hands Kokoro. Needs espeak-ng on `PATH` and the model's folder
//! (`SEKKA_EDU_KOKORO`), so it's run by hand:
//! `cargo test -p sekka-voice --test hints -- --ignored`.

use std::path::PathBuf;
use std::process::Command;

use sekka_voice::hints::{WORDS, said};

/// The sounds `line` is said with, as Kokoro gets them.
fn sounds(line: &str) -> String {
    let data = PathBuf::from(std::env::var_os("SEKKA_EDU_KOKORO").expect("SEKKA_EDU_KOKORO"));
    let espeak = sekka_voice::which("espeak-ng").expect("espeak-ng on PATH");
    let out = Command::new(espeak)
        .args(["-q", "--ipa", "-v", "en-us", "--path"])
        .arg(&data)
        .arg(said(line))
        .output()
        .expect("espeak-ng");
    String::from_utf8_lossy(&out.stdout).split_whitespace().collect::<Vec<_>>().join(" ")
}

#[test]
#[ignore]
fn hinted_words_sound_right() {
    let want = [
        ("You [read](red) that so well!", "ɹˈɛd"),
        ("Hoo! I'm Fumi.", "fˈuːmiː"),
        ("Hooray! Three stars!", "hɚɹˈeɪ"),
        ("Pon the tanuki", "tˈɑːnuːkˈiː"),
        ("The soroban", "sˈoʊɹoʊbˌɑːn"),
        ("Anzan", "ˈɑːnzɑːn"),
    ];
    assert_eq!(want.len(), WORDS.len() + 1, "every word in WORDS is heard here");
    for (line, sound) in want {
        let s = sounds(line);
        assert!(s.contains(sound), "{line:?} is said {s:?}, not with {sound:?}");
    }
}
