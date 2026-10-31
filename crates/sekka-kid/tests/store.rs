//! Profiles, the log, the tracker's stretches of play and the report built
//! from them.

use std::path::{Path, PathBuf};

use sekka_kid::activity::{GRACE_MS, IDLE_MS};
use sekka_kid::clock;
use sekka_kid::{Animal, AppEntry, Event, Grade, KidColor, NewProfile, Round, Store, Tracker, report};

fn scratch(name: &str) -> PathBuf {
    let d = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("sekka-kid-{name}"));
    let _ = std::fs::remove_dir_all(&d);
    d
}

fn kid(store: &Store, name: &str) -> sekka_kid::Profile {
    store
        .add(NewProfile { name: name.into(), animal: Animal::Fox, color: KidColor::Amber, grade: Grade::Kindergarten })
        .unwrap()
}

fn round(app: &str, ended: i64, items: u32, first_try: u32, finished: bool, skill: &str) -> Event {
    Event::Round(Round {
        app: app.into(),
        activity: "a".into(),
        title: "A".into(),
        skills: vec![skill.into()],
        started: ended - 60_000,
        ended,
        items,
        first_try,
        hints: 1,
        stars: 2,
        finished,
        response_ms: Some(2_000),
        words_per_minute: None,
    })
}

#[test]
fn profiles_are_added_changed_and_removed() {
    let store = Store::at(scratch("profiles"));
    assert!(store.profiles().is_empty(), "no folder yet: no kids");
    let a = kid(&store, "  Hana  ");
    let b = kid(&store, "Ren");
    assert_eq!(a.name, "Hana", "names are trimmed");
    assert_ne!(a.id, b.id);
    assert_eq!(store.profiles().iter().map(|p| p.name.as_str()).collect::<Vec<_>>(), ["Hana", "Ren"]);

    let mut changed = a.clone();
    changed.daily_minutes = Some(20);
    changed.animal = Animal::Owl;
    store.update(&changed).unwrap();
    assert_eq!(store.profile(&a.id).unwrap().daily_minutes, Some(20));
    assert_eq!(store.profile(&a.id).unwrap().animal, Animal::Owl);

    assert!(
        store
            .add(NewProfile { name: "   ".into(), animal: Animal::Cat, color: KidColor::Sky, grade: Grade::First })
            .is_err(),
        "a kid needs a name"
    );

    store.append(&a.id, &round("x", 1, 1, 1, true, "s")).unwrap();
    assert!(store.kid_dir(&a.id).exists());
    store.remove(&a.id).unwrap();
    assert!(store.profile(&a.id).is_none());
    assert!(!store.kid_dir(&a.id).exists(), "their log goes with them");
    assert_eq!(store.profiles().len(), 1);
}

#[test]
fn kids_added_at_once_are_all_kept() {
    let root = scratch("concurrent");
    let threads: Vec<_> = (0..8)
        .map(|i| {
            let root = root.clone();
            std::thread::spawn(move || kid(&Store::at(root), &format!("Kid {i}")))
        })
        .collect();
    for t in threads {
        t.join().unwrap();
    }
    assert_eq!(Store::at(root).profiles().len(), 8);
}

#[test]
fn the_log_skips_lines_it_cannot_read() {
    let store = Store::at(scratch("log"));
    let k = kid(&store, "Mio");
    store.append(&k.id, &round("x", 10, 8, 6, true, "s")).unwrap();
    let path = store.kid_dir(&k.id).join("activity.jsonl");
    let mut text = std::fs::read_to_string(&path).unwrap();
    text.push_str("{\"kind\":\"from-the-future\",\"x\":1}\n{\"kind\":\"active\",\"app\"");
    std::fs::write(&path, text).unwrap();
    assert_eq!(store.events(&k.id).len(), 1);
}

#[test]
fn progress_is_the_apps_own_and_has_a_default() {
    #[derive(Default, serde::Serialize, serde::Deserialize, PartialEq, Debug)]
    struct P {
        lesson: u32,
    }
    let store = Store::at(scratch("progress"));
    let k = kid(&store, "Sora");
    assert_eq!(store.progress::<P>(&k.id, "computer.sekka.Anzan"), P::default());
    store.save_progress(&k.id, "computer.sekka.Anzan", &P { lesson: 4 }).unwrap();
    assert_eq!(store.progress::<P>(&k.id, "computer.sekka.Anzan").lesson, 4);
}

