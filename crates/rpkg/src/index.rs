//! The package index: every CRAN package with the facts RCraft shows, sorted
//! by name and searched without the network.

use std::cmp::Ordering;
use std::collections::HashMap;

use crate::table::Table;

/// The columns read from `packages.rds`. `Published` is the date CRAN lists;
/// older databases call it `Date/Publication`.
pub const DB_COLUMNS: &[&str] = &[
    "Package",
    "Version",
    "Priority",
    "Depends",
    "Imports",
    "LinkingTo",
    "Suggests",
    "License",
    "NeedsCompilation",
    "Title",
    "Description",
    "Author",
    "Maintainer",
    "URL",
    "BugReports",
    "SystemRequirements",
    "Published",
    "Date/Publication",
    "Reverse depends",
    "Reverse imports",
    "Reverse linking to",
];

/// The fields read from `PACKAGES.gz`.
pub const DCF_FIELDS: &[&str] = &[
    "Package",
    "Version",
    "Priority",
    "Depends",
    "Imports",
    "LinkingTo",
    "Suggests",
    "License",
    "NeedsCompilation",
];

/// Seconds after which a cached index is offered for a refresh: a week.
pub const DEFAULT_MAX_AGE: u64 = 7 * 24 * 60 * 60;

/// One package. A field CRAN does not give is an empty string.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Package {
    pub name: String,
    pub version: String,
    pub priority: String,
    pub title: String,
    pub description: String,
    pub authors: String,
    pub maintainer: String,
    pub license: String,
    pub depends: String,
    pub imports: String,
    pub linking_to: String,
    pub suggests: String,
    /// Whether installing from source needs a compiler; `None` when not stated.
    pub needs_compilation: Option<bool>,
    pub url: String,
    pub bug_reports: String,
    pub system_requirements: String,
    /// The date CRAN published this version, as `YYYY-MM-DD` when known.
    pub published: String,
    /// Packages on CRAN that need this one installed (those that Depend on, Import
    /// or LinkTo it), comma separated; empty when the source does not say.
    pub used_by: String,
}

/// The names of a [`Package`]'s fields in the order [`Package::to_fields`]
/// gives them, which is the order the cache stores them in.
pub const FIELD_NAMES: [&str; 18] = [
    "name",
    "version",
    "priority",
    "title",
    "description",
    "authors",
    "maintainer",
    "license",
    "depends",
    "imports",
    "linking_to",
    "suggests",
    "needs_compilation",
    "url",
    "bug_reports",
    "system_requirements",
    "published",
    "used_by",
];

impl Package {
    /// The fields as text, in [`FIELD_NAMES`] order.
    pub fn to_fields(&self) -> [String; 18] {
        [
            self.name.clone(),
            self.version.clone(),
            self.priority.clone(),
            self.title.clone(),
            self.description.clone(),
            self.authors.clone(),
            self.maintainer.clone(),
            self.license.clone(),
            self.depends.clone(),
            self.imports.clone(),
            self.linking_to.clone(),
            self.suggests.clone(),
            match self.needs_compilation {
                Some(true) => "yes".into(),
                Some(false) => "no".into(),
                None => String::new(),
            },
            self.url.clone(),
            self.bug_reports.clone(),
            self.system_requirements.clone(),
            self.published.clone(),
            self.used_by.clone(),
        ]
    }

    /// The inverse of [`Package::to_fields`]; `None` unless `fields` has
    /// exactly the 18 fields.
    pub fn from_fields(fields: &[String]) -> Option<Package> {
        let [name, version, priority, title, description, authors, maintainer, license, depends, imports, linking_to, suggests, needs, url, bug_reports, system_requirements, published, used_by] =
            fields
        else {
            return None;
        };
        Some(Package {
            name: name.clone(),
            version: version.clone(),
            priority: priority.clone(),
            title: title.clone(),
            description: description.clone(),
            authors: authors.clone(),
            maintainer: maintainer.clone(),
            license: license.clone(),
            depends: depends.clone(),
            imports: imports.clone(),
            linking_to: linking_to.clone(),
            suggests: suggests.clone(),
            needs_compilation: match needs.as_str() {
                "yes" => Some(true),
                "no" => Some(false),
                _ => None,
            },
            url: url.clone(),
            bug_reports: bug_reports.clone(),
            system_requirements: system_requirements.clone(),
            published: published.clone(),
            used_by: used_by.clone(),
        })
    }

