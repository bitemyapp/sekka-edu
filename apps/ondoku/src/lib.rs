//! Ondoku 音読, a reading buddy. In Japanese schools, ondoku is reading
//! aloud: the daily homework a grown-up listens to and signs. Here a kid
//! reads aloud to Fumi the owl, who listens (Whisper, through Koe, on this
//! computer), follows along word by word, sounds out or says a word when
//! they're stuck, and can read any page aloud herself (Kokoro, through
//! `sekka-voice`). The books run from kindergarten's first decodable
//! readers to 5th grade, the library's own and children's classics.
//!
//! Each kid reads as their Sekka Kid profile: bookmarks and times through
//! stay in their folder, and each sitting with a book goes to their log
//! (words read alone, words helped with, words a minute) for Sekka Kids.

pub mod app;
pub mod art;
pub mod follow;
pub mod library;
pub mod listen;
pub mod phonics;
pub mod progress;
pub mod scene;
pub mod words;

pub use app::{APP_ID, Msg, Ondoku};
