//! The animals on kids' cards, drawn from circles and curves: no pictures,
//! so they're crisp at any size and in any theme.

use std::f64::consts::PI;

use aizome_kit::layout::{Point, Vec2};
use aizome_kit::sidestep_ui::kurbo::{Affine, BezPath, Circle, Ellipse, Shape};
use aizome_kit::sidestep_ui::{Canvas, Color, LineCap, StrokeStyle};
use sekka_kid::{Animal, KidColor};

/// What the animal's doing.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Mood {
    #[default]
    Happy,
    /// Eyes closed for a blink.
    Blink,
    /// Delighted: eyes in smiling arcs, mouth open.
    Cheer,
    /// Asleep.
    Sleep,
}

/// The kid's colour.
pub fn kid_color(c: KidColor) -> Color {
    Color::hex(c.rgb())
}

struct Fur {
    head: Color,
    muzzle: Color,
    ear_inner: Color,
    ink: Color,
}

fn fur(animal: Animal) -> Fur {
    let ink = Color::hex(0x2e211b);
    match animal {
        Animal::Cat => {
            Fur { head: Color::hex(0xf0b06a), muzzle: Color::hex(0xfde8cf), ear_inner: Color::hex(0xf59a9a), ink }
        }
        Animal::Fox => {
            Fur { head: Color::hex(0xf07a33), muzzle: Color::hex(0xfff3e4), ear_inner: Color::hex(0x5a3020), ink }
        }
        Animal::Rabbit => {
            Fur { head: Color::hex(0xf6efe9), muzzle: Color::hex(0xffffff), ear_inner: Color::hex(0xf7a8b8), ink }
        }
        Animal::Bear => {
            Fur { head: Color::hex(0xa8754a), muzzle: Color::hex(0xe9c9a3), ear_inner: Color::hex(0x7d5434), ink }
        }
        Animal::Frog => {
            Fur { head: Color::hex(0x7cc56b), muzzle: Color::hex(0xc9eab0), ear_inner: Color::hex(0x5ea84f), ink }
        }
        Animal::Panda => {
            Fur { head: Color::hex(0xfbfbf8), muzzle: Color::hex(0xffffff), ear_inner: Color::hex(0x2b2b30), ink }
        }
        Animal::Owl => {
            Fur { head: Color::hex(0x9b7653), muzzle: Color::hex(0xf3e2c4), ear_inner: Color::hex(0x6e5036), ink }
        }
        Animal::Tanuki => {
            Fur { head: Color::hex(0xb08a63), muzzle: Color::hex(0xf2e2c8), ear_inner: Color::hex(0x4a362a), ink }
        }
    }
}

fn circle(c: &mut Canvas, at: Point, r: f64, color: Color) {
    c.fill(&Circle::new(at, r), color);
}

fn ellipse(c: &mut Canvas, at: Point, rx: f64, ry: f64, angle: f64, color: Color) {
    c.fill(&Ellipse::new(at, (rx, ry), angle), color);
}

/// A rounded triangle: an ear.
fn ear(c: &mut Canvas, base: Point, tip: Point, width: f64, color: Color) {
    let d = tip - base;
    let n = Vec2::new(-d.y, d.x).normalize() * (width / 2.0);
    let mut p = BezPath::new();
    p.move_to(base + n);
    p.quad_to(base + n * 0.6 + d * 0.9, tip);
    p.quad_to(base - n * 0.6 + d * 0.9, base - n);
    p.close_path();
    c.fill(&p, color);
}

