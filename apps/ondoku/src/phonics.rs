//! A word in the pieces a beginning reader sounds out: letters, and the
//! letter teams that make one sound (sh, ck, ee, ai, igh, ar), with a
//! silent e at the end shown as silent (k·i·t·e). Fumi shows them over a
//! word a kid taps in the first books, as a teacher's finger would.

/// One piece of a word.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Chunk {
    pub text: String,
    /// The e at the end of "kite": there, but not sounded.
    pub silent: bool,
}

/// Teams of letters read as one sound, longest first.
const TEAMS: &[&str] = &[
    "tch", "igh", "dge", "sh", "ch", "th", "wh", "ph", "ck", "ng", "qu", "ee", "ea", "ai", "ay", "oa", "ow", "ou",
    "oo", "oi", "oy", "au", "aw", "ew", "ue", "ar", "er", "ir", "or", "ur", "ll", "ss", "ff", "zz",
];

fn vowel(c: char) -> bool {
    matches!(c, 'a' | 'e' | 'i' | 'o' | 'u')
}

/// `word`'s letters in chunks (its punctuation left out), its case kept.
pub fn chunks(word: &str) -> Vec<Chunk> {
    let letters: Vec<char> = word.chars().filter(|c| c.is_alphabetic()).collect();
    let lower: Vec<char> = letters.iter().map(|c| c.to_ascii_lowercase()).collect();
    let n = lower.len();
    // Vowel, consonant, e at the end: the e is silent and makes the vowel
    // say its name.
    let magic = n >= 3 && lower[n - 1] == 'e' && !vowel(lower[n - 2]) && vowel(lower[n - 3]);
    let end = if magic { n - 1 } else { n };
    let mut out = Vec::new();
    let mut i = 0;
    while i < end {
        let team = TEAMS.iter().find(|t| {
            let t: Vec<char> = t.chars().collect();
            i + t.len() <= end && lower[i..i + t.len()] == t[..]
        });
        let len = team.map_or(1, |t| t.len());
        out.push(Chunk { text: letters[i..i + len].iter().collect(), silent: false });
        i += len;
    }
    if magic {
        out.push(Chunk { text: letters[n - 1].to_string(), silent: true });
    }
    out
}

/// The piece of a word read wrong as `heard`, when it's the only one that
/// differs and the word was read with as many letters: "bud" read as
/// "bad" is the u, "ship" read as "chip" the sh.
pub fn wrong_piece(pieces: &[Chunk], heard: &str) -> Option<usize> {
    let mut printed = Vec::new();
    for (k, p) in pieces.iter().enumerate() {
        printed.extend(p.text.chars().map(|c| (k, c.to_ascii_lowercase())));
    }
    let heard: Vec<char> = heard.chars().filter(|c| c.is_alphabetic()).map(|c| c.to_ascii_lowercase()).collect();
    // Letter for letter, each that differs is in a piece; a team read as
    // another its length ("ch" for "sh") is still one piece.
    if printed.len() != heard.len() {
        return None;
    }
    let mut wrong = printed.iter().zip(&heard).filter(|((_, p), h)| p != *h).map(|((k, _), _)| *k);
    let first = wrong.next()?;
    wrong.all(|k| k == first).then_some(first)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn show(word: &str) -> String {
        chunks(word)
            .iter()
            .map(|c| if c.silent { format!("({})", c.text) } else { c.text.clone() })
            .collect::<Vec<_>>()
            .join("·")
    }

    #[test]
    fn words_in_the_pieces_they_are_sounded_in() {
        assert_eq!(show("cat"), "c·a·t");
        assert_eq!(show("Ship,"), "Sh·i·p");
        assert_eq!(show("kite"), "k·i·t·(e)");
        assert_eq!(show("rain"), "r·ai·n");
        assert_eq!(show("night"), "n·igh·t");
        assert_eq!(show("duck"), "d·u·ck");
        assert_eq!(show("the"), "th·e");
        assert_eq!(show("star"), "s·t·ar");
        assert_eq!(show("green"), "g·r·ee·n");
    }

    #[test]
    fn the_sound_read_wrong() {
        assert_eq!(wrong_piece(&chunks("Bud"), "bad"), Some(1));
        assert_eq!(wrong_piece(&chunks("ship"), "chip"), Some(0));
        assert_eq!(wrong_piece(&chunks("hens"), "pens"), Some(0));
        assert_eq!(wrong_piece(&chunks("kite"), "kit"), None, "a letter fewer");
        assert_eq!(wrong_piece(&chunks("dig"), "dog"), Some(1));
        assert_eq!(wrong_piece(&chunks("cat"), "dot"), None, "more than one sound");
        assert_eq!(wrong_piece(&chunks("cat"), "cat"), None);
    }
}
