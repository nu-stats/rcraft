//! The `rc` command forms of the Command window: `rc install fixest`,
//! `rc search panel data`, `rc describe fixest`, `rc update`, `rc installed`
//! and `rc remove`. They are the typed way to what the Packages window does
//! (`ssc install` and `net search` for R), and they never run on their own:
//! an install, update or remove opens the same list to say yes to.
//!
//! A line is an `rc` command only when `rc` is followed by one of the command
//! words, so an R object called `rc` (`rc <- 3`, `rc(1)`) is still R.

use crate::packages::{is_package_name, is_version, InstallFrom};

/// What an `rc` line asks for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Rc {
    Help,
    Install {
        packages: Vec<String>,
        /// One older version, for a single package.
        version: Option<String>,
        from: InstallFrom,
    },
    Remove {
        packages: Vec<String>,
    },
    /// The packages to update, or every one that has an update when empty.
    Update {
        packages: Vec<String>,
    },
    Search {
        words: String,
    },
    Describe {
        package: String,
    },
    /// The installed packages, narrowed by part of a name when given.
    Installed {
        filter: String,
    },
}

/// The command words after `rc`.
pub const COMMANDS: &[&str] = &[
    "describe",
    "help",
    "install",
    "installed",
    "remove",
    "search",
    "update",
];

/// What `rc help` prints.
pub const HELP: &str = "\
rc install PACKAGE [PACKAGE ...] [, version(1.2.3) from(source|binary) provider(cran)]
    install from CRAN, after you say yes to the list (Packages window)
rc remove PACKAGE [PACKAGE ...]
    remove from RCraft's library, after you say yes
rc update [PACKAGE ...]
    update the named packages, or all that have an update, after you say yes
rc search WORDS
    search the CRAN package list on this computer (download it in the Packages window)
rc describe PACKAGE
    what CRAN says about a package: version, needs, links
rc installed [PART OF A NAME]
    the packages R has installed
Press Tab after \"rc \" or after a package name's first letters to complete it.
";

/// Reads a line typed in the Command window. `None` when it is not an `rc`
/// command (it goes to R as it is); `Some(Err(..))` when it is one but is
/// wrong, with a message to show.
pub fn parse(line: &str) -> Option<Result<Rc, String>> {
    let t = line.trim();
    let rest = t.strip_prefix("rc")?;
    if !rest.is_empty() && !rest.starts_with(char::is_whitespace) {
        return None;
    }
    let rest = rest.trim_start();
    if rest.is_empty() {
        return Some(Ok(Rc::Help));
    }
    let (word, tail) = match rest.split_once(char::is_whitespace) {
        Some((w, t)) => (w, t.trim()),
        None => (rest, ""),
    };
    if !COMMANDS.contains(&word) {
        // `rc <- 3` and the like belong to R
        return None;
    }
    Some(command(word, tail))
}

/// `name` or `name(argument)` options after a comma.
fn options(text: &str) -> Result<Vec<(String, String)>, String> {
    let mut out = Vec::new();
    let mut rest = text.trim();
    while !rest.is_empty() {
        let end = rest
            .find(|c: char| c == '(' || c.is_whitespace())
            .unwrap_or(rest.len());
        let name = rest.get(..end).unwrap_or("");
        if name.is_empty() {
            return Err(format!("Cannot read the options at \"{rest}\"."));
        }
        let after = rest.get(end..).unwrap_or("");
        if let Some(inner) = after.strip_prefix('(') {
            let Some(close) = inner.find(')') else {
                return Err(format!("The option {name}( is not closed."));
            };
            out.push((
                name.to_ascii_lowercase(),
                inner.get(..close).unwrap_or("").trim().to_string(),
            ));
            rest = inner.get(close + 1..).unwrap_or("").trim_start();
        } else {
            out.push((name.to_ascii_lowercase(), String::new()));
            rest = after.trim_start();
        }
    }
    Ok(out)
}

