//! Each hint in the books changes how Fumi says its words: one that
//! doesn't is spelt wrong, or isn't needed. Heard as Kokoro hears them, as
//! in sekka-voice's hints test: needs espeak-ng on `PATH` and the model's
//! folder (`SEKKA_EDU_KOKORO`), so it's run by hand:
//! `cargo test -p sekka-ondoku --test hints -- --ignored`.

use std::path::PathBuf;
use std::process::Command;

use sekka_ondoku::library;
use sekka_voice::hints::{pieces, said, shown};

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
fn every_hint_in_the_books_changes_what_is_said() {
    let mut hints = 0;
    for b in library::all() {
        for page in &b.voiced {
            let words: Vec<&str> = page.split_whitespace().collect();
            for (i, w) in words.iter().enumerate() {
                if !pieces(w).iter().any(|p| p.said.is_some()) {
                    continue;
                }
                hints += 1;
                // The words around it, with the hint and without.
                let around = &words[i.saturating_sub(6)..(i + 7).min(words.len())];
                let with = around.join(" ");
                let without =
                    around.iter().map(|x| if x == w { shown(x) } else { (*x).to_owned() }).collect::<Vec<_>>();
                let (hinted, plain) = (sounds(&with), sounds(&without.join(" ")));
                assert_ne!(hinted, plain, "{}: {w} changes nothing in {with:?}", b.id);
            }
        }
    }
    assert!(hints >= 20, "the books' hints are all heard: {hints}");
}
