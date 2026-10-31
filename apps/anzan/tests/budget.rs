//! How long Anzan's frames take, from the clock's tick to the damage
//! rasterised at 2×: a bead sliding to the beam, and a still screen.
//! `cargo test --release -p sekka-anzan --test budget -- --ignored --nocapture`.

use std::time::{Duration, Instant};

use aizome_kit::Appearance;
use aizome_kit::layout::{Point, Rect};
use aizome_kit::sidestep_ui::{Modifiers, NamedKey};
use aizome_kit::testing::Harness;
use sekka_anzan::Anzan;
use sekka_anzan::abacus::Bead;
use sekka_anzan::lessons::LESSONS;
use sekka_anzan::scene;
use sekka_kid::{Animal, Grade, KidColor, NewProfile, Store, clock};
use sekka_kid_kit::stage::Fit;

/// Draw the damage as the window's next frame would; the share of the
/// window it covers.
fn draw(h: &mut Harness<Anzan>, w: aizome_kit::WindowKey) -> f64 {
    let mut damage = h.take_damage(w);
    // A part that changed damages its rectangle twice (as it was, as it is).
    damage.dedup_by(|a, b| a == b);
    damage.sort_by(|a, b| a.x0.total_cmp(&b.x0).then(a.y0.total_cmp(&b.y0)));
    damage.dedup_by(|a, b| a == b);
    let size = h.size(w);
    if damage.is_empty() {
        return 0.0;
    }
    let _ = h.render_damage(w, &damage, 2.0);
    let area: f64 = damage.iter().map(|r| r.width() * r.height()).sum();
    (area / (size.width * size.height)).min(1.0)
}

fn report(name: &str, mut v: Vec<Duration>, area: f64) -> f64 {
    v.sort();
    let p = |q: f64| v[((v.len() - 1) as f64 * q) as usize].as_secs_f64() * 1e3;
    eprintln!(
        "anzan {name}: p50 {:.2} ms, p95 {:.2} ms, max {:.2} ms, {:.0}% of the window drawn (n={}, at 2×)",
        p(0.5),
        p(0.95),
        p(1.0),
        area * 100.0,
        v.len()
    );
    p(0.95)
}

#[test]
#[ignore]
fn a_bead_slide_and_a_still_screen() {
    let dir = std::path::Path::new(env!("CARGO_TARGET_TMPDIR")).join("anzan-budget");
    let _ = std::fs::remove_dir_all(&dir);
    let store = Store::at(&dir);
    let kid = store
        .add(NewProfile {
            name: "Hana".into(),
            animal: Animal::Fox,
            color: KidColor::Coral,
            grade: Grade::Kindergarten,
        })
        .unwrap();
    let mut p = sekka_anzan::progress::Progress::default();
    p.intros.insert(LESSONS[0].id.to_owned());
    store.save_progress(&kid.id, sekka_anzan::APP_ID, &p).unwrap();
    let mut app = Anzan::new(store, true);
    let t = app.manual_clock(clock::now_ms());
    let mut h = Harness::with(app, Appearance::default(), false);
    let w = h.window();
    h.named(w, NamedKey::Enter, Modifiers::NONE);
    let fit = Fit::of_size(h.size(w));
    let at = |p: Point| Point::new(fit.origin.x + p.x * fit.scale, fit.origin.y + p.y * fit.scale);
    // Free play: beads that slide and nothing else that changes.
    h.type_text(w, "f");
    let _ = draw(&mut h, w);

    let (mut slide, mut still) = (Vec::new(), Vec::new());
    let (mut slide_area, mut still_area) = (0.0f64, 0.0f64);
    for round in 0..40 {
        // Up, then home again: a bead's whole slide, frame by frame.
        let bead = scene::frame(3).bead_center(0, Bead::Earth(0), if round % 2 == 0 { 0.0 } else { 1.0 });
        h.click(w, at(bead));
        for _ in 0..10 {
            let start = Instant::now();
            t.set(t.get() + 20);
            h.advance(Duration::from_millis(20));
            let area = draw(&mut h, w);
            slide.push(start.elapsed());
            slide_area = slide_area.max(area);
        }
        // Pon says the number after a moment: let him.
        t.set(t.get() + 700);
        h.advance(Duration::from_millis(700));
        let _ = draw(&mut h, w);
        // Still: a tick with nothing moving draws nothing.
        let start = Instant::now();
        t.set(t.get() + 20);
        h.advance(Duration::from_millis(20));
        still_area = still_area.max(draw(&mut h, w));
        still.push(start.elapsed());
    }
    let p95 = report("bead slide", slide, slide_area);
    let still_p50 = {
        let mut v = still.clone();
        v.sort();
        v[v.len() / 2].as_secs_f64() * 1e3
    };
    report("still", still, still_area);
    // What a new question costs: the whole window.
    let size = h.size(w);
    let whole: Vec<Duration> = (0..20)
        .map(|_| {
            let start = Instant::now();
            let _ = h.render_rect(w, Rect::new(0.0, 0.0, size.width, size.height), 2.0);
            start.elapsed()
        })
        .collect();
    report("whole window", whole, 1.0);
    assert!(p95 < 16.0, "a slide's frames should fit a 60 Hz frame at least");
    assert!(still_p50 < 0.5, "a still screen draws nothing (but a blink)");
}

/// Where a whole frame's time goes, region by region (for tuning).
#[test]
#[ignore]
fn where_a_whole_frame_goes() {
    let dir = std::path::Path::new(env!("CARGO_TARGET_TMPDIR")).join("anzan-regions");
    let _ = std::fs::remove_dir_all(&dir);
    let store = Store::at(&dir);
    store
        .add(NewProfile {
            name: "Hana".into(),
            animal: Animal::Fox,
            color: KidColor::Coral,
            grade: Grade::Kindergarten,
        })
        .unwrap();
    let mut h = Harness::with(Anzan::new(store, true), Appearance::default(), false);
    let w = h.window();
    for (screen, setup) in [("who", ""), ("map", "\n"), ("free", "f")] {
        if !setup.is_empty() {
            h.type_text(w, setup);
        }
        let _ = h.take_damage(w);
        for (name, r) in [
            ("whole", Rect::new(0.0, 0.0, 1000.0, 700.0)),
            ("empty sky", Rect::new(560.0, 80.0, 700.0, 110.0)),
            ("top band", Rect::new(0.0, 0.0, 1000.0, 110.0)),
            ("middle", Rect::new(0.0, 280.0, 1000.0, 500.0)),
            ("bottom", Rect::new(0.0, 560.0, 1000.0, 700.0)),
        ] {
            let mut v: Vec<Duration> = (0..12)
                .map(|_| {
                    let s = Instant::now();
                    let _ = h.render_rect(w, r, 2.0);
                    s.elapsed()
                })
                .collect();
            v.sort();
            eprintln!("{screen} {name}: p50 {:.2} ms", v[6].as_secs_f64() * 1e3);
        }
    }
}
