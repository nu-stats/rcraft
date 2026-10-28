//! The package page: everything the index knows about one package, arranged
//! for reading (the `ssc describe` of RCraft).
//!
//! Package metadata is written by the package's author, so it is data to show,
//! never something to follow: links are listed as text and only ever opened by
//! a click, and only `http` and `https` links are offered.

use crate::index::{parse_dependencies, Dependency, Index, Package};

/// Packages that come with R and are never installed from CRAN.
pub const BASE_PACKAGES: &[&str] = &[
    "base",
    "compiler",
    "datasets",
    "graphics",
    "grDevices",
    "grid",
    "methods",
    "parallel",
    "splines",
    "stats",
    "stats4",
    "tcltk",
    "tools",
    "utils",
];

/// Whether `name` is a package that comes with R.
pub fn is_base_package(name: &str) -> bool {
    BASE_PACKAGES.contains(&name)
}

/// Where a needed package comes from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Source {
    /// Comes with R.
    Base,
    /// Listed in the index.
    Cran,
    /// Not in the index (removed from CRAN, or from another repository).
    Unknown,
}

/// A package this one needs, with where it comes from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Needed {
    pub dependency: Dependency,
    pub source: Source,
}

/// The kind of need.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Depends,
    Imports,
    LinkingTo,
    Suggests,
}

impl Kind {
    pub fn label(self) -> &'static str {
        match self {
            Kind::Depends => "Depends",
            Kind::Imports => "Imports",
            Kind::LinkingTo => "LinkingTo",
            Kind::Suggests => "Suggests",
        }
    }
}

/// Everything shown on a package's page.
#[derive(Debug, Clone, PartialEq)]
pub struct Description {
    pub package: Package,
    /// The package's own web links (`URL` field), http and https only.
    pub links: Vec<String>,
    /// Where bugs are reported, when it is a web link.
    pub bug_reports: Option<String>,
    pub needs: Vec<(Kind, Vec<Needed>)>,
    /// Names of the packages that need this one, sorted.
    pub used_by: Vec<String>,
}

/// The web links in a field like `https://a.org, <https://b.org/x>`. Anything
/// that is not an `http` or `https` link is dropped.
pub fn web_links(field: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for raw in field.split(|c: char| c.is_whitespace() || c == ',' || c == ';') {
        let t = raw.trim_matches(|c: char| matches!(c, '<' | '>' | '(' | ')' | '"' | '\'' | '.'));
        let lower = t.to_ascii_lowercase();
        if (lower.starts_with("https://") || lower.starts_with("http://"))
            && t.len() > 8
            && !t.chars().any(|c| c.is_control())
            && !out.iter().any(|o| o == t)
        {
            out.push(t.to_string());
        }
    }
    out
}

/// The page for `name`, or `None` when the index has no such package.
pub fn describe(index: &Index, name: &str) -> Option<Description> {
    let package = index.get(name)?.clone();
    let source_of = |d: &Dependency| {
        if is_base_package(&d.name) {
            Source::Base
        } else if index.get(&d.name).is_some() {
            Source::Cran
        } else {
            Source::Unknown
        }
    };
    let mut needs = Vec::new();
    for (kind, field) in [
        (Kind::Depends, &package.depends),
        (Kind::Imports, &package.imports),
        (Kind::LinkingTo, &package.linking_to),
        (Kind::Suggests, &package.suggests),
    ] {
        let list: Vec<Needed> = parse_dependencies(field)
            .into_iter()
            .map(|d| Needed {
                source: source_of(&d),
                dependency: d,
            })
            .collect();
        if !list.is_empty() {
            needs.push((kind, list));
        }
    }
    let mut used_by: Vec<String> = package
        .used_by
        .split(',')
        .map(str::trim)
        .filter(|n| !n.is_empty())
        .map(str::to_string)
        .collect();
    used_by.sort_by_key(|n| n.to_lowercase());
    Some(Description {
        links: web_links(&package.url),
        bug_reports: web_links(&package.bug_reports).into_iter().next(),
        package,
        needs,
        used_by,
    })
}

