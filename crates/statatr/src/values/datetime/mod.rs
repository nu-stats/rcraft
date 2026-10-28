//! Dates and times.
//!
//! Behaviour reference: `plan/stata/dates.md`, checked by
//! `cargo xtask corpus dates` against probes 033 to 035.
//!
//! Stata dates count from 01jan1960:
//!
//! | Unit | Counts | Format |
//! |---|---|---|
//! | datetime/c | milliseconds, 86,400 seconds a day | `%tc` |
//! | datetime/C | milliseconds, with leap seconds | `%tC` |
//! | date | days | `%td` |
//! | weekly | weeks, 52 a year | `%tw` |
//! | monthly, quarterly, half-yearly | periods | `%tm`, `%tq`, `%th` |
//! | yearly | the year itself | `%ty` |
//!
//! Every function takes and returns `f64` the way Stata expressions do:
//! missing or invalid input gives `.`.

mod leap;
mod parse;

pub use leap::{cofc_from_big, cofc_to_big};
pub use parse::{parse_clock, parse_date, parse_period, Period};

use crate::values::missing::{is_missing, SYSMISS};

/// 01jan0100, the first date Stata accepts.
pub const MIN_DAY: i64 = -679_350;
/// 31dec9999, the last.
pub const MAX_DAY: i64 = 2_936_549;
pub const MS_PER_DAY: f64 = 86_400_000.0;

/// Days from 01jan1970 to 01jan1960.
const EPOCH_SHIFT: i64 = 3653;

pub fn is_leap_year(y: i64) -> bool {
    (y % 4 == 0 && y % 100 != 0) || y % 400 == 0
}

pub fn days_in_month(y: i64, m: i64) -> i64 {
    match m {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if is_leap_year(y) => 29,
        2 => 28,
        _ => 0,
    }
}

/// Day number (01jan1960 = 0) of a proleptic Gregorian date. The caller
/// checks the date is valid.
pub fn day_number(y: i64, m: i64, d: i64) -> i64 {
    // Howard Hinnant's days_from_civil, shifted to 1960.
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let mp = (m + 9) % 12;
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468 + EPOCH_SHIFT
}

/// (year, month, day) of a day number.
pub fn civil(day: i64) -> (i64, i64, i64) {
    let z = day - EPOCH_SHIFT + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(m <= 2);
    (y, m, d)
}

/// The whole day a (possibly fractional) daily date falls on, if it is a
/// date Stata can show (0100 to 9999).
pub fn whole_day(d: f64) -> Option<i64> {
    if is_missing(d) || !d.is_finite() {
        return None;
    }
    let day = d.floor();
    (day >= MIN_DAY as f64 && day <= MAX_DAY as f64).then_some(day as i64)
}

fn exact_int(v: f64) -> Option<i64> {
    (!is_missing(v) && v.is_finite() && v.fract() == 0.0 && v.abs() < 1e15).then_some(v as i64)
}

fn exact_floor(v: f64) -> Option<i64> {
    (!is_missing(v) && v.is_finite() && v.abs() < 1e15).then_some(v.floor() as i64)
}

fn num(v: i64) -> f64 {
    v as f64
}

fn or_missing(v: Option<f64>) -> f64 {
    v.unwrap_or(SYSMISS)
}

/// `mdy(M, D, Y)`
pub fn mdy(m: f64, d: f64, y: f64) -> f64 {
    or_missing((|| {
        let (m, d, y) = (exact_int(m)?, exact_int(d)?, exact_int(y)?);
        if !(100..=9999).contains(&y) || !(1..=12).contains(&m) || d < 1 || d > days_in_month(y, m)
        {
            return None;
        }
        Some(num(day_number(y, m, d)))
    })())
}

/// `isleapyear(Y)`
pub fn isleapyear(y: f64) -> f64 {
    or_missing(
        exact_int(y)
            .filter(|y| (100..=9999).contains(y))
            .map(|y| f64::from(u8::from(is_leap_year(y)))),
    )
}

