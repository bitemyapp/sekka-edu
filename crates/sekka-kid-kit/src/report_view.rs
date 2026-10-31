//! A kid's report, for grown-ups, in Aizome's own look: the week at a
//! glance, minutes a day by app, how each skill is coming along, and the
//! latest rounds and milestones.

use aizome_kit::layout::{Point, Rect};
use aizome_kit::prelude::*;
use aizome_kit::sidestep_ui::kurbo::RoundedRect;
use aizome_kit::sidestep_ui::{Canvas, Color, StrokeStyle, TextStyle};
use aizome_kit::theme::FontSpec;
use aizome_kit::{column, row};
use sekka_kid::clock;
use sekka_kid::report::{AppSummary, DayPlay, Report};

use crate::avatar::{self, Mood};

/// A skill's id as a grown-up would say it.
pub fn skill_name(id: &str) -> String {
    match id {
        "numbers-to-4" => "Numbers to 4".into(),
        "numbers-to-9" => "Numbers to 9".into(),
        "numbers-to-99" => "Numbers to 99".into(),
        "reading-beads" => "Reading the beads".into(),
        "picturing-beads" => "Picturing the beads".into(),
        "adding-to-9" => "Adding within 9".into(),
        "taking-from-9" => "Taking away within 9".into(),
        "adding-to-18" => "Adding within 18".into(),
        "taking-from-18" => "Taking away within 18".into(),
        "little-friends" => "Little friends (pairs that make 5)".into(),
        "big-friends" => "Big friends (pairs that make 10)".into(),
        "place-value" => "Tens and ones".into(),
        "mental-abacus" => "Mental abacus".into(),
        other => {
            let s = other.replace('-', " ");
            let mut c = s.chars();
            c.next().map(|f| f.to_uppercase().collect::<String>() + c.as_str()).unwrap_or_default()
        }
    }
}

/// Minutes as "1 h 5 min", "12 min", "under a minute"; from ten hours,
/// whole hours ("12 h").
pub fn minutes(m: f64) -> String {
    let m = m.round() as i64;
    match m {
        0 => "under a minute".into(),
        1..=59 => format!("{m} min"),
        600.. => format!("{} h", (m + 30) / 60),
        _ if m % 60 == 0 => format!("{} h", m / 60),
        _ => format!("{} h {} min", m / 60, m % 60),
    }
}

fn percent(v: Option<f64>) -> String {
    v.map(|v| format!("{:.0}%", v * 100.0)).unwrap_or_else(|| "—".into())
}

/// "4 pm".
pub fn hour(h: u32) -> String {
    match h {
        0 => "midnight".into(),
        12 => "noon".into(),
        1..=11 => format!("{h} am"),
        _ => format!("{} pm", h - 12),
    }
}

fn rounds(n: u32) -> String {
    if n == 1 { "1 round".into() } else { format!("{n} rounds") }
}

/// An app's line in the legend: its time, and its rounds, stars and first
/// tries where it has any ("9 min" alone for an app played without rounds).
fn app_line(a: &AppSummary) -> String {
    let mut parts = vec![minutes(a.minutes)];
    if a.rounds > 0 {
        parts.push(rounds(a.rounds));
    }
    if a.stars > 0 {
        parts.push(format!("{} ★", a.stars));
    }
    if a.items > 0 {
        parts.push(format!("{} right first time", percent(Some(f64::from(a.first_try) / f64::from(a.items)))));
    }
    parts.join(" · ")
}

fn stars(n: u8) -> String {
    (0..3).map(|i| if i < n { '★' } else { '☆' }).collect()
}

fn tile<M: Send + 'static>(value: String, label: &str) -> Element<M> {
    container(
        column![text(value).size(22.0).bold().tabular(), text(label.to_owned()).role(Role::Text3).size(12.0)]
            .spacing(2.0),
    )
    .padding([10.0, 14.0])
    .fill(Tone::Raised)
    .border(Edge::Line)
    .into()
}

/// Colours for apps in the chart, in order.
fn app_color(i: usize) -> Color {
    [0xe8553d, 0x5d8fd6, 0x43a85f, 0xf2b33d, 0xb05cc4, 0x45a5e8].map(Color::hex)[i % 6]
}

#[derive(Clone, Debug, PartialEq, Hash)]
struct Chart {
    /// Each day, and its minutes by app (in `apps` order), in tenths.
    days: Vec<(clock::Day, Vec<u32>)>,
    limit: Option<u32>,
}

/// The days a report covers, as a caption says them: "25–31 Oct",
/// "18 Oct – 6 Nov".
fn span(days: &[DayPlay]) -> String {
    let (Some(a), Some(b)) = (days.first().map(|d| d.day), days.last().map(|d| d.day)) else {
        return String::new();
    };
    if (a.year, a.month) == (b.year, b.month) {
        format!("{}–{}", a.day, b.short())
    } else {
        format!("{} – {}", a.short(), b.short())
    }
}

