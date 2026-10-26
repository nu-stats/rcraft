//! Finds the conda installs that Pinokio manages, so R kept inside them
//! (for example in an app's own environment) is found without any setting.
//!
//! Pinokio's home folder comes from `PINOKIO_HOME`, or else from the `home`
//! entry in `~/.pinokio/config.json`. Its conda installs are `bin/miniforge`
//! and `bin/miniconda` inside that home. Only a path is read from the file;
//! it is never changed.

use std::io::Read;
use std::path::{Path, PathBuf};

/// The config file is small; anything larger is not read.
const MAX_CONFIG_BYTES: u64 = 64 * 1024;

/// The conda base folders under a Pinokio home.
pub fn conda_roots(pinokio_home: &Path) -> Vec<PathBuf> {
    ["miniforge", "miniconda"]
        .iter()
        .map(|name| pinokio_home.join("bin").join(name))
        .collect()
}

/// Reads at most [`MAX_CONFIG_BYTES`] of `~/.pinokio/config.json` and returns
/// its `home` entry.
pub fn home_from_config_file(user_home: &Path) -> Option<PathBuf> {
    let file = std::fs::File::open(user_home.join(".pinokio").join("config.json")).ok()?;
    let mut text = String::new();
    file.take(MAX_CONFIG_BYTES).read_to_string(&mut text).ok()?;
    home_from_config(&text)
}

/// The string value of the `home` key in a JSON object, without a JSON
/// library: the file is tiny and only this one value is wanted.
pub fn home_from_config(text: &str) -> Option<PathBuf> {
    let mut from = 0;
    while let Some(found) = text.get(from..)?.find("\"home\"") {
        let after = from + found + "\"home\"".len();
        from = after;
        let rest = text.get(after..)?.trim_start();
        let Some(rest) = rest.strip_prefix(':') else {
            continue; // the text "home" used as a value, not a key
        };
        let Some(rest) = rest.trim_start().strip_prefix('"') else {
            continue;
        };
        if let Some(value) = read_json_string(rest) {
            if !value.is_empty() {
                return Some(PathBuf::from(value));
            }
        }
    }
    None
}

/// Reads a JSON string body (after the opening quote) up to the closing quote.
fn read_json_string(text: &str) -> Option<String> {
    let mut out = String::new();
    let mut chars = text.chars();
    loop {
        match chars.next()? {
            '"' => return Some(out),
            '\\' => match chars.next()? {
                '\\' => out.push('\\'),
                '"' => out.push('"'),
                '/' => out.push('/'),
                'n' => out.push('\n'),
                't' => out.push('\t'),
                'u' => {
                    let hex: String = chars.by_ref().take(4).collect();
                    let code = u32::from_str_radix(&hex, 16).ok()?;
                    out.push(char::from_u32(code)?);
                }
                _ => return None,
            },
            c => out.push(c),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_the_home_entry() {
        let text =
            r#"{ "access": {"host": "10.0.0.1"}, "home": "C:\\pinokio", "mode": "desktop" }"#;
        assert_eq!(home_from_config(text), Some(PathBuf::from("C:\\pinokio")));
    }

    #[test]
    fn skips_home_used_as_a_value() {
        let text = r#"{ "theme": "home", "home": "/Users/a/pinokio" }"#;
        assert_eq!(
            home_from_config(text),
            Some(PathBuf::from("/Users/a/pinokio"))
        );
    }

    #[test]
    fn reads_unicode_escapes() {
        let text = r#"{"home": "/h\u00e9"}"#;
        assert_eq!(home_from_config(text), Some(PathBuf::from("/hé")));
    }

    #[test]
    fn bad_or_missing_input_gives_none() {
        assert_eq!(home_from_config(""), None);
        assert_eq!(home_from_config("{}"), None);
        assert_eq!(home_from_config(r#"{"home": ""}"#), None);
        assert_eq!(home_from_config(r#"{"home": "unterminated"#), None);
        assert_eq!(home_from_config(r#"{"home": 5}"#), None);
        assert_eq!(home_from_config(r#"{"home": "bad\q"}"#), None);
        assert_eq!(home_from_config(r#"{"home": "\ud800"}"#), None);
        assert_eq!(home_from_config("\"home\""), None);
    }

    #[test]
    fn conda_roots_are_under_bin() {
        let roots = conda_roots(Path::new("/p"));
        assert_eq!(roots.len(), 2);
        assert!(roots.contains(&Path::new("/p").join("bin").join("miniforge")));
    }

    #[test]
    fn a_missing_config_file_is_fine() {
        assert_eq!(
            home_from_config_file(Path::new("definitely/not/here")),
            None
        );
    }
}
