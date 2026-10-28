//! CRAN Task Views: the curated lists of packages for a field (Econometrics,
//! Survival, Bayesian, Spatial and the rest), read from `Views.rds`.
//!
//! `src/contrib/Views.rds` holds a list of `ctv` objects, one per view, each
//! with its name, topic, maintainer, date, address and a table of the
//! packages in it (with a flag for the core ones). Only those fields are kept.
//! The cache is a small text file of its own, written the way the index cache is.

use std::collections::HashSet;

use crate::rds::{Data, Node};
use crate::{Error, Result};

/// Most task views accepted (CRAN has about 50).
pub const MAX_VIEWS: usize = 500;
/// Most packages in one view.
pub const MAX_PACKAGES_PER_VIEW: usize = 20_000;
const MAGIC: &str = "RCRANVIEWS1";

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ViewPackage {
    pub name: String,
    /// One of the view's core packages.
    pub core: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TaskView {
    /// The short name, such as `Econometrics`.
    pub name: String,
    /// The title, such as `Econometrics` or `Actuarial Science`.
    pub topic: String,
    pub maintainer: String,
    /// The date of the view's version, as CRAN writes it.
    pub updated: String,
    /// The view's own web page.
    pub url: String,
    /// Core packages first, then the rest, each group sorted by name.
    pub packages: Vec<ViewPackage>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Views {
    views: Vec<TaskView>,
}

fn text(n: &Node) -> String {
    match &n.data {
        Data::Strings(v) => v.first().cloned().flatten().unwrap_or_default(),
        _ => String::new(),
    }
}

fn field<'a>(names: &[String], items: &'a [Node], name: &str) -> Option<&'a Node> {
    let i = names.iter().position(|n| n == name)?;
    items.get(i)
}

impl Views {
    pub fn new(mut views: Vec<TaskView>) -> Views {
        views.retain(|v| !v.name.is_empty());
        views.sort_by_key(|v| v.name.to_lowercase());
        views.dedup_by(|b, a| a.name == b.name);
        for v in &mut views {
            v.packages.sort_by(|a, b| {
                b.core
                    .cmp(&a.core)
                    .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
            });
            v.packages.dedup_by(|b, a| a.name == b.name);
        }
        Views { views }
    }

    /// Reads the value `readRDS("Views.rds")` gives.
    pub fn from_node(root: &Node) -> Result<Views> {
        let Data::List(items) = &root.data else {
            return Err(Error::Format("the task view list is not a list".into()));
        };
        if items.len() > MAX_VIEWS {
            return Err(Error::TooBig(format!("more than {MAX_VIEWS} task views")));
        }
        let mut views = Vec::with_capacity(items.len());
        for item in items {
            let Data::List(fields) = &item.data else {
                continue;
            };
            let names: Vec<String> = match item.attr("names").map(|n| &n.data) {
                Some(Data::Strings(v)) => v.iter().map(|s| s.clone().unwrap_or_default()).collect(),
                _ => continue,
            };
            let get = |n: &str| field(&names, fields, n).map(text).unwrap_or_default();
            let mut packages = Vec::new();
            if let Some(Node {
                data: Data::List(cols),
                attrs,
            }) = field(&names, fields, "packagelist")
            {
                let col_names: Vec<String> = attrs
                    .iter()
                    .find(|(k, _)| k == "names")
                    .and_then(|(_, n)| match &n.data {
                        Data::Strings(v) => {
                            Some(v.iter().map(|s| s.clone().unwrap_or_default()).collect())
                        }
                        _ => None,
                    })
                    .unwrap_or_default();
                let name_col = col_names.iter().position(|n| n == "name");
                let core_col = col_names.iter().position(|n| n == "core");
                if let Some(Node {
                    data: Data::Strings(pkgs),
                    ..
                }) = name_col.and_then(|i| cols.get(i))
                {
                    if pkgs.len() > MAX_PACKAGES_PER_VIEW {
                        return Err(Error::TooBig("a task view lists too many packages".into()));
                    }
                    let core: Vec<i32> = match core_col.and_then(|i| cols.get(i)).map(|n| &n.data) {
                        Some(Data::Ints(v)) => v.clone(),
                        _ => Vec::new(),
                    };
                    for (i, p) in pkgs.iter().enumerate() {
                        if let Some(name) = p {
                            packages.push(ViewPackage {
                                name: name.clone(),
                                core: core.get(i).is_some_and(|c| *c == 1),
                            });
                        }
                    }
                }
            }
            views.push(TaskView {
                name: get("name"),
                topic: get("topic"),
                maintainer: get("maintainer"),
                updated: get("version"),
                url: get("source"),
                packages,
            });
        }
        if views.iter().all(|v| v.name.is_empty()) {
            return Err(Error::Format("no task views found".into()));
        }
        Ok(Views::new(views))
    }

