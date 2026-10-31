# Developing Sekka Edu

## Layout

| Path | What |
| --- | --- |
| `crates/sekka-kid` | Sekka Kid profiles: the store, the activity log, the time tracker and reports. No UI; no kit. |
| `crates/sekka-kid-kit` | What apps share on screen, on Aizome's kit: the kids' look and stage, animal avatars, "Who's playing?", the new-player form, the grown-ups' door, the report view, and one-instance launching (Soroban's). |
| `crates/sekka-voice` | The voice apps speak with: Kokoro-82M through sherpa-onnx, or espeak-ng. Speaks on its own thread, keeps what it has said, makes lines ahead. No kit. |
| `apps/anzan` | Anzan, the mental abacus (`sekka-anzan`). |
| `apps/ondoku` | Ondoku, the reading buddy (`sekka-ondoku`), and its books (`library/`). |
| `apps/kids` | Sekka Kids, profiles and reports for grown-ups (`sekka-kids`). |
| `docs/ideas.md` | Ideas for what else to make. |
| `nix/package.nix`, `flake.nix` | The package, built against Aizome's kit from a pinned Sekka. |
| `nix/kokoro.nix` | Kokoro's model, as sherpa-onnx packs it. |
| `nix/whisper-base-en.nix` | The Whisper model Ondoku listens with, pinned as Koe pins it. |
| `tools/voice-check` | Compare voices by what Whisper hears of them. |
| `scripts/wx` | Build and test on wx-workstation. |

## The kit comes from Sekka

The workspace takes `aizome-kit` by path from a Sekka checkout beside this
one: `../sekka/desktop/aizome/crates/aizome-kit`. The kit takes Sukumo and its
sibling crates from the same tree. Ondoku takes Koe, Aizome's dictation
(`apps/koe`, as the library `koe`), the same way, with its own sibling
`aizome-ipc`. This is Uta's arrangement, with Sekka in
place of Aizome's former repository. Develop with a checkout, or a worktree,
at `../sekka`, inside its `nix develop .#aizome` shell. The shell has Rust
1.95, the libraries the kit loads, and `AIZOME_FONT_DIR`.

`Cargo.lock` started as `desktop/aizome/Cargo.lock`, so it holds the versions
Aizome builds with. Cargo applies only this workspace's `[patch]` sections. None
are needed today: the kit builds with crates.io's versions of what Aizome
patches.

`flake.nix` pins Sekka by revision (`inputs.sekka`) and follows its Nixpkgs.
The package copies the kit's crates and Sukumo's out of that revision, as
Sekka's `nix/uta.nix` does for Uta, and builds with Nixpkgs' own
`rustPlatform`. To build against a newer kit:

1. Move `inputs.sekka` to the new revision, and `../sekka` to the same.
2. In the dev shell, run `cargo update --workspace`, which adds what the kit
   now needs and leaves other locked versions alone. Then check that
   `cargo test --locked --workspace` passes.
3. Run `nix flake lock` and `nix build .#sekka-edu`.

## Building and testing

Builds and tests of anything on the kit run on wx-workstation, not on the
laptop. `scripts/wx` does the round trip:

```
SEKKA=../sekka SEKKA_REV=<sekka revision> scripts/wx run cargo test --locked --workspace
```

`sync` copies this tree and only the kit's part of `../sekka` to
`~/sekka-work/edu` there, checksumming rather than trusting times. `shell`
roots Sekka's `#aizome` dev shell for `SEKKA_REV` and builds Aizome's fonts
once. `run` does both, then runs the command in the dev shell with
`CARGO_TARGET_DIR` on `/dev/shm` and the rustc wrapper run by bash. wx's
`/bin/sh` is dash, which drops the hyphenated variables the kit's tests need.
It also fetches Andika once, for Ondoku's pages
(`SEKKA_EDU_READING_FONT`). It builds whisper.cpp for any x86-64 with AVX2
(`GGML_NATIVE=OFF`), not just wx's Zen 5, so binaries built there run on the
laptop. The sync is one way: copy back a `Cargo.lock` that cargo changed
there, or build with `--locked`.