    /// The packages this one needs installed first: Depends, Imports and
    /// LinkingTo, without R itself and without repeats.
    pub fn hard_dependencies(&self) -> Vec<Dependency> {
        let mut out: Vec<Dependency> = Vec::new();
        for field in [&self.depends, &self.imports, &self.linking_to] {
            for d in parse_dependencies(field) {
                if !out.iter().any(|o| o.name == d.name) {
                    out.push(d);
                }
            }
        }
        out
    }

    /// Fills each field that is empty here from `other`.
    fn fill_from(&mut self, other: &Package) {
        macro_rules! fill {
            ($($f:ident),*) => { $( if self.$f.is_empty() { self.$f = other.$f.clone(); } )* };
        }
        fill!(
            version,
            priority,
            title,
            description,
            authors,
            maintainer,
            license,
            depends,
            imports,
            linking_to,
            suggests,
            url,
            bug_reports,
            system_requirements,
            published,
            used_by
        );
        if self.needs_compilation.is_none() {
            self.needs_compilation = other.needs_compilation;
        }
    }
}

/// A package named in a dependency field, with its version condition.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Dependency {
    pub name: String,
    /// What is inside the parentheses, such as `>= 1.0`.
    pub constraint: Option<String>,
}

/// Reads a field like `R (>= 3.5), Rcpp (>= 1.0.1), stats`. R itself is
/// dropped; blank items and items that are not package names are skipped.
pub fn parse_dependencies(field: &str) -> Vec<Dependency> {
    field
        .split(',')
        .filter_map(|item| {
            let item = item.trim();
            let (name, rest) = match item.split_once('(') {
                Some((n, r)) => (n.trim(), Some(r)),
                None => (item, None),
            };
            let name = name.split_whitespace().next().unwrap_or("");
            if name.is_empty() || name == "R" || !is_package_name(name) {
                return None;
            }
            let constraint = rest
                .and_then(|r| r.split_once(')'))
                .map(|(c, _)| c.trim().to_string())
                .filter(|c| !c.is_empty());
            Some(Dependency {
                name: name.to_string(),
                constraint,
            })
        })
        .collect()
}

/// R's rule: letters, digits and dots, starting with a letter.
pub fn is_package_name(s: &str) -> bool {
    let mut chars = s.chars();
    matches!(chars.next(), Some(c) if c.is_ascii_alphabetic())
        && s.chars().all(|c| c.is_ascii_alphanumeric() || c == '.')
}

/// How much the index knows about each package.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Detail {
    /// Titles, descriptions and the rest (from `packages.rds`).
    Full,
    /// Names, versions and dependencies only (from `PACKAGES.gz`).
    NamesOnly,
}

/// The packages, sorted by name ignoring case.
#[derive(Debug, Clone, PartialEq)]
pub struct Index {
    packages: Vec<Package>,
    /// When this was fetched, in seconds since 1970.
    pub fetched_at: u64,
    pub detail: Detail,
}

/// Orders names ignoring ASCII case, then by exact bytes.
fn cmp_names(a: &str, b: &str) -> Ordering {
    a.bytes()
        .map(|c| c.to_ascii_lowercase())
        .cmp(b.bytes().map(|c| c.to_ascii_lowercase()))
        .then_with(|| a.cmp(b))
}

fn cmp_ci(a: &str, b: &str) -> Ordering {
    a.bytes()
        .map(|c| c.to_ascii_lowercase())
        .cmp(b.bytes().map(|c| c.to_ascii_lowercase()))
}

impl Index {
    /// An index of `packages`: sorted, with unnamed entries dropped and, for a
    /// name that appears twice, the first kept.
    pub fn new(mut packages: Vec<Package>, fetched_at: u64, detail: Detail) -> Index {
        packages.retain(|p| !p.name.is_empty());
        packages.sort_by(|a, b| cmp_names(&a.name, &b.name));
        packages.dedup_by(|b, a| a.name == b.name);
        Index {
            packages,
            fetched_at,
            detail,
        }
    }

