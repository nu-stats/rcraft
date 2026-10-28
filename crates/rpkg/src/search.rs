//! Keyword search over the index: ranked, offline, and the same answer every
//! time for the same words.
//!
//! Every word of the query must match somewhere (names, titles, descriptions,
//! authors and maintainers); a package's score is the sum over the words of its
//! best match for each. A name that is the word beats a name that starts with
//! it, which beats a name that contains it, which beats a whole word in the
//! title, and so on down to a part of a word in the description. Ties go to the
//! shorter name, then alphabetical order.

use std::collections::HashSet;

use crate::index::{Index, Package};

/// Most results returned by default.
pub const DEFAULT_LIMIT: usize = 200;
/// Most words of a query used (the rest are ignored).
pub const MAX_TERMS: usize = 12;

const NAME_EXACT: u32 = 1000;
const NAME_PREFIX: u32 = 600;
/// A name that starts with one word of several: names rarely hold two words, so
/// this counts for less than a whole word in the title.
const NAME_PREFIX_OF_MANY: u32 = 150;
const NAME_CONTAINS_OF_MANY: u32 = 80;
/// Added once when the words of a query appear together, in order, in the title.
const TITLE_PHRASE: u32 = 300;
/// Per extra mention in the description, up to `MENTIONS_MAX` mentions.
const MENTION: u32 = 4;
const MENTIONS_MAX: usize = 5;
const NAME_CONTAINS: u32 = 400;
const TITLE_WORD: u32 = 200;
const TITLE_PART: u32 = 100;
const DESC_WORD: u32 = 50;
const DESC_PART: u32 = 20;
const PEOPLE: u32 = 5;
/// Added per doubling of the number of packages that need this one, so the
/// widely used package wins between otherwise equal matches, never more than
/// `POPULARITY_MAX`.
const POPULARITY: u32 = 30;
const POPULARITY_MAX: u32 = 360;

/// Narrowing that does not depend on the words.
#[derive(Debug, Clone, Default)]
pub struct Filters {
    /// Part of the licence text (`GPL`, `MIT`), ignoring case.
    pub licence: Option<String>,
    /// Only packages that do (`true`) or do not (`false`) need compiling.
    pub needs_compilation: Option<bool>,
    /// Only these package names, for example the members of a task view.
    pub only: Option<HashSet<String>>,
    /// Only packages published on or after this date (`YYYY-MM-DD`).
    pub published_since: Option<String>,
}

impl Filters {
    fn allows(&self, p: &Package) -> bool {
        if let Some(l) = &self.licence {
            if !p.license.to_lowercase().contains(&l.trim().to_lowercase()) {
                return false;
            }
        }
        if let Some(c) = self.needs_compilation {
            if p.needs_compilation != Some(c) {
                return false;
            }
        }
        if let Some(only) = &self.only {
            if !only.contains(&p.name) {
                return false;
            }
        }
        if let Some(d) = &self.published_since {
            // dates are YYYY-MM-DD, so text order is date order; no date never matches
            if p.published.is_empty() || p.published.as_str() < d.as_str() {
                return false;
            }
        }
        true
    }
}

/// One result: the package's place in [`Index::packages`] and its score.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Hit {
    pub position: usize,
    pub score: u32,
}

/// Lower-cased copies of the searched fields, made once per index so a search
/// per keystroke stays quick.
pub struct Searcher {
    entries: Vec<Entry>,
}

struct Entry {
    name: String,
    title: String,
    /// The title with everything but letters and digits turned into spaces.
    title_flat: String,
    title_words: HashSet<String>,
    description: String,
    description_words: HashSet<String>,
    people: String,
    /// Bonus for being widely used.
    popularity: u32,
}

fn words(s: &str) -> impl Iterator<Item = &str> {
    s.split(|c: char| !c.is_alphanumeric())
        .filter(|w| !w.is_empty())
}

/// The words of a query: lower case, at most [`MAX_TERMS`], without repeats.
pub fn terms(query: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for w in words(&query.to_lowercase()) {
        if !out.iter().any(|o| o == w) {
            out.push(w.to_string());
        }
        if out.len() == MAX_TERMS {
            break;
        }
    }
    out
}

