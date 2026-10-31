# Sekka Edu

**Educational software for [Sekka](https://sekka.computer), made for kids and
the grown-ups who look after them.**

Sekka Edu is where Sekka's learning apps start, outside Sekka's own repository,
as [Uta](https://github.com/bitemyapp/uta) did. They're written on Aizome's
kit, so they look and behave like the rest of the desktop, and they share
**Sekka Kid** profiles: each kid has one card, and their time, rounds and
progress follow them from app to app.

There are two so far: **Anzan**, a mental abacus for kindergarten and first
grade, and **Ondoku**, a reading buddy for kindergarten to 5th grade.
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

## Ondoku 音読: a reading buddy

In Japanese schools, ondoku is reading aloud: each evening a child reads to
a grown-up, who listens and signs their reading card. In Ondoku the listener
is Fumi, a little owl.

- **Read to Fumi.** Press the green button and read the page aloud. Each word
  lights up as Fumi hears it. A word left out gets dots under it, and going
  back over words doesn't lose your place.
- **Help with a word.** Stop on a word and Fumi helps. In the first books she
  shows its sounds first (h·e·n·s), then says it if you're still stuck. Tap
  any word to hear it.
- **Or listen first.** The blue button has Fumi read the page, lighting each
  word as she says it, so a kid can follow with a finger and then read it
  back.
- **Four shelves, kindergarten to 5th grade**, each opening at a kid's grade:
  - *First books*: six little books, one for each short vowel in the order
    phonics is taught (cat, dig, fox, pup, hen) and one mixing them. Every
    word can be sounded out with the vowels taught so far, apart from a few
    first sight words such as "the" and "said".
  - *Next books*: letter pairs (ship, chip), blends (frog, snap), silent e
    (bike, ride) and vowel teams (rain, seed).
  - *Story books*: stories with talking in them, riddles to read and guess,
    and how an acorn becomes an oak.
  - *Chapter books*: a lighthouse adventure, why the moon changes shape, and
    the true story of Snowflake Bentley.
- **Classics on every shelf**, marked with a gold corner: Mother Goose,
  McGuffey's First Reader, The Owl and the Pussy-cat, Peter Rabbit, Squirrel
  Nutkin, Aesop's fables, The Three Little Pigs, The Elves and the Shoemaker,
  A Child's Garden of Verses, two Just So Stories, The Velveteen Rabbit, and
  the first chapters of Alice, The Wizard of Oz, The Wind in the Willows and
  Winnie-the-Pooh. All are in the public domain, from Project Gutenberg.
- **Reading again is the point.** Each time through a book puts a star on its
  cover, because rereading is how reading gets smooth. A bookmark keeps each
  kid's place.
- **Type made for learning to read.** Pages are set in Andika, which shapes
  "a" and "g" the way children write them.

The microphone is on only while the green button is, and it switches itself
off after 40 seconds of quiet. Fumi listens with Whisper, through Koe,
Aizome's dictation, on this computer. What she hears is never saved and never
sent anywhere.

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
For reading, it shows words read alone, words Fumi helped with, and words a
minute, a common measure of how smoothly a child reads. It also edits the
kid's card and sets **daily time across every Sekka Edu app**. When time
runs out, an app lets the round or page in progress finish, then says
goodnight. A grown-up can give ten more minutes.

Everything is kept in your home folder (`~/.local/share/sekka-edu`) and never
leaves the computer.

## Before you rely on it

- **The grown-ups' door is a times-table question.** It keeps small hands
  away from settings and limits. It isn't parental control: a kid who can
  use a terminal or another account can get past it. Real controls are
  planned for Sekka itself.
- **Profiles belong to one Linux account.** Kids sharing a login pick their
  card at the start; separate accounts each have their own kids.
- **It's new.** Anzan has run in headless tests and on one developer's
  desktop. Ondoku has run in headless tests, with a computer voice reading
  its pages to it. The Nix package builds and passes its tests. Neither app
  has been tried by children yet, and there's no Sekka catalogue entry.
- **English only.** Anzan is written for kindergarten and first grade,
  Ondoku for kindergarten to 5th grade.
- **Fumi's hearing is Whisper's**, and Whisper wasn't made for young
  children's voices. She may miss words a small child reads. She may also
  take a word that's nearly right as right: she's meant to be generous.
  She's a buddy to practise with, not a reading test. Ondoku needs a
  microphone, and Whisper's model is 148 MB of the package.
- **The voice takes room and a moment.** Pon and Fumi speak with Kokoro,
  whose model is 362 MB of the package, and each app uses about half a
  gigabyte of memory with it loaded. A line not said before takes a moment
  to make the first time (the apps make lines they expect to need while the
  kid listens), and is kept after that. Built without Kokoro, they speak
  with espeak-ng, which is robotic and harder to follow.

## Trying it

With Nix, on x86_64 Linux in a Wayland session:

```
nix run github:bitemyapp/sekka-edu          # Anzan
nix run github:bitemyapp/sekka-edu#ondoku   # Ondoku
nix run github:bitemyapp/sekka-edu#kids     # Sekka Kids
```

[DEVELOPERS.md](DEVELOPERS.md) covers building, testing, how the apps share
profiles, and adding an app to the catalogue.

## Licence

MIT or Apache-2.0, at your option.
