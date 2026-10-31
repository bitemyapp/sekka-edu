//! Pronunciation hints: how the voice should say a word it would say
//! wrong. Kokoro is given its sounds by espeak-ng's rules (sherpa-onnx asks
//! espeak-ng for each line), and espeak-ng, where Kokoro isn't, reads by
//! the same rules, so a hint is a spelling those rules read right.
//!
//! A line carries its own hints, for a word read two ways: `[read](red)`
//! shows "read" and says "red", as in "You [read](red) that so well!".
//! Words said wrong wherever they're written are in [`WORDS`], and need no
//! hint.

/// Words the voice says wrong wherever they're written, matched whole and
/// in any case, and spellings it says right.
pub const WORDS: &[(&str, &str)] = &[
    // Fumi, Ondoku's owl: FOO-mee, not FYOO-mee.
    ("fumi", "Foomee"),
    // Hoo-RAY, not HOR-ay.
    ("hooray", "hurray"),
    // Pon is a tanuki: TAH-noo-KEE, not TAN-oo-kee.
    ("tanuki", "tanu-key"),
    // The abacus and its sums: SO-ro-bahn, AHN-zahn.
    ("soroban", "sohrohbahn"),
    ("anzan", "Ahnzahn"),
];

/// A piece of a line: text, or a word with its hint.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Piece<'a> {
    /// What's shown.
    pub text: &'a str,
    /// How it's said, when that's its hint.
    pub said: Option<&'a str>,
}

/// `line` in pieces: the text between hints, and each hint.
pub fn pieces(line: &str) -> Vec<Piece<'_>> {
    let mut out = Vec::new();
    let mut rest = line;
    while let Some(mid) = rest.find("](") {
        let (Some(open), Some(close)) = (rest[..mid].rfind('['), rest[mid + 2..].find(')')) else { break };
        out.push(Piece { text: &rest[..open], said: None });
        out.push(Piece { text: &rest[open + 1..mid], said: Some(&rest[mid + 2..mid + 2 + close]) });
        rest = &rest[mid + 3 + close..];
    }
    out.push(Piece { text: rest, said: None });
    out.retain(|p| !p.text.is_empty() || p.said.is_some());
    out
}

/// `line` as it's shown: each hint's word, without how it's said.
pub fn shown(line: &str) -> String {
    pieces(line).iter().map(|p| p.text).collect()
}

/// `line` as the voice is given it: each hint's spelling in place of its
/// word, and the words in [`WORDS`] respelt.
pub fn said(line: &str) -> String {
    let mut out = String::new();
    for p in pieces(line) {
        match p.said {
            Some(s) => out.push_str(s),
            None => respell(p.text, &mut out),
        }
    }
    out
}

/// `word`, with a hint to say it as `said` (none if that's empty): a kid's
/// name as a grown-up spelt how it sounds.
pub fn hint(word: &str, said: &str) -> String {
    let said = said.trim();
    if said.is_empty() || said == word { word.to_owned() } else { format!("[{word}]({said})") }
}

fn respell(text: &str, out: &mut String) {
    let mut word = None;
    for (i, c) in text.char_indices().chain([(text.len(), ' ')]) {
        if c.is_alphabetic() {
            word.get_or_insert(i);
            continue;
        }
        if let Some(start) = word.take() {
            let w = &text[start..i];
            out.push_str(WORDS.iter().find(|(k, _)| k.eq_ignore_ascii_case(w)).map_or(w, |(_, say)| say));
        }
        if i < text.len() {
            out.push(c);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_hint_is_shown_as_its_word_and_said_as_its_spelling() {
        let line = "You [read](red) that so well!";
        assert_eq!(shown(line), "You read that so well!");
        assert_eq!(said(line), "You red that so well!");
        assert_eq!(said("“[live](liv),"), "“liv,");
        assert_eq!(shown("“[live](liv),"), "“live,");
        assert_eq!(
            pieces("a [b](c) d"),
            [Piece { text: "a ", said: None }, Piece { text: "b", said: Some("c") }, Piece { text: " d", said: None },]
        );
    }

    #[test]
    fn brackets_that_arent_hints_are_kept() {
        for line in ["[Illustration]", "(a) and [b]", "a](b", "[a](b", "x ]( y"] {
            assert_eq!(shown(line), line);
            assert_eq!(said(line), line);
        }
        assert_eq!(said("[x [read](red)"), "[x red");
    }

    #[test]
    fn some_words_are_respelt_wherever_they_are() {
        assert_eq!(said("Hoo! I'm Fumi. Fumi's riddles!"), "Hoo! I'm Foomee. Foomee's riddles!");
        assert_eq!(said("Hooray! Pon the tanuki."), "hurray! Pon the tanu-key.");
        assert_eq!(said("Fumiko and fumigate"), "Fumiko and fumigate", "whole words only");
        assert_eq!(said("[Fumi](Fumi)"), "Fumi", "a hint wins");
    }

    #[test]
    fn a_name_spelt_how_it_sounds() {
        assert_eq!(hint("Mei", "May"), "[Mei](May)");
        assert_eq!(hint("Mei", " "), "Mei");
        assert_eq!(hint("Mei", "Mei"), "Mei");
        assert_eq!(said(&format!("Hi, {}!", hint("Siobhan", "Shiv-awn"))), "Hi, Shiv-awn!");
        assert_eq!(shown(&format!("Hi, {}!", hint("Siobhan", "Shiv-awn"))), "Hi, Siobhan!");
    }
}
