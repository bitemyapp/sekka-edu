//! Sekka Kids, run headlessly: reports across apps, the door before
//! changes, the daily allowance, removing a kid; and a picture.

use std::path::{Path, PathBuf};
use std::rc::Rc;

use aizome_kit::Appearance;
use aizome_kit::layout::{Point, Size, Vec2};
use aizome_kit::testing::{Harness, assert_golden};
use aizome_kit::theme::ThemeChoice;
use sekka_kid::{Animal, AppEntry, Event, Grade, KidColor, NewProfile, Round, Store, clock};
use sekka_kid_kit::gate::GateMsg;
use sekka_kids::app::FormMsg;
use sekka_kids::{Kids, Msg};

fn scratch(name: &str) -> PathBuf {
    let d = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("kids-{name}"));
    let _ = std::fs::remove_dir_all(&d);
    d
}

/// Five in the afternoon, today.
fn afternoon() -> i64 {
    clock::day_start(clock::now_ms()) + 17 * 3_600_000
}

fn round(app: &str, title: &str, skill: &str, ended: i64, first_try: u32, finished: bool) -> Event {
    Event::Round(Round {
        app: app.into(),
        activity: title.to_lowercase(),
        title: title.into(),
        skills: vec![skill.into()],
        started: ended - 120_000,
        ended,
        items: 8,
        first_try,
        hints: 1,
        stars: if first_try >= 7 { 3 } else { 2 },
        finished,
        response_ms: Some(2_400),
    })
}

/// Two kids; Hana has played Anzan and another app this week.
fn store(name: &str) -> Store {
    let s = Store::at(scratch(name));
    s.register_app(&AppEntry {
        id: "computer.sekka.Anzan".into(),
        name: "Anzan".into(),
        blurb: "The mental abacus".into(),
    })
    .unwrap();
    s.register_app(&AppEntry {
        id: "computer.sekka.Sangaku".into(),
        name: "Sangaku".into(),
        blurb: "Temple geometry".into(),
    })
    .unwrap();
    let hana = s
        .add(NewProfile {
            name: "Hana".into(),
            animal: Animal::Fox,
            color: KidColor::Coral,
            grade: Grade::Kindergarten,
        })
        .unwrap();
    s.add(NewProfile { name: "Ren".into(), animal: Animal::Panda, color: KidColor::Sky, grade: Grade::First }).unwrap();
    let now = afternoon();
    let day = 86_400_000;
    let anzan = "computer.sekka.Anzan";
    for (d, mins, app) in
        [(6, 12, anzan), (4, 18, anzan), (3, 9, "computer.sekka.Sangaku"), (1, 22, anzan), (0, 8, anzan)]
    {
        let from = now - d * day - 30 * 60_000;
        s.append(&hana.id, &Event::Active { app: app.into(), from, to: from + mins * 60_000 }).unwrap();
    }
    for (d, ft, skill) in
        [(6, 4, "little-friends"), (4, 6, "little-friends"), (1, 7, "little-friends"), (1, 8, "numbers-to-9")]
    {
        s.append(&hana.id, &round(anzan, "Little friends", skill, now - d * day - 20 * 60_000, ft, true)).unwrap();
    }
    s.append(&hana.id, &round(anzan, "Peek!", "picturing-beads", now - 25 * 60_000, 2, false)).unwrap();
    s.append(
        &hana.id,
        &Event::Milestone { app: anzan.into(), at: now - day, text: "Lit lantern 9: Little friends".into() },
    )
    .unwrap();
    s
}

fn kids(name: &str, theme: ThemeChoice) -> (Harness<Kids>, Store) {
    let s = store(name);
    let mut app = Kids::new(s.clone());
    app.clock = Rc::new(afternoon);
    (Harness::with(app, Appearance { theme, ..Appearance::default() }, false), s)
}

#[test]
fn a_report_across_apps() {
    let (h, _) = kids("report", ThemeChoice::Day);
    let w = h.window();
    let texts = h.texts(w);
    for want in [
        "Hana",
        "Ren",
        "Anzan",
        "Sangaku",
        "1 h 9 min",
        "5 of 7",
        "4 of 5",
        "Little friends (pairs that make 5)",
        "coming along",
    ] {
        assert!(texts.iter().any(|t| t.contains(want)), "{want} in {texts:?}");
    }
    let r = h.program().report.as_ref().unwrap();
    assert_eq!(r.totals.days_played, 5);
    assert!((r.totals.minutes - 69.0).abs() < 1e-6);
}

