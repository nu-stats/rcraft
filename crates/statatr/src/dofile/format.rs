//! Numbers as Stata writes them into macros: `local x = 1/3` keeps the text
//! `.3333333333333333`, Stata's `%18.0g` without its spaces; `numlist` keeps
//! `%15.0g` and `string()` gives `%9.0g` (all three observed in Stata 19 with
//! made-up values). The rules are the `%g` ones RCraft's R helper
//! `stata_fmt` follows (`data_r/returned.rs`), written again here for the
//! values the translator works out itself.

use crate::values::missing::{is_missing, missing_code, missing_label};

/// `x` in Stata's `%w.0g`, without the spaces around it.
pub fn g(x: f64, w: usize) -> String {
    if is_missing(x) || !x.is_finite() {
        return missing_label(missing_code(x).unwrap_or(0));
    }
    if x == 0.0 {
        return "0".into();
    }
    let ax = x.abs();
    let w = w.max(5);
    let exp_width: usize = if ax.log10().floor().abs() >= 100.0 {
        3
    } else {
        2
    };
    let ed = w.saturating_sub(4 + exp_width).max(2);
    let sign = if x < 0.0 { "-" } else { "" };
    match fixed(ax, w - 1, ed) {
        Some(f) => format!("{sign}{f}"),
        None => format!("{sign}{}", e_text(ax, ed)),
    }
}

/// `ax` in fixed notation within `budget` characters, if that shows at
/// least `ed` significant digits (else Stata uses e notation).
fn fixed(ax: f64, budget: usize, ed: usize) -> Option<String> {
    let intdig = if ax >= 1.0 {
        format!("{:.0}", ax.floor()).len()
    } else {
        0
    };
    if intdig + 1 > budget {
        return None;
    }
    let dec = budget - intdig - 1;
    let z = if ax < 1.0 {
        (-ax.log10().floor() - 1.0) as i64
    } else {
        0
    };
    let sig = if ax >= 1.0 {
        (intdig + dec) as i64
    } else {
        dec as i64 - z
    };
    if sig < ed as i64 {
        return None;
    }
    let s = round_text(false, dec, ax);
    // rounding up may have added a digit in front
    let int_part = s.strip_prefix('0').unwrap_or(&s);
    let int_len = int_part.split('.').next().map_or(0, str::len);
    if int_len > intdig {
        let up = if ax < 1.0 {
            1.0
        } else {
            10f64.powi(i32::try_from(intdig).unwrap_or(308))
        };
        return fixed(up, budget, ed);
    }
    let mut s = s;
    if s.contains('.') {
        s = s.trim_end_matches('0').trim_end_matches('.').to_string();
    }
    if let Some(rest) = s.strip_prefix("0.") {
        s = format!(".{rest}");
    }
    Some(s)
}

/// `ax` in e notation with `digits` significant digits: `1.00000000000e+20`.
fn e_text(ax: f64, digits: usize) -> String {
    let s = round_text(true, digits.max(1) - 1, ax);
    let (mant, exp) = s.split_once('e').unwrap_or((&s, "0"));
    let exp: i64 = exp.parse().unwrap_or(0);
    let sign = if exp < 0 { '-' } else { '+' };
    format!("{mant}e{sign}{:02}", exp.unsigned_abs())
}

/// `ax` to `digits` places (after the point, or of the mantissa in e
/// notation), an exact half rounded away from zero as Stata's %g does. A
/// tie is read off the double's exact digits (Rust prints them exactly), so
/// 1428571428.5714285, whose 17-digit form ends in 5, is not taken for one.
fn round_text(e: bool, digits: usize, ax: f64) -> String {
    let show = |v: f64| {
        if e {
            format!("{v:.digits$e}")
        } else {
            format!("{v:.digits$}")
        }
    };
    let out = show(ax);
    if ax == 0.0 {
        return out;
    }
    let exact = format!("{ax:.80e}");
    let (mant, exp) = exact.split_once('e').unwrap_or((&exact, "0"));
    let exp: i64 = exp.parse().unwrap_or(0);
    let keep = if e {
        digits as i64 + 1
    } else {
        digits as i64 + exp + 1
    };
    let mant: String = mant.chars().filter(char::is_ascii_digit).collect();
    if (1..18).contains(&keep) {
        let rest = mant.get(usize::try_from(keep).unwrap_or(0)..).unwrap_or("");
        if rest.starts_with('5') && rest.chars().skip(1).all(|c| c == '0') {
            // the next double up is past the half, by as little as can be
            return show(f64::from_bits(ax.to_bits() + 1));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::values::missing::{missing, SYSMISS};

    /// `local x = exp` in Stata 19, for these values.
    #[test]
    fn macros_hold_statas_18_0g() {
        let cases: &[(f64, &str)] = &[
            (1.0 / 3.0, ".3333333333333333"),
            (-1.0 / 3.0, "-.3333333333333333"),
            (2.0 / 3.0, ".6666666666666666"),
            (1e20, "1.00000000000e+20"),
            (1e-5, ".00001"),
            (1e15, "1000000000000000"),
            (1e16, "1.00000000000e+16"),
            (-1e16, "-1.00000000000e+16"),
            (12345678901234567.0, "1.23456789012e+16"),
            (1e-7, "1.00000000000e-07"),
            (1e-6, "1.00000000000e-06"),
            (0.0000012345, "1.23450000000e-06"),
            (0.000012345, ".000012345"),
            (1.0 / 7.0 * 1e10, "1428571428.571429"),
            (1.0 / 7.0 * 1e14, "14285714285714.29"),
            (1.0 / 7.0 * 1e15, "142857142857142.8"),
            (1.0 / 7.0 * 1e16, "1428571428571429"),
            (1.0 / 7.0 * 1e-4, ".0000142857142857"),
            (1.0 / 7.0 * 1e-5, "1.42857142857e-06"),
            (123_456_789.123_456_79, "123456789.1234568"),
            (2.0 / 3.0 * 1e-3, ".0006666666666667"),
            (-2.0 / 3.0 * 1e-5, "-6.66666666667e-06"),
            (1e100, "1.0000000000e+100"),
            (-1e-100, "-1.0000000000e-100"),
            (999999999999999.9, "999999999999999.9"),
            (9999999999999999.0, "1.00000000000e+16"),
            (9007199254740992.0, "9007199254740992"),
            (std::f64::consts::PI, "3.141592653589793"),
            (0.1 + 0.2, ".3"),
            (1234567.25, "1234567.25"),
            (100.0, "100"),
            (-0.5, "-.5"),
            (2.5, "2.5"),
            (0.7 + 0.1, ".7999999999999999"),
            (0.0, "0"),
            (SYSMISS, "."),
            (missing(1), ".a"),
            (f64::INFINITY, "."),
        ];
        for (x, want) in cases {
            assert_eq!(g(*x, 18), *want, "{x}");
        }
    }

    /// numlist's elements (%15.0g) and string() (%9.0g), from Stata 19.
    #[test]
    fn numlists_and_string_use_their_own_widths() {
        assert_eq!(g(0.123_456_789_012_345_68, 15), ".1234567890123");
        assert_eq!(g(1.123456789012345, 15), "1.123456789012");
        assert_eq!(g(1e20, 15), "1.00000000e+20");
        assert_eq!(g(123456789012345.0, 15), "1.23456789e+14");
        assert_eq!(g(0.7 + 0.1, 15), ".8");
        assert_eq!(g(1.0 / 3.0, 9), ".3333333");
    }
}
