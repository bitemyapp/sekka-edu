# Sekka Edu

**Educational software for [Sekka](https://sekka.computer), made for kids and
the grown-ups who look after them.**

Sekka Edu is where Sekka's learning apps start, outside Sekka's own repository,
as [Uta](https://github.com/bitemyapp/uta) did. They're written on Aizome's
kit, so they look and behave like the rest of the desktop, and they share
**Sekka Kid** profiles: each kid has one card, and their time, rounds and
progress follow them from app to app.

The first app is **Anzan**, a mental abacus for kindergarten and first grade.
[docs/ideas.md](docs/ideas.md) has the list of what else might come.

## Anzan 暗算: the mental abacus

Anzan teaches the Japanese abacus, the soroban, and then takes it away, so
kids move the beads in their heads. That's anzan, mental calculation.

- **The beads are asleep until they touch the beam.** Push one up and it
  wakes, smiling; send it home and it dozes off. The big bead on top is five,
  a whole hand.
- **Pon the tanuki asks the questions.** He shows each number big, with a
  ten-frame of dots for kids who don't read numerals yet, and reads every
  question aloud in a warm, natural voice (Kokoro, which runs on this
  computer: nothing is sent anywhere).
- **Sixteen lessons along a lantern path**, in five chapters:
  - *Wake the beads*: ones, then the big bead, then reading and making 0 to 9.
  - *Peekaboo*: the beads show for a moment, then a curtain closes. Say what
    you saw, or make it again. Peeks get shorter as kids get good at them.
  - *More and less*: adding and taking away with the beads, and **little
    friends**, the pairs that make five, for when the earth beads run out.
  - *Tens* (first grade): a second rod, place value, and **big friends**, the
    pairs that make ten.
  - *In your head*: ghost beads that fade while you add, then numbers that
    flash by one at a time.
- **Help that teaches the method.** A hint lights up the beads to push next
  and says why ("Bring the big bead down", "Send 4 beads home"). After two
  misses, the beads show the answer by moving themselves, a push at a time.
- **Every finished round lights its lantern**, with up to three stars for
  answers right the first time. Kids can replay any lit lesson for more
  stars. There are no streaks, coins or timers pressing them, and nothing to
  lose by taking a day off.
- **Free play**: three rods to set any number, which Pon then says aloud.
- **Keys work too.** Digits answer, ↑ and ↓ push earth beads, ⇧↑ and ⇧↓ the
  big bead, ← and → change rods, H gives a hint and Return says "done".
  First graders can start at *Tens* or skip ahead through kindergarten's
  chapters.

It follows Aizome's Night and Day: lantern-lit indigo at night, warm paper
and hills by day. Reduced motion stops the beads sliding and the confetti.

## Sekka Kid profiles and Sekka Kids

Any Sekka Edu app can add a kid: a name, an animal, a colour and a grade.
Each app then writes to that kid's log:

- **Time played**: only time with input counts, so a window left open counts
  nothing.
- **Rounds**: questions asked, how many were right first time, hints, stars,
  how long answers took, and whether the round was finished or left part way.
- **Milestones**, such as the first time a lantern is lit.

**Sekka Kids** is the grown-ups' app. For each kid it shows minutes a day by
app, days played and sittings, rounds finished, how often answers are right
the first time, when they usually play, and how each skill is coming along.
It also edits the kid's card and sets **daily time across every Sekka Edu
app**. When time runs out, an app lets the round being played finish, then
says goodnight. A grown-up can give ten more minutes.

Everything is kept in your home folder (`~/.local/share/sekka-edu`) and never
leaves the computer.

## Before you rely on it

- **The grown-ups' door is a times-table question.** It keeps small hands
  away from settings and limits. It isn't parental control: a kid who can
  use a terminal or another account can get past it. Real controls are
  planned for Sekka itself.
- **Profiles belong to one Linux account.** Kids sharing a login pick their
  card at the start; separate accounts each have their own kids.
- **It's new.** It has run in headless tests and on one developer's desktop,
  and its Nix package builds and passes its tests. It hasn't been tried by
  children yet, and there's no Sekka catalogue entry.
- **English only**, written for kindergarten and first grade.
- **Pon's voice takes room and a moment.** Kokoro's model is 362 MB of the
  package, and Anzan uses about half a gigabyte of memory with it loaded.
  A line Pon hasn't said before takes a moment to make the first time
  (Anzan makes a round's lines while the kid listens), and is kept after
  that. Built without Kokoro, Pon speaks with espeak-ng, which is robotic
  and harder to follow.

## Trying it

With Nix, on x86_64 Linux in a Wayland session:

```
nix run github:bitemyapp/sekka-edu          # Anzan
nix run github:bitemyapp/sekka-edu#kids     # Sekka Kids
```

[DEVELOPERS.md](DEVELOPERS.md) covers building, testing, how the apps share
profiles, and adding an app to the catalogue.

## Licence

MIT or Apache-2.0, at your option.