/// Minutes a day, stacked by app, with the day's allowance as a line.
/// Weekends are shaded. Days are labelled "Sun 25" where there's room;
/// otherwise by the date over the weekday's initial; and in a narrow
/// window, every other date (or fewer), always ending on today.
fn chart<M: Send + 'static>(days: &[DayPlay], apps: &[String], limit: Option<u32>) -> Element<M> {
    let state = Chart {
        days: days
            .iter()
            .map(|d| {
                (d.day, apps.iter().map(|a| (d.minutes.get(a).copied().unwrap_or(0.0) * 10.0).round() as u32).collect())
            })
            .collect(),
        limit,
    };
    canvas(state, |c, b, s, theme| {
        let label = theme.style(FontSpec::ui(11.0), theme.role(Role::Text2));
        let initial = theme.style(FontSpec::ui(10.0), theme.role(Role::Text3));
        let full = |d: &clock::Day| format!("{} {}", d.weekday_name(), d.day);
        let n = s.days.len().max(1);
        let slot = (b.width() - 42.0) / n as f64;
        let widest = s.days.iter().map(|(d, _)| width(&full(d), &label)).fold(0.0, f64::max);
        let whole = widest + 8.0 <= slot;
        // Every `every`th date, counted back from today, when even dates
        // alone would touch.
        let every = ((width("28", &label) + 6.0) / slot).ceil().max(1.0) as usize;
        let plot = Rect::new(b.x0 + 34.0, b.y0 + 8.0, b.x1 - 8.0, b.y1 - if whole || every > 1 { 22.0 } else { 34.0 });
        let slot = plot.width() / n as f64;
        for (i, (d, _)) in s.days.iter().enumerate() {
            if d.weekday == 0 || d.weekday == 6 {
                let x = plot.x0 + slot * i as f64;
                c.fill_rect(Rect::new(x.round(), plot.y0, (x + slot).round(), plot.y1), theme.sunk());
            }
        }
        let most = s.days.iter().map(|(_, v)| v.iter().sum::<u32>()).max().unwrap_or(0) as f64 / 10.0;
        let top = most.max(f64::from(s.limit.unwrap_or(0))).max(10.0);
        let step = if top > 60.0 {
            30.0
        } else if top > 20.0 {
            10.0
        } else {
            5.0
        };
        let top = (top / step).ceil() * step;
        let y = |m: f64| plot.y1 - plot.height() * m / top;
        let small = theme.style(FontSpec::mono(10.0), theme.role(Role::Text3));
        let mut m = 0.0;
        while m <= top {
            c.fill_rect(Rect::new(plot.x0, y(m).round(), plot.x1, y(m).round() + 1.0), theme.line());
            c.draw_label(&format!("{m:.0}"), &small, Point::new(b.x0, y(m) - 7.0));
            m += step;
        }
        for (i, (d, mins)) in s.days.iter().enumerate() {
            let x = plot.x0 + slot * i as f64 + slot * 0.2;
            let w = slot * 0.6;
            let mut base = 0.0;
            for (k, tenths) in mins.iter().enumerate() {
                let v = f64::from(*tenths) / 10.0;
                if v <= 0.0 {
                    continue;
                }
                let r = Rect::new(x, y(base + v), x + w, y(base));
                c.fill(&RoundedRect::from_rect(r, 2.0), app_color(k));
                base += v;
            }
            let mid = x + w / 2.0;
            if whole {
                centred(c, mid, &full(d), &label, plot.y1 + 5.0);
            } else if (n - 1 - i) % every == 0 {
                centred(c, mid, &d.day.to_string(), &label, plot.y1 + 5.0);
                if every == 1 {
                    centred(c, mid, &d.weekday_name()[..1], &initial, plot.y1 + 18.0);
                }
            }
        }
        if let Some(l) = s.limit {
            let ly = y(f64::from(l)).round();
            let dash = StrokeStyle::new(1.5).with_dash(vec![5.0, 4.0], 0.0);
            c.stroke(
                &aizome_kit::sidestep_ui::kurbo::Line::new((plot.x0, ly), (plot.x1, ly)),
                &dash,
                theme.role(Role::Warning),
            );
        }
    })
    .height(180.0)
    .into()
}

fn width(t: &str, style: &TextStyle) -> f64 {
    aizome_kit::sidestep_ui::measure(t, style).width
}

/// `t` centred on `mid`, its top at `top`.
fn centred(c: &mut Canvas, mid: f64, t: &str, style: &TextStyle, top: f64) {
    c.draw_label(t, style, Point::new(mid - width(t, style) / 2.0, top));
}