    pub fn views(&self) -> &[TaskView] {
        &self.views
    }

    pub fn is_empty(&self) -> bool {
        self.views.is_empty()
    }

    pub fn get(&self, name: &str) -> Option<&TaskView> {
        self.views.iter().find(|v| v.name == name)
    }

    /// The names of the packages in the view called `name`.
    pub fn members_of(&self, name: &str) -> HashSet<String> {
        self.get(name)
            .map(|v| v.packages.iter().map(|p| p.name.clone()).collect())
            .unwrap_or_default()
    }

    /// The views that list `package`.
    pub fn views_of(&self, package: &str) -> Vec<&TaskView> {
        self.views
            .iter()
            .filter(|v| v.packages.iter().any(|p| p.name == package))
            .collect()
    }

    /// The text kept in the cache.
    pub fn to_text(&self, fetched_at: u64) -> String {
        let mut out = format!(
            "{MAGIC}\nfetched\t{fetched_at}\ncount\t{}\n",
            self.views.len()
        );
        for v in &self.views {
            out.push_str(&format!(
                "view\t{}\t{}\t{}\t{}\t{}\t{}\n",
                esc(&v.name),
                esc(&v.topic),
                esc(&v.maintainer),
                esc(&v.updated),
                esc(&v.url),
                v.packages.len()
            ));
            for p in &v.packages {
                out.push_str(&format!("pkg\t{}\t{}\n", esc(&p.name), u8::from(p.core)));
            }
        }
        out
    }

    /// Reads the cache text back: the views and when they were fetched.
    pub fn from_text(text: &str) -> Result<(Views, u64)> {
        let mut lines = text.split('\n');
        if lines.next() != Some(MAGIC) {
            return Err(Error::Format("not an RCraft task view cache".into()));
        }
        let mut num = |key: &str| -> Result<u64> {
            lines
                .next()
                .and_then(|l| l.strip_prefix(key))
                .and_then(|r| r.strip_prefix('\t'))
                .and_then(|n| n.parse().ok())
                .ok_or_else(|| Error::Format(format!("the task view cache lacks {key}")))
        };
        let fetched = num("fetched")?;
        let count =
            usize::try_from(num("count")?).map_err(|_| Error::Format("a bad count".into()))?;
        if count > MAX_VIEWS {
            return Err(Error::TooBig(format!("more than {MAX_VIEWS} task views")));
        }
        let mut views = Vec::with_capacity(count);
        for _ in 0..count {
            let line = lines.next().ok_or(Error::Truncated)?;
            let f: Vec<&str> = line.split('\t').collect();
            let [tag, name, topic, maintainer, updated, url, n] = f.as_slice() else {
                return Err(Error::Format(
                    "a task view line is not in the format".into(),
                ));
            };
            if *tag != "view" {
                return Err(Error::Format("expected a view".into()));
            }
            let n: usize = n
                .parse()
                .map_err(|_| Error::Format("a bad package count".into()))?;
            if n > MAX_PACKAGES_PER_VIEW {
                return Err(Error::TooBig("a task view lists too many packages".into()));
            }
            let mut packages = Vec::with_capacity(n.min(2000));
            for _ in 0..n {
                let line = lines.next().ok_or(Error::Truncated)?;
                let f: Vec<&str> = line.split('\t').collect();
                let [tag, pname, core] = f.as_slice() else {
                    return Err(Error::Format("a package line is not in the format".into()));
                };
                if *tag != "pkg" {
                    return Err(Error::Format("expected a package".into()));
                }
                packages.push(ViewPackage {
                    name: unesc(pname)?,
                    core: *core == "1",
                });
            }
            views.push(TaskView {
                name: unesc(name)?,
                topic: unesc(topic)?,
                maintainer: unesc(maintainer)?,
                updated: unesc(updated)?,
                url: unesc(url)?,
                packages,
            });
        }
        if lines.next() != Some("") || lines.next().is_some() {
            return Err(Error::Format(
                "the task view cache has extra lines or is cut short".into(),
            ));
        }
        Ok((Views::new(views), fetched))
    }
}

fn esc(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('\t', "\\t")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
}