impl Description {
    /// Whether the package (or something it needs) must be compiled from
    /// source when there is no binary.
    pub fn needs_compiler(&self) -> bool {
        self.package.needs_compilation == Some(true)
    }

    /// The page as plain text, for the Command window and the log.
    pub fn to_text(&self) -> String {
        let p = &self.package;
        let mut out = format!("{} {}", p.name, p.version);
        if !p.title.is_empty() {
            out.push_str(&format!(" - {}", p.title));
        }
        out.push('\n');
        let mut line = |label: &str, value: &str| {
            if !value.is_empty() {
                out.push_str(&format!("{label}: {value}\n"));
            }
        };
        line("Published", &p.published);
        line("Licence", &p.license);
        line("Maintainer", &p.maintainer);
        line("Authors", &p.authors);
        line(
            "Needs a compiler to build from source",
            match p.needs_compilation {
                Some(true) => "yes",
                Some(false) => "no",
                None => "",
            },
        );
        line("System requirements", &p.system_requirements);
        if !self.links.is_empty() {
            line("Links", &self.links.join(", "));
        }
        if let Some(b) = &self.bug_reports {
            line("Bug reports", b);
        }
        for (kind, list) in &self.needs {
            let items: Vec<String> = list
                .iter()
                .map(|n| {
                    let mut s = n.dependency.name.clone();
                    if let Some(c) = &n.dependency.constraint {
                        s.push_str(&format!(" ({c})"));
                    }
                    match n.source {
                        Source::Base => s.push_str(" [comes with R]"),
                        Source::Unknown => s.push_str(" [not in the CRAN list]"),
                        Source::Cran => {}
                    }
                    s
                })
                .collect();
            line(kind.label(), &items.join(", "));
        }
        if !self.used_by.is_empty() {
            let shown: Vec<&str> = self.used_by.iter().take(20).map(String::as_str).collect();
            let more = self.used_by.len().saturating_sub(shown.len());
            let mut s = format!(
                "{} packages need this one: {}",
                self.used_by.len(),
                shown.join(", ")
            );
            if more > 0 {
                s.push_str(&format!(", and {more} more"));
            }
            out.push_str(&s);
            out.push('\n');
        }
        if !p.description.is_empty() {
            out.push('\n');
            out.push_str(&p.description);
            out.push('\n');
        }
        out
    }
}

