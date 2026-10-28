//! Strings to dates: `date()`, `clock()`, `Clock()`, `monthly()`, ...
//!
//! Behaviour reference: `plan/stata/dates.md` ("Strings to dates"),
//! checked against every line of probe `035-date-strings`.
//!
//! The documented rules: punctuation becomes a space (except a period
//! inside a number, kept so seconds can have a fraction), letters and digits
//! are separated, and the pieces are matched against the mask. A run of
//! digits longer than a mask element is split to fit it (`20061201` with
//! `YMD`). `#` skips one piece, or everything left when it ends the mask.

use super::{day_number, days_in_month, leap, MS_PER_DAY};
use crate::values::missing::{is_missing, SYSMISS};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Period {
    Week,
    Month,
    Quarter,
    Half,
    Year,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Part {
    Day,
    Month,
    Year { century: Option<i64> },
    Hour,
    Minute,
    Second,
    Quarter,
    Week,
    Half,
    Skip,
}

#[derive(Debug, Clone, PartialEq)]
enum Piece {
    Number(String),
    Word(String),
    /// A period between two digits, kept for fractions of a second.
    Point,
}

fn parse_mask(mask: &str, period: Option<Period>) -> Option<Vec<Part>> {
    let mut parts = Vec::new();
    let mut century = String::new();
    for c in mask.chars() {
        if c.is_ascii_digit() {
            century.push(c);
            continue;
        }
        let part = match c {
            ' ' => continue,
            'D' => Part::Day,
            'M' => Part::Month,
            'Y' => {
                let prefix = if century.is_empty() {
                    None
                } else {
                    Some(century.parse().ok()?)
                };
                century.clear();
                parts.push(Part::Year { century: prefix });
                continue;
            }
            'h' => Part::Hour,
            'm' => Part::Minute,
            's' => Part::Second,
            'Q' if period == Some(Period::Quarter) => Part::Quarter,
            'W' if period == Some(Period::Week) => Part::Week,
            'H' if period == Some(Period::Half) => Part::Half,
            '#' => Part::Skip,
            _ => return None,
        };
        if !century.is_empty() {
            return None; // digits only go before Y
        }
        parts.push(part);
    }
    century.is_empty().then_some(parts)
}

fn pieces(text: &str) -> Vec<Piece> {
    let chars: Vec<char> = text.chars().collect();
    let mut out = Vec::new();
    let mut current = String::new();
    let flush = |current: &mut String, out: &mut Vec<Piece>| {
        if current.is_empty() {
            return;
        }
        let piece = if current.chars().all(|c| c.is_ascii_digit()) {
            Piece::Number(std::mem::take(current))
        } else {
            Piece::Word(std::mem::take(current).to_lowercase())
        };
        out.push(piece);
    };
    for (i, &c) in chars.iter().enumerate() {
        let prev = i.checked_sub(1).and_then(|j| chars.get(j)).copied();
        let next = chars.get(i + 1).copied();
        if c.is_ascii_digit() || c.is_alphabetic() {
            let switches = current
                .chars()
                .last()
                .is_some_and(|last| last.is_ascii_digit() != c.is_ascii_digit());
            if switches {
                flush(&mut current, &mut out);
            }
            current.push(c);
        } else {
            flush(&mut current, &mut out);
            let between_digits = prev.is_some_and(|p| p.is_ascii_digit())
                && next.is_some_and(|n| n.is_ascii_digit());
            if c == '.' && between_digits {
                out.push(Piece::Point);
            }
        }
    }
    flush(&mut current, &mut out);
    out
}

const MONTHS: [&str; 12] = [
    "january",
    "february",
    "march",
    "april",
    "may",
    "june",
    "july",
    "august",
    "september",
    "october",
    "november",
    "december",
];

fn month_name(word: &str) -> Option<i64> {
    if word.chars().count() < 3 {
        return None;
    }
    MONTHS
        .iter()
        .position(|name| name.starts_with(word))
        .and_then(|i| i64::try_from(i + 1).ok())
}

#[derive(Default)]
struct Found {
    year: Option<(i64, usize, Option<i64>)>,
    month: Option<i64>,
    day: Option<i64>,
    hour: Option<i64>,
    minute: Option<i64>,
    second: Option<f64>,
    meridian: Option<bool>,
    sub: Option<i64>,
}

/// Matches `text` against `mask`. `split` allows a run of digits to be cut
/// to fit (the period functions do not do that).
fn matched(text: &str, mask: &str, period: Option<Period>, split: bool) -> Option<Found> {
    let parts = parse_mask(mask, period)?;
    let mut queue: Vec<Piece> = pieces(text);
    if let Some(period) = period {
        // The mask names exactly the year and the period's own part.
        let wanted = match period {
            Period::Year => None,
            Period::Month => Some(Part::Month),
            Period::Quarter => Some(Part::Quarter),
            Period::Half => Some(Part::Half),
            Period::Week => Some(Part::Week),
        };
        let years = parts
            .iter()
            .filter(|p| matches!(p, Part::Year { .. }))
            .count();
        let own = parts.iter().filter(|p| Some(**p) == wanted).count();
        if years != 1 || own != usize::from(wanted.is_some()) || parts.len() != years + own {
            return None;
        }
        // In 2006m1 or 2006q3 the letter only separates the numbers.
        queue
            .retain(|p| !matches!(p, Piece::Word(w) if ["m", "q", "h", "w"].contains(&w.as_str())));
    }
    queue.reverse(); // pop from the end
    let mut found = Found::default();
    let last = parts.len().checked_sub(1);
    for (index, part) in parts.iter().enumerate() {
        drop_points(&mut queue);
        if *part == Part::Skip {
            if Some(index) == last {
                queue.clear();
            } else {
                queue.pop()?;
            }
            continue;
        }
        if *part == Part::Month {
            if let Some(Piece::Word(w)) = queue.last() {
                found.month = Some(month_name(w)?);
                queue.pop();
                continue;
            }
        }
        let next_len = match queue.last() {
            Some(Piece::Number(d)) => d.len(),
            _ => 0,
        };
        let width = match part {
            // a four-digit year is a full year even when the mask gives a century
            Part::Year { century: Some(_) } if next_len == 4 => 4,
            Part::Year { century: Some(_) } => 2,
            Part::Year { century: None } => 4,
            _ => 2,
        };
        let digits = take_number(&mut queue, width, split)?;
        if matches!(part, Part::Year { century: Some(_) }) && digits.len() != 2 && digits.len() != 4
        {
            return None; // `20Y` needs a two- or four-digit year
        }
        let value: i64 = digits.parse().ok()?;
        match part {
            Part::Day => found.day = Some(value),
            Part::Month => found.month = Some(value),
            Part::Year { century } => found.year = Some((value, digits.len(), *century)),
            Part::Hour => found.hour = Some(value),
            Part::Minute => found.minute = Some(value),
            Part::Second => {
                let mut seconds = value as f64;
                if queue.last() == Some(&Piece::Point) {
                    queue.pop();
                    let fraction = take_number(&mut queue, usize::MAX, false)?;
                    seconds += format!("0.{fraction}").parse::<f64>().ok()?;
                }
                found.second = Some(seconds);
            }
            Part::Quarter | Part::Week | Part::Half => found.sub = Some(value),
            Part::Skip => {}
        }
        if matches!(part, Part::Hour | Part::Minute | Part::Second) {
            meridian(&mut queue, &mut found);
        }
    }
    queue.retain(|p| *p != Piece::Point);
    queue.is_empty().then_some(found)
}

/// A period before a value is just a separator; only one right after the
/// seconds starts a fraction.
fn drop_points(queue: &mut Vec<Piece>) {
    while queue.last() == Some(&Piece::Point) {
        queue.pop();
    }
}

fn take_number(queue: &mut Vec<Piece>, width: usize, split: bool) -> Option<String> {
    let Some(Piece::Number(digits)) = queue.pop() else {
        return None;
    };
    if split && digits.len() > width {
        let (head, tail) = digits.split_at(width);
        let head = head.to_string();
        queue.push(Piece::Number(tail.to_string()));
        return Some(head);
    }
    Some(digits)
}

/// `am`, `pm`, `a.m.`, `p.m.` after a time.
fn meridian(queue: &mut Vec<Piece>, found: &mut Found) {
    match queue.last() {
        Some(Piece::Word(w)) if w == "am" || w == "pm" => {
            found.meridian = Some(w == "pm");
            queue.pop();
        }
        Some(Piece::Word(w)) if w == "a" || w == "p" => {
            let pm = w == "p";
            let len = queue.len();
            if len >= 2 && queue.get(len - 2) == Some(&Piece::Word("m".into())) {
                queue.truncate(len - 2);
                found.meridian = Some(pm);
            }
        }
        _ => {}
    }
}

/// The full year: two-digit years take the mask's century or the largest
/// year not after `topyear`.
fn full_year(found: &Found, topyear: Option<f64>) -> Option<i64> {
    let Some((value, digits, century)) = found.year else {
        return Some(1960);
    };
    let year = match (century, topyear) {
        (Some(c), _) if digits <= 2 => c * 100 + value,
        (Some(_), _) => value,
        (None, Some(top)) if digits <= 2 && !is_missing(top) => {
            let top = top.floor() as i64;
            let mut y = top - (top - value).rem_euclid(100);
            if y > top {
                y -= 100;
            }
            y
        }
        _ => value,
    };
    (100..=9999).contains(&year).then_some(year)
}

fn day_of(found: &Found, topyear: Option<f64>) -> Option<i64> {
    let y = full_year(found, topyear)?;
    let m = found.month.unwrap_or(1);
    let d = found.day.unwrap_or(1);
    if !(1..=12).contains(&m) || d < 1 || d > days_in_month(y, m) {
        return None;
    }
    Some(day_number(y, m, d))
}

/// Milliseconds into the day, with the second (which may be 60.x).
fn time_of(found: &Found) -> Option<(f64, f64)> {
    let mut h = found.hour.unwrap_or(0);
    let mi = found.minute.unwrap_or(0);
    let s = found.second.unwrap_or(0.0);
    if let Some(pm) = found.meridian {
        if !(1..=12).contains(&h) {
            return None;
        }
        h = match (h, pm) {
            (12, false) => 0,
            (12, true) => 12,
            (h, true) => h + 12,
            (h, false) => h,
        };
    }
    if !(0..24).contains(&h) || !(0..60).contains(&mi) || !(0.0..61.0).contains(&s) {
        return None;
    }
    Some((((h * 60 + mi) * 60) as f64 * 1000.0, s))
}

/// `date(s, mask [, topyear])`
pub fn parse_date(text: &str, mask: &str, topyear: Option<f64>) -> f64 {
    matched(text, mask, None, true)
        .and_then(|found| {
            time_of(&found).filter(|(_, s)| *s < 60.0)?;
            day_of(&found, topyear)
        })
        .map_or(SYSMISS, |d| d as f64)
}

/// `clock(s, mask [, topyear])` or, with `big`, `Clock()`.
pub fn parse_clock(text: &str, mask: &str, topyear: Option<f64>, big: bool) -> f64 {
    let Some(found) = matched(text, mask, None, true) else {
        return SYSMISS;
    };
    let (Some(day), Some((ms, s))) = (day_of(&found, topyear), time_of(&found)) else {
        return SYSMISS;
    };
    let c = day as f64 * MS_PER_DAY + ms;
    if s >= 60.0 {
        // only a real leap second, and only in datetime/C
        if !big || ms != 86_340_000.0 || !leap::is_leap_day(day) {
            return SYSMISS;
        }
        return leap::cofc_to_big(c + 59_000.0) + 1000.0 + (s - 60.0) * 1000.0;
    }
    let c = c + s * 1000.0;
    if big {
        leap::cofc_to_big(c)
    } else {
        c
    }
}

/// `weekly()`, `monthly()`, `quarterly()`, `halfyearly()`, `yearly()`
pub fn parse_period(text: &str, mask: &str, topyear: Option<f64>, period: Period) -> f64 {
    let Some(found) = matched(text, mask, Some(period), false) else {
        return SYSMISS;
    };
    let Some(y) = full_year(&found, topyear) else {
        return SYSMISS;
    };
    let years = (y - 1960) as f64;
    let sub = |n: i64| -> Option<f64> {
        let k = found.sub.unwrap_or(1);
        (1..=n).contains(&k).then_some((k - 1) as f64)
    };
    let value = match period {
        Period::Year => Some(y as f64),
        Period::Month => {
            let m = found.month.unwrap_or(1);
            (1..=12)
                .contains(&m)
                .then_some(years * 12.0 + (m - 1) as f64)
        }
        Period::Quarter => sub(4).map(|k| years * 4.0 + k),
        Period::Half => sub(2).map(|k| years * 2.0 + k),
        Period::Week => sub(52).map(|k| years * 52.0 + k),
    };
    value.unwrap_or(SYSMISS)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn documented_examples() {
        let d = |s: &str, m: &str| parse_date(s, m, None);
        let dec1 = day_number(2006, 12, 1) as f64;
        assert_eq!(d("01dec2006", "DMY"), dec1);
        assert_eq!(d("December 1, 2006", "MDY"), dec1);
        assert_eq!(d("2006-12-01", "YMD"), dec1);
        assert_eq!(d("20061201", "YMD"), dec1);
        assert_eq!(d("01-12-06", "DM20Y"), dec1);
        assert!(is_missing(d("31apr2006", "DMY")));
        assert!(is_missing(d("01dec2006 extra", "DMY")));
        assert_eq!(d("01dec2006 extra", "DMY#"), dec1);
        assert_eq!(
            parse_date("01dec98", "DMY", Some(2020.0)),
            day_number(1998, 12, 1) as f64
        );
        let c = parse_clock("Fri Dec 01 14:22:43 CST 2006", "#MDhms#Y", None, false);
        assert_eq!(c, dec1 * MS_PER_DAY + 51_763_000.0);
        let pm = parse_clock("01dec2006 2:22 p.m.", "DMYhm", None, false);
        assert_eq!(
            pm,
            dec1 * MS_PER_DAY + (14 * 3600 + 22 * 60) as f64 * 1000.0
        );
        assert!(is_missing(parse_clock(
            "31dec2016 23:59:60",
            "DMYhms",
            None,
            false
        )));
        assert!(!is_missing(parse_clock(
            "31dec2016 23:59:60",
            "DMYhms",
            None,
            true
        )));
        assert_eq!(parse_period("2020m1", "YM", None, Period::Month), 720.0);
        assert_eq!(parse_period("2020q2", "YQ", None, Period::Quarter), 241.0);
    }
}
