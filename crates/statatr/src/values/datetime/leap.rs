//! Leap seconds, for datetime/C (`%tC`, `Clock()`, `Cofc()`, `cofC()`).
//!
//! The 27 leap seconds added at the end of these UTC days, from the IERS
//! list. Behaviour reference: probe `034-date-functions` (every one of
//! them, plus days without one).

use super::{day_number, MS_PER_DAY};
use crate::values::missing::{is_missing, SYSMISS};

const LEAP_DAYS: [(i64, i64, i64); 27] = [
    (1972, 6, 30),
    (1972, 12, 31),
    (1973, 12, 31),
    (1974, 12, 31),
    (1975, 12, 31),
    (1976, 12, 31),
    (1977, 12, 31),
    (1978, 12, 31),
    (1979, 12, 31),
    (1981, 6, 30),
    (1982, 6, 30),
    (1983, 6, 30),
    (1985, 6, 30),
    (1987, 12, 31),
    (1989, 12, 31),
    (1990, 12, 31),
    (1992, 6, 30),
    (1993, 6, 30),
    (1994, 6, 30),
    (1995, 12, 31),
    (1997, 6, 30),
    (1998, 12, 31),
    (2005, 12, 31),
    (2008, 12, 31),
    (2012, 6, 30),
    (2015, 6, 30),
    (2016, 12, 31),
];

/// Midnight (in datetime/c) ending each leap-second day.
fn midnights() -> impl Iterator<Item = f64> {
    LEAP_DAYS
        .iter()
        .map(|&(y, m, d)| (day_number(y, m, d) + 1) as f64 * MS_PER_DAY)
}

pub fn is_leap_day(day: i64) -> bool {
    LEAP_DAYS
        .iter()
        .any(|&(y, m, d)| day_number(y, m, d) == day)
}

/// `Cofc(c)`: datetime/c to datetime/C, in whole milliseconds as in Stata.
pub fn cofc_to_big(c: f64) -> f64 {
    if is_missing(c) || !c.is_finite() {
        return SYSMISS;
    }
    let c = c.floor();
    let inserted = midnights().filter(|&m| m <= c).count();
    c + 1000.0 * inserted as f64
}

/// `cofC(C)`: datetime/C to datetime/c. A time inside a leap second maps to
/// the midnight that follows it.
pub fn cofc_from_big(big: f64) -> f64 {
    if is_missing(big) || !big.is_finite() {
        return SYSMISS;
    }
    let big = big.floor();
    let mut inserted = 0.0;
    for m in midnights() {
        // the leap second before m runs over [m + 1000*inserted, +1000)
        let leap_start = m + 1000.0 * inserted;
        if big < leap_start {
            break;
        }
        if big < leap_start + 1000.0 {
            return m;
        }
        inserted += 1.0;
    }
    big - 1000.0 * inserted
}

/// The datetime/c time a clock would show for a datetime/C value, and
/// whether it falls inside a leap second (then the clock shows 23:59:60 but
/// the returned time is 23:59:59 of the same fraction).
pub fn wall_clock(big: f64) -> (f64, bool) {
    if is_missing(big) || !big.is_finite() {
        return (SYSMISS, false);
    }
    let mut inserted = 0.0;
    for m in midnights() {
        let leap_start = m + 1000.0 * inserted;
        if big < leap_start {
            break;
        }
        if big < leap_start + 1000.0 {
            return (m - 1000.0 + (big - leap_start), true);
        }
        inserted += 1.0;
    }
    (big - 1000.0 * inserted, false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_the_manual() {
        // [D] Datetime translation: noon of 23nov2010 is 1,606,132,800,000 in
        // datetime/c and 1,606,132,824,000 in datetime/C.
        assert_eq!(cofc_to_big(1_606_132_800_000.0), 1_606_132_824_000.0);
        assert_eq!(cofc_from_big(1_606_132_824_000.0), 1_606_132_800_000.0);
        assert!(is_leap_day(day_number(2016, 12, 31)));
        assert!(!is_leap_day(day_number(2016, 12, 30)));
    }

    #[test]
    fn leap_second_shows_as_second_60() {
        let midnight = (day_number(2016, 12, 31) + 1) as f64 * MS_PER_DAY;
        // 26 leap seconds came before this one
        let leap_start = midnight + 26_000.0;
        let (wall, leap) = wall_clock(leap_start + 500.0);
        assert!(leap);
        assert_eq!(wall, midnight - 500.0);
        assert_eq!(cofc_from_big(leap_start + 500.0), midnight);
        assert_eq!(cofc_from_big(leap_start + 1000.0), midnight);
    }
}
