//! The package list of a project: which packages, at which versions, a session
//! used, saved in a small text file so the same set can be installed again on
//! another day or another computer.
//!
//! ```text
//! # RCraft package list (version 1)
//! r<TAB>4.4.3
//! mirror<TAB>https://cloud.r-project.org
//! saved<TAB>2026-10-28
//! package<TAB>dplyr<TAB>1.1.4<TAB>cran
//! package<TAB>stats<TAB>4.4.3<TAB>base
//! package<TAB>mine<TAB>0.1<TAB>other
//! ```
//!
//! `cran` is a package installed from CRAN (it can be installed again), `base`
//! comes with R, and `other` is anything else (GitHub, a local file) which RCraft
//! records but cannot restore. The reader is strict and bounded: a file that is
//! not in this form, or is too large, is an error and nothing is half-read.
//! Package metadata is data, never instructions; nothing here runs it.

use std::cmp::Ordering;
use std::collections::HashMap;

use crate::index::{is_package_name, Index};
use crate::plan::compare_versions;
use crate::refresh::validate_mirror;

/// The first line of every list.
pub const HEADER: &str = "# RCraft package list (version 1)";
/// Largest list text read, in bytes.
pub const MAX_BYTES: usize = 2 * 1024 * 1024;
/// Most packages in a list.
pub const MAX_PACKAGES: usize = 5000;

/// Where a recorded package came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LockSource {
    Cran,
    Base,
    Other,
}

impl LockSource {
    fn word(self) -> &'static str {
        match self {
            LockSource::Cran => "cran",
            LockSource::Base => "base",
            LockSource::Other => "other",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Locked {
    pub name: String,
    pub version: String,
    pub source: LockSource,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Lock {
    /// The R the list was made with, such as `4.4.3`.
    pub r_version: String,
    /// The mirror the packages came from.
    pub mirror: String,
    /// The day it was saved (`YYYY-MM-DD`), as given by the caller.
    pub saved: String,
    /// Sorted by name, ignoring case.
    pub packages: Vec<Locked>,
}

/// A version as CRAN writes it.
fn is_version(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 64
        && s.chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '-')
}

fn is_day(s: &str) -> bool {
    s.is_empty() || (s.len() <= 10 && s.chars().all(|c| c.is_ascii_digit() || c == '-'))
}

/// What the caller knows about one installed package.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    pub name: String,
    pub version: String,
    /// R's `Priority` field: `base`, `recommended` or empty.
    pub priority: String,
    /// R's `Repository` field (`CRAN` for packages installed from CRAN).
    pub repository: String,
}

/// Where an installed package came from, from what R records about it.
pub fn source_of(entry: &Entry) -> LockSource {
    if entry.priority == "base" {
        LockSource::Base
    } else if matches!(entry.repository.as_str(), "CRAN" | "RSPM" | "P3M")
        || entry.priority == "recommended"
    {
        LockSource::Cran
    } else {
        LockSource::Other
    }
}

impl Lock {
    /// A list of `entries`, taking those whose names are in `only` when given.
    pub fn from_entries(
        entries: &[Entry],
        only: Option<&[String]>,
        r_version: &str,
        mirror: &str,
        saved: &str,
    ) -> Lock {
        let mut packages: Vec<Locked> = entries
            .iter()
            .filter(|e| only.is_none_or(|names| names.contains(&e.name)))
            .filter(|e| is_package_name(&e.name) && is_version(&e.version))
            .map(|e| Locked {
                name: e.name.clone(),
                version: e.version.clone(),
                source: source_of(e),
            })
            .collect();
        packages.sort_by_key(|p| p.name.to_lowercase());
        packages.dedup_by(|b, a| a.name == b.name);
        packages.truncate(MAX_PACKAGES);
        Lock {
            r_version: r_version.to_string(),
            mirror: mirror.to_string(),
            saved: saved.to_string(),
            packages,
        }
    }

    /// The file text.
    pub fn to_text(&self) -> String {
        let mut out = format!("{HEADER}\n");
        out.push_str(&format!("r\t{}\n", self.r_version));
        out.push_str(&format!("mirror\t{}\n", self.mirror));
        out.push_str(&format!("saved\t{}\n", self.saved));
        for p in &self.packages {
            out.push_str(&format!(
                "package\t{}\t{}\t{}\n",
                p.name,
                p.version,
                p.source.word()
            ));
        }
        out
    }