fn unesc(s: &str) -> Result<String> {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        if c != '\\' {
            out.push(c);
            continue;
        }
        match chars.next() {
            Some('\\') => out.push('\\'),
            Some('t') => out.push('\t'),
            Some('n') => out.push('\n'),
            Some('r') => out.push('\r'),
            _ => return Err(Error::Format("a bad escape in the task view cache".into())),
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn strings(items: &[&str]) -> Node {
        Node {
            data: Data::Strings(items.iter().map(|s| Some((*s).to_string())).collect()),
            attrs: vec![],
        }
    }

    fn named(names: &[&str], data: Data) -> Node {
        Node {
            data,
            attrs: vec![("names".into(), strings(names))],
        }
    }

    fn view_node(name: &str, topic: &str, packages: &[(&str, i32)]) -> Node {
        let table = named(
            &["name", "core"],
            Data::List(vec![
                strings(&packages.iter().map(|p| p.0).collect::<Vec<_>>()),
                Node {
                    data: Data::Ints(packages.iter().map(|p| p.1).collect()),
                    attrs: vec![],
                },
            ]),
        );
        named(
            &[
                "name",
                "topic",
                "maintainer",
                "email",
                "version",
                "source",
                "packagelist",
                "archived",
            ],
            Data::List(vec![
                strings(&[name]),
                strings(&[topic]),
                strings(&["A. Person"]),
                strings(&["a@example.org"]),
                strings(&["2026-10-22"]),
                strings(&["https://example.org/view"]),
                table,
                strings(&["old"]),
            ]),
        )
    }

    fn root() -> Node {
        Node {
            data: Data::List(vec![
                view_node(
                    "Survival",
                    "Survival Analysis",
                    &[("survival", 1), ("coxme", 0), ("KMsurv", 0)],
                ),
                view_node(
                    "Econometrics",
                    "Econometrics",
                    &[("plm", 1), ("AER", 1), ("sandwich", 0)],
                ),
            ]),
            attrs: vec![],
        }
    }

    #[test]
    fn views_are_read_sorted_with_core_packages_first() {
        let v = Views::from_node(&root()).unwrap_or_default();
        let names: Vec<&str> = v.views().iter().map(|v| v.name.as_str()).collect();
        assert_eq!(names, vec!["Econometrics", "Survival"]);
        let surv = v.get("Survival");
        assert_eq!(surv.map(|s| s.topic.as_str()), Some("Survival Analysis"));
        assert_eq!(surv.map(|s| s.updated.as_str()), Some("2026-10-22"));
        let pk: Vec<(&str, bool)> = surv
            .map(|s| {
                s.packages
                    .iter()
                    .map(|p| (p.name.as_str(), p.core))
                    .collect()
            })
            .unwrap_or_default();
        assert_eq!(
            pk,
            vec![("survival", true), ("coxme", false), ("KMsurv", false)]
        );
        assert!(v.members_of("Econometrics").contains("AER"));
        assert!(v.members_of("Nope").is_empty());
        let of: Vec<&str> = v
            .views_of("sandwich")
            .iter()
            .map(|v| v.name.as_str())
            .collect();
        assert_eq!(of, vec!["Econometrics"]);
    }

    #[test]
    fn wrong_shapes_are_errors_and_odd_entries_are_skipped() {
        assert!(Views::from_node(&strings(&["x"])).is_err());
        let mut r = root();
        if let Data::List(items) = &mut r.data {
            items.push(strings(&["not a view"]));
        }
        assert_eq!(Views::from_node(&r).map(|v| v.views().len()), Ok(2));
        assert!(Views::from_node(&Node {
            data: Data::List(vec![]),
            attrs: vec![]
        })
        .is_err());
    }

    #[test]
    fn the_cache_text_round_trips_and_damage_is_an_error() {
        let v = Views::from_node(&root()).unwrap_or_default();
        let text = v.to_text(1234);
        assert_eq!(Views::from_text(&text), Ok((v.clone(), 1234)));
        for (i, _) in text.match_indices('\n') {
            if i + 1 < text.len() {
                assert!(
                    Views::from_text(text.get(..=i).unwrap_or("")).is_err(),
                    "cut at {i}"
                );
            }
        }
        assert!(Views::from_text(&format!("{text}extra\n")).is_err());
        assert!(Views::from_text("RCRANVIEWS2\n").is_err());
        assert!(Views::from_text(&text.replace("count\t2", "count\t999999")).is_err());
        // awkward characters in a field survive
        let odd = Views::new(vec![TaskView {
            name: "X".into(),
            topic: "tab\there\nand \\ slash".into(),
            ..TaskView::default()
        }]);
        assert_eq!(Views::from_text(&odd.to_text(1)).map(|(v, _)| v), Ok(odd));
    }
}
