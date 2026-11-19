//! StatCraft's do-file language front end: reading a do-file into lines
//! (`script`), splitting a command line into its parts (`syntax`) and parsing
//! expressions (`expr`). Pure text in, structures out. Vendored (all but this
//! file and `commands.rs`); see the
//! crate's `lib.rs`.

pub mod commands;
pub mod expr;
pub mod script;
pub mod syntax;

pub use expr::{parse, parse_prefix, BinaryOp, Expr, ExprError, UnaryOp};

#[cfg(test)]
mod tests {
    use super::*;

    /// Stata 19 takes a `+` sign only before a number written out (probe
    /// 752). RCraft's own copy of the rule gave wrong answers for these four
    /// forms until the parser was resynced with StatCraft's.
    #[test]
    fn a_plus_sign_after_an_operator_or_a_prefix_is_refused() {
        for (text, message) in [
            ("1 + +2", "1+ invalid name"),
            ("1++2", "1+ invalid name"),
            ("!+3", "invalid syntax"),
            ("~+3", "invalid syntax"),
        ] {
            match parse(text) {
                Ok(e) => panic!("{text} should not parse, got {e:?}"),
                Err(e) => assert_eq!((e.message.as_str(), e.code), (message, 198), "{text}"),
            }
        }
    }

    #[test]
    fn a_plus_sign_before_a_written_number_still_reads() {
        for text in ["+5", "1 - +2", "2^+2", "1 == +1", "1 + (+2)", "1 + !0"] {
            assert!(parse(text).is_ok(), "{text}");
        }
    }

    #[test]
    fn a_plus_sign_before_a_name_or_call_is_refused() {
        for (text, message, code) in [
            ("+x", "+x invalid name", 198),
            ("+abs(-5)", "unknown function +abs()", 133),
        ] {
            match parse(text) {
                Ok(e) => panic!("{text} should not parse, got {e:?}"),
                Err(e) => assert_eq!((e.message.as_str(), e.code), (message, code), "{text}"),
            }
        }
    }
}
