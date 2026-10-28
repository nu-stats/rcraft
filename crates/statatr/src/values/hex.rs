//! The `%21x` hexadecimal format, which shows a double exactly:
//! `+1.921fb54442d18X+001` is `(1 + 0x921fb54442d18 / 2^52) * 2^1`.
//!
//! Behaviour reference: probes `000-environment` and `031-format-grid`.
//! Zero is written `+0.0000000000000X-3ff`, as Stata writes it.

use crate::values::missing::parse_missing;

const MANTISSA_BITS: u32 = 52;
const MANTISSA_MASK: u64 = (1 << MANTISSA_BITS) - 1;
const EXPONENT_BIAS: i32 = 1023;

/// Writes a value in `%21x` form. Missing values show their bit patterns
/// too: `.` is `+1.0000000000000X+3ff` (probes 034 and 100).
pub fn to_hex(v: f64) -> String {
    let bits = v.to_bits();
    let sign = if v.is_sign_negative() { '-' } else { '+' };
    let mantissa = bits & MANTISSA_MASK;
    let biased = i32::try_from((bits >> MANTISSA_BITS) & 0x7ff).unwrap_or(0);
    if biased == 0 {
        // zero and subnormals: leading digit 0, smallest exponent
        return format!("{sign}0.{mantissa:013x}X-3ff");
    }
    let exponent = biased - EXPONENT_BIAS;
    let exp_sign = if exponent < 0 { '-' } else { '+' };
    format!(
        "{sign}1.{mantissa:013x}X{exp_sign}{:03x}",
        exponent.unsigned_abs()
    )
}

/// Reads a value written in `%21x` form (or a missing value).
pub fn from_hex(text: &str) -> Option<f64> {
    let text = text.trim();
    if text.starts_with('.') {
        return parse_missing(text);
    }
    let (negative, rest) = match text.as_bytes().first()? {
        b'+' => (false, text.get(1..)?),
        b'-' => (true, text.get(1..)?),
        _ => (false, text),
    };
    let (lead, rest) = rest.split_once('.')?;
    let (digits, exp) = rest.split_once(['X', 'x'])?;
    let (exp_negative, exp_digits) = match exp.as_bytes().first()? {
        b'+' => (false, exp.get(1..)?),
        b'-' => (true, exp.get(1..)?),
        _ => (false, exp),
    };
    if digits.len() > 13 || digits.is_empty() {
        return None;
    }
    let mantissa = u64::from_str_radix(digits, 16).ok()? << (4 * (13 - digits.len()));
    let exponent = i32::from_str_radix(exp_digits, 16).ok()?;
    let exponent = if exp_negative { -exponent } else { exponent };
    let sign_bit = u64::from(negative) << 63;
    let bits = match lead {
        "0" => {
            if exponent != -EXPONENT_BIAS {
                return None;
            }
            sign_bit | mantissa
        }
        "1" => {
            let biased = u64::try_from(exponent + EXPONENT_BIAS).ok()?;
            if biased == 0 || biased > 0x7fe {
                return None;
            }
            sign_bit | (biased << MANTISSA_BITS) | mantissa
        }
        _ => return None,
    };
    Some(f64::from_bits(bits))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::values::missing::{missing, MAX_DOUBLE};

    #[test]
    fn matches_recorded_values() {
        // probe 000 and 010
        assert_eq!(to_hex(0.1), "+1.999999999999aX-004");
        assert_eq!(to_hex(MAX_DOUBLE), "+1.fffffffffffffX+3fe");
        assert_eq!(to_hex(-MAX_DOUBLE), "-1.fffffffffffffX+3fe");
        assert_eq!(to_hex(f64::EPSILON), "+1.0000000000000X-034");
        assert_eq!(to_hex(0.0), "+0.0000000000000X-3ff");
        assert_eq!(to_hex(1.0), "+1.0000000000000X+000");
        assert_eq!(to_hex(missing(0)), "+1.0000000000000X+3ff");
        assert_eq!(to_hex(missing(1)), "+1.0010000000000X+3ff");
    }

    #[test]
    fn round_trips() {
        for v in [
            0.1,
            -3.75,
            1e300,
            -1e-300,
            5e-324,
            0.0,
            123_456_789.0,
            MAX_DOUBLE,
        ] {
            assert_eq!(from_hex(&to_hex(v)), Some(v), "{v}");
        }
        assert_eq!(from_hex(".z"), Some(missing(26)));
        assert_eq!(from_hex("+1.8X+001"), Some(3.0));
        assert_eq!(from_hex("1.0000000000000X+400"), None);
        assert_eq!(from_hex("garbage"), None);
    }
}