fn on_day(d: f64, f: impl Fn(i64, (i64, i64, i64)) -> i64) -> f64 {
    or_missing(whole_day(d).map(|day| num(f(day, civil(day)))))
}

pub fn year(d: f64) -> f64 {
    on_day(d, |_, (y, _, _)| y)
}
pub fn month(d: f64) -> f64 {
    on_day(d, |_, (_, m, _)| m)
}
pub fn day(d: f64) -> f64 {
    on_day(d, |_, (_, _, dd)| dd)
}
/// 0 is Sunday. 01jan1960 was a Friday. Stata also gives a day of the week
/// for 01jan10000, one day past the last valid date (probe 034).
pub fn dow(d: f64) -> f64 {
    if !is_missing(d) && d.floor() == (MAX_DAY + 1) as f64 {
        return num((MAX_DAY + 1 + 5).rem_euclid(7));
    }
    on_day(d, |day, _| (day + 5).rem_euclid(7))
}
pub fn doy(d: f64) -> f64 {
    on_day(d, |day, (y, _, _)| day - day_number(y, 1, 1) + 1)
}
/// Weeks run from 1 January in blocks of 7 days; week 52 takes the rest of
/// the year.
pub fn week(d: f64) -> f64 {
    on_day(d, |day, (y, _, _)| week_of(day, y))
}
fn week_of(day: i64, y: i64) -> i64 {
    ((day - day_number(y, 1, 1)) / 7 + 1).min(52)
}
pub fn quarter(d: f64) -> f64 {
    on_day(d, |_, (_, m, _)| (m - 1) / 3 + 1)
}
pub fn halfyear(d: f64) -> f64 {
    on_day(d, |_, (_, m, _)| (m - 1) / 6 + 1)
}
/// Outside 0100 to 9999 Stata returns arbitrary numbers here; StatCraft
/// returns `.` (see `plan/stata/dates.md`, "Known differences").
pub fn wofd(d: f64) -> f64 {
    on_day(d, |day, (y, _, _)| (y - 1960) * 52 + week_of(day, y) - 1)
}
pub fn mofd(d: f64) -> f64 {
    on_day(d, |_, (y, m, _)| (y - 1960) * 12 + m - 1)
}
pub fn qofd(d: f64) -> f64 {
    on_day(d, |_, (y, m, _)| (y - 1960) * 4 + (m - 1) / 3)
}
pub fn hofd(d: f64) -> f64 {
    on_day(d, |_, (y, m, _)| (y - 1960) * 2 + (m - 1) / 6)
}
pub fn yofd(d: f64) -> f64 {
    on_day(d, |_, (y, _, _)| y)
}

/// First day of a period: `per_year` periods a year, each `months` long
/// (weeks are handled separately).
fn first_day(period: f64, per_year: i64, months: i64) -> f64 {
    or_missing((|| {
        let p = exact_floor(period)?;
        let y = 1960 + p.div_euclid(per_year);
        let k = p.rem_euclid(per_year);
        if !(100..=9999).contains(&y) {
            return None;
        }
        Some(num(day_number(y, k * months + 1, 1)))
    })())
}

pub fn dofw(w: f64) -> f64 {
    or_missing((|| {
        let w = exact_floor(w)?;
        let y = 1960 + w.div_euclid(52);
        if !(100..=9999).contains(&y) {
            return None;
        }
        Some(num(day_number(y, 1, 1) + 7 * w.rem_euclid(52)))
    })())
}
pub fn dofm(m: f64) -> f64 {
    first_day(m, 12, 1)
}
pub fn dofq(q: f64) -> f64 {
    first_day(q, 4, 3)
}
pub fn dofh(h: f64) -> f64 {
    first_day(h, 2, 6)
}
pub fn dofy(y: f64) -> f64 {
    or_missing(
        exact_floor(y)
            .filter(|y| (100..=9999).contains(y))
            .map(|y| num(day_number(y, 1, 1))),
    )
}