    /// Builds the index from the two source tables. `db` (from
    /// `packages.rds`) is preferred; `dcf` (from `PACKAGES.gz`) fills gaps and
    /// adds packages the database does not list yet. With only `dcf` the
    /// detail is [`Detail::NamesOnly`].
    pub fn from_tables(db: Option<&Table>, dcf: Option<&Table>, fetched_at: u64) -> Index {
        let mut primary = db.map(packages_from).unwrap_or_default();
        let detail = if db.is_some() {
            Detail::Full
        } else {
            Detail::NamesOnly
        };
        let fallback = dcf.map(packages_from).unwrap_or_default();
        let lookup: HashMap<&str, &Package> =
            fallback.iter().map(|p| (p.name.as_str(), p)).collect();
        for p in &mut primary {
            if let Some(f) = lookup.get(p.name.as_str()) {
                p.fill_from(f);
            }
        }
        let known: std::collections::HashSet<String> =
            primary.iter().map(|p| p.name.clone()).collect();
        primary.extend(
            fallback
                .iter()
                .filter(|p| !known.contains(&p.name))
                .cloned(),
        );
        Index::new(primary, fetched_at, detail)
    }

    pub fn len(&self) -> usize {
        self.packages.len()
    }

    pub fn is_empty(&self) -> bool {
        self.packages.is_empty()
    }

    /// Every package, sorted by name.
    pub fn packages(&self) -> &[Package] {
        &self.packages
    }

    /// The package named exactly `name`.
    pub fn get(&self, name: &str) -> Option<&Package> {
        let i = self
            .packages
            .binary_search_by(|p| cmp_names(&p.name, name))
            .ok()?;
        self.packages.get(i)
    }

    /// The package called `name`, ignoring case.
    pub fn get_ignoring_case(&self, name: &str) -> Option<&Package> {
        let i = self
            .packages
            .partition_point(|p| cmp_ci(&p.name, name) == Ordering::Less);
        self.packages
            .get(i)
            .filter(|p| cmp_ci(&p.name, name) == Ordering::Equal)
    }

    /// Packages whose name starts with `prefix`, ignoring case, sorted.
    pub fn with_prefix(&self, prefix: &str) -> &[Package] {
        let lower = prefix.to_ascii_lowercase();
        let starts = |p: &Package| {
            p.name
                .get(..lower.len())
                .is_some_and(|s| s.eq_ignore_ascii_case(&lower))
        };
        let from = self
            .packages
            .partition_point(|p| cmp_ci(&p.name, &lower) == Ordering::Less);
        let len = self
            .packages
            .get(from..)
            .map(|rest| rest.iter().take_while(|p| starts(p)).count())
            .unwrap_or(0);
        self.packages.get(from..from + len).unwrap_or(&[])
    }

    /// Seconds since this index was fetched, given the clock `now`.
    pub fn age(&self, now: u64) -> u64 {
        now.saturating_sub(self.fetched_at)
    }

    /// Whether the index is older than `max_age` seconds at `now`.
    pub fn is_stale(&self, now: u64, max_age: u64) -> bool {
        self.age(now) > max_age
    }
}

