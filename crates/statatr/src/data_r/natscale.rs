//! `_natscale #lo #hi #n`: about n round label values over a range, the
//! programmer's command Stata's graphs take their axis labels from. It
//! leaves r(n), r(min), r(max) and r(delta).
//!
//! The rule is StatCraft's (`crates/graph/src/scale.rs` there), decoded
//! from Stata 19's answers alone: probes 225 and 226 and about 65,000 more
//! ranges put through Stata as sweeps. A label set L·d .. U·d with step
//! d = 1, 2 or 5 × 10^k is scored (0.67 · inside + 0.33 · outside) / range,
//! plus 0.12 · |labels − n| / n, where inside and outside are how far the end
//! labels sit inside or beyond the data. The lower end is chosen with the
//! upper one rounded out, then the upper end; a step with more than n + 1
//! labels is dropped, and the cheapest step wins (the smaller on a tie).
//!
//! The values here are L·d, U·d and d as doubles. Stata's carry the residue
//! of its own arithmetic in the last bits (r(min) is 2^-55, not 0, for some
//! ranges); those lines agree to 8 digits except where the label is 0.

use super::{no, Out, Session};
use crate::expr_r::Untranslated;
use crate::reader::Command;

/// The r() `_natscale` leaves, in the order `return list` shows them.
pub(super) const R_NATSCALE: &[&str] = &["n", "min", "max", "delta"];

/// Labels `min, min + delta, ..., max`.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Natscale {
    min: f64,
    max: f64,
    delta: f64,
}

impl Natscale {
    /// How many labels there are (r(max) may be the data's own top, above
    /// the last label).
    fn count(&self) -> f64 {
        if self.delta.is_nan()
            || self.delta <= 0.0
            || !self.min.is_finite()
            || !self.max.is_finite()
        {
            return 1.0;
        }
        ((self.max - self.min) / self.delta + 1e-9)
            .floor()
            .clamp(0.0, 200.0)
            + 1.0
    }
}

/// About `n` round labels over `[lo, hi]`.
fn natscale(lo: f64, hi: f64, n: f64) -> Natscale {
    let n = n.max(1.0);
    let (lo, hi) = if lo <= hi { (lo, hi) } else { (hi, lo) };
    if hi == lo {
        // one value: a step of a tenth of its size around it
        let step = if lo == 0.0 {
            1.0
        } else {
            nice_up(lo.abs() / 10.0)
        };
        return Natscale {
            min: lo - step,
            max: lo + step,
            delta: step,
        };
    }
    let range = hi - lo;
    let z0 = range.log10().floor() as i32;
    let mut best: Option<(f64, Natscale)> = None;
    for z in z0 - 3..z0 + 3 {
        for q in [1.0, 2.0, 5.0] {
            let Some((cost, scale, labels)) = step_labels(lo, hi, n, q, q * 10f64.powi(z)) else {
                continue;
            };
            if labels > n + 1.0 {
                continue;
            }
            // the first (smallest) step wins a tie
            if best.as_ref().is_none_or(|(c, _)| cost < *c) {
                best = Some((cost, scale));
            }
        }
    }
    best.map_or(
        Natscale {
            min: lo,
            max: hi,
            delta: range,
        },
        |(_, s)| s,
    )
}

/// How well labels `l·d .. u·d` fit the data: end labels inside the data
/// cost 0.67 a unit, beyond it 0.33, both over the range, and each label
/// more or fewer than `n` 0.12 / n.
fn label_cost(lo: f64, hi: f64, n: f64, d: f64, l: f64, u: f64) -> f64 {
    let (bottom, top) = (l * d, u * d);
    let inside = (bottom - lo).max(0.0) + (hi - top).max(0.0);
    let outside = (lo - bottom).max(0.0) + (top - hi).max(0.0);
    (0.67 * inside + 0.33 * outside) / (hi - lo) + 0.12 * (u - l + 1.0 - n).abs() / n
}

/// The best labels with step `d` (= `q` × 10^k): the score, the scale and
/// the number of labels, or `None` when no two labels fit.
fn step_labels(lo: f64, hi: f64, n: f64, q: f64, d: f64) -> Option<(f64, Natscale, f64)> {
    let eps = 1e-9;
    let (x0, x1) = (lo / d, hi / d);
    let (l_out, l_in) = ((x0 + eps).floor(), (x0 - eps).ceil());
    let (u_in, u_out) = ((x1 + eps).floor(), (x1 - eps).ceil());
    // the lower end is chosen with the upper one rounded out, which Stata
    // takes as 1 when the data end between -1 and 0 steps
    let u0 = if x1 > -1.0 && x1 < 0.0 { 1.0 } else { u_out };
    // an end moves inside only when three labels or more remain
    let l = if l_in != l_out
        && u0 - l_in + 1.0 >= 3.0
        && label_cost(lo, hi, n, d, l_in, u0) < label_cost(lo, hi, n, d, l_out, u0)
    {
        l_in
    } else {
        l_out
    };
    // with steps of 2 × 10^k and two labels, a top label inside the data by
    // less than the bottom one is below it ends at the data (r(max) is the
    // data's top) and counts as exact
    let mut top_is_data = false;
    let u_in_allowed = u_in != u_out
        && u_in > l
        && if u_in - l + 1.0 >= 3.0 {
            true
        } else if q == 2.0 && x1 - u_in < x0 - l {
            top_is_data = true;
            true
        } else {
            false
        };
    let cost_out = label_cost(lo, hi, n, d, l, u_out);
    let cost_in = if u_in_allowed {
        let c = label_cost(lo, hi, n, d, l, u_in);
        if top_is_data {
            c - 0.67 * (hi - u_in * d) / (hi - lo)
        } else {
            c
        }
    } else {
        f64::INFINITY
    };
    let (u, cost) = if cost_in < cost_out {
        (u_in, cost_in)
    } else {
        (u_out, cost_out)
    };
    if u <= l {
        return None;
    }
    let max = if top_is_data && u == u_in { hi } else { u * d };
    Some((
        cost,
        Natscale {
            min: l * d,
            max,
            delta: d,
        },
        u - l + 1.0,
    ))
}