fn period_of(y: f64, k: f64, per_year: i64) -> f64 {
    or_missing((|| {
        let (y, k) = (exact_int(y)?, exact_int(k)?);
        // year 100 is refused here although mdy() accepts it (probe 034)
        if !(101..=9999).contains(&y) || k < 1 || k > per_year {
            return None;
        }
        Some(num((y - 1960) * per_year + k - 1))
    })())
}
pub fn yw(y: f64, w: f64) -> f64 {
    period_of(y, w, 52)
}
pub fn ym(y: f64, m: f64) -> f64 {
    period_of(y, m, 12)
}
pub fn yq(y: f64, q: f64) -> f64 {
    period_of(y, q, 4)
}
pub fn yh(y: f64, h: f64) -> f64 {
    period_of(y, h, 2)
}

// ------------------------------------------------------------ times

/// `cofd(d)`: midnight of a day in datetime/c; `.` outside 0100 to 9999.
pub fn cofd(d: f64) -> f64 {
    match whole_day(d) {
        Some(_) => d * MS_PER_DAY,
        None => SYSMISS,
    }
}

/// `dofc(c)`
pub fn dofc(c: f64) -> f64 {
    if is_missing(c) || !c.is_finite() {
        SYSMISS
    } else {
        (c / MS_PER_DAY).floor()
    }
}

/// Whole milliseconds since midnight (fractions of a millisecond are
/// dropped, as in Stata).
fn ms_of_day(c: f64) -> f64 {
    let c = c.floor();
    c - (c / MS_PER_DAY).floor() * MS_PER_DAY
}

pub fn hh(c: f64) -> f64 {
    if is_missing(c) {
        return SYSMISS;
    }
    (ms_of_day(c) / 3_600_000.0).floor()
}
pub fn mm(c: f64) -> f64 {
    if is_missing(c) {
        return SYSMISS;
    }
    ((ms_of_day(c) % 3_600_000.0) / 60_000.0).floor()
}
pub fn ss(c: f64) -> f64 {
    if is_missing(c) {
        return SYSMISS;
    }
    (ms_of_day(c) % 60_000.0) / 1000.0
}

fn divide(v: f64, by: f64) -> f64 {
    if is_missing(v) {
        SYSMISS
    } else {
        v / by
    }
}
fn scale(v: f64, by: f64) -> f64 {
    if is_missing(v) {
        SYSMISS
    } else {
        v * by
    }
}
pub fn hours(ms: f64) -> f64 {
    divide(ms, 3_600_000.0)
}
pub fn minutes(ms: f64) -> f64 {
    divide(ms, 60_000.0)
}
pub fn seconds(ms: f64) -> f64 {
    divide(ms, 1000.0)
}
pub fn msofhours(h: f64) -> f64 {
    scale(h, 3_600_000.0)
}
pub fn msofminutes(m: f64) -> f64 {
    scale(m, 60_000.0)
}
pub fn msofseconds(s: f64) -> f64 {
    scale(s, 1000.0)
}

/// Milliseconds for a time of day; `None` unless 0 <= h < 24, 0 <= m < 60
/// and 0 <= s < 60.
fn time_ms(h: f64, m: f64, s: f64) -> Option<f64> {
    if [h, m, s].iter().any(|v| is_missing(*v) || !v.is_finite()) {
        return None;
    }
    if !(0.0..24.0).contains(&h) || !(0.0..60.0).contains(&m) || !(0.0..60.0).contains(&s) {
        return None;
    }
    Some((h.floor() * 3600.0 + m.floor() * 60.0) * 1000.0 + s * 1000.0)
}

pub fn hms(h: f64, m: f64, s: f64) -> f64 {
    or_missing(time_ms(h, m, s))
}