fn names(text: &str, what: &str) -> Result<Vec<String>, String> {
    let list: Vec<String> = text
        .split(|c: char| c.is_whitespace() || c == ',')
        .filter(|w| !w.is_empty())
        .map(str::to_string)
        .collect();
    if let Some(bad) = list.iter().find(|p| !is_package_name(p)) {
        return Err(format!("\"{bad}\" is not a package name."));
    }
    if list.is_empty() && !what.is_empty() {
        return Err(format!(
            "{what} needs a package name, for example: {what} fixest"
        ));
    }
    Ok(list)
}

fn command(word: &str, tail: &str) -> Result<Rc, String> {
    // the part before the first comma, and the options after it
    let (main, opts) = match tail.split_once(',') {
        Some((m, o)) => (m.trim(), options(o)?),
        None => (tail, Vec::new()),
    };
    match word {
        "help" => Ok(Rc::Help),
        "search" => {
            if tail.trim().is_empty() {
                return Err("rc search needs some words, for example: rc search panel data".into());
            }
            Ok(Rc::Search {
                words: tail.trim().to_string(),
            })
        }
        "describe" => {
            let list = names(main, "rc describe")?;
            match list.as_slice() {
                [one] => Ok(Rc::Describe {
                    package: one.clone(),
                }),
                _ => Err("rc describe takes one package: rc describe fixest".into()),
            }
        }
        "installed" => Ok(Rc::Installed {
            filter: tail.trim().to_string(),
        }),
        "remove" => Ok(Rc::Remove {
            packages: names(main, "rc remove")?,
        }),
        "update" => Ok(Rc::Update {
            packages: names(main, "")?,
        }),
        _ => {
            // install
            let packages = names(main, "rc install")?;
            let mut version = None;
            let mut from = InstallFrom::Default;
            for (name, arg) in opts {
                match name.as_str() {
                    "version" => {
                        if !is_version(&arg) {
                            return Err(format!(
                                "\"{arg}\" is not a version, for example version(1.2.3)."
                            ));
                        }
                        version = Some(arg);
                    }
                    "from" => {
                        from = match arg.to_ascii_lowercase().as_str() {
                            "source" => InstallFrom::Source,
                            "binary" => InstallFrom::Binary,
                            "cran" | "" => InstallFrom::Default,
                            other => {
                                return Err(format!(
                                    "from({other}) is not known; use from(source) or from(binary)."
                                ))
                            }
                        };
                    }
                    "provider" => {
                        if !arg.eq_ignore_ascii_case("cran") {
                            return Err(format!(
                                "provider({arg}) is not available yet; only provider(cran) is."
                            ));
                        }
                    }
                    other => {
                        return Err(format!(
                            "{other} is not an option of rc install; the options are version(), from() and provider()."
                        ));
                    }
                }
            }
            if version.is_some() && packages.len() != 1 {
                return Err(
                    "version() goes with one package: rc install fixest, version(0.11.2)".into(),
                );
            }
            Ok(Rc::Install {
                packages,
                version,
                from,
            })
        }
    }
}

/// R code for `rc installed`: prints the installed packages (those whose name
/// contains `filter`, ignoring case, when given), with the line typed as a
/// comment above it so the log and script show both.
pub fn installed_code(line: &str, filter: &str) -> String {
    let comment = format!("# {}", line.trim());
    let body = if filter.is_empty() {
        "local({ ip <- installed.packages(); print(ip[, c(\"Package\", \"Version\", \"LibPath\"), drop = FALSE], quote = FALSE) })".to_string()
    } else {
        format!(
            "local({{ ip <- installed.packages(); keep <- grepl({}, ip[, \"Package\"], ignore.case = TRUE, fixed = TRUE); \
             print(ip[keep, c(\"Package\", \"Version\", \"LibPath\"), drop = FALSE], quote = FALSE) }})",
            crate::import::r_string(filter)
        )
    };
    format!("{comment}\n{body}")
}

/// Where in the line the word being typed starts, and what it could become.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Completion {
    /// Byte index in the line where the word being completed starts.
    pub from: usize,
    /// The words it could become, sorted.
    pub options: Vec<String>,
}

