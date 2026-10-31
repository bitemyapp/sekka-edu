# Developing Sekka Edu

## Layout

| Path | What |
| --- | --- |
| `crates/sekka-kid` | Sekka Kid profiles: the store, the activity log, the time tracker and reports. No UI; no kit. |
| `crates/sekka-kid-kit` | What apps share on screen, on Aizome's kit: the kids' look and stage, animal avatars, "Who's playing?", the new-player form, the grown-ups' door, the report view, and one-instance launching (Soroban's). |
| `apps/anzan` | Anzan, the mental abacus (`sekka-anzan`). |
| `apps/kids` | Sekka Kids, profiles and reports for grown-ups (`sekka-kids`). |
| `docs/ideas.md` | Ideas for what else to make. |
| `nix/package.nix`, `flake.nix` | The package, built against Aizome's kit from a pinned Sekka. |
| `scripts/wx` | Build and test on wx-workstation. |

## The kit comes from Sekka

The workspace takes `aizome-kit` by path from a Sekka checkout beside this
one: `../sekka/desktop/aizome/crates/aizome-kit`. The kit takes Sukumo and its
sibling crates from the same tree. This is Uta's arrangement, with Sekka in
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
The sync is one way: copy back a `Cargo.lock` that cargo changed there, or
build with `--locked`.

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
  changes, a daily limit reaching the report, removing a kid, and pictures:
  each period (Week, Two weeks, Month) with a month of play, the smallest
  window, the door and the card.

The pictures in `tests/golden` are drawn at scale 1 with Aizome's fonts.
`AIZOME_UPDATE_GOLDENS=1` redraws them; look at them before committing.

These are component tests. Nothing here has run in a live Aizome session or
been played by children. To run a window without touching a live session, use
the headless sway in `sukumo/scripts/headless-wayland`, with `WAYLAND_DISPLAY`
and `DISPLAY` unset.

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
| `sound` | Sounds synthesised once to WAV and played with `pw-play`, `paplay` or `aplay`; Pon's voice through `espeak-ng`. Both stay silent without the programs. |
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