/// The report.
pub fn view<M: Clone + Send + 'static>(r: &Report) -> Element<M> {
    let kid = &r.kid;
    let face = canvas((kid.animal, kid.color), |c, b, (animal, color), _| {
        avatar::paint(c, b.center(), b.height() * 0.34, *animal, Some(*color), Mood::Happy);
    })
    .width(72.0)
    .height(72.0);
    let since = clock::day_of(kid.created);
    let today = match (r.minutes_left, kid.daily_minutes) {
        (Some(left), Some(limit)) => {
            format!("Today: {} of {} ({} left)", minutes(r.today_minutes), minutes(f64::from(limit)), minutes(left))
        }
        _ => format!("Today: {}", minutes(r.today_minutes)),
    };
    let header = row![
        face,
        column![
            text(kid.name.clone()).size(26.0).bold(),
            text(format!("{} · playing since {} {}", kid.grade.name(), since.short(), since.year)).role(Role::Text2),
            text(today).role(Role::Text3),
        ]
        .spacing(3.0),
    ]
    .spacing(16.0);

    let t = &r.totals;
    let days = r.days.len();
    let tiles = crate::tiles::tiles(
        vec![
            tile(minutes(t.minutes), &format!("played in {days} days")),
            tile(format!("{} of {days}", t.days_played), "days played"),
            tile(format!("{}", t.sittings), "sittings"),
            tile(format!("{} of {}", t.finished, t.rounds), "rounds finished"),
            tile(percent(t.accuracy()), "right first time"),
        ],
        125.0,
        10.0,
    );
    let mut more = Vec::new();
    if let Some(ms) = t.median_response_ms {
        more.push(format!("Answers in about {:.1} s", f64::from(ms) / 1000.0));
    }
    if t.rounds > 0 {
        more.push(format!("{:.1} hints a round", f64::from(t.hints) / f64::from(t.rounds)));
    }
    if let Some(h) = t.usual_hour {
        more.push(format!("Usually plays around {}", hour(h)));
    }

    // Apps, most played first, coloured as in the chart.
    let order: Vec<String> = r.apps.iter().map(|a| a.app.clone()).collect();
    let legend: Vec<Element<M>> = r
        .apps
        .iter()
        .enumerate()
        .map(|(i, a)| {
            let swatch =
                canvas(i, |c, b, i, _| c.fill(&RoundedRect::from_rect(b.inflate(-2.0, -2.0), 2.0), app_color(*i)))
                    .width(12.0)
                    .height(12.0);
            row![swatch, text(a.name.clone()).semibold(), text(app_line(a)).role(Role::Text2),].spacing(8.0).into()
        })
        .collect();

    let skills: Vec<Element<M>> = r
        .skills
        .iter()
        .map(|s| {
            let trend = match (s.recent, s.earlier) {
                (Some(a), Some(b)) if a > b + 0.1 => ("coming along", Role::Success),
                (Some(a), Some(b)) if a + 0.1 < b => ("finding it harder lately", Role::Warning),
                (Some(_), Some(_)) => ("steady", Role::Text2),
                _ => ("new", Role::Text3),
            };
            let accuracy = (s.items > 0).then(|| f64::from(s.first_try) / f64::from(s.items));
            form_row(
                skill_name(&s.skill),
                row![
                    text(format!("{} · {}", rounds(s.rounds), percent(accuracy))).role(Role::Text2).tabular(),
                    text(trend.0).role(trend.1),
                ]
                .spacing(10.0),
            )
            .into()
        })
        .collect();

    let recent: Vec<Element<M>> = r
        .recent
        .iter()
        .map(|round| {
            let d = clock::day_of(round.ended);
            let when = format!("{} {} · {}", d.weekday_name(), d.day, clock::time_of(round.ended));
            let how = if round.finished {
                format!("{} of {} first time {}", round.first_try, round.items, stars(round.stars))
            } else {
                format!("left after {} questions", round.items)
            };
            row![
                text(when).role(Role::Text3).mono().size(12.0).width(120.0),
                text(round.title.clone()).semibold().width(170.0),
                text(how).role(if round.finished { Role::Text } else { Role::Text3 }),
            ]
            .spacing(10.0)
            .into()
        })
        .collect();

    let milestones: Vec<Element<M>> = r
        .milestones
        .iter()
        .map(|(at, app, what)| {
            let d = clock::day_of(*at);
            row![
                text(format!("{} {}", d.weekday_name(), d.short())).role(Role::Text3).mono().size(12.0).width(120.0),
                text(what.clone()),
                text(format!("· {app}")).role(Role::Text3),
            ]
            .spacing(10.0)
            .into()
        })
        .collect();

    let mut page: Vec<Element<M>> = vec![header.into(), tiles];
    if !more.is_empty() {
        page.push(text(more.join(" · ")).role(Role::Text2).wrap().into());
    }
    page.push(caption(format!("Minutes a day · {}", span(&r.days))));
    page.push(chart(&r.days, &order, kid.daily_minutes));
    page.push(aizome_kit::widgets::layout::column(legend).spacing(6.0).into());
    if !skills.is_empty() {
        page.push(form_section("Skills", skills));
    }
    if !milestones.is_empty() {
        page.push(caption("Milestones"));
        page.push(aizome_kit::widgets::layout::column(milestones).spacing(6.0).into());
    }
    page.push(caption("Latest rounds"));
    if recent.is_empty() {
        page.push(text("Nothing played yet.").role(Role::Text3).into());
    } else {
        page.push(aizome_kit::widgets::layout::column(recent).spacing(6.0).into());
    }
    scroll(aizome_kit::widgets::layout::column(page).spacing(14.0).padding(24.0)).into()
}