/// The longest start shared by all of `words` (ignoring nothing: case matters).
pub fn common_prefix(words: &[String]) -> String {
    let Some(first) = words.first() else {
        return String::new();
    };
    let mut end = first.len();
    for w in words {
        end = end.min(
            first
                .char_indices()
                .zip(w.chars())
                .take_while(|((_, a), b)| a == b)
                .map(|((i, a), _)| i + a.len_utf8())
                .last()
                .unwrap_or(0),
        );
    }
    first.get(..end).unwrap_or("").to_string()
}

/// Completions for the line typed so far, when it is an `rc` line. `cran`
/// lists CRAN package names starting with the given text and `installed` the
/// installed ones; both are asked only when needed.
pub fn complete(
    line: &str,
    cran: &dyn Fn(&str) -> Vec<String>,
    installed: &dyn Fn(&str) -> Vec<String>,
) -> Option<Completion> {
    let lead = line.len() - line.trim_start().len();
    let t = line.get(lead..)?;
    let rest = t.strip_prefix("rc")?;
    if !rest.starts_with(char::is_whitespace) {
        return None;
    }
    // the word being typed is everything after the last space or comma
    let at = line
        .rfind(|c: char| c.is_whitespace() || c == ',')
        .map_or(0, |i| i + 1);
    let prefix = line.get(at..)?;
    let before = line.get(..at)?;
    let words: Vec<&str> = before.split_whitespace().collect();
    let mut options: Vec<String> = match words.as_slice() {
        [_rc] => COMMANDS
            .iter()
            .filter(|c| c.starts_with(prefix))
            .map(|c| (*c).to_string())
            .collect(),
        [_, command, ..] => {
            let after_comma = before.contains(',');
            match *command {
                "install" if after_comma => ["version(", "from(", "provider("]
                    .iter()
                    .filter(|o| o.starts_with(prefix))
                    .map(|o| (*o).to_string())
                    .collect(),
                "install" | "describe" if is_package_name(prefix) => cran(prefix),
                "remove" | "update" if is_package_name(prefix) => installed(prefix),
                _ => Vec::new(),
            }
        }
        [] => Vec::new(),
    };
    options.sort();
    options.dedup();
    options.truncate(60);
    Some(Completion { from: at, options })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ok(line: &str) -> Rc {
        match parse(line) {
            Some(Ok(rc)) => rc,
            other => panic!("{line:?} gave {other:?}"),
        }
    }

    fn err(line: &str) -> String {
        match parse(line) {
            Some(Err(e)) => e,
            other => panic!("{line:?} gave {other:?}"),
        }
    }

    #[test]
    fn install_takes_names_and_options() {
        assert_eq!(
            ok("rc install fixest"),
            Rc::Install {
                packages: vec!["fixest".into()],
                version: None,
                from: InstallFrom::Default
            }
        );
        assert_eq!(
            ok("  rc  install  dplyr tidyr,  from(source) provider(CRAN) "),
            Rc::Install {
                packages: vec!["dplyr".into(), "tidyr".into()],
                version: None,
                from: InstallFrom::Source
            }
        );
        assert_eq!(
            ok("rc install fixest, version(0.11.2)"),
            Rc::Install {
                packages: vec!["fixest".into()],
                version: Some("0.11.2".into()),
                from: InstallFrom::Default
            }
        );
        assert!(matches!(
            ok("rc install a, from(binary)"),
            Rc::Install {
                from: InstallFrom::Binary,
                ..
            }
        ));
    }

    #[test]
    fn the_other_forms_read() {
        assert_eq!(
            ok("rc search panel data"),
            Rc::Search {
                words: "panel data".into()
            }
        );
        assert_eq!(
            ok("rc describe fixest"),
            Rc::Describe {
                package: "fixest".into()
            }
        );
        assert_eq!(ok("rc update"), Rc::Update { packages: vec![] });
        assert_eq!(
            ok("rc update a b"),
            Rc::Update {
                packages: vec!["a".into(), "b".into()]
            }
        );
        assert_eq!(
            ok("rc installed"),
            Rc::Installed {
                filter: String::new()
            }
        );
        assert_eq!(
            ok("rc installed  mod"),
            Rc::Installed {
                filter: "mod".into()
            }
        );
        assert_eq!(
            ok("rc remove zoo"),
            Rc::Remove {
                packages: vec!["zoo".into()]
            }
        );
        assert_eq!(ok("rc"), Rc::Help);
        assert_eq!(ok("rc help"), Rc::Help);
    }

    #[test]
    fn mistakes_are_said_in_words() {
        assert!(err("rc install").contains("needs a package name"));
        assert!(err("rc install fi xest!").contains("not a package name"));
        assert!(err("rc install a b, version(1.0)").contains("one package"));
        assert!(err("rc install a, version(1/../2)").contains("not a version"));
        assert!(err("rc install a, provider(github)").contains("not available yet"));
        assert!(err("rc install a, from(cd)").contains("not known"));
        assert!(err("rc install a, speed(3)").contains("not an option"));
        assert!(err("rc install a, from(source").contains("not closed"));
        assert!(err("rc search").contains("some words"));
        assert!(err("rc describe a b").contains("one package"));
        assert!(err("rc remove").contains("needs a package name"));
    }

    #[test]
    fn r_code_that_happens_to_start_with_rc_goes_to_r() {
        for line in [
            "rc <- 3",
            "rc(1, 2)",
            "rc$x",
            "rcx install",
            "rc = 5",
            "rc[1]",
            "rcparams",
            "print(rc)",
            "rc foo",
        ] {
            assert!(parse(line).is_none(), "{line:?} should be left to R");
        }
    }

    #[test]
    fn installed_becomes_r_with_the_typed_line_above_it() {
        let all = installed_code("rc installed", "");
        assert!(all.starts_with("# rc installed\n"));
        assert!(all.contains("installed.packages()"));
        let some = installed_code("rc installed a\"b", "a\"b");
        assert!(some.contains("fixed = TRUE"));
        assert!(some.contains("\"a\\\"b\""), "{some}");
    }

    #[test]
    fn tab_completes_commands_package_names_and_options() {
        let cran = |p: &str| {
            ["fixest", "fixerr", "zoo"]
                .iter()
                .filter(|n| n.starts_with(p))
                .map(|n| (*n).to_string())
                .collect()
        };
        let inst = |p: &str| {
            ["dplyr", "data.table"]
                .iter()
                .filter(|n| n.starts_with(p))
                .map(|n| (*n).to_string())
                .collect()
        };
        let c = complete("rc ins", &cran, &inst);
        assert_eq!(
            c,
            Some(Completion {
                from: 3,
                options: vec!["install".into(), "installed".into()]
            })
        );
        let c = complete("rc install fix", &cran, &inst);
        assert_eq!(
            c,
            Some(Completion {
                from: 11,
                options: vec!["fixerr".into(), "fixest".into()]
            })
        );
        assert_eq!(
            common_prefix(&["fixerr".to_string(), "fixest".to_string()]),
            "fixe"
        );
        let c = complete("rc install a, v", &cran, &inst);
        assert_eq!(c.map(|c| c.options), Some(vec!["version(".to_string()]));
        let c = complete("rc remove d", &cran, &inst);
        assert_eq!(
            c.map(|c| c.options),
            Some(vec!["data.table".to_string(), "dplyr".to_string()])
        );
        // other lines are not completed here
        assert_eq!(complete("plot(", &cran, &inst), None);
        assert_eq!(complete("rcx", &cran, &inst), None);
        // nothing known after search
        assert_eq!(
            complete("rc search pa", &cran, &inst).map(|c| c.options),
            Some(vec![])
        );
    }

    #[test]
    fn the_common_start_of_words_is_found_whatever_their_characters() {
        let w = |l: &[&str]| l.iter().map(|s| (*s).to_string()).collect::<Vec<_>>();
        assert_eq!(common_prefix(&w(&["abc", "abd"])), "ab");
        assert_eq!(common_prefix(&w(&["abc"])), "abc");
        assert_eq!(common_prefix(&w(&["x", "y"])), "");
        assert_eq!(common_prefix(&w(&["é1", "é2"])), "é");
        assert_eq!(common_prefix(&[]), "");
    }
}
