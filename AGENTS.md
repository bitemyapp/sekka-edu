# Working on Sekka Edu

Read DEVELOPERS.md for the layout, the shared Sekka Kid data, and how to build
and test. Develop on a topic branch from main.

- Build and test anything on Aizome's kit on wx-workstation with `scripts/wx`,
  not on the laptop. The kit comes from a Sekka checkout at `../sekka`; keep
  `flake.nix`'s Sekka revision and `Cargo.lock` in step with it.
- Look at the pictures the tests draw (`tests/golden`) after changing what's
  on screen, by Day and by Night, before committing them.
- Kids' apps: no streaks, currencies or pressure. Let rounds finish before
  the day's allowance ends play. Put grown-ups' choices behind the gate, and
  keep everything on the machine.
- Report honestly: Anzan has run in a live session, Ondoku only in headless
  tests (and with a computer voice reading to it), neither with children.
  Say which you checked.
- README prose describes the experience and its limits for families.
  Implementation details belong in DEVELOPERS.md.
