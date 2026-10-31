//! What a grown-up sees: a kid's play over the last days, across apps.

use std::collections::BTreeMap;

use crate::activity::{Event, Round};
use crate::clock::{self, Day};
use crate::profile::Profile;
use crate::store::Store;

/// One day's play.
#[derive(Clone, Debug, PartialEq)]
pub struct DayPlay {
    pub day: Day,
    /// When it began, in Unix milliseconds.
    pub start: i64,
    /// Minutes in each app, by app id.
    pub minutes: BTreeMap<String, f64>,
}

impl DayPlay {
    pub fn total(&self) -> f64 {
        self.minutes.values().sum()
    }
}

/// One app's share.
#[derive(Clone, Debug, PartialEq)]
pub struct AppSummary {
    pub app: String,
    pub name: String,
    pub minutes: f64,
    pub rounds: u32,
    pub finished: u32,
    pub items: u32,
    pub first_try: u32,
    pub stars: u32,
    pub last_played: Option<i64>,
}

/// How a skill is going, across the apps that practise it.
#[derive(Clone, Debug, PartialEq)]
pub struct SkillSummary {
    pub skill: String,
    pub rounds: u32,
    pub items: u32,
    pub first_try: u32,
    /// First-try share of the latest rounds (half, up to three), against the
    /// rounds before: whether it's coming along.
    pub recent: Option<f64>,
    pub earlier: Option<f64>,
    pub last: i64,
}

/// Engagement over the whole period.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Totals {
    pub minutes: f64,
    pub days_played: u32,
    /// Separate sittings: stretches of play more than ten minutes apart.
    pub sittings: u32,
    pub rounds: u32,
    pub finished: u32,
    pub items: u32,
    pub first_try: u32,
    pub hints: u32,
    pub stars: u32,
    pub median_response_ms: Option<u32>,
    /// The hour of the day most played in (0 to 23).
    pub usual_hour: Option<u32>,
}

impl Totals {
    /// Answered right the first time, of everything asked.
    pub fn accuracy(&self) -> Option<f64> {
        (self.items > 0).then(|| f64::from(self.first_try) / f64::from(self.items))
    }

    /// Rounds played to the end, of rounds started.
    pub fn completion(&self) -> Option<f64> {
        (self.rounds > 0).then(|| f64::from(self.finished) / f64::from(self.rounds))
    }
}

/// A kid's report.
#[derive(Clone, Debug, PartialEq)]
pub struct Report {
    pub kid: Profile,
    /// The days of the period, oldest first, today last.
    pub days: Vec<DayPlay>,
    pub apps: Vec<AppSummary>,
    pub skills: Vec<SkillSummary>,
    pub totals: Totals,
    /// The latest rounds, newest first.
    pub recent: Vec<Round>,
    /// Milestones, newest first: (when, app name, what).
    pub milestones: Vec<(i64, String, String)>,
    /// Minutes played today, and what's left of the day's allowance.
    pub today_minutes: f64,
    pub minutes_left: Option<f64>,
}

/// Gaps longer than this start a new sitting.
const SITTING_GAP_MS: i64 = 10 * 60_000;

/// App `id`'s summary, begun if it's the first seen.
fn summary<'a>(apps: &'a mut BTreeMap<String, AppSummary>, store: &Store, id: &str) -> &'a mut AppSummary {
    apps.entry(id.to_owned()).or_insert_with(|| AppSummary {
        app: id.to_owned(),
        name: store.app_name(id),
        minutes: 0.0,
        rounds: 0,
        finished: 0,
        items: 0,
        first_try: 0,
        stars: 0,
        last_played: None,
    })
}