The tests:

- `sekka-kid`: profiles, concurrent adds, a log with unreadable lines, the
  tracker's stretches of play, and a report across days, apps and skills.
- Anzan's units: the abacus, the soroban methods for every sum to 99 (direct,
  little friends, big friends, both), every lesson's questions across seeds,
  rounds, hints, peeks, flashes, progress and unlocking, and the sounds.
- `apps/anzan/tests/anzan.rs`, on the kit's headless harness with a clock the
  test moves: making a first player, a whole round played by clicking beads
  where they're drawn, its log and progress, the daily allowance and the
  grown-ups' door, keys, peeks, and pictures of every screen by Day and Night.
- `apps/kids/tests/kids.rs`: a report across two apps, the door before
  changes, a daily limit reaching the report, reading rounds counted in
  words, removing a kid, and pictures.
- Ondoku's units:
  - printed and heard words (punctuation, numbers, words that sound the
    same);
  - following a page (skips, misreadings, going back, guesses, help);
  - phonics chunks, the book format and pagination;
  - every bundled book reading;
  - a check that every word of the first books can be sounded out with the
    short vowels taught so far, or is one of the listed sight words.
- `apps/ondoku/tests/ondoku.rs`, with pretended ears (what's heard is sent
  in): a first reader, a whole book read to Fumi and its log, a word tapped
  where it's drawn, help when stuck, the day ending at a page, every page of
  every book fitting on the paper in its level's type, and pictures.
- `apps/ondoku/tests/ears.rs` (`--ignored`, needs Kokoro, sherpa-onnx and a
  Whisper model): real listening through Koe, with Kokoro reading a page to
  a pretend `pw-record`. Whisper hears it in stretches cut at the pauses, and
  the page is followed to its end. A word read wrongly ("red" for "tan") is
  marked, not taken as read.

The pictures in `tests/golden` are drawn at scale 1 with Aizome's fonts.
`AIZOME_UPDATE_GOLDENS=1` redraws them; look at them before committing.

These are component tests. Anzan has run in a live Aizome session; Ondoku
hasn't been tried by a person reading to it, and neither app has been played
by children. To run a window without touching a live session, use the
headless sway in `sukumo/scripts/headless-wayland`, with `WAYLAND_DISPLAY` and
`DISPLAY` unset.

## Sekka Kid: the shared data

`Store::default_root()` is `$SEKKA_KID_DIR`, else `$XDG_DATA_HOME/sekka-edu`,
else `~/.local/share/sekka-edu`:

```
profiles.json                 { "version": 1, "kids": [Profile] }
profiles.lock                 flock'd while profiles.json is changed
catalogue/<app id>.json       { id, name, blurb }: each app that has run
kids/<kid id>/activity.jsonl  the log: one event a line, appended
kids/<kid id>/apps/<id>.json  each app's own progress for the kid
```

Events are tagged by `kind`:

- `active`: `{ app, from, to }`, in Unix milliseconds.
- `round`: `{ app, activity, title, skills, started, ended, items,
  first_try, hints, stars, finished, response_ms }`.
- `milestone`: `{ app, at, text }`.

Each event is one `write` to a file opened for appending, so apps logging at
once don't interleave lines. Whole files are written to a temporary file and
renamed over the old one. Readers skip lines they can't parse, so a newer
app's events don't break an older reader.

`Tracker` turns input into stretches of active play. Each input extends the
stretch. A minute without input ends it, ten seconds after the last input.
`flush` writes what has been played so far, so a crash loses at most the time
since the last flush. Anzan flushes every 30 seconds and whenever the screen
changes.

`Profile::daily_minutes` is the day's allowance across the catalogue.
`Store::played_today` adds the kid's `active` time for the local day from
every app. An app adds its own unflushed time with `Tracker::unwritten`.