/// Draw `animal`'s head, `r` its radius, centred on `at`, on a disc of the
/// kid's colour when `badge` is given.
pub fn paint(c: &mut Canvas, at: Point, r: f64, animal: Animal, badge: Option<KidColor>, mood: Mood) {
    if let Some(k) = badge {
        let col = kid_color(k);
        circle(c, at, r * 1.32, col.mix(Color::WHITE, 0.55));
        c.stroke(&Circle::new(at, r * 1.32), &StrokeStyle::new((r * 0.08).max(1.5)), col);
    }
    let f = fur(animal);
    let x = at.x;
    let y = at.y;

    // Ears and what stands above the head.
    match animal {
        Animal::Cat => {
            for s in [-1.0, 1.0] {
                ear(
                    c,
                    Point::new(x + s * r * 0.55, y - r * 0.55),
                    Point::new(x + s * r * 0.82, y - r * 1.18),
                    r * 0.62,
                    f.head,
                );
                ear(
                    c,
                    Point::new(x + s * r * 0.57, y - r * 0.6),
                    Point::new(x + s * r * 0.78, y - r * 1.0),
                    r * 0.32,
                    f.ear_inner,
                );
            }
        }
        Animal::Fox => {
            for s in [-1.0, 1.0] {
                ear(
                    c,
                    Point::new(x + s * r * 0.5, y - r * 0.5),
                    Point::new(x + s * r * 0.9, y - r * 1.32),
                    r * 0.7,
                    f.head,
                );
                ear(
                    c,
                    Point::new(x + s * r * 0.55, y - r * 0.62),
                    Point::new(x + s * r * 0.84, y - r * 1.15),
                    r * 0.34,
                    f.ear_inner,
                );
            }
        }
        Animal::Rabbit => {
            for s in [-1.0, 1.0] {
                let e = Point::new(x + s * r * 0.36, y - r * 1.2);
                ellipse(c, e, r * 0.24, r * 0.68, s * 0.12, f.head);
                ellipse(c, e + Vec2::new(0.0, r * 0.06), r * 0.12, r * 0.5, s * 0.12, f.ear_inner);
            }
        }
        Animal::Bear | Animal::Panda | Animal::Tanuki => {
            for s in [-1.0, 1.0] {
                let e = Point::new(x + s * r * 0.7, y - r * 0.68);
                circle(c, e, r * 0.32, if animal == Animal::Panda { f.ear_inner } else { f.head });
                if animal != Animal::Panda {
                    circle(c, e, r * 0.17, f.ear_inner);
                }
            }
        }
        Animal::Frog => {
            for s in [-1.0, 1.0] {
                circle(c, Point::new(x + s * r * 0.45, y - r * 0.72), r * 0.36, f.head);
            }
        }
        Animal::Owl => {
            for s in [-1.0, 1.0] {
                ear(
                    c,
                    Point::new(x + s * r * 0.5, y - r * 0.62),
                    Point::new(x + s * r * 0.85, y - r * 1.12),
                    r * 0.42,
                    f.head,
                );
            }
        }
    }

    // The head.
    let head = Ellipse::new(at, (r, r * 0.92), 0.0);
    c.fill(&head, f.head);
    c.stroke(&head, &StrokeStyle::new((r * 0.035).max(1.0)), f.head.mix(Color::BLACK, 0.18));

    // Markings.
    match animal {
        Animal::Fox => {
            for s in [-1.0, 1.0] {
                ellipse(c, Point::new(x + s * r * 0.42, y + r * 0.3), r * 0.48, r * 0.4, s * -0.4, f.muzzle);
            }
        }
        Animal::Panda => {
            for s in [-1.0, 1.0] {
                ellipse(c, Point::new(x + s * r * 0.36, y - r * 0.02), r * 0.24, r * 0.3, s * 0.5, f.ear_inner);
            }
        }
        Animal::Tanuki => {
            // The mask.
            let mut p = BezPath::new();
            p.move_to(Point::new(x - r * 0.82, y - r * 0.1));
            p.quad_to(Point::new(x - r * 0.4, y - r * 0.38), Point::new(x, y - r * 0.08));
            p.quad_to(Point::new(x + r * 0.4, y - r * 0.38), Point::new(x + r * 0.82, y - r * 0.1));
            p.quad_to(Point::new(x + r * 0.6, y + r * 0.32), Point::new(x, y + r * 0.12));
            p.quad_to(Point::new(x - r * 0.6, y + r * 0.32), Point::new(x - r * 0.82, y - r * 0.1));
            c.fill(&p, f.ear_inner);
        }
        Animal::Owl => {
            for s in [-1.0, 1.0] {
                circle(c, Point::new(x + s * r * 0.38, y - r * 0.05), r * 0.36, f.muzzle);
            }
        }
        _ => {}
    }

    // The muzzle.
    let muzzle_at = Point::new(x, y + r * 0.36);
    match animal {
        Animal::Frog | Animal::Owl => {}
        Animal::Fox => {}
        _ => ellipse(c, muzzle_at, r * 0.42, r * 0.3, 0.0, f.muzzle),
    }

    // Eyes.
    let (eye_dx, eye_y, eye_r) = match animal {
        Animal::Frog => (r * 0.45, y - r * 0.72, r * 0.16),
        Animal::Owl => (r * 0.38, y - r * 0.05, r * 0.17),
        _ => (r * 0.36, y - r * 0.04, r * 0.12),
    };
    let eye_ink = if animal == Animal::Panda { Color::WHITE } else { f.ink };
    for s in [-1.0, 1.0] {
        let e = Point::new(x + s * eye_dx, eye_y);
        if matches!(animal, Animal::Frog) && !matches!(mood, Mood::Blink | Mood::Sleep) {
            circle(c, e, eye_r * 1.45, Color::WHITE);
        }
        match mood {
            Mood::Happy => {
                circle(c, e, eye_r, eye_ink);
                circle(c, e + Vec2::new(-eye_r * 0.32, -eye_r * 0.36), eye_r * 0.36, Color::WHITE);
            }
            Mood::Cheer => arc(c, e, eye_r * 1.05, PI, 2.0 * PI, eye_ink, eye_r * 0.5),
            Mood::Blink | Mood::Sleep => arc(c, e, eye_r * 1.05, 0.15 * PI, 0.85 * PI, eye_ink, eye_r * 0.45),
        }
    }

    // Nose and mouth.
    let ink = f.ink;
    let stroke = (r * 0.06).max(1.2);
    match animal {
        Animal::Owl => {
            let mut beak = BezPath::new();
            beak.move_to(Point::new(x - r * 0.12, y + r * 0.2));
            beak.line_to(Point::new(x + r * 0.12, y + r * 0.2));
            beak.line_to(Point::new(x, y + r * 0.42));
            beak.close_path();
            c.fill(&beak, Color::hex(0xf2a93b));
        }
        Animal::Frog => {
            let (a, b) = if mood == Mood::Cheer { (0.08, 0.92) } else { (0.15, 0.85) };
            arc(c, Point::new(x, y + r * 0.02), r * 0.5, a * PI, b * PI, ink, stroke);
        }
        _ => {
            ellipse(c, Point::new(x, y + r * 0.22), r * 0.11, r * 0.08, 0.0, ink);
            if mood == Mood::Cheer {
                let mut m = BezPath::new();
                m.move_to(Point::new(x - r * 0.16, y + r * 0.36));
                m.quad_to(Point::new(x, y + r * 0.66), Point::new(x + r * 0.16, y + r * 0.36));
                m.close_path();
                c.fill(&m, Color::hex(0xb8463a));
            } else if mood != Mood::Sleep {
                arc(c, Point::new(x - r * 0.09, y + r * 0.3), r * 0.09, 0.0, PI, ink, stroke);
                arc(c, Point::new(x + r * 0.09, y + r * 0.3), r * 0.09, 0.0, PI, ink, stroke);
            }
        }
    }

    // Cheeks.
    if mood != Mood::Sleep {
        let blush = Color::rgba(0.96, 0.42, 0.45, 0.42);
        for s in [-1.0, 1.0] {
            ellipse(c, Point::new(x + s * r * 0.6, y + r * 0.26), r * 0.14, r * 0.09, 0.0, blush);
        }
    }
}

/// An arc of a circle at `at`, from angle `a0` to `a1` (radians, clockwise
/// from three o'clock, y down), stroked round.
pub fn arc(c: &mut Canvas, at: Point, r: f64, a0: f64, a1: f64, color: Color, width: f64) {
    let arc = aizome_kit::sidestep_ui::kurbo::Arc::new(at, (r, r), a0, a1 - a0, 0.0);
    let path: BezPath = arc.path_elements(0.1).collect();
    c.stroke(&path, &StrokeStyle::new(width).with_cap(LineCap::Round), color);
}

/// `paint`, turned by `angle` about `at` (a head tilting).
pub fn paint_tilted(
    c: &mut Canvas,
    at: Point,
    r: f64,
    animal: Animal,
    badge: Option<KidColor>,
    mood: Mood,
    angle: f64,
) {
    c.saved(|c| {
        c.concat(Affine::rotate_about(angle, at));
        paint(c, at, r, animal, badge, mood);
    });
}