#[test]
fn the_tracker_counts_play_and_not_idleness() {
    let store = Store::at(scratch("tracker"));
    let k = kid(&store, "Yui");
    let t0 = clock::day_start(clock::now_ms()) + 10 * 3_600_000;
    let mut t = Tracker::new(store.clone(), &k.id, "app");
    // Thirty seconds of clicking.
    for s in 0..=30 {
        t.touch(t0 + s * 1000);
    }
    assert_eq!(t.unwritten(t0 + 31_000), 31_000, "up to now, inside the grace");
    t.flush(t0 + 35_000);
    // Gone for five minutes, then back for ten seconds.
    let back = t0 + 30_000 + 5 * 60_000;
    t.touch(back);
    t.touch(back + 10_000);
    t.finish(back + 10_000 + 2 * IDLE_MS);

    let spans: Vec<(i64, i64)> = store
        .events(&k.id)
        .into_iter()
        .filter_map(|e| match e {
            Event::Active { from, to, .. } => Some((from, to)),
            _ => None,
        })
        .collect();
    let total: i64 = spans.iter().map(|(a, b)| b - a).sum();
    assert_eq!(total, (30_000 + GRACE_MS) + (10_000 + GRACE_MS), "{spans:?}");
    assert!(spans.windows(2).all(|w| w[0].1 <= w[1].0), "no stretch counted twice: {spans:?}");
    assert_eq!(store.played_today(&k.id, back + 3 * IDLE_MS), total);
}

#[test]
fn the_report_adds_up_days_apps_skills_and_engagement() {
    let store = Store::at(scratch("report"));
    store
        .register_app(&AppEntry { id: "computer.sekka.Anzan".into(), name: "Anzan".into(), blurb: "Abacus".into() })
        .unwrap();
    let mut k = kid(&store, "Kai");
    k.daily_minutes = Some(30);
    store.update(&k).unwrap();
    let now = clock::day_start(clock::now_ms()) + 17 * 3_600_000;
    let yesterday = clock::day_start(now - 3_600_000 * 20) + 16 * 3_600_000;
    let id = &k.id;
    let anzan = "computer.sekka.Anzan";
    store.append(id, &Event::Active { app: anzan.into(), from: yesterday, to: yesterday + 12 * 60_000 }).unwrap();
    store.append(id, &Event::Active { app: "other".into(), from: now - 600_000, to: now - 300_000 }).unwrap();
    store.append(id, &Event::Active { app: anzan.into(), from: now - 300_000, to: now - 60_000 }).unwrap();
    // Long ago: outside the week.
    store
        .append(id, &Event::Active { app: anzan.into(), from: now - 30 * 86_400_000, to: now - 29 * 86_400_000 })
        .unwrap();
    store.append(id, &round(anzan, yesterday + 60_000, 8, 4, true, "little-friends")).unwrap();
    store.append(id, &round(anzan, now - 200_000, 8, 8, true, "little-friends")).unwrap();
    store.append(id, &round(anzan, now - 100_000, 3, 1, false, "numbers-to-9")).unwrap();
    store.append(id, &Event::Milestone { app: anzan.into(), at: now - 90_000, text: "Lit a lantern".into() }).unwrap();

    let r = report(&store, &k, now, 7);
    assert_eq!(r.days.len(), 7);
    assert_eq!(r.days.last().unwrap().start, clock::day_start(now));
    let today = r.days.last().unwrap();
    assert!((today.minutes[anzan] - 4.0).abs() < 1e-9);
    assert!((today.minutes["other"] - 5.0).abs() < 1e-9);
    assert!((r.days[5].minutes[anzan] - 12.0).abs() < 1e-9);
    assert!((r.totals.minutes - 21.0).abs() < 1e-9, "the month-old stretch is outside: {}", r.totals.minutes);
    assert_eq!(r.totals.days_played, 2);
    assert_eq!(r.totals.sittings, 2, "today's two stretches are one sitting");
    assert_eq!((r.totals.rounds, r.totals.finished), (3, 2));
    assert_eq!((r.totals.items, r.totals.first_try), (19, 13));
    assert_eq!(r.totals.median_response_ms, Some(2_000));
    assert_eq!(r.totals.usual_hour, Some(16), "yesterday's twelve minutes at four");
    assert_eq!(r.apps[0].name, "Anzan", "named from the catalogue, most played first");
    assert_eq!(r.apps[1].name, "other", "an app that never said is named by its id");
    let lf = r.skills.iter().find(|s| s.skill == "little-friends").unwrap();
    assert_eq!((lf.rounds, lf.items, lf.first_try), (2, 16, 12));
    assert_eq!(r.recent[0].skills, ["numbers-to-9"], "newest first");
    assert_eq!(r.milestones[0].1, "Anzan");
    assert!((r.today_minutes - 9.0).abs() < 1e-9);
    assert!((r.minutes_left.unwrap() - 21.0).abs() < 1e-9);
}