## Adding an app to the catalogue

1. Depend on `sekka-kid` and `sekka-kid-kit`. Give the app an id under
   `computer.sekka.` and call `Store::register_app` when it starts.
2. Start with "Who's playing?" (`who`) and the new-player form (`newkid`).
3. Keep a `Tracker` for the kid. Call `touch` on every input, `flush` now and
   then and on screen changes, and `finish` when the kid leaves.
4. Log each round as `Event::Round`, finished or left part way, with
   `skills` drawn from a shared vocabulary so reports can follow a skill
   across apps. Log `Event::Milestone` for what a grown-up would like to
   hear about.
5. Check the allowance before starting a round, and again when one ends.
   Never stop a kid in the middle of a round.
6. Keep app-specific progress with `Store::progress` and `save_progress`.
7. Put the grown-ups' choices behind `gate`.

## Anzan's parts

| Module | What |
| --- | --- |
| `abacus` | Rods, beads, flicks and pushes. |
| `method` | The soroban methods: direct, little friends and big friends, as pushes with reasons. Hints and "showing how" come from these. |
| `lessons` | The chapters, the sixteen lessons and their introductions, and each round's questions (seeded). |
| `round` | A round's state: peeks, flashes, answers, hints, showing how, scoring, and the log record. Time comes in from outside. |
| `progress` | What Anzan keeps for each kid: stars, introductions heard, peek length, sound and voice, extra time today. |
| `scene` | The screens as one canvas, laid out in a 1000 × 700 design scaled to the window: what's drawn, hit-testing, painting. |
| `art` | The soroban, beads with faces, Pon's bubble, ten-frames, lanterns, stars, the curtain, confetti. |
| `sound` | Sounds synthesised once to WAV and played with `pw-play`, `paplay` or `aplay`; silent without them. Pon's voice is `sekka-voice`. |
| `app` | The program: screens, input, the beads' slides and cheers, timers, the tracker, the allowance and the grown-ups' sheet. |

Frames tick every 20 ms, but only while something moves: a slide, a cheer, a
peek or flash, the curtain, or the summary's stars. A still screen draws
nothing.

The canvas's `Hash` leaves out what moves by itself, and `scene::parts`
gives each moving thing its own rectangle: each soroban rod, Pon, the peek's
bar, the summary's stars, the kid's marker, blinking faces and the pressed
button. A slide redraws one rod; a new question or screen redraws the whole
window. Big gradients are expensive at twice the scale, so the sky is drawn
in fine flat bands and the frame flat; shadows are hard, as in Aizome. The
beads keep their gradients.

`cargo test --release -p sekka-anzan --test budget -- --ignored --nocapture`
times frames from the clock's tick to the damage drawn at 2×. On
wx-workstation (load average 11 to 20, shared) on 2026-10-31:

| Frame | p50 | p95 |
| --- | --- | --- |
| A bead sliding (free play, three rods) | 1.7 ms | 2.8 ms |
| The whole window (a new question or screen) | 7.5 ms | 9.4 ms |
| Still | nothing drawn | |

A blink redraws the awake beads' rods and Pon, about 5 ms every 3.7 s.
Aizome's budget is 4 ms from an event to its frame: slides are inside it,
whole-window changes aren't yet.

## Ondoku's parts

