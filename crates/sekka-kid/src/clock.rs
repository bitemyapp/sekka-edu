//! Time as kids and grown-ups count it: Unix milliseconds, and the local
//! days they fall on.

use std::time::{SystemTime, UNIX_EPOCH};

/// Now, in Unix milliseconds.
pub fn now_ms() -> i64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_millis() as i64).unwrap_or(0)
}

/// A local calendar day.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Day {
    pub year: i32,
    /// 1 to 12.
    pub month: u32,
    /// 1 to 31.
    pub day: u32,
    /// 0 (Sunday) to 6.
    pub weekday: u32,
}

const WEEKDAYS: [&str; 7] = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];
const MONTHS: [&str; 12] = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];

impl Day {
    /// "Mon".
    pub fn weekday_name(self) -> &'static str {
        WEEKDAYS[self.weekday as usize % 7]
    }

    /// "26 Oct".
    pub fn short(self) -> String {
        format!("{} {}", self.day, MONTHS[(self.month as usize + 11) % 12])
    }
}

fn local(ms: i64) -> libc::tm {
    let secs = ms.div_euclid(1000) as libc::time_t;
    // SAFETY: localtime_r writes the broken-down time into `tm`, which is
    // plain data, and reads only `secs`.
    unsafe {
        let mut tm: libc::tm = std::mem::zeroed();
        libc::localtime_r(&secs, &mut tm);
        tm
    }
}

/// The local day `ms` falls on.
pub fn day_of(ms: i64) -> Day {
    let tm = local(ms);
    Day { year: tm.tm_year + 1900, month: (tm.tm_mon + 1) as u32, day: tm.tm_mday as u32, weekday: tm.tm_wday as u32 }
}

/// The local hour (0 to 23) `ms` falls in.
pub fn hour_of(ms: i64) -> u32 {
    local(ms).tm_hour as u32
}

/// When the local day holding `ms` began, in Unix milliseconds.
pub fn day_start(ms: i64) -> i64 {
    let mut tm = local(ms);
    tm.tm_hour = 0;
    tm.tm_min = 0;
    tm.tm_sec = 0;
    tm.tm_isdst = -1;
    // SAFETY: mktime normalises and reads the plain-data `tm`.
    let secs = unsafe { libc::mktime(&mut tm) };
    secs * 1000
}

/// When the local day after the one holding `ms` begins. Days aren't all
/// 24 hours long (daylight saving), so this asks the calendar.
pub fn next_day_start(ms: i64) -> i64 {
    // 36 hours after midnight is the next day's midday, whatever the
    // clocks did in between.
    day_start(day_start(ms) + 36 * 3_600_000)
}

/// The local time of day `ms` falls at, as "16:05".
pub fn time_of(ms: i64) -> String {
    let tm = local(ms);
    format!("{:02}:{:02}", tm.tm_hour, tm.tm_min)
}