/// The smallest of 1, 2 and 5 times a power of ten at or above `v`.
fn nice_up(v: f64) -> f64 {
    let e = v.log10().floor();
    let base = 10f64.powf(e);
    for q in [1.0, 2.0, 5.0, 10.0] {
        if q * base >= v * (1.0 - 1e-12) {
            return q * base;
        }
    }
    10.0 * base
}

/// A double as an R literal that reads back to the same bits: R's decimal
/// reading can be off by a unit in the last place, its hexadecimal is not.
fn r_exact(v: f64) -> String {
    if v == 0.0 {
        return "0".into();
    }
    if !v.is_normal() {
        return format!("{v:?}");
    }
    let bits = v.to_bits();
    let sign = if bits >> 63 == 1 { "-" } else { "" };
    let exp = ((bits >> 52) & 0x7ff) as i64 - 1023;
    let mant = bits & 0x000f_ffff_ffff_ffff;
    format!("{sign}0x1.{mant:013x}p{exp}")
}

impl Session {
    /// `_natscale #lo #hi #n`, with the values written out (macros are
    /// replaced before this).
    pub(super) fn natscale(&mut self, c: &Command, out: &mut Out) -> Result<(), Untranslated> {
        let words: Vec<&str> = c.parts.main.split_whitespace().collect();
        let nums: Vec<f64> = words.iter().filter_map(|w| w.parse::<f64>().ok()).collect();
        let [lo, hi, n] = nums[..] else {
            return Err(no(
                "_natscale takes three numbers written out (Stata: invalid syntax)",
            ));
        };
        if words.len() != 3 || !c.options.is_empty() {
            return Err(no("_natscale takes three numbers and no options"));
        }
        if !lo.is_finite() || !hi.is_finite() || n.is_nan() || n < 1.0 {
            return Err(no("_natscale with these values is not translated yet"));
        }
        let s = natscale(lo, hi, n.floor());
        self.helpers.insert("stata_returned");
        out.line(format!(
            "stata_r <- list(n = {}, min = {}, max = {}, delta = {})",
            r_exact(s.count()),
            r_exact(s.min),
            r_exact(s.max),
            r_exact(s.delta)
        ));
        self.r_new = Some((R_NATSCALE, "_natscale".into()));
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn check(lo: f64, hi: f64, n: f64, want: (f64, f64, f64)) {
        let s = natscale(lo, hi, n);
        assert!(
            (s.min - want.0).abs() < 1e-9
                && (s.max - want.1).abs() < 1e-9
                && (s.delta - want.2).abs() < 1e-12,
            "{lo} {hi}: {s:?}"
        );
    }

    #[test]
    fn ranges_stata_labelled() {
        check(0.011333, 9.17738, 5.0, (0.0, 10.0, 2.0));
        check(-4.30709, 3.72757, 5.0, (-4.0, 4.0, 2.0));
        check(13.0, 17.6389, 5.0, (13.0, 18.0, 1.0));
        check(0.0, 1.13, 5.0, (0.0, 1.0, 0.5));
        check(0.0, 2.66, 5.0, (0.0, 2.5, 0.5));
        check(0.0, 2.67, 5.0, (0.0, 3.0, 1.0));
        check(5.12, 14.99, 5.0, (5.0, 15.0, 5.0));
        check(250.0, 520.1154906339852, 4.0, (200.0, 500.0, 100.0));
        check(-0.11, 10.0, 5.0, (0.0, 10.0, 2.0));
        // r(max) is the data's top: labels -1000 and -980
        let s = natscale(-993.191_289_454_414_2, -977.074_994_507_977_7, 3.0);
        assert!(s.min == -1000.0 && s.max == -977.074_994_507_977_7 && s.delta == 20.0);
        assert_eq!(s.count(), 2.0);
    }

    #[test]
    fn doubles_are_written_exactly() {
        assert_eq!(r_exact(0.0), "0");
        assert_eq!(r_exact(1.0), "0x1.0000000000000p0");
        assert_eq!(r_exact(-0.2), "-0x1.999999999999ap-3");
        assert_eq!(r_exact(0.01), "0x1.47ae147ae147bp-7");
    }
}
