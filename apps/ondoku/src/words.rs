//! Words as a page prints them and as a reader says them. A printed word
//! keeps its punctuation for drawing ("said," "'Now,"); its key is what's
//! read: lower case, letters and digits, numbers in words. What Whisper
//! writes down is keyed the same way, so "10" heard matches "ten" printed,
//! "Mr." matches "Mr.", and "there" matches "their" (a reader can't be
//! heard spelling). A word read two ways carries a hint to how it's read
//! here (`[read](red)`, sekka-voice's hints): read either way, it's right.

use sekka_voice::hints;

/// A word on the page.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Word {
    /// As printed, punctuation and all.
    pub text: String,
    /// What's read: empty for a lone dash or quote, which needn't be.
    pub key: String,
    /// The word as it's said, when it has a hint: "red," for
    /// "[read](red),".
    pub said: Option<String>,
}

impl Word {
    /// A word as the page has it, with its hint if it has one.
    pub fn new(text: &str) -> Word {
        let pieces = hints::pieces(text);
        let shown: String = pieces.iter().map(|p| p.text).collect();
        let said = pieces.iter().any(|p| p.said.is_some()).then(|| hints::said(text));
        Word { key: key(&shown), text: shown, said }
    }

    /// Whether `heard` (a key) is this word read aloud: as it's spelt, or
    /// as its hint says it.
    pub fn is(&self, heard: &str) -> bool {
        same(&self.key, heard) || self.said.as_deref().is_some_and(|s| same(&key(s), heard))
    }

    /// Whether it's read at all (a lone "—" isn't).
    pub fn spoken(&self) -> bool {
        !self.key.is_empty()
    }
}

/// The words of `text`, split at spaces.
pub fn words(text: &str) -> Vec<Word> {
    text.split_whitespace().map(Word::new).collect()
}

const ONES: [&str; 20] = [
    "zero",
    "one",
    "two",
    "three",
    "four",
    "five",
    "six",
    "seven",
    "eight",
    "nine",
    "ten",
    "eleven",
    "twelve",
    "thirteen",
    "fourteen",
    "fifteen",
    "sixteen",
    "seventeen",
    "eighteen",
    "nineteen",
];
const TENS: [&str; 10] = ["", "", "twenty", "thirty", "forty", "fifty", "sixty", "seventy", "eighty", "ninety"];

/// `n` in words, run together as a key ("twentyone"); digits past 9999
/// stay digits.
fn number(n: u64) -> String {
    match n {
        0..=19 => ONES[n as usize].to_owned(),
        20..=99 => {
            format!("{}{}", TENS[(n / 10) as usize], if n.is_multiple_of(10) { "" } else { ONES[(n % 10) as usize] })
        }
        100..=999 => {
            format!(
                "{}hundred{}",
                ONES[(n / 100) as usize],
                if n.is_multiple_of(100) { String::new() } else { number(n % 100) }
            )
        }
        1000..=9999 => format!(
            "{}thousand{}",
            ONES[(n / 1000) as usize],
            if n.is_multiple_of(1000) { String::new() } else { number(n % 1000) }
        ),
        _ => n.to_string(),
    }
}

/// What a printed or heard word is read as.
pub fn key(word: &str) -> String {
    let lower = word.to_lowercase();
    // Digits as words; letters and digits kept, the rest dropped (an
    // apostrophe too: "don't" is "dont" either way).
    let mut out = String::new();
    let mut digits = String::new();
    for ch in lower.chars() {
        if ch.is_ascii_digit() {
            digits.push(ch);
            continue;
        }
        if !digits.is_empty() {
            out.push_str(&number(digits.parse().unwrap_or(0)));
            digits.clear();
        }
        if ch.is_alphanumeric() {
            out.push(ch);
        }
    }
    if !digits.is_empty() {
        out.push_str(&number(digits.parse().unwrap_or(0)));
    }
    match out.as_str() {
        "mister" => "mr".into(),
        "missus" | "missis" => "mrs".into(),
        _ => out,
    }
}

/// What Whisper wrote down, as keys: bracketed asides ("[BLANK_AUDIO]",
/// "(laughs)") and hyphens' halves dealt with.
pub fn heard(text: &str) -> Vec<String> {
    let mut plain = String::new();
    let mut depth = 0;
    for ch in text.chars() {
        match ch {
            '[' | '(' | '*' if depth == 0 => depth = 1,
            ']' | ')' | '*' if depth == 1 => depth = 0,
            _ if depth == 0 => plain.push(if ch == '-' || ch == '—' { ' ' } else { ch }),
            _ => {}
        }
    }
    plain.split_whitespace().map(key).filter(|k| !k.is_empty()).collect()
}

