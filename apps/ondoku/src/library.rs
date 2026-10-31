//! The books: Sekka Edu's own, written for the order phonics is taught in,
//! and children's classics from Project Gutenberg (`library/SOURCES.md`).
//! Each is a text file in `library/<shelf>/`: `key: value` lines, a blank
//! line, then pages separated by `===` lines, paragraphs (or stanzas) by
//! blank lines, and lines of verse kept on lines of their own.

use sekka_kid::{Animal, Grade};

include!(concat!(env!("OUT_DIR"), "/books.rs"));

/// The shelves, by reading level.
pub const LEVELS: [Level; 4] = [
    Level { n: 1, name: "First books", grades: "Kindergarten" },
    Level { n: 2, name: "Next books", grades: "1st grade" },
    Level { n: 3, name: "Story books", grades: "2nd and 3rd grade" },
    Level { n: 4, name: "Chapter books", grades: "4th and 5th grade" },
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Level {
    pub n: u8,
    pub name: &'static str,
    pub grades: &'static str,
}

/// The shelf a kid in `grade` starts at.
pub fn level_for(grade: Grade) -> u8 {
    match grade {
        Grade::Preschool | Grade::Kindergarten => 1,
        Grade::First => 2,
        Grade::Second | Grade::Third => 3,
        Grade::Fourth | Grade::Fifth => 4,
    }
}

/// What's on a cover.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Emblem {
    Animal(Animal),
    /// One of the kit's icons, by name.
    Icon(&'static str),
}

const ICONS: [&str; 13] = [
    "bug",
    "sun",
    "moon",
    "star",
    "cloud-rain",
    "snowflake",
    "leaf",
    "house",
    "bell",
    "music",
    "book-open",
    "umbrella",
    "clock",
];

fn emblem(name: &str) -> Option<Emblem> {
    if let Some(a) = Animal::ALL.iter().find(|a| a.name().eq_ignore_ascii_case(name)) {
        return Some(Emblem::Animal(*a));
    }
    ICONS.iter().find(|i| **i == name).map(|i| Emblem::Icon(i))
}

#[derive(Clone, Debug, PartialEq)]
pub struct Book {
    pub id: String,
    /// From Project Gutenberg rather than written here.
    pub classic: bool,
    pub title: String,
    pub author: Option<String>,
    pub year: Option<u32>,
    pub source: Option<String>,
    pub level: u8,
    pub skills: Vec<String>,
    pub emblem: Emblem,
    /// The cover's colour, `0xrrggbb`.
    pub color: u32,
    pub about: String,
    /// Each page's text: paragraphs apart by a blank line, verse lines by
    /// a line break.
    pub pages: Vec<String>,
}

impl Book {
    /// `text` in the library's format.
    pub fn parse(id: &str, classic: bool, text: &str) -> Result<Book, String> {
        let text = text.replace("\r\n", "\n");
        let (head, body) = text.split_once("\n\n").ok_or("no blank line after the header")?;
        let mut b = Book {
            id: id.to_owned(),
            classic,
            title: String::new(),
            author: None,
            year: None,
            source: None,
            level: 0,
            skills: Vec::new(),
            emblem: Emblem::Icon("book-open"),
            color: 0x5d8fd6,
            about: String::new(),
            pages: Vec::new(),
        };
        for line in head.lines() {
            let (k, v) = line.split_once(':').ok_or_else(|| format!("not `key: value`: {line}"))?;
            let v = v.trim();
            match k.trim() {
                "title" => b.title = v.to_owned(),
                "author" => b.author = Some(v.to_owned()),
                "year" => b.year = v.parse().ok(),
                "source" => b.source = Some(v.to_owned()),
                "level" => b.level = v.parse().map_err(|_| format!("level {v}"))?,
                "skills" => b.skills = v.split(',').map(|s| s.trim().to_owned()).filter(|s| !s.is_empty()).collect(),
                "about" => b.about = v.to_owned(),
                "cover" => {
                    let (e, c) = v.split_once(',').ok_or("cover: emblem, #colour")?;
                    b.emblem = emblem(e.trim()).ok_or_else(|| format!("no emblem {e}"))?;
                    b.color = u32::from_str_radix(c.trim().trim_start_matches('#'), 16)
                        .map_err(|_| format!("cover colour {c}"))?;
                }
                other => return Err(format!("unknown key {other}")),
            }
        }
        if b.title.is_empty() || !(1..=4).contains(&b.level) {
            return Err("a title and a level from 1 to 4".into());
        }
        let mut page = Vec::new();
        for line in body.lines().chain(std::iter::once("===")) {
            if line.trim() == "===" {
                let p = page.join("\n").trim().to_owned();
                if !p.is_empty() {
                    b.pages.push(p);
                }
                page.clear();
            } else {
                page.push(line.trim_end());
            }
        }
        if b.pages.is_empty() {
            return Err("no pages".into());
        }
        Ok(b)
    }

    pub fn words(&self) -> usize {
        self.pages.iter().map(|p| p.split_whitespace().count()).sum()
    }

    /// "Beatrix Potter, 1902".
    pub fn byline(&self) -> Option<String> {
        match (&self.author, self.year) {
            (Some(a), Some(y)) => Some(format!("{a}, {y}")),
            (Some(a), None) => Some(a.clone()),
            _ => None,
        }
    }
}

/// Every bundled book, by level, then the library's own before classics,
/// then title.
pub fn all() -> Vec<Book> {
    let mut books: Vec<Book> = BOOKS
        .iter()
        .filter_map(|(shelf, id, text)| match Book::parse(id, *shelf == "classics", text) {
            Ok(b) => Some(b),
            Err(e) => {
                log::warn!("library/{shelf}/{id}.txt: {e}");
                None
            }
        })
        .collect();
    books.sort_by(|a, b| (a.level, a.classic, &a.title).cmp(&(b.level, b.classic, &b.title)));
    books
}

/// Split each page that doesn't `fit` into pages that do: whole paragraphs
/// while they fit together, then a paragraph's lines of verse or its
/// sentences, then (only for a sentence too long by itself) its words. A
/// page that fits is left alone.
pub fn paginate(book: &mut Book, fits: impl Fn(&str) -> bool) {
    let mut out = Vec::new();
    for page in std::mem::take(&mut book.pages) {
        if fits(&page) {
            out.push(page);
            continue;
        }
        // The pieces, smallest first: paragraphs, each cut as far as it must be.
        let mut pieces: Vec<(String, &str)> = Vec::new();
        for para in page.split("\n\n").filter(|p| !p.trim().is_empty()) {
            if fits(para) {
                pieces.push((para.to_owned(), "\n\n"));
                continue;
            }
            let verse = para.lines().count() > 1;
            let parts: Vec<String> = if verse { para.lines().map(str::to_owned).collect() } else { sentences(para) };
            let glue = if verse { "\n" } else { " " };
            for (i, part) in parts.into_iter().enumerate() {
                let join = if i == 0 { "\n\n" } else { glue };
                if fits(&part) {
                    pieces.push((part, join));
                } else {
                    for (k, w) in part.split_whitespace().enumerate() {
                        pieces.push((w.to_owned(), if k == 0 { join } else { " " }));
                    }
                }
            }
        }
        let mut current = String::new();
        for (piece, join) in pieces {
            let candidate = if current.is_empty() { piece.clone() } else { format!("{current}{join}{piece}") };
            if current.is_empty() || fits(&candidate) {
                current = candidate;
            } else {
                out.push(std::mem::replace(&mut current, piece));
            }
        }
        if !current.is_empty() {
            out.push(current);
        }
    }
    book.pages = out;
}

/// A paragraph's sentences: each ends at . ! or ?, with any closing quote
/// or bracket, before a space.
fn sentences(para: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut current = String::new();
    let words: Vec<&str> = para.split_whitespace().collect();
    for w in words {
        if !current.is_empty() {
            current.push(' ');
        }
        current.push_str(w);
        let end = w.trim_end_matches(['"', '\'', '’', '”', ')', ']']);
        if end.ends_with(['.', '!', '?']) && !matches!(end, "Mr." | "Mrs." | "Dr." | "St.") {
            out.push(std::mem::take(&mut current));
        }
    }
    if !current.is_empty() {
        out.push(current);
    }
    out
}

/// A page's paragraphs, each its lines.
pub fn paragraphs(page: &str) -> Vec<Vec<&str>> {
    page.split("\n\n")
        .map(|p| p.lines().filter(|l| !l.trim().is_empty()).collect())
        .filter(|p: &Vec<&str>| !p.is_empty())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::phonics;

    #[test]
    fn a_book_reads_from_its_file() {
        let b = Book::parse(
            "t",
            false,
            "title: Test\nlevel: 1\nskills: short-a, fluency\ncover: cat, #e8553d\nabout: A test.\n\nSam has a cat.\n\nThe cat is fat.\n===\nA mat.\n",
        )
        .unwrap();
        assert_eq!(b.pages, ["Sam has a cat.\n\nThe cat is fat.", "A mat."]);
        assert_eq!(b.emblem, Emblem::Animal(Animal::Cat));
        assert_eq!(b.skills, ["short-a", "fluency"]);
        assert_eq!(paragraphs(&b.pages[0]), [vec!["Sam has a cat."], vec!["The cat is fat."]]);
        assert!(Book::parse("t", false, "title: X\n\nhi").is_err());
    }

    #[test]
    fn long_pages_are_split_where_they_break_best() {
        let mut b = Book::parse(
            "t",
            true,
            "title: T\nlevel: 4\nskills: fluency\ncover: owl, #333333\nabout: x\n\nOne two three. Four five six.\n\nSeven eight.\n===\nShort.\n===\nA b\nc d\ne f\n",
        )
        .unwrap();
        paginate(&mut b, |t| t.split_whitespace().count() <= 4);
        assert_eq!(b.pages, ["One two three.", "Four five six.", "Seven eight.", "Short.", "A b\nc d", "e f"]);
        assert_eq!(sentences("Mr. Brown said \"Hi!\" Then he went."), ["Mr. Brown said \"Hi!\"", "Then he went."]);
    }

    #[test]
    fn every_bundled_book_reads() {
        assert_eq!(all().len(), BOOKS.len(), "a bundled book failed to read (see the log)");
        for b in all() {
            assert!(!b.about.is_empty(), "{}: about", b.id);
            assert!(!b.skills.is_empty(), "{}: skills", b.id);
        }
    }

    /// Words a kindergartner meets by sight before they can sound them out
    /// (from the Dolch pre-primer and primer lists).
    const SIGHT: &[&str] = &[
        "a", "the", "is", "i", "to", "and", "see", "my", "we", "he", "she", "you", "said", "was", "look", "go", "no",
        "so", "of", "for", "are", "here", "do", "what", "where", "like", "come", "me", "be", "they", "have", "her",
        "all",
    ];

    /// The short vowels in the order the first books teach them.
    const ORDER: [(&str, char); 5] =
        [("short-a", 'a'), ("short-i", 'i'), ("short-o", 'o'), ("short-u", 'u'), ("short-e", 'e')];

    /// A word a kindergartner can sound out with these short vowels: one
    /// vowel, single consonants around it, and the ends ck, ll, ss, ff and zz.
    fn sounds_out(word: &str, vowels: &[char]) -> bool {
        let pieces = phonics::chunks(word);
        let vs: Vec<_> = pieces.iter().filter(|c| c.text.chars().any(|ch| "aeiou".contains(ch))).collect();
        vs.len() == 1
            && vs[0].text.len() == 1
            && vowels.contains(&vs[0].text.chars().next().unwrap())
            && !pieces.iter().any(|c| c.silent)
            && pieces.iter().all(|c| c.text.len() == 1 || ["ck", "ll", "ss", "ff", "zz"].contains(&c.text.as_str()))
    }

    #[test]
    fn the_first_books_can_be_sounded_out() {
        for b in all().into_iter().filter(|b| b.level == 1 && !b.classic) {
            let Some(last) = ORDER.iter().rposition(|(skill, _)| b.skills.iter().any(|s| s == skill)) else {
                panic!("{}: a first book teaches a short vowel", b.id);
            };
            let vowels: Vec<char> = ORDER[..=last].iter().map(|(_, v)| *v).collect();
            for page in &b.pages {
                for word in page.split_whitespace() {
                    let w = crate::words::key(word);
                    if w.is_empty() || SIGHT.contains(&w.as_str()) {
                        continue;
                    }
                    // Names are taught as they come: capitalised words
                    // still have to sound out.
                    assert!(sounds_out(&w, &vowels), "{}: {word:?} can't be sounded out with {vowels:?}", b.id);
                }
            }
        }
    }
}