impl Searcher {
    /// Prepares `index` for searching.
    pub fn new(index: &Index) -> Searcher {
        let entries = index
            .packages()
            .iter()
            .map(|p| {
                let title = p.title.to_lowercase();
                let description = p.description.to_lowercase();
                Entry {
                    name: p.name.to_lowercase(),
                    title_words: words(&title).map(str::to_string).collect(),
                    title_flat: words(&title).collect::<Vec<_>>().join(" "),
                    title,
                    description_words: words(&description).map(str::to_string).collect(),
                    description,
                    people: format!("{} {}", p.authors, p.maintainer).to_lowercase(),
                    popularity: popularity(p),
                }
            })
            .collect();
        Searcher { entries }
    }

    /// How many packages this was prepared for.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// The best matches for `query` among the packages `filters` allows, best
    /// first, at most `limit`. With no words in the query every allowed package
    /// matches, in name order. `index` must be the one this was prepared for;
    /// otherwise nothing is returned.
    pub fn search(&self, index: &Index, query: &str, filters: &Filters, limit: usize) -> Vec<Hit> {
        if self.entries.len() != index.len() {
            return Vec::new();
        }
        let terms = terms(query);
        let whole = query.trim().to_lowercase();
        let phrase = if terms.len() > 1 {
            terms.join(" ")
        } else {
            String::new()
        };
        let mut hits: Vec<Hit> = Vec::new();
        for (position, (entry, p)) in self.entries.iter().zip(index.packages()).enumerate() {
            if !filters.allows(p) {
                continue;
            }
            let mut total = 0u32;
            let mut all = true;
            for t in &terms {
                match entry.score(t, terms.len() > 1) {
                    0 => {
                        all = false;
                        break;
                    }
                    s => total += s,
                }
            }
            if all && entry.name == whole {
                total += NAME_EXACT;
            }
            if all {
                total += entry.popularity;
            }
            if all && !phrase.is_empty() && entry.title_flat.contains(&phrase) {
                total += TITLE_PHRASE;
            }
            if all {
                hits.push(Hit {
                    position,
                    score: total,
                });
            }
        }
        let packages = index.packages();
        let ranked = !terms.is_empty();
        hits.sort_by(|a, b| {
            b.score
                .cmp(&a.score)
                .then_with(|| {
                    if !ranked {
                        return std::cmp::Ordering::Equal;
                    }
                    let la = packages.get(a.position).map_or(0, |p| p.name.len());
                    let lb = packages.get(b.position).map_or(0, |p| p.name.len());
                    la.cmp(&lb)
                })
                .then(a.position.cmp(&b.position))
        });
        hits.truncate(limit);
        hits
    }
}

impl Entry {
    /// The best match of `term` in this package, 0 for none.
    fn score(&self, term: &str, of_many: bool) -> u32 {
        let mut best = 0;
        if self.name == term {
            best = NAME_EXACT;
        } else if self.name.starts_with(term) {
            best = if of_many {
                NAME_PREFIX_OF_MANY
            } else {
                NAME_PREFIX
            };
        } else if self.name.contains(term) {
            best = if of_many {
                NAME_CONTAINS_OF_MANY
            } else {
                NAME_CONTAINS
            };
        }
        if best < TITLE_WORD && self.title_words.contains(term) {
            best = TITLE_WORD;
        }
        if best < TITLE_PART && self.title.contains(term) {
            best = TITLE_PART;
        }
        if best < DESC_WORD && self.description_words.contains(term) {
            best = DESC_WORD;
        }
        if best < DESC_PART && self.description.contains(term) {
            best = DESC_PART;
        }
        if best < PEOPLE && self.people.contains(term) {
            best = PEOPLE;
        }
        if best > 0 {
            let mentions = self.description.matches(term).take(MENTIONS_MAX).count();
            best += MENTION * u32::try_from(mentions).unwrap_or(0);
        }
        best
    }
}

