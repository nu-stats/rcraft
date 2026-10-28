//! What an install will do, worked out before anything runs: the packages
//! asked for, the ones they need that are missing or too old, and what cannot
//! be done (a name CRAN does not list, a needed package that was removed).
//!
//! R does the installing and does its own resolving; this is so the user can
//! see the list and say yes (AGENTS.md: every install shows what it will do
//! and waits for a clear yes), and so problems are named before a long
//! download, not after it.

use std::cmp::Ordering;
use std::collections::{HashMap, HashSet, VecDeque};

use crate::describe::is_base_package;
use crate::index::{Dependency, Index};

/// Most packages one plan will list.
pub const MAX_STEPS: usize = 2000;

/// Compares two versions the way R does: split at dots and dashes, compare the
/// parts as numbers, a missing part counting as lower than any number.
pub fn compare_versions(a: &str, b: &str) -> Ordering {
    fn parts(v: &str) -> Vec<u64> {
        v.split(['.', '-'])
            .map(|p| p.parse::<u64>().unwrap_or(0))
            .collect()
    }
    let (pa, pb) = (parts(a), parts(b));
    for i in 0..pa.len().max(pb.len()) {
        match (pa.get(i), pb.get(i)) {
            (Some(x), Some(y)) => match x.cmp(y) {
                Ordering::Equal => {}
                other => return other,
            },
            (Some(_), None) => return Ordering::Greater,
            (None, Some(_)) => return Ordering::Less,
            (None, None) => {}
        }
    }
    Ordering::Equal
}

/// Whether `version` meets a condition such as `>= 1.0`. A condition this does
/// not understand is taken as met (R will say if it is not).
pub fn satisfies(version: &str, constraint: &str) -> bool {
    let c = constraint.trim();
    let (op, wanted) = if let Some(r) = c.strip_prefix(">=") {
        (">=", r)
    } else if let Some(r) = c.strip_prefix("<=") {
        ("<=", r)
    } else if let Some(r) = c.strip_prefix("==") {
        ("==", r)
    } else if let Some(r) = c.strip_prefix('>') {
        (">", r)
    } else if let Some(r) = c.strip_prefix('<') {
        ("<", r)
    } else {
        return true;
    };
    let wanted = wanted.trim();
    if wanted.is_empty() {
        return true;
    }
    let ord = compare_versions(version, wanted);
    match op {
        ">=" => ord != Ordering::Less,
        "<=" => ord != Ordering::Greater,
        "==" => ord == Ordering::Equal,
        ">" => ord == Ordering::Greater,
        _ => ord == Ordering::Less,
    }
}

/// Why a package is in the plan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Why {
    /// The user asked for it.
    Asked,
    /// Another package in the plan needs it.
    NeededBy(String),
}

/// One package to install.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Step {
    pub name: String,
    /// The version CRAN offers.
    pub version: String,
    /// The version installed now, when this replaces one.
    pub replaces: Option<String>,
    pub why: Why,
    /// Whether building from source needs a compiler. A binary needs none.
    pub needs_compilation: Option<bool>,
}

/// Something that stops or may spoil the install.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Problem {
    /// Not in the CRAN list; `similar` is a name that differs only in case.
    NotFound {
        name: String,
        similar: Option<String>,
    },
    /// Needed by `by` but not in the CRAN list (it was removed, or is on another
    /// repository).
    MissingDependency { name: String, by: String },
    /// `by` needs a newer version of `name` than CRAN offers.
    TooOld {
        name: String,
        needed: String,
        offered: String,
        by: String,
    },
}

impl Problem {
    pub fn message(&self) -> String {
        match self {
            Problem::NotFound {
                name,
                similar: Some(s),
            } => format!("There is no package \"{name}\" on CRAN. Did you mean \"{s}\"?"),
            Problem::NotFound { name, .. } => format!("There is no package \"{name}\" on CRAN."),
            Problem::MissingDependency { name, by } => {
                format!("{by} needs \"{name}\", which is not on CRAN (it may have been removed).")
            }
            Problem::TooOld {
                name,
                needed,
                offered,
                by,
            } => format!("{by} needs {name} {needed}, but CRAN offers {offered}."),
        }
    }
}

/// The result of planning.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Plan {
    /// What would be installed: the packages asked for first, then what they
    /// need, each group sorted by name.
    pub steps: Vec<Step>,
    /// Asked for and already installed at this version or newer: `(name, version)`.
    pub already: Vec<(String, String)>,
    pub problems: Vec<Problem>,
    /// More packages were needed than the plan lists.
    pub cut_short: bool,
}

impl Plan {
    /// Whether the install can go ahead: something to install and nothing in
    /// the way.
    pub fn can_run(&self) -> bool {
        !self.steps.is_empty() && self.problems.is_empty() && !self.cut_short
    }