| Module | What |
| --- | --- |
| `library` | The books: their file format, the shelves (levels 1 to 4 for kindergarten, 1st, 2nd–3rd and 4th–5th grade), and splitting pages too long for their level's type. `build.rs` bundles every file under `library/`. |
| `words` | Words as printed (punctuation kept for drawing) and as said: a key that's lower case, without punctuation, numbers in words, and sound-alikes (there, their) taken as the same. |
| `follow` | A page being read: what Whisper heard lined up with the words still to read, the words read, helped with and missed, and how far a guess mid-sentence has the reader. |
| `listen` | Fumi's ears: the microphone through `pw-record`, Whisper through Koe, stretches of reading cut at pauses, guesses while a stretch goes on, "stuck" after a quiet, and a mute while Fumi talks. |
| `phonics` | A word in the pieces it's sounded out in (sh, ee, igh, a silent e), for the first books. |
| `progress` | What Ondoku keeps for each kid: bookmarks, times through each book, words a minute, the grown-ups' choices. |
| `scene`, `art` | The screens as one canvas in the 1000 × 700 design: the shelves, covers, a page laid out word by word (so each can be lit and tapped), Fumi, the microphone's rings. |
| `app` | The program: who's reading, the shelves, a book read to Fumi or by her, help, the log, the allowance, the grown-ups' sheet. |

**Koe as a library.** Ondoku depends on `aizome-koe` by path and uses its
library:
- `koe::engine::Recognizer`: Whisper through whisper.cpp, the model kept
  loaded, `with_prompt` per page.
- `koe::audio`'s `PcmDecoder`, `rms` and `has_signal`: PipeWire's raw
  samples, loudness, and the gate that keeps silence from Whisper.
- `koe::models`: Koe's pinned catalogue and the folder its downloads go to.