/// `kid`'s report over the `days` days to `now`, today included.
pub fn report(store: &Store, kid: &Profile, now: i64, days: usize) -> Report {
    let events = store.events(&kid.id);
    let days = days.max(1);

    // The days, from the calendar (they aren't all 24 hours long).
    let mut starts = vec![clock::day_start(now)];
    while starts.len() < days {
        let earliest = starts[starts.len() - 1];
        starts.push(clock::day_start(earliest - 3_600_000));
    }
    starts.reverse();
    let period_start = starts[0];
    let mut by_day: Vec<DayPlay> =
        starts.iter().map(|&s| DayPlay { day: clock::day_of(s), start: s, minutes: BTreeMap::new() }).collect();

    let mut apps: BTreeMap<String, AppSummary> = BTreeMap::new();
    let mut totals = Totals::default();
    let mut hours = [0.0f64; 24];
    let mut spans: Vec<(i64, i64)> = Vec::new();
    let mut rounds: Vec<&Round> = Vec::new();
    let mut milestones = Vec::new();

    for e in &events {
        match e {
            Event::Active { app: id, from, to } => {
                let (from, to) = ((*from).max(period_start), (*to).min(now));
                if to <= from {
                    continue;
                }
                spans.push((from, to));
                let a = summary(&mut apps, store, id);
                a.last_played = Some(a.last_played.map_or(to, |l| l.max(to)));
                // Split across days, and across hours for the usual hour.
                for (i, d) in by_day.iter_mut().enumerate() {
                    let end = starts.get(i + 1).copied().unwrap_or(i64::MAX);
                    let (a0, b0) = (from.max(d.start), to.min(end));
                    if b0 > a0 {
                        *d.minutes.entry(id.clone()).or_default() += (b0 - a0) as f64 / 60_000.0;
                    }
                }
                let mut t = from;
                while t < to {
                    let step = (to - t).min(15 * 60_000);
                    hours[clock::hour_of(t + step / 2) as usize] += step as f64;
                    t += step;
                }
                a.minutes += (to - from) as f64 / 60_000.0;
            }
            Event::Round(r) if r.ended >= period_start => {
                let a = summary(&mut apps, store, &r.app);
                a.rounds += 1;
                a.finished += u32::from(r.finished);
                a.items += r.items;
                a.first_try += r.first_try;
                a.stars += u32::from(r.stars);
                a.last_played = Some(a.last_played.map_or(r.ended, |l| l.max(r.ended)));
                rounds.push(r);
            }
            Event::Milestone { app: id, at, text } if *at >= period_start => {
                milestones.push((*at, store.app_name(id), text.clone()));
            }
            _ => {}
        }
    }

    // Engagement.
    totals.minutes = by_day.iter().map(DayPlay::total).sum();
    totals.days_played = by_day.iter().filter(|d| d.total() >= 0.5).count() as u32;
    spans.sort();
    let mut last_end: Option<i64> = None;
    for (a, b) in &spans {
        if last_end.is_none_or(|e| *a - e > SITTING_GAP_MS) {
            totals.sittings += 1;
        }
        last_end = Some(last_end.map_or(*b, |e| e.max(*b)));
    }
    for r in &rounds {
        totals.rounds += 1;
        totals.finished += u32::from(r.finished);
        totals.items += r.items;
        totals.first_try += r.first_try;
        totals.hints += r.hints;
        totals.stars += u32::from(r.stars);
    }
    let mut responses: Vec<u32> = rounds.iter().filter_map(|r| r.response_ms).collect();
    responses.sort_unstable();
    totals.median_response_ms = responses.get(responses.len() / 2).copied();
    totals.usual_hour =
        hours.iter().enumerate().filter(|(_, ms)| **ms > 0.0).max_by(|a, b| a.1.total_cmp(b.1)).map(|(h, _)| h as u32);

    // Skills, oldest rounds first.
    let mut skills: BTreeMap<String, Vec<&Round>> = BTreeMap::new();
    for r in &rounds {
        for s in &r.skills {
            skills.entry(s.clone()).or_default().push(r);
        }
    }
    let share = |rs: &[&Round]| {
        let (items, right) = rs.iter().fold((0u32, 0u32), |(i, f), r| (i + r.items, f + r.first_try));
        (items > 0).then(|| f64::from(right) / f64::from(items))
    };
    let skills = skills
        .into_iter()
        .map(|(skill, rs)| {
            // The latest half (up to three rounds) against the rounds before.
            let split = rs.len() - rs.len().div_ceil(2).min(3);
            SkillSummary {
                skill,
                rounds: rs.len() as u32,
                items: rs.iter().map(|r| r.items).sum(),
                first_try: rs.iter().map(|r| r.first_try).sum(),
                recent: share(&rs[split..]),
                earlier: share(&rs[..split]),
                last: rs.iter().map(|r| r.ended).max().unwrap_or(0),
            }
        })
        .collect();

    let mut apps: Vec<AppSummary> = apps.into_values().collect();
    apps.sort_by(|a, b| b.minutes.total_cmp(&a.minutes).then(a.name.cmp(&b.name)));
    let recent = rounds.iter().rev().take(20).map(|r| (*r).clone()).collect();
    milestones.reverse();
    milestones.truncate(20);

    let today_minutes = store.played_today(&kid.id, now) as f64 / 60_000.0;
    Report {
        kid: kid.clone(),
        days: by_day,
        apps,
        skills,
        totals,
        recent,
        milestones,
        today_minutes,
        minutes_left: kid.daily_minutes.map(|m| (f64::from(m) - today_minutes).max(0.0)),
    }
}