fn popularity(p: &Package) -> u32 {
    let n = u32::try_from(p.used_by_count()).unwrap_or(u32::MAX);
    (POPULARITY * (u32::BITS - n.saturating_add(1).leading_zeros() - 1)).min(POPULARITY_MAX)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::index::{sample, Detail};

    fn names(ix: &Index, hits: &[Hit]) -> Vec<String> {
        hits.iter()
            .filter_map(|h| ix.packages().get(h.position))
            .map(|p| p.name.clone())
            .collect()
    }

    fn find(ix: &Index, q: &str) -> Vec<String> {
        let s = Searcher::new(ix);
        names(ix, &s.search(ix, q, &Filters::default(), 50))
    }

    fn pkg(name: &str, title: &str, description: &str) -> Package {
        Package {
            name: name.into(),
            title: title.into(),
            description: description.into(),
            ..Package::default()
        }
    }

    #[test]
    fn words_match_titles_and_every_word_must_match() {
        let ix = sample();
        assert_eq!(find(&ix, "robust"), vec!["sandwich"]);
        assert_eq!(find(&ix, "fixed effects"), vec!["fixest"]);
        assert_eq!(find(&ix, "time series"), vec!["zoo"]);
        assert_eq!(find(&ix, "time cheese"), Vec::<String>::new());
        assert_eq!(find(&ix, "econometrics"), vec!["AER"]);
        assert_eq!(find(&ix, "ECONOMETRICS!!"), vec!["AER"]);
        assert_eq!(find(&ix, "   "), find(&ix, ""));
    }

    #[test]
    fn a_name_beats_a_prefix_beats_a_part_beats_the_title_beats_the_description() {
        let ix = Index::new(
            vec![
                pkg("model", "Something else", ""),
                pkg("modeling", "Something else", ""),
                pkg("remodel", "Something else", ""),
                pkg("alpha", "A model of things", ""),
                pkg("beta", "Remodelling", ""),
                pkg("gamma", "Nothing", "Fits a model to data"),
                pkg("delta", "Nothing", "Remodelled"),
            ],
            0,
            Detail::Full,
        );
        assert_eq!(
            find(&ix, "model"),
            vec!["model", "modeling", "remodel", "alpha", "beta", "gamma", "delta"]
        );
    }

    #[test]
    fn ties_go_to_the_shorter_name_then_to_alphabetical_order() {
        let ix = sample();
        // AER and aer both are the word; the same length, so the index order decides
        assert_eq!(find(&ix, "aer"), vec!["AER", "aer"]);
        let ix = Index::new(
            vec![
                pkg("tools2", "stats helper", ""),
                pkg("tools", "stats helper", ""),
                pkg("atools", "stats helper", ""),
            ],
            0,
            Detail::Full,
        );
        assert_eq!(find(&ix, "stats"), vec!["tools", "atools", "tools2"]);
    }

    #[test]
    fn dotted_names_match_separate_words() {
        let ix = Index::new(
            vec![pkg("data.table", "Extension of data.frame", "")],
            0,
            Detail::Full,
        );
        assert_eq!(find(&ix, "data table"), vec!["data.table"]);
        assert_eq!(find(&ix, "data.table"), vec!["data.table"]);
    }

    #[test]
    fn filters_narrow_the_results() {
        let mut ps = sample().packages().to_vec();
        for p in &mut ps {
            let (licence, published, compiled) = match p.name.as_str() {
                "fixest" => ("GPL-3", "2025-06-01", Some(true)),
                "zoo" => ("GPL-2 | GPL-3", "2024-01-01", Some(false)),
                "sandwich" => ("GPL-2 | GPL-3", "", Some(false)),
                "Rcpp" => ("GPL (>= 2)", "2025-02-01", Some(true)),
                _ => ("MIT + file LICENSE", "2025-03-01", None),
            };
            p.license = licence.into();
            p.published = published.into();
            p.needs_compilation = compiled;
        }
        let ix = Index::new(ps, 0, Detail::Full);
        let s = Searcher::new(&ix);
        let run = |f: Filters| names(&ix, &s.search(&ix, "", &f, 50));
        let all = run(Filters::default());
        assert_eq!(all.len(), 6);
        assert_eq!(
            run(Filters {
                licence: Some("gpl".into()),
                ..Filters::default()
            }),
            vec!["fixest", "Rcpp", "sandwich", "zoo"]
        );
        assert_eq!(
            run(Filters {
                needs_compilation: Some(true),
                ..Filters::default()
            }),
            vec!["fixest", "Rcpp"]
        );
        // from 2025 on; "sandwich" has no date and never matches a date filter
        assert_eq!(
            run(Filters {
                published_since: Some("2025-03-01".into()),
                ..Filters::default()
            }),
            vec!["AER", "aer", "fixest"]
        );
        let only: HashSet<String> = ["zoo".to_string(), "AER".to_string()].into();
        assert_eq!(
            run(Filters {
                only: Some(only),
                ..Filters::default()
            }),
            vec!["AER", "zoo"]
        );
    }

    #[test]
    fn the_limit_and_a_mismatched_index_are_handled() {
        let ix = sample();
        let s = Searcher::new(&ix);
        assert_eq!(s.search(&ix, "", &Filters::default(), 2).len(), 2);
        assert_eq!(s.search(&ix, "", &Filters::default(), 0).len(), 0);
        let other = Index::new(vec![pkg("one", "", "")], 0, Detail::Full);
        assert!(s.search(&other, "one", &Filters::default(), 10).is_empty());
        assert_eq!(s.len(), 6);
    }

    #[test]
    fn a_long_query_is_cut_to_the_first_words() {
        let q = (0..40)
            .map(|i| format!("w{i}"))
            .collect::<Vec<_>>()
            .join(" ");
        assert_eq!(terms(&q).len(), MAX_TERMS);
        assert_eq!(terms("a A a b"), vec!["a", "b"]);
    }
}