/// Words that sound the same: read aloud, either is right.
const SOUND_ALIKE: &[&[&str]] = &[
    &["to", "too", "two"],
    &["there", "their", "theyre"],
    &["see", "sea"],
    &["for", "four", "fore"],
    &["one", "won"],
    &["i", "eye"],
    &["no", "know"],
    &["here", "hear"],
    &["right", "write"],
    &["son", "sun"],
    &["be", "bee"],
    &["dear", "deer"],
    &["tail", "tale"],
    &["ate", "eight"],
    &["by", "buy", "bye"],
    &["whole", "hole"],
    &["new", "knew"],
    &["night", "knight"],
    &["meet", "meat"],
    &["pair", "pear"],
    &["flower", "flour"],
    &["blew", "blue"],
    &["road", "rode"],
    &["wood", "would"],
    &["hare", "hair"],
    &["sail", "sale"],
    &["tail", "tale"],
    &["week", "weak"],
    &["your", "youre"],
    &["not", "knot"],
    &["so", "sew"],
    &["oh", "o"],
    &["its", "its"],
    &["red", "read"],
    &["wait", "weight"],
    &["whose", "whos"],
    &["were", "where", "wear"],
    &["ok", "okay"],
];

fn sound_alike(a: &str, b: &str) -> bool {
    SOUND_ALIKE.iter().any(|set| set.contains(&a) && set.contains(&b))
}

/// Edit distance, up to `cap` (then `cap + 1`).
fn distance(a: &str, b: &str, cap: usize) -> usize {
    let (a, b): (Vec<char>, Vec<char>) = (a.chars().collect(), b.chars().collect());
    if a.len().abs_diff(b.len()) > cap {
        return cap + 1;
    }
    let mut row: Vec<usize> = (0..=b.len()).collect();
    for (i, x) in a.iter().enumerate() {
        let mut prev = row[0];
        row[0] = i + 1;
        for (j, y) in b.iter().enumerate() {
            let here = row[j + 1];
            row[j + 1] = (row[j + 1] + 1).min(row[j] + 1).min(prev + usize::from(x != y));
            prev = here;
        }
    }
    row[b.len()].min(cap + 1)
}

/// Whether `heard` is `printed` read aloud: the same word, one that sounds
/// the same, the same less or plus an "s", or (in words of five letters or
/// more) one letter off, as Whisper's spelling of a name may be.
pub fn same(printed: &str, heard: &str) -> bool {
    if printed == heard || sound_alike(printed, heard) {
        return true;
    }
    let s = |a: &str, b: &str| a.len() >= 3 && a.strip_suffix('s') == Some(b);
    if s(printed, heard) || s(heard, printed) {
        return true;
    }
    printed.chars().count() >= 5 && distance(printed, heard, 1) <= 1
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn printed_words_keep_their_punctuation_and_read_plainly() {
        let w = words("'Now, my dears,' said old Mrs. Rabbit—");
        let keys: Vec<&str> = w.iter().map(|w| w.key.as_str()).collect();
        assert_eq!(keys, ["now", "my", "dears", "said", "old", "mrs", "rabbit"]);
        assert_eq!(w[0].text, "'Now,");
        assert!(!Word::new("—").spoken());
        assert_eq!(key("don't"), "dont");
        assert_eq!(key("10"), "ten");
        assert_eq!(key("21st"), "twentyonest");
        assert_eq!(key("fir-tree"), "firtree");
    }

    #[test]
    fn what_whisper_writes_is_keyed_alike() {
        assert_eq!(heard(" Sam has 10 cats. [BLANK_AUDIO]"), ["sam", "has", "ten", "cats"]);
        assert_eq!(heard("Cotton-tail (laughs) and Peter"), ["cotton", "tail", "and", "peter"]);
        assert_eq!(heard("Mister McGregor"), ["mr", "mcgregor"]);
        assert!(heard("*music*").is_empty());
    }

    #[test]
    fn sounding_the_same_is_reading_it() {
        assert!(same("their", "there"));
        assert!(same("two", "too"));
        assert!(same("cats", "cat"));
        assert!(same("mcgregor", "macgregor"));
        assert!(!same("cat", "cap"));
        assert!(!same("sat", "set"));
        assert!(!same("is", "i"));
    }

    #[test]
    fn a_word_with_a_hint_is_printed_plainly_and_read_either_way() {
        let w = Word::new("[read](red),");
        assert_eq!((w.text.as_str(), w.key.as_str(), w.said.as_deref()), ("read,", "read", Some("red,")));
        assert!(w.is("read") && w.is("red"));
        assert!(!w.is("reed"));
        assert!(!Word::new("bow").is("bough"), "only where a hint says");
        assert!(Word::new("[bow](bough)").is("bough"));
        assert_eq!(words("You [read](red) it.").len(), 3);
        let w = Word::new("sneezed—'[Kertyschoo](Kertishoo)!'");
        assert_eq!(w.said.as_deref(), Some("sneezed—'Kertishoo!'"), "the rest of it kept");
    }
}