/// `dhms(d, h, m, s)`: the day is truncated to a whole day (probe 034).
pub fn dhms(d: f64, h: f64, m: f64, s: f64) -> f64 {
    or_missing((|| {
        Some(exact_floor(d)? as f64 * MS_PER_DAY + time_ms(h, m, s)?)
    })())
}

pub fn mdyhms(mo: f64, d: f64, y: f64, h: f64, mi: f64, s: f64) -> f64 {
    let day = mdy(mo, d, y);
    if is_missing(day) {
        return SYSMISS;
    }
    or_missing(time_ms(h, mi, s).map(|t| day * MS_PER_DAY + t))
}

/// `Cmdyhms()`: like `mdyhms()` in datetime/C, where 23:59:60 is valid on
/// the days a leap second was added.
pub fn big_mdyhms(mo: f64, d: f64, y: f64, h: f64, mi: f64, s: f64) -> f64 {
    let day = mdy(mo, d, y);
    if is_missing(day) {
        return SYSMISS;
    }
    if s >= 60.0 {
        if h != 23.0 || mi != 59.0 || !leap::is_leap_day(day as i64) || s >= 61.0 {
            return SYSMISS;
        }
        let before = day * MS_PER_DAY + 86_399_000.0;
        return cofc_to_big(before) + 1000.0 + (s - 60.0) * 1000.0;
    }
    cofc_to_big(mdyhms(mo, d, y, h, mi, s))
}

pub fn big_dhms(d: f64, h: f64, m: f64, s: f64) -> f64 {
    cofc_to_big(dhms(d, h, m, s))
}

pub fn big_hms(h: f64, m: f64, s: f64) -> f64 {
    cofc_to_big(hms(h, m, s))
}

/// `Cofd(d)`
pub fn big_cofd(d: f64) -> f64 {
    cofc_to_big(cofd(d))
}

/// `dofC(C)`
pub fn dofbig(c: f64) -> f64 {
    dofc(cofc_from_big(c))
}

/// `hhC`, `mmC`, `ssC`: hour, minute and second of a datetime/C value;
/// `ssC` is 60.x during a leap second.
pub fn hh_big(c: f64) -> f64 {
    hh(leap::wall_clock(c).0)
}
pub fn mm_big(c: f64) -> f64 {
    mm(leap::wall_clock(c).0)
}
pub fn ss_big(c: f64) -> f64 {
    let (wall, in_leap) = leap::wall_clock(c);
    // during the leap second the wall clock shows 23:59:59.xxx
    ss(wall) + if in_leap { 1.0 } else { 0.0 }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn calendar_round_trips() {
        assert_eq!(day_number(1960, 1, 1), 0);
        assert_eq!(day_number(2010, 1, 20), 18_282);
        assert_eq!(day_number(100, 1, 1), MIN_DAY);
        assert_eq!(day_number(9999, 12, 31), MAX_DAY);
        for day in (MIN_DAY..=MAX_DAY).step_by(997) {
            let (y, m, d) = civil(day);
            assert_eq!(day_number(y, m, d), day);
        }
    }

    #[test]
    fn functions() {
        assert_eq!(mdy(2.0, 29.0, 2024.0), 23_435.0);
        assert!(is_missing(mdy(2.0, 29.0, 2023.0)));
        assert!(is_missing(mdy(1.0, 1.0, 99.0)));
        assert_eq!(dow(0.0), 5.0);
        assert_eq!(week(day_number(2024, 12, 31) as f64), 52.0);
        assert_eq!(dofw(wofd(18_282.0)), day_number(2010, 1, 15) as f64);
        assert_eq!(yw(1960.0, 1.0), 0.0);
        assert!(is_missing(yw(1960.0, 53.0)));
        assert_eq!(isleapyear(1900.0), 0.0);
        assert!(is_missing(hms(24.0, 0.0, 0.0)));
        assert_eq!(ss(-0.5), 59.999);
    }
}