    /// Reads a list. Every line is checked; the first thing wrong is the error,
    /// with its line number.
    pub fn parse(text: &str) -> Result<Lock, String> {
        if text.len() > MAX_BYTES {
            return Err(format!("The list is larger than {MAX_BYTES} bytes."));
        }
        let mut lines = text.lines();
        if lines.next().map(str::trim_end) != Some(HEADER) {
            return Err("This is not an RCraft package list (the first line is wrong).".into());
        }
        let mut lock = Lock::default();
        let mut seen: HashMap<String, usize> = HashMap::new();
        for (i, raw) in lines.enumerate() {
            let n = i + 2;
            let line = raw.trim_end_matches('\r');
            if line.trim().is_empty() || line.starts_with('#') {
                continue;
            }
            let f: Vec<&str> = line.split('\t').collect();
            match f.as_slice() {
                ["r", v] => {
                    if !is_version(v) {
                        return Err(format!("Line {n}: \"{v}\" is not an R version."));
                    }
                    lock.r_version = (*v).to_string();
                }
                ["mirror", m] => {
                    lock.mirror = validate_mirror(m).map_err(|e| format!("Line {n}: {e}"))?;
                }
                ["saved", d] => {
                    if !is_day(d) {
                        return Err(format!("Line {n}: \"{d}\" is not a date."));
                    }
                    lock.saved = (*d).to_string();
                }
                ["package", name, version, source] => {
                    if !is_package_name(name) {
                        return Err(format!("Line {n}: \"{name}\" is not a package name."));
                    }
                    if !is_version(version) {
                        return Err(format!("Line {n}: \"{version}\" is not a version."));
                    }
                    let source = match *source {
                        "cran" => LockSource::Cran,
                        "base" => LockSource::Base,
                        "other" => LockSource::Other,
                        other => {
                            return Err(format!("Line {n}: \"{other}\" is not a source."));
                        }
                    };
                    if let Some(first) = seen.insert((*name).to_string(), n) {
                        return Err(format!(
                            "Line {n}: \"{name}\" is listed twice (first on line {first})."
                        ));
                    }
                    if lock.packages.len() >= MAX_PACKAGES {
                        return Err(format!("The list has more than {MAX_PACKAGES} packages."));
                    }
                    lock.packages.push(Locked {
                        name: (*name).to_string(),
                        version: (*version).to_string(),
                        source,
                    });
                }
                _ => return Err(format!("Line {n} is not in the list format.")),
            }
        }
        lock.packages.sort_by_key(|p| p.name.to_lowercase());
        Ok(lock)
    }
}

/// What to do about one package of a list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    /// Installed at the listed version.
    AlreadyThere,
    /// CRAN's current version is the listed one.
    InstallCurrent,
    /// The listed version is older than CRAN's: from the archive.
    InstallOlder,
    /// Cannot be restored, and why.
    Skip(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RestoreStep {
    pub name: String,
    /// The version in the list.
    pub wanted: String,
    /// The version installed now.
    pub have: Option<String>,
    pub action: Action,
}

/// What restoring a list would do.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Restore {
    pub steps: Vec<RestoreStep>,
    /// A caution about the R the list was made with.
    pub r_note: Option<String>,
}

impl Restore {
    /// Packages to install at CRAN's current version.
    pub fn current(&self) -> Vec<&str> {
        self.of(|a| *a == Action::InstallCurrent)
            .map(|s| s.name.as_str())
            .collect()
    }

    /// Packages to install as an older version: `(name, version)`.
    pub fn older(&self) -> Vec<(&str, &str)> {
        self.of(|a| *a == Action::InstallOlder)
            .map(|s| (s.name.as_str(), s.wanted.as_str()))
            .collect()
    }

    fn of<'a>(
        &'a self,
        keep: impl Fn(&Action) -> bool + 'a,
    ) -> impl Iterator<Item = &'a RestoreStep> + 'a {
        self.steps.iter().filter(move |s| keep(&s.action))
    }

    /// Whether there is anything to install.
    pub fn can_run(&self) -> bool {
        self.steps
            .iter()
            .any(|s| matches!(s.action, Action::InstallCurrent | Action::InstallOlder))
    }
}

