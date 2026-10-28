//! The catalog of menu entries and their coverage status.
//!
//! Every user-visible action is an entry here (AGENTS.md section 3).
//! `cargo xtask coverage` renders `docs/coverage.md` from [`ENTRIES`].
//! Entries with a dialog have a [`rcraft_rcodegen::dialog::Spec`] in an area
//! module ([`data`]).

pub mod data;
pub mod data_tools;
pub mod generalised;
pub mod inference;
pub mod models;
pub mod postest;
pub mod stats;

/// Where an entry sits in the menus.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Area {
    File,
    Data,
    Statistics,
    Graphs,
    Reports,
    Packages,
    Stata,
    Help,
}

impl Area {
    pub fn label(self) -> &'static str {
        match self {
            Area::File => "File",
            Area::Data => "Data",
            Area::Statistics => "Statistics",
            Area::Graphs => "Graphs",
            Area::Reports => "Reports",
            Area::Packages => "Packages",
            Area::Stata => "Stata mode",
            Area::Help => "Help",
        }
    }
}

/// How far an entry has got. `Verified` means its generated R, run in the
/// pinned R, gives the expected result (AGENTS.md section 5).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Status {
    Planned,
    Partial,
    Verified,
}

impl Status {
    pub fn label(self) -> &'static str {
        match self {
            Status::Planned => "planned",
            Status::Partial => "partial",
            Status::Verified => "verified",
        }
    }
}

/// One menu entry.
#[derive(Clone, Copy, Debug)]
pub struct Entry {
    pub name: &'static str,
    pub area: Area,
    pub status: Status,
}

const fn partial(name: &'static str, area: Area) -> Entry {
    Entry {
        name,
        area,
        status: Status::Partial,
    }
}

const fn planned(name: &'static str, area: Area) -> Entry {
    Entry {
        name,
        area,
        status: Status::Planned,
    }
}

/// The inventory. Seeded from ROADMAP.md; grows with each phase.
pub const ENTRIES: &[Entry] = &[
    partial("Use my file", Area::File),
    planned("Save data", Area::File),
    partial("New variable", Area::Data),
    partial("Recode", Area::Data),
    partial("Convert type", Area::Data),
    partial("Reference level", Area::Data),
    partial("Combine levels", Area::Data),
    partial("Reorder levels", Area::Data),
    partial("Text case and spaces", Area::Data),
    partial("Find and replace text", Area::Data),
    partial("Extract part of text", Area::Data),
    partial("Text to date", Area::Data),
    partial("Date part", Area::Data),
    partial("Filter rows", Area::Data),
    partial("Sort rows", Area::Data),
    partial("Rename variable", Area::Data),
    partial("Variable label", Area::Data),
    partial("Value labels", Area::Data),
    partial("Labelled to factor", Area::Data),
    partial("Rename object", Area::Data),
    partial("Remove object", Area::Data),
    partial("Save object", Area::File),
    planned("Join", Area::Data),
    planned("Reshape", Area::Data),
    partial("Summary statistics", Area::Statistics),
    partial("Frequency table", Area::Statistics),
    partial("Cross-tabulation", Area::Statistics),
    partial("Correlation", Area::Statistics),
    partial("Missing-data patterns", Area::Statistics),
    partial("Normality test", Area::Statistics),
    partial("One-sample t-test", Area::Statistics),
    partial("Two-sample t-test", Area::Statistics),
    partial("Paired t-test", Area::Statistics),
    partial("Test of proportions", Area::Statistics),
    partial("Exact binomial test", Area::Statistics),
    partial("Fisher's exact test", Area::Statistics),
    partial("Mann-Whitney test", Area::Statistics),
    partial("Wilcoxon signed-rank test", Area::Statistics),
    partial("Kruskal-Wallis test", Area::Statistics),
    partial("Variance test", Area::Statistics),
    partial("Linear model", Area::Statistics),
    partial("One-way ANOVA", Area::Statistics),
    partial("Factorial ANOVA", Area::Statistics),
    partial("Logistic model", Area::Statistics),
    partial("Poisson model", Area::Statistics),
    partial("Negative binomial model", Area::Statistics),
    partial("Ordinal model", Area::Statistics),
    partial("Multinomial model", Area::Statistics),
    partial("Model fit", Area::Statistics),
    partial("Compare models", Area::Statistics),
    partial("Predicted values and residuals", Area::Statistics),
    partial("Model diagnostics", Area::Statistics),
    partial("Robust standard errors", Area::Statistics),
    planned("Histogram", Area::Graphs),
    planned("Scatter plot", Area::Graphs),
    planned("Model table", Area::Reports),
    planned("Search packages", Area::Packages),
    planned("Browse task views", Area::Packages),
    planned("Describe package", Area::Packages),
    planned("Install package", Area::Packages),
    planned("Installed packages", Area::Packages),
    planned("Update packages", Area::Packages),
    planned("Stata mode switch", Area::Stata),
    planned("Translate Stata code", Area::Stata),
    planned("Translate do-file", Area::Stata),
    planned("Open do-file and run", Area::Stata),
    planned("Do-file dry-run report", Area::Stata),
    planned("Stata and R side by side", Area::Stata),
    planned("Stata differences guide", Area::Stata),
    planned("Help topic", Area::Help),
    planned("Search help", Area::Help),
    planned("Vignettes", Area::Help),
    planned("RCraft manual", Area::Help),
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_are_unique() {
        let mut names: Vec<&str> = ENTRIES.iter().map(|e| e.name).collect();
        names.sort_unstable();
        let before = names.len();
        names.dedup();
        assert_eq!(before, names.len());
    }
}