/// Collapses runs of whitespace (including newlines) to single spaces.
fn tidy(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// The packages in a table with any of the source columns.
fn packages_from(t: &Table) -> Vec<Package> {
    let mut out = Vec::with_capacity(t.rows);
    for row in 0..t.rows {
        let text = |c: &str| t.get(c, row).map(tidy).unwrap_or_default();
        let name = text("Package");
        if name.is_empty() {
            continue;
        }
        let published = {
            let p = text("Published");
            let p = if p.is_empty() {
                text("Date/Publication")
            } else {
                p
            };
            // a date and time becomes the date
            p.split_whitespace().next().unwrap_or("").to_string()
        };
        out.push(Package {
            name,
            version: text("Version"),
            priority: text("Priority"),
            title: text("Title"),
            description: text("Description"),
            authors: text("Author"),
            maintainer: text("Maintainer"),
            license: text("License"),
            depends: text("Depends"),
            imports: text("Imports"),
            linking_to: text("LinkingTo"),
            suggests: text("Suggests"),
            needs_compilation: match text("NeedsCompilation").to_ascii_lowercase().as_str() {
                "yes" => Some(true),
                "no" => Some(false),
                _ => None,
            },
            url: text("URL"),
            bug_reports: text("BugReports"),
            system_requirements: text("SystemRequirements"),
            published,
            used_by: used_by(t, row),
        });
    }
    out
}

/// A small made-up index for tests and examples; it never contacts anything.
pub fn sample() -> Index {
    let p = |name: &str, version: &str, title: &str, imports: &str| Package {
        name: name.into(),
        version: version.into(),
        title: title.into(),
        imports: imports.into(),
        license: "GPL-3".into(),
        published: "2025-03-01".into(),
        needs_compilation: Some(false),
        ..Package::default()
    };
    Index::new(
        vec![
            p(
                "zoo",
                "1.8-12",
                "S3 Infrastructure for Regular and Irregular Time Series",
                "",
            ),
            p(
                "fixest",
                "0.12.1",
                "Fast Fixed-Effects Estimations",
                "Rcpp (>= 1.0), stats",
            ),
            p(
                "Rcpp",
                "1.0.14",
                "Seamless R and C++ Integration",
                "methods, utils",
            ),
            p(
                "AER",
                "1.2-14",
                "Applied Econometrics with R",
                "car, lmtest, sandwich",
            ),
            p(
                "aer",
                "0.1",
                "A decoy that differs from AER only in case",
                "",
            ),
            p(
                "sandwich",
                "3.1-1",
                "Robust Covariance Matrix Estimators",
                "",
            ),
        ],
        1_700_000_000,
        Detail::Full,
    )
}

/// The packages that need this one, from the three "Reverse ..." columns of the
/// database, each name once, in the order found.
fn used_by(t: &Table, row: usize) -> String {
    let mut seen: Vec<&str> = Vec::new();
    for col in ["Reverse depends", "Reverse imports", "Reverse linking to"] {
        for name in t.get(col, row).unwrap_or("").split(',') {
            let name = name.trim();
            if !name.is_empty() && !seen.contains(&name) {
                seen.push(name);
            }
        }
    }
    seen.join(", ")
}

impl Package {
    /// How many packages on CRAN need this one.
    pub fn used_by_count(&self) -> usize {
        self.used_by
            .split(',')
            .filter(|n| !n.trim().is_empty())
            .count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn names(ps: &[Package]) -> Vec<&str> {
        ps.iter().map(|p| p.name.as_str()).collect()
    }

    #[test]
    fn packages_are_sorted_ignoring_case_and_found_by_name_and_prefix() {
        let ix = sample();
        assert_eq!(
            names(ix.packages()),
            vec!["AER", "aer", "fixest", "Rcpp", "sandwich", "zoo"]
        );
        assert_eq!(ix.get("fixest").map(|p| p.version.as_str()), Some("0.12.1"));
        assert_eq!(ix.get("Fixest"), None);
        assert_eq!(ix.get("aer").map(|p| p.version.as_str()), Some("0.1"));
        assert_eq!(
            ix.get_ignoring_case("FIXEST").map(|p| p.name.as_str()),
            Some("fixest")
        );
        assert_eq!(names(ix.with_prefix("a")), vec!["AER", "aer"]);
        assert_eq!(names(ix.with_prefix("RC")), vec!["Rcpp"]);
        assert_eq!(names(ix.with_prefix("s")), vec!["sandwich"]);
        assert!(ix.with_prefix("q").is_empty());
        assert_eq!(ix.with_prefix("").len(), 6);
        assert!(ix.with_prefix("zoooo").is_empty());
    }

    #[test]
    fn staleness_uses_the_clock_it_is_given() {
        let ix = sample();
        let t = ix.fetched_at;
        assert!(!ix.is_stale(t, DEFAULT_MAX_AGE));
        assert!(!ix.is_stale(t + DEFAULT_MAX_AGE, DEFAULT_MAX_AGE));
        assert!(ix.is_stale(t + DEFAULT_MAX_AGE + 1, DEFAULT_MAX_AGE));
        // a clock set before the fetch is not stale and does not underflow
        assert_eq!(ix.age(0), 0);
    }

    #[test]
    fn dependency_fields_are_parsed() {
        let d = parse_dependencies("R (>= 3.5.0), Rcpp (>= 1.0.1),\n  stats, , 9bad, data.table");
        let got: Vec<(&str, Option<&str>)> = d
            .iter()
            .map(|d| (d.name.as_str(), d.constraint.as_deref()))
            .collect();
        assert_eq!(
            got,
            vec![
                ("Rcpp", Some(">= 1.0.1")),
                ("stats", None),
                ("data.table", None)
            ]
        );
        let p = Package {
            depends: "R (>= 3.5), a".into(),
            imports: "a, b (>= 2)".into(),
            linking_to: "c".into(),
            ..Package::default()
        };
        let names: Vec<String> = p.hard_dependencies().into_iter().map(|d| d.name).collect();
        assert_eq!(names, vec!["a", "b", "c"]);
    }

    #[test]
    fn two_sources_are_merged_with_the_database_preferred() {
        let db = Table {
            names: vec![
                "Package".into(),
                "Version".into(),
                "Title".into(),
                "Published".into(),
            ],
            columns: vec![
                vec![Some("alpha".into()), Some("beta".into())],
                vec![Some("2.0".into()), Some("1.0".into())],
                vec![Some("Alpha\n  tools".into()), None],
                vec![Some("2025-01-02 10:11:12 UTC".into()), None],
            ],
            rows: 2,
        };
        let dcf = Table {
            names: vec![
                "Package".into(),
                "Version".into(),
                "Imports".into(),
                "NeedsCompilation".into(),
            ],
            columns: vec![
                vec![
                    Some("alpha".into()),
                    Some("beta".into()),
                    Some("gamma".into()),
                ],
                vec![Some("1.9".into()), Some("1.0".into()), Some("0.5".into())],
                vec![Some("beta".into()), None, None],
                vec![Some("yes".into()), Some("no".into()), None],
            ],
            rows: 3,
        };
        let ix = Index::from_tables(Some(&db), Some(&dcf), 5);
        assert_eq!(ix.detail, Detail::Full);
        assert_eq!(names(ix.packages()), vec!["alpha", "beta", "gamma"]);
        let a = ix.get("alpha");
        assert_eq!(a.map(|p| p.version.as_str()), Some("2.0"));
        assert_eq!(a.map(|p| p.title.as_str()), Some("Alpha tools"));
        assert_eq!(a.map(|p| p.published.as_str()), Some("2025-01-02"));
        assert_eq!(a.map(|p| p.imports.as_str()), Some("beta"));
        assert_eq!(a.and_then(|p| p.needs_compilation), Some(true));
        // only in PACKAGES.gz: installable, no title
        assert_eq!(ix.get("gamma").map(|p| p.title.as_str()), Some(""));
        let names_only = Index::from_tables(None, Some(&dcf), 5);
        assert_eq!(names_only.detail, Detail::NamesOnly);
        assert_eq!(names_only.len(), 3);
    }

    #[test]
    fn duplicates_and_unnamed_rows_are_dropped() {
        let t = Table {
            names: vec!["Package".into(), "Version".into()],
            columns: vec![
                vec![Some("a".into()), None, Some("a".into()), Some("b".into())],
                vec![
                    Some("1".into()),
                    Some("9".into()),
                    Some("2".into()),
                    Some("3".into()),
                ],
            ],
            rows: 4,
        };
        let ix = Index::from_tables(None, Some(&t), 0);
        assert_eq!(names(ix.packages()), vec!["a", "b"]);
        assert_eq!(ix.get("a").map(|p| p.version.as_str()), Some("1"));
    }

    #[test]
    fn fields_round_trip() {
        for p in sample().packages() {
            assert_eq!(Package::from_fields(&p.to_fields()), Some(p.clone()));
        }
        assert_eq!(Package::from_fields(&["x".to_string()]), None);
        assert_eq!(FIELD_NAMES.len(), Package::default().to_fields().len());
    }
}