#[test]
fn changes_need_a_grown_up_and_reach_every_app() {
    let (mut h, s) = kids("changes", ThemeChoice::Day);
    let w = h.window();
    h.send(Msg::Edit);
    // Sheets are drawn in the window they cover.
    let sheet = w;
    let q = h.texts(sheet).into_iter().find(|t| t.starts_with("What is ")).expect("the door");
    let n: Vec<u32> =
        q.trim_start_matches("What is ").trim_end_matches('?').split(" × ").map(|x| x.parse().unwrap()).collect();
    h.send(Msg::Gate(GateMsg::Answer("12".into())));
    h.send(Msg::Gate(GateMsg::Submit));
    assert!(!h.texts(sheet).iter().any(|t| t == "Edit card"), "a wrong answer keeps it shut");
    h.send(Msg::Gate(GateMsg::Answer((n[0] * n[1]).to_string())));
    h.send(Msg::Gate(GateMsg::Submit));
    assert!(h.texts(sheet).iter().any(|t| t == "Edit card"));
    h.send(Msg::Form(FormMsg::Limit(true)));
    h.send(Msg::Form(FormMsg::Minutes(25.0)));
    h.send(Msg::Form(FormMsg::Name("Hana-chan".into())));
    h.send(Msg::Form(FormMsg::Save));
    let hana = s.profiles().into_iter().find(|k| k.name == "Hana-chan").unwrap();
    assert_eq!(hana.daily_minutes, Some(25));
    assert!(h.texts(w).iter().any(|t| t.starts_with("Today: 8 min of 25 min")), "{:?}", h.texts(w));
    // Open now: removing asks only to be sure.
    h.send(Msg::Select(1));
    h.send(Msg::Remove);
    h.send(Msg::Removed);
    assert_eq!(s.profiles().iter().map(|k| k.name.clone()).collect::<Vec<_>>(), ["Hana-chan"]);
}

#[test]
fn a_picture_of_a_report() {
    for theme in [ThemeChoice::Day, ThemeChoice::Night] {
        let (mut h, _) = kids(&format!("golden-{theme:?}"), theme);
        let w = h.window();
        let suffix = if theme == ThemeChoice::Day { "day" } else { "night" };
        assert_golden(&mut h, w, format!("tests/golden/report-{suffix}.png"), 1.0);
    }
}

/// Three more weeks of Hana's play before this one, some days off, over
/// both apps: enough for the longer periods to fill their charts.
fn a_month_before(s: &Store) {
    let hana = s.profiles().into_iter().find(|k| k.name == "Hana").unwrap();
    let now = afternoon();
    let day = 86_400_000;
    let anzan = "computer.sekka.Anzan";
    for d in 7..30 {
        if d % 3 == 1 {
            continue;
        }
        let mins = 6 + (d * 7) % 19;
        let app = if d % 4 == 0 { "computer.sekka.Sangaku" } else { anzan };
        let from = now - d * day - 40 * 60_000;
        s.append(&hana.id, &Event::Active { app: app.into(), from, to: from + mins * 60_000 }).unwrap();
        if app == anzan {
            let ft = 3 + (d % 5) as u32;
            s.append(&hana.id, &round(anzan, "Big friends", "big-friends", from + 10 * 60_000, ft, true)).unwrap();
        }
    }
}

#[test]
fn pictures_of_longer_periods_and_a_narrow_window() {
    for theme in [ThemeChoice::Day, ThemeChoice::Night] {
        let (mut h, s) = kids(&format!("periods-{theme:?}"), theme);
        a_month_before(&s);
        let w = h.window();
        let suffix = if theme == ThemeChoice::Day { "day" } else { "night" };
        for (period, name) in [(1, "two-weeks"), (2, "month")] {
            h.send(Msg::Period(period));
            assert_golden(&mut h, w, format!("tests/golden/report-{name}-{suffix}.png"), 1.0);
        }
        // The smallest the window goes, a month in it.
        h.resize(w, Size::new(720.0, 480.0));
        assert_golden(&mut h, w, format!("tests/golden/report-narrow-{suffix}.png"), 1.0);
        // Down to the chart: every other date fits there.
        h.scroll(w, Point::new(500.0, 300.0), Vec2::new(0.0, 220.0));
        assert_golden(&mut h, w, format!("tests/golden/report-narrow-chart-{suffix}.png"), 1.0);
    }
}

#[test]
fn pictures_of_the_door_and_the_card() {
    for theme in [ThemeChoice::Day, ThemeChoice::Night] {
        let (mut h, _) = kids(&format!("sheets-{theme:?}"), theme);
        let w = h.window();
        let suffix = if theme == ThemeChoice::Day { "day" } else { "night" };
        h.send(Msg::Edit);
        assert_golden(&mut h, w, format!("tests/golden/door-{suffix}.png"), 1.0);
        let q = h.texts(w).into_iter().find(|t| t.starts_with("What is ")).unwrap();
        let n: Vec<u32> =
            q.trim_start_matches("What is ").trim_end_matches('?').split(" × ").map(|x| x.parse().unwrap()).collect();
        h.send(Msg::Gate(GateMsg::Answer((n[0] * n[1]).to_string())));
        h.send(Msg::Gate(GateMsg::Submit));
        h.send(Msg::Form(FormMsg::Limit(true)));
        assert_golden(&mut h, w, format!("tests/golden/card-{suffix}.png"), 1.0);
    }
}
