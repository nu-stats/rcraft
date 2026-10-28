//! Missing values: `.` and `.a` to `.z`.
//!
//! Behaviour reference: `plan/stata/missing-values.md`.
//!
//! In memory a numeric value is an `f64`, and the 27 missing values use the
//! same doubles the `.dta` format uses: `.` is 2^1023 and each extended code
//! adds 2^983 (1/4096 of the mantissa). Every missing value is larger than
//! every number, so comparisons and sorting order them correctly with no
//! extra work: numbers < `.` < `.a` < ... < `.z`.

/// Number of missing codes: `.` plus `.a` to `.z`.
pub const CODES: u8 = 27;

const BASE_BITS: u64 = 0x7fe0_0000_0000_0000;
const STEP_BITS: u64 = 1 << 40;

/// `.`, the system missing value.
pub const SYSMISS: f64 = f64::from_bits(BASE_BITS);

/// Largest nonmissing double, `+1.fffffffffffffX+3fe`.
pub const MAX_DOUBLE: f64 = f64::from_bits(0x7fdf_ffff_ffff_ffff);

/// The missing value for a code: 0 is `.`, 1 is `.a`, ..., 26 is `.z`.
/// Codes above 26 give `.z`.
pub fn missing(code: u8) -> f64 {
    let code = u64::from(code.min(CODES - 1));
    f64::from_bits(BASE_BITS + code * STEP_BITS)
}

/// True for `.` to `.z` and for anything else above the largest double
/// (infinity and NaN included).
pub fn is_missing(v: f64) -> bool {
    v.is_nan() || v > MAX_DOUBLE
}

/// The code of a missing value (0 for `.`), or `None` for a number. Values
/// between two codes count as the lower one, and infinity or NaN count as
/// `.`.
pub fn missing_code(v: f64) -> Option<u8> {
    if !is_missing(v) {
        return None;
    }
    if v.is_nan() || v.is_infinite() || v < SYSMISS {
        return Some(0);
    }
    let steps = (v.to_bits() - BASE_BITS) / STEP_BITS;
    Some(u8::try_from(steps).unwrap_or(CODES - 1).min(CODES - 1))
}

/// How a missing value is written: `.`, `.a`, ..., `.z`.
pub fn missing_label(code: u8) -> String {
    match code {
        0 => ".".to_string(),
        c => {
            let letter = char::from(b'a' + (c.min(CODES - 1) - 1));
            format!(".{letter}")
        }
    }
}

/// Parses `.`, `.a`, ..., `.z`.
pub fn parse_missing(text: &str) -> Option<f64> {
    let rest = text.strip_prefix('.')?;
    match rest.as_bytes() {
        [] => Some(SYSMISS),
        [c @ b'a'..=b'z'] => Some(missing(c - b'a' + 1)),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn codes_match_the_dta_bit_patterns() {
        assert_eq!(missing(0).to_bits(), 0x7fe0_0000_0000_0000);
        assert_eq!(missing(1).to_bits(), 0x7fe0_0100_0000_0000);
        assert_eq!(missing(2).to_bits(), 0x7fe0_0200_0000_0000);
        assert_eq!(missing(26).to_bits(), 0x7fe0_1a00_0000_0000);
    }

    #[test]
    fn order_is_numbers_then_dot_then_letters() {
        assert!(MAX_DOUBLE < missing(0));
        assert!(missing(0) < missing(1));
        assert!(missing(25) < missing(26));
        assert!(is_missing(SYSMISS) && !is_missing(1e300));
    }

    #[test]
    fn codes_round_trip() {
        for code in 0..CODES {
            let v = missing(code);
            assert!(is_missing(v));
            assert_eq!(missing_code(v), Some(code));
            assert_eq!(parse_missing(&missing_label(code)), Some(v));
        }
        assert_eq!(missing_label(1), ".a");
        assert_eq!(missing_label(26), ".z");
        assert_eq!(missing_code(1.0), None);
        assert_eq!(missing_code(f64::INFINITY), Some(0));
        assert_eq!(missing_code(f64::NAN), Some(0));
        assert_eq!(parse_missing(".ab"), None);
        assert_eq!(parse_missing(".A"), None);
    }
}