#[cfg(test)]
mod ranking_tests {
    use super::*;
    use crate::index::Detail;

    fn pkg(name: &str, title: &str, used_by: &str) -> Package {
        Package {
            name: name.into(),
            title: title.into(),
            used_by: used_by.into(),
            ..Package::default()
        }
    }

    fn top(ix: &Index, q: &str) -> Vec<String> {
        let s = Searcher::new(ix);
        s.search(ix, q, &Filters::default(), 10)
            .iter()
            .filter_map(|h| ix.packages().get(h.position))
            .map(|p| p.name.clone())
            .collect()
    }

    #[test]
    fn the_widely_used_package_wins_between_equal_matches() {
        let many = (0..64)
            .map(|i| format!("p{i}"))
            .collect::<Vec<_>>()
            .join(", ");
        let ix = Index::new(
            vec![
                pkg("aa", "Fixed Effects Models", ""),
                pkg("bbbbbb", "Fixed Effects Models", &many),
            ],
            0,
            Detail::Full,
        );
        assert_eq!(top(&ix, "fixed effects"), vec!["bbbbbb", "aa"]);
        assert_eq!(ix.get("bbbbbb").map(|p| p.used_by_count()), Some(64));
    }

    #[test]
    fn a_dotted_name_typed_whole_beats_packages_that_hold_its_parts() {
        let ix = Index::new(
            vec![
                pkg("data.table", "Extension of data.frame", ""),
                pkg("datatable2", "Data and table helpers", &"x, ".repeat(40)),
                pkg("tablemaker", "Make a table of data", ""),
            ],
            0,
            Detail::Full,
        );
        assert_eq!(
            top(&ix, "data.table").first().map(String::as_str),
            Some("data.table")
        );
    }

    #[test]
    fn words_together_in_the_title_beat_words_apart() {
        let ix = Index::new(
            vec![
                pkg("apart", "Models for data that are mixed", ""),
                pkg("together", "Mixed models for data", ""),
            ],
            0,
            Detail::Full,
        );
        assert_eq!(top(&ix, "mixed models"), vec!["together", "apart"]);
    }
}