The model is `SEKKA_EDU_WHISPER`, which the package sets to Whisper base.en
(`nix/whisper-base-en.nix`, Koe's own pin), or else one Koe has downloaded,
English ones first. Ondoku doesn't use Koe's session service: it records for
itself, and only while the green button is on.

**Listening.**
- The ears run on a thread of their own with a small Tokio runtime. They
  record 16 kHz mono and cut a stretch at a 0.7 s pause, or at 12 s. Each
  stretch goes to Whisper, prompted with "A child reading aloud." and the
  page's words, Koe's way of hinting vocabulary.
- While a stretch goes on, a guess is made at it every second, so the page
  can show how far the reader has got.
- Four seconds of quiet after reading has begun is "stuck". Forty switch the
  microphone off.
- Whenever Fumi speaks, the voice says for how long (`sekka-voice`'s
  `on_speaking`), and the ears hear nothing for that time, so her voice
  isn't taken for the reader's.

**Following.** `follow::Page::hear` lines up the heard words with the
printed words still to read: an edit distance in which a printed word not
heard costs 1, a different word heard in its place costs 1, and a heard
word not on the page costs 0.6, so repeats and "um"s are cheap. The page
moves on to just after the last printed word heard. A word left out or
misread before it is marked missed; a misread last word isn't passed.
Words match as printed, sounding the same, with or without a final s, or
(five letters and up) one letter off. Fumi is generous on purpose.

**The library.** `library/<shelf>/<id>.txt`: `key: value` lines (`title`,
`author`, `year`, `source`, `level`, `skills`, `cover`, `about`), a blank
line, then pages separated by `===` lines.
- Paragraphs, or stanzas, are separated by blank lines. Lines of verse stay
  on lines of their own.
- `cover` is an animal from the avatars or a Lucide icon the kit has, and a
  colour.
- `skills` names what a book practises, for Sekka Kids' report: the phonics
  patterns, `sight-words`, `fluency`, `expression`, `rhyme`, `poetry` and
  `vocabulary`.
- `library/own` holds the books written for Ondoku, in the order phonics is
  usually taught:
  - Level 1: short a, i, o, u and e, then all of them, with a few first
    sight words. A test checks every word can be sounded out with the
    vowels taught so far.
  - Level 2: letter pairs, blends, silent e and vowel teams.
  - Level 3: r-controlled vowels, dialogue, riddles, non-fiction.
  - Level 4: longer narrative and non-fiction.
- `library/classics` holds public-domain books from Project Gutenberg, with
  their header and licence removed. `library/SOURCES.md` lists each one's
  ebook number, what was included, and what was left out and why.
- At launch, a page that won't fit on the paper in its level's type (36, 32,
  24 and 22 px from level 1 up) is split: at paragraphs, then lines of verse
  or sentences, then words.

**Type.** Pages are set in Andika (SIL, OFL), made for children learning to
read: `SEKKA_EDU_READING_FONT` names its regular file. The package and
`scripts/wx` set it. Without it, pages are set in Archivo, and the pictures
won't match.

**What's logged.** Each sitting with a book is one `round`:
- `activity` is the book's id and `skills` its skills.
- `items` is the words on the pages read, `first_try` the words read alone,
  and `hints` the words Fumi said.
- `finished` means it was read to the end, and `words_per_minute` is the
  words read or helped with over the time from the first sound of reading
  on each page to the last word heard.
- Milestones are a book read to the end, and read three times.

Sekka Kids shows rounds that practise `fluency` in words, not questions.
The day's allowance ends reading at a page turn, with the next page kept.

## The voice

`sekka-voice` speaks with [Kokoro-82M](https://huggingface.co/hexgrad/Kokoro-82M)
v1.0 (Apache-2.0) as `af_heart` (speaker 3 of sherpa-onnx's
`kokoro-multi-lang-v1_0`), a little slower than its own pace. It runs on
the processor through [sherpa-onnx](https://github.com/k2-fsa/sherpa-onnx)'s
C API. The library is loaded with `dlopen` when the voice starts rather
than linked, so a build needs neither it nor the model.

- `SEKKA_EDU_KOKORO` names the model's folder, which is `nix/kokoro.nix`'s
  output. `SEKKA_EDU_SHERPA_ONNX` names `libsherpa-onnx-c-api.so`, which
  is otherwise looked up on the library path. The package's wrapper sets
  both.
- `kokoro.rs` mirrors sherpa-onnx 1.13's structures. Another version
  isn't used. Each structure goes over with zeroed room after it, for the
  fields sherpa-onnx adds at the end. If Kokoro can't load, the voice
  speaks with espeak-ng, and Anzan's grown-ups' sheet says which voice
  is speaking.
- Lines are made on the voice's own thread, at a lower priority than the
  app, on two threads of onnxruntime's. They're kept as WAV files under
  `$XDG_CACHE_HOME/sekka-edu/voice/<engine>` (the newest 4000). `say` cuts
  in. `prepare` queues lines to make while nothing is being said: Anzan
  prepares a round's introduction, questions, praise and ending when the
  round starts, the kids' greetings, and 0 to 100 for free play. Praise
  runs through a fixed order from a place set when the round starts, so
  it can be made ahead.

Why Kokoro: as of 2026, it leads small voices that run on a processor
for quality. To check what it does for Pon, every line Anzan says (its
lesson introductions, prompts, praise, hints and numbers: 56 lines, 340
words) was spoken by espeak-ng with Anzan's former settings
(`en-us+f3`, 135 words a minute) and by Kokoro, and Whisper `base.en`
(whisper.cpp 1.9.5) wrote down what it heard. On wx-workstation, on
2026-10-31:

| Voice | Words Whisper got wrong | Lines heard exactly |
| --- | --- | --- |
| espeak-ng | 88 of 340 (26%) | 15 of 56 |
| Kokoro `af_heart` | 8 of 340 (2.4%) | 50 of 56 |

espeak-ng's errors included "Each little bead is one" heard as
"B.T.L.B. is one", "Show me seven" as "Show me someone", and "fifteen" as
"50". Most of Kokoro's were spelling, such as "10's" for "tens". Its one
mistaken word was "bead" heard as "beat", a word espeak-ng also got wrong.
Kokoro made a line in a quarter to a third of the time the line lasts,
on two threads.

`tools/voice-check/run.sh FOLDER` repeats the comparison. Its header lists
what it needs.

`cargo test -p sekka-voice --test kokoro -- --ignored`, with both
variables set, speaks three lines through the binding. It writes them
under the target's temporary folder to listen to or transcribe.