/// What it would take to get the packages of `lock` onto this computer, given
/// what is `installed` (name to version) and CRAN's `index` when it has been
/// downloaded. `current_r` is this R's version, for the note.
pub fn restore_plan(
    index: Option<&Index>,
    installed: &HashMap<String, String>,
    lock: &Lock,
    current_r: &str,
) -> Restore {
    let mut steps = Vec::with_capacity(lock.packages.len());
    for p in &lock.packages {
        let have = installed.get(&p.name).cloned();
        let same = have
            .as_deref()
            .is_some_and(|h| compare_versions(h, &p.version) == Ordering::Equal);
        let action = if same {
            Action::AlreadyThere
        } else {
            match p.source {
                LockSource::Base => Action::Skip("comes with R".to_string()),
                LockSource::Other => Action::Skip(
                    "was not installed from CRAN, so RCraft cannot install it again".to_string(),
                ),
                LockSource::Cran => match index {
                    None => Action::Skip("download the package list to install this".to_string()),
                    Some(ix) => match ix.get(&p.name) {
                        None => Action::Skip("is no longer on CRAN".to_string()),
                        Some(c) => match compare_versions(&p.version, &c.version) {
                            Ordering::Equal => Action::InstallCurrent,
                            Ordering::Less => Action::InstallOlder,
                            Ordering::Greater => Action::Skip(format!(
                                "CRAN offers {}, which is older than the listed {}",
                                c.version, p.version
                            )),
                        },
                    },
                },
            }
        };
        steps.push(RestoreStep {
            name: p.name.clone(),
            wanted: p.version.clone(),
            have,
            action,
        });
    }
    let minor = |v: &str| v.split('.').take(2).collect::<Vec<_>>().join(".");
    let r_note = (!lock.r_version.is_empty()
        && !current_r.is_empty()
        && minor(&lock.r_version) != minor(current_r))
    .then(|| {
        format!(
            "The list was made with R {}; this is R {current_r}. Packages from the list may \
             not all work with this R.",
            lock.r_version
        )
    });
    Restore { steps, r_note }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::index::{Detail, Package};

    fn entry(name: &str, version: &str, priority: &str, repository: &str) -> Entry {
        Entry {
            name: name.into(),
            version: version.into(),
            priority: priority.into(),
            repository: repository.into(),
        }
    }

    fn installed() -> Vec<Entry> {
        vec![
            entry("stats", "4.4.3", "base", ""),
            entry("dplyr", "1.1.4", "", "CRAN"),
            entry("MASS", "7.3-60", "recommended", ""),
            entry("mine", "0.1", "", ""),
            entry("Rcpp", "1.0.14", "", "RSPM"),
        ]
    }

    #[test]
    fn a_list_records_each_packages_source_and_round_trips() {
        let lock = Lock::from_entries(
            &installed(),
            None,
            "4.4.3",
            "https://cloud.r-project.org",
            "2026-10-28",
        );
        let names: Vec<&str> = lock.packages.iter().map(|p| p.name.as_str()).collect();
        assert_eq!(names, vec!["dplyr", "MASS", "mine", "Rcpp", "stats"]);
        let src: Vec<LockSource> = lock.packages.iter().map(|p| p.source).collect();
        assert_eq!(
            src,
            vec![
                LockSource::Cran,
                LockSource::Cran,
                LockSource::Other,
                LockSource::Cran,
                LockSource::Base
            ]
        );
        let text = lock.to_text();
        assert!(text.starts_with("# RCraft package list (version 1)\nr\t4.4.3\n"));
        assert!(text.contains("package\tdplyr\t1.1.4\tcran\n"));
        assert_eq!(Lock::parse(&text), Ok(lock));
    }

    #[test]
    fn only_the_chosen_packages_are_listed() {
        let only = vec!["dplyr".to_string(), "stats".to_string(), "nope".to_string()];
        let lock = Lock::from_entries(&installed(), Some(&only), "4.4.3", "https://x.org", "");
        assert_eq!(lock.packages.len(), 2);
    }

    #[test]
    fn a_bad_list_is_an_error_naming_the_line() {
        let ok = "# RCraft package list (version 1)\nr\t4.4.3\n";
        for (text, want) in [
            ("hello", "first line"),
            ("", "first line"),
            ("# RCraft package list (version 2)\n", "first line"),
            (&format!("{ok}package\ta b\t1.0\tcran\n"), "Line 3"),
            (&format!("{ok}package\tx\t1.0 beta\tcran\n"), "Line 3"),
            (&format!("{ok}package\tx\t1.0\tgithub\n"), "Line 3"),
            (&format!("{ok}package\tx\t1.0\n"), "Line 3"),
            (&format!("{ok}mirror\thttp://insecure.org\n"), "Line 3"),
            (&format!("{ok}mystery\tx\n"), "Line 3"),
            (
                &format!("{ok}package\tx\t1.0\tcran\npackage\tx\t2.0\tcran\n"),
                "Line 4",
            ),
            (&format!("{ok}r\t4.4;rm\n"), "Line 3"),
        ] {
            let e = Lock::parse(text);
            assert!(
                e.as_ref().is_err_and(|m| m.contains(want)),
                "{text:?}: {e:?}"
            );
        }
        assert!(Lock::parse(&format!("{ok}{}", "#".repeat(MAX_BYTES))).is_err());
    }

    #[test]
    fn comments_blank_lines_and_crlf_are_tolerated_and_the_size_is_capped() {
        let text = "# RCraft package list (version 1)\r\n\r\n# a note\r\nr\t4.4.3\r\npackage\tb\t1.0\tcran\r\npackage\tA\t2.0\tcran\r\n";
        let lock = Lock::parse(text);
        let names: Vec<String> = lock
            .map(|l| l.packages.into_iter().map(|p| p.name).collect())
            .unwrap_or_default();
        assert_eq!(names, vec!["A", "b"]);
        let mut many = String::from("# RCraft package list (version 1)\n");
        for i in 0..=MAX_PACKAGES {
            many.push_str(&format!("package\tp{i}\t1.0\tcran\n"));
        }
        assert!(Lock::parse(&many).is_err_and(|e| e.contains("more than")));
    }

    fn index() -> Index {
        let p = |n: &str, v: &str| Package {
            name: n.into(),
            version: v.into(),
            ..Package::default()
        };
        Index::new(
            vec![p("dplyr", "1.1.4"), p("Rcpp", "1.1.2"), p("gone", "")],
            0,
            Detail::Full,
        )
    }

    #[test]
    fn restoring_installs_missing_current_and_older_versions_and_says_what_it_cannot() {
        let lock = Lock {
            r_version: "4.4.3".into(),
            mirror: "https://x.org".into(),
            saved: String::new(),
            packages: vec![
                Locked {
                    name: "dplyr".into(),
                    version: "1.1.4".into(),
                    source: LockSource::Cran,
                },
                Locked {
                    name: "Rcpp".into(),
                    version: "1.0.14".into(),
                    source: LockSource::Cran,
                },
                Locked {
                    name: "mine".into(),
                    version: "0.1".into(),
                    source: LockSource::Other,
                },
                Locked {
                    name: "stats".into(),
                    version: "4.4.3".into(),
                    source: LockSource::Base,
                },
                Locked {
                    name: "vanished".into(),
                    version: "1.0".into(),
                    source: LockSource::Cran,
                },
                Locked {
                    name: "future".into(),
                    version: "9.0".into(),
                    source: LockSource::Cran,
                },
            ],
        };
        let mut ix_pkgs = index().packages().to_vec();
        ix_pkgs.push(Package {
            name: "future".into(),
            version: "2.0".into(),
            ..Package::default()
        });
        let ix = Index::new(ix_pkgs, 0, Detail::Full);
        let have: HashMap<String, String> = [("Rcpp", "1.1.2"), ("stats", "4.4.3")]
            .iter()
            .map(|(n, v)| ((*n).to_string(), (*v).to_string()))
            .collect();
        let r = restore_plan(Some(&ix), &have, &lock, "4.4.3");
        let by_name: HashMap<&str, &Action> = r
            .steps
            .iter()
            .map(|s| (s.name.as_str(), &s.action))
            .collect();
        assert_eq!(by_name["dplyr"], &Action::InstallCurrent);
        // installed at a different version than listed, and CRAN has moved on
        assert_eq!(by_name["Rcpp"], &Action::InstallOlder);
        assert_eq!(by_name["stats"], &Action::AlreadyThere);
        assert!(
            matches!(by_name["mine"], Action::Skip(m) if m.contains("not installed from CRAN"))
        );
        assert!(matches!(by_name["vanished"], Action::Skip(m) if m.contains("no longer on CRAN")));
        assert!(
            matches!(by_name["future"], Action::Skip(m) if m.contains("older than the listed"))
        );
        assert_eq!(r.current(), vec!["dplyr"]);
        assert_eq!(r.older(), vec![("Rcpp", "1.0.14")]);
        assert!(r.can_run());
        assert_eq!(r.r_note, None);
    }

    #[test]
    fn without_the_package_list_nothing_from_cran_can_be_installed_and_a_different_r_is_noted() {
        let lock = Lock {
            r_version: "4.3.1".into(),
            packages: vec![Locked {
                name: "dplyr".into(),
                version: "1.1.0".into(),
                source: LockSource::Cran,
            }],
            ..Lock::default()
        };
        let r = restore_plan(None, &HashMap::new(), &lock, "4.4.3");
        assert!(
            matches!(&r.steps[0].action, Action::Skip(m) if m.contains("download the package list"))
        );
        assert!(!r.can_run());
        assert!(r
            .r_note
            .as_deref()
            .is_some_and(|n| n.contains("R 4.3.1") && n.contains("R 4.4.3")));
        // the same minor version is not worth a note
        let same = restore_plan(
            None,
            &HashMap::new(),
            &Lock {
                r_version: "4.4.0".into(),
                ..lock
            },
            "4.4.3",
        );
        assert_eq!(same.r_note, None);
    }
}