/// One line about a package for a list of search results:
/// `name version  title (published 2026-06-26, used by 26)`.
pub fn summary_line(p: &Package) -> String {
    let mut line = format!("{} {}", p.name, p.version);
    if !p.title.is_empty() {
        line.push_str(&format!("  {}", p.title));
    }
    let mut notes = Vec::new();
    if !p.published.is_empty() {
        notes.push(format!("published {}", p.published));
    }
    let used = p.used_by_count();
    if used > 0 {
        notes.push(format!("used by {used}"));
    }
    if !notes.is_empty() {
        line.push_str(&format!(" ({})", notes.join(", ")));
    }
    line
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::index::Detail;

    fn index() -> Index {
        let p = |name: &str| Package {
            name: name.into(),
            version: "1.0".into(),
            ..Package::default()
        };
        let mut main = p("main");
        main.title = "The Main Package".into();
        main.description = "Does the main thing.".into();
        main.depends = "R (>= 4.0), dep".into();
        main.imports = "stats, dep (>= 2.0), gone".into();
        main.suggests = "testthat".into();
        main.linking_to = "dep".into();
        main.url =
            "https://main.example.org, <http://old.example.org/x>. See also ftp://nope.example.org"
                .into();
        main.bug_reports = "https://github.com/x/main/issues".into();
        main.used_by = "zeta, Alpha, beta".into();
        main.needs_compilation = Some(true);
        main.system_requirements = "libfoo".into();
        Index::new(vec![main, p("dep"), p("testthat")], 0, Detail::Full)
    }

    #[test]
    fn the_page_collects_needs_with_their_sources_and_users_sorted() {
        let d = describe(&index(), "main");
        let d = d.unwrap_or_else(|| panic!("no page"));
        let kinds: Vec<Kind> = d.needs.iter().map(|(k, _)| *k).collect();
        assert_eq!(
            kinds,
            vec![
                Kind::Depends,
                Kind::Imports,
                Kind::LinkingTo,
                Kind::Suggests
            ]
        );
        let imports = &d.needs[1].1;
        let got: Vec<(&str, Source)> = imports
            .iter()
            .map(|n| (n.dependency.name.as_str(), n.source))
            .collect();
        assert_eq!(
            got,
            vec![
                ("stats", Source::Base),
                ("dep", Source::Cran),
                ("gone", Source::Unknown)
            ]
        );
        assert_eq!(d.used_by, vec!["Alpha", "beta", "zeta"]);
        assert!(d.needs_compiler());
        assert!(describe(&index(), "nope").is_none());
    }

    #[test]
    fn only_web_links_are_kept() {
        let d = describe(&index(), "main").unwrap_or_else(|| panic!("no page"));
        assert_eq!(
            d.links,
            vec!["https://main.example.org", "http://old.example.org/x"]
        );
        assert_eq!(
            d.bug_reports.as_deref(),
            Some("https://github.com/x/main/issues")
        );
        for bad in [
            "javascript:alert(1)",
            "file:///etc/passwd",
            "https://",
            "https://a\u{7}b.org",
            "see the website",
        ] {
            assert!(web_links(bad).is_empty(), "{bad:?}");
        }
        assert_eq!(
            web_links("HTTPS://UPPER.org/A"),
            vec!["HTTPS://UPPER.org/A"]
        );
    }

    #[test]
    fn the_text_form_names_what_matters() {
        let t = describe(&index(), "main")
            .map(|d| d.to_text())
            .unwrap_or_default();
        assert!(t.starts_with("main 1.0 - The Main Package\n"), "{t}");
        assert!(t.contains("Depends: dep\n"), "{t}");
        assert!(
            t.contains(
                "Imports: stats [comes with R], dep (>= 2.0), gone [not in the CRAN list]\n"
            ),
            "{t}"
        );
        assert!(
            t.contains("3 packages need this one: Alpha, beta, zeta\n"),
            "{t}"
        );
        assert!(t.contains("System requirements: libfoo\n"));
        assert!(t.contains("Needs a compiler to build from source: yes\n"));
        assert!(t.ends_with("Does the main thing.\n"));
    }

    #[test]
    fn a_long_list_of_users_is_cut_in_the_text() {
        let mut p = Package {
            name: "popular".into(),
            ..Package::default()
        };
        p.used_by = (0..50)
            .map(|i| format!("u{i:02}"))
            .collect::<Vec<_>>()
            .join(", ");
        let ix = Index::new(vec![p], 0, Detail::Full);
        let t = describe(&ix, "popular")
            .map(|d| d.to_text())
            .unwrap_or_default();
        assert!(t.contains("50 packages need this one: u00,"), "{t}");
        assert!(t.contains("and 30 more"), "{t}");
    }
}

#[cfg(test)]
mod summary_tests {
    use super::*;

    #[test]
    fn a_search_line_has_name_version_title_date_and_users() {
        let p = Package {
            name: "fixest".into(),
            version: "0.14.2".into(),
            title: "Fast Fixed-Effects Estimations".into(),
            published: "2026-06-26".into(),
            used_by: "a, b, c".into(),
            ..Package::default()
        };
        assert_eq!(
            summary_line(&p),
            "fixest 0.14.2  Fast Fixed-Effects Estimations (published 2026-06-26, used by 3)"
        );
        let bare = Package {
            name: "x".into(),
            version: "1".into(),
            ..Package::default()
        };
        assert_eq!(summary_line(&bare), "x 1");
    }
}