    /// The names to hand to the install call: only the packages asked for, since
    /// R fetches what they need.
    pub fn asked(&self) -> Vec<&str> {
        self.steps
            .iter()
            .filter(|s| s.why == Why::Asked)
            .map(|s| s.name.as_str())
            .collect()
    }

    /// Names that may need a compiler when no binary is offered.
    pub fn may_compile(&self) -> Vec<&str> {
        self.steps
            .iter()
            .filter(|s| s.needs_compilation == Some(true))
            .map(|s| s.name.as_str())
            .collect()
    }
}

/// Plans installing `requested` given what is `installed` (name to version).
pub fn plan(index: &Index, installed: &HashMap<String, String>, requested: &[&str]) -> Plan {
    let mut out = Plan::default();
    let mut asked: Vec<Step> = Vec::new();
    let mut needed: Vec<Step> = Vec::new();
    let mut seen: HashSet<String> = HashSet::new();
    // (dependency to look at, the package that needs it)
    let mut queue: VecDeque<(Dependency, String)> = VecDeque::new();

    for name in requested {
        if !seen.insert((*name).to_string()) {
            continue;
        }
        let Some(p) = index.get(name) else {
            out.problems.push(Problem::NotFound {
                name: (*name).to_string(),
                similar: index.get_ignoring_case(name).map(|p| p.name.clone()),
            });
            continue;
        };
        match installed.get(&p.name) {
            Some(v) if compare_versions(v, &p.version) != Ordering::Less => {
                out.already.push((p.name.clone(), v.clone()));
                continue;
            }
            _ => {}
        }
        asked.push(Step {
            name: p.name.clone(),
            version: p.version.clone(),
            replaces: installed.get(&p.name).cloned(),
            why: Why::Asked,
            needs_compilation: p.needs_compilation,
        });
        for d in p.hard_dependencies() {
            queue.push_back((d, p.name.clone()));
        }
    }

    while let Some((dep, by)) = queue.pop_front() {
        if is_base_package(&dep.name) {
            continue;
        }
        let have = installed.get(&dep.name);
        let met = |v: &str| dep.constraint.as_deref().is_none_or(|c| satisfies(v, c));
        if have.is_some_and(|v| met(v)) {
            continue;
        }
        // already in the plan (asked for, or needed by something else)?
        let in_plan = |s: &Step| s.name == dep.name;
        if let Some(s) = asked.iter().chain(needed.iter()).find(|s| in_plan(s)) {
            if let Some(c) = &dep.constraint {
                if !satisfies(&s.version, c) {
                    out.problems.push(Problem::TooOld {
                        name: dep.name.clone(),
                        needed: c.clone(),
                        offered: s.version.clone(),
                        by,
                    });
                }
            }
            continue;
        }
        let Some(p) = index.get(&dep.name) else {
            if !out
                .problems
                .iter()
                .any(|q| matches!(q, Problem::MissingDependency { name, .. } if *name == dep.name))
            {
                out.problems.push(Problem::MissingDependency {
                    name: dep.name.clone(),
                    by,
                });
            }
            continue;
        };
        if let Some(c) = &dep.constraint {
            if !satisfies(&p.version, c) {
                out.problems.push(Problem::TooOld {
                    name: dep.name.clone(),
                    needed: c.clone(),
                    offered: p.version.clone(),
                    by: by.clone(),
                });
            }
        }
        if asked.len() + needed.len() >= MAX_STEPS {
            out.cut_short = true;
            break;
        }
        needed.push(Step {
            name: p.name.clone(),
            version: p.version.clone(),
            replaces: have.cloned(),
            why: Why::NeededBy(by),
            needs_compilation: p.needs_compilation,
        });
        for d in p.hard_dependencies() {
            queue.push_back((d, p.name.clone()));
        }
    }

    asked.sort_by_key(|s| s.name.to_lowercase());
    needed.sort_by_key(|s| s.name.to_lowercase());
    out.steps = asked;
    out.steps.extend(needed);
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::index::{Detail, Package};

    fn p(name: &str, version: &str, depends: &str, imports: &str, compiled: bool) -> Package {
        Package {
            name: name.into(),
            version: version.into(),
            depends: depends.into(),
            imports: imports.into(),
            needs_compilation: Some(compiled),
            ..Package::default()
        }
    }

    fn index() -> Index {
        Index::new(
            vec![
                p(
                    "app",
                    "2.0",
                    "R (>= 4.0), core",
                    "stats, util (>= 1.2), cpp",
                    false,
                ),
                p("core", "1.5", "", "util", false),
                p("util", "1.3-1", "", "", false),
                p("cpp", "0.9", "", "", true),
                p("old", "1.0", "", "util (>= 9.0)", false),
                p("lost", "1.0", "", "vanished", false),
                p("Mixed", "1.0", "", "", false),
            ],
            0,
            Detail::Full,
        )
    }

    fn have(items: &[(&str, &str)]) -> HashMap<String, String> {
        items
            .iter()
            .map(|(n, v)| ((*n).to_string(), (*v).to_string()))
            .collect()
    }

    fn names(steps: &[Step]) -> Vec<&str> {
        steps.iter().map(|s| s.name.as_str()).collect()
    }

    #[test]
    fn versions_compare_the_way_r_does() {
        assert_eq!(compare_versions("1.2-3", "1.2.3"), Ordering::Equal);
        assert_eq!(compare_versions("1.10", "1.9"), Ordering::Greater);
        assert_eq!(compare_versions("1.0", "1.0.0"), Ordering::Less);
        assert_eq!(compare_versions("0.12.1", "0.12.10"), Ordering::Less);
        assert!(satisfies("1.3-1", ">= 1.2"));
        assert!(!satisfies("1.1", ">= 1.2"));
        assert!(satisfies("1.2", "== 1.2"));
        assert!(satisfies("1.2", "< 2"));
        assert!(!satisfies("2.0", "> 2.0"));
        assert!(satisfies("1.0", "weird"));
    }

    #[test]
    fn everything_missing_is_listed_with_who_needs_it() {
        let pl = plan(&index(), &have(&[("stats", "4.4.3")]), &["app"]);
        assert_eq!(names(&pl.steps), vec!["app", "core", "cpp", "util"]);
        assert!(pl.problems.is_empty());
        assert!(pl.can_run());
        assert_eq!(pl.asked(), vec!["app"]);
        assert_eq!(pl.steps[0].why, Why::Asked);
        let util = pl.steps.iter().find(|s| s.name == "util");
        // asked for by app (util >= 1.2) and core: the first to need it is named
        assert!(matches!(util.map(|s| &s.why), Some(Why::NeededBy(by)) if by == "app"));
        assert_eq!(pl.may_compile(), vec!["cpp"]);
    }

    #[test]
    fn what_is_installed_and_new_enough_is_left_out() {
        let pl = plan(
            &index(),
            &have(&[("core", "1.5"), ("util", "1.2"), ("cpp", "1.0")]),
            &["app"],
        );
        assert_eq!(names(&pl.steps), vec!["app"]);
        let pl = plan(
            &index(),
            &have(&[("util", "1.1"), ("core", "1.5"), ("cpp", "0.9")]),
            &["app"],
        );
        // util 1.1 is too old for app's util (>= 1.2): it is updated
        assert_eq!(names(&pl.steps), vec!["app", "util"]);
        assert_eq!(pl.steps[1].replaces.as_deref(), Some("1.1"));
    }

    #[test]
    fn asking_for_what_is_already_there_installs_nothing() {
        let pl = plan(&index(), &have(&[("core", "1.6")]), &["core"]);
        assert!(pl.steps.is_empty());
        assert_eq!(pl.already, vec![("core".to_string(), "1.6".to_string())]);
        assert!(!pl.can_run());
        // an older one is replaced
        let pl = plan(&index(), &have(&[("core", "1.4")]), &["core"]);
        assert_eq!(pl.steps[0].replaces.as_deref(), Some("1.4"));
    }

    #[test]
    fn problems_are_named_before_anything_runs() {
        let pl = plan(&index(), &have(&[]), &["nope", "mixed", "lost", "old"]);
        assert!(!pl.can_run());
        let msgs: Vec<String> = pl.problems.iter().map(Problem::message).collect();
        assert!(
            msgs.contains(&"There is no package \"nope\" on CRAN.".to_string()),
            "{msgs:?}"
        );
        assert!(
            msgs.iter().any(|m| m.contains("Did you mean \"Mixed\"")),
            "{msgs:?}"
        );
        assert!(
            msgs.iter().any(|m| m.contains("lost needs \"vanished\"")),
            "{msgs:?}"
        );
        assert!(
            msgs.iter()
                .any(|m| m.contains("old needs util 9.0 ") || m.contains("old needs util >= 9.0")),
            "{msgs:?}"
        );
    }

    #[test]
    fn cycles_and_repeats_do_not_loop() {
        let ix = Index::new(
            vec![p("a", "1", "", "b", false), p("b", "1", "", "a", false)],
            0,
            Detail::Full,
        );
        let pl = plan(&ix, &have(&[]), &["a", "a", "b"]);
        assert_eq!(names(&pl.steps), vec!["a", "b"]);
    }

    #[test]
    fn a_huge_chain_is_cut_short() {
        let n = MAX_STEPS + 50;
        let ps: Vec<Package> = (0..n)
            .map(|i| p(&format!("p{i}"), "1", "", &format!("p{}", i + 1), false))
            .collect();
        let ix = Index::new(ps, 0, Detail::Full);
        let pl = plan(&ix, &have(&[]), &["p0"]);
        assert!(pl.cut_short);
        assert!(pl.steps.len() <= MAX_STEPS);
        assert!(!pl.can_run());
    }
}
