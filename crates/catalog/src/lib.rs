//! The catalog of menu entries and their coverage status.
//!
//! Every user-visible action is an entry here (AGENTS.md section 3).
//! `cargo xtask coverage` renders `docs/coverage.md` from [`ENTRIES`].
//! Entries with a dialog have a [`rcraft_rcodegen::dialog::Spec`] in an area
//! module ([`data`]).

pub mod bayes;
pub mod combine;
pub mod data;
pub mod data_tools;
pub mod examples;
pub mod generalised;
pub mod graphs;
pub mod inference;
pub mod iv;
pub mod models;
pub mod multilevel;
pub mod multivariate;
pub mod panel;
pub mod plot_export;
pub mod postest;
pub mod report;
pub mod resampling;
pub mod save;
pub mod stats;
pub mod survey;
pub mod survival;
pub mod timeseries;

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

#[allow(dead_code)]
const fn partial(name: &'static str, area: Area) -> Entry {
    Entry {
        name,
        area,
        status: Status::Partial,
    }
}

const fn verified(name: &'static str, area: Area) -> Entry {
    Entry {
        name,
        area,
        status: Status::Verified,
    }
}

const fn planned(name: &'static str, area: Area) -> Entry {
    Entry {
        name,
        area,
        status: Status::Planned,
    }
}

/// Every dialog of the catalog, once each (a spec listed in two menus is not
/// repeated).
pub fn all_specs() -> Vec<&'static rcraft_rcodegen::dialog::Spec> {
    let lists: [&[&'static rcraft_rcodegen::dialog::Spec]; 11] = [
        data::SPECS,
        data::OBJECT_SPECS,
        stats::SPECS,
        graphs::SPECS,
        postest::MODEL_SPECS,
        report::MODEL_REPORTS,
        report::TABLE_REPORTS,
        report::REPORT_SPECS,
        examples::SPECS,
        save::SPECS,
        plot_export::PLOT_SPECS,
    ];
    let mut out: Vec<&'static rcraft_rcodegen::dialog::Spec> = Vec::new();
    for spec in lists.iter().flat_map(|l| l.iter()) {
        if !out.iter().any(|s| s.menu == spec.menu) {
            out.push(spec);
        }
    }
    out
}

/// The inventory. Seeded from ROADMAP.md; grows with each phase.
pub const ENTRIES: &[Entry] = &[
    verified("Use my file", Area::File),
    verified("Example data", Area::File),
    verified("Text file", Area::File),
    verified("Excel workbook", Area::File),
    verified("Stata, SPSS or SAS", Area::File),
    verified("R data", Area::File),
    verified("Parquet or Feather", Area::File),
    verified("Save workspace", Area::File),
    verified("New variable", Area::Data),
    verified("Recode", Area::Data),
    verified("Convert type", Area::Data),
    verified("Reference level", Area::Data),
    verified("Combine levels", Area::Data),
    verified("Reorder levels", Area::Data),
    verified("Text case and spaces", Area::Data),
    verified("Find and replace text", Area::Data),
    verified("Extract part of text", Area::Data),
    verified("Text to date", Area::Data),
    verified("Date part", Area::Data),
    verified("Filter rows", Area::Data),
    verified("Sort rows", Area::Data),
    verified("Rename variable", Area::Data),
    verified("Variable label", Area::Data),
    verified("Value labels", Area::Data),
    verified("Labelled to factor", Area::Data),
    verified("Rename object", Area::Data),
    verified("Remove object", Area::Data),
    verified("Save object", Area::File),
    verified("Join", Area::Data),
    verified("Append", Area::Data),
    verified("Aggregate", Area::Data),
    verified("Wide to long", Area::Data),
    verified("Long to wide", Area::Data),
    verified("Duplicates", Area::Data),
    verified("Random sample", Area::Data),
    verified("Drop missing values", Area::Data),
    verified("Summary statistics", Area::Statistics),
    verified("Frequency table", Area::Statistics),
    verified("Cross-tabulation", Area::Statistics),
    verified("Correlation", Area::Statistics),
    verified("Missing-data patterns", Area::Statistics),
    verified("Normality test", Area::Statistics),
    verified("One-sample t-test", Area::Statistics),
    verified("Two-sample t-test", Area::Statistics),
    verified("Paired t-test", Area::Statistics),
    verified("Test of proportions", Area::Statistics),
    verified("Exact binomial test", Area::Statistics),
    verified("Fisher's exact test", Area::Statistics),
    verified("Mann-Whitney test", Area::Statistics),
    verified("Wilcoxon signed-rank test", Area::Statistics),
    verified("Kruskal-Wallis test", Area::Statistics),
    verified("Variance test", Area::Statistics),
    verified("Agreement (kappa)", Area::Statistics),
    verified(
        "Agreement of several raters (Fleiss' kappa)",
        Area::Statistics,
    ),
    verified("Reliability (intraclass correlation)", Area::Statistics),
    verified("Linear model", Area::Statistics),
    verified("One-way ANOVA", Area::Statistics),
    verified("Factorial ANOVA", Area::Statistics),
    verified("Logistic model", Area::Statistics),
    verified("Poisson model", Area::Statistics),
    verified("Negative binomial model", Area::Statistics),
    verified("Ordinal model", Area::Statistics),
    verified("Multinomial model", Area::Statistics),
    verified("Fixed-effects regression", Area::Statistics),
    verified("Fixed-effects count or binary model", Area::Statistics),
    verified("Panel model", Area::Statistics),
    verified("Hausman test (fixed or random effects)", Area::Statistics),
    verified("Dynamic panel model (GMM)", Area::Statistics),
    verified("Panel instrumental variables", Area::Statistics),
    verified("Panel unit-root test", Area::Statistics),
    verified("Panel serial-correlation test", Area::Statistics),
    verified("Principal components", Area::Statistics),
    verified("Factor analysis", Area::Statistics),
    verified("Cluster analysis", Area::Statistics),
    verified("Discriminant analysis", Area::Statistics),
    verified("Bootstrap", Area::Statistics),
    verified("Permutation test", Area::Statistics),
    verified("Bayesian regression", Area::Statistics),
    verified("Kaplan-Meier survival curves", Area::Statistics),
    verified("Log-rank test", Area::Statistics),
    verified("Cox proportional hazards model", Area::Statistics),
    verified("Parametric survival model", Area::Statistics),
    verified("Survey means and totals", Area::Statistics),
    verified("Survey proportions and tables", Area::Statistics),
    verified("Survey regression", Area::Statistics),
    verified("Two-stage least squares", Area::Statistics),
    verified("Heckman selection model", Area::Statistics),
    verified("Linear mixed model", Area::Statistics),
    verified("Logistic or Poisson mixed model", Area::Statistics),
    verified("ARIMA model", Area::Statistics),
    verified("Autocorrelations", Area::Statistics),
    verified("Model fit", Area::Statistics),
    verified("Compare models", Area::Statistics),
    verified("Predicted values and residuals", Area::Statistics),
    verified("Model diagnostics", Area::Statistics),
    verified("Diagnostic plots", Area::Statistics),
    verified("Robust standard errors", Area::Statistics),
    verified("Test terms", Area::Statistics),
    verified("Sums of squares (Type II/III)", Area::Statistics),
    verified("Custom contrasts", Area::Statistics),
    verified("Average marginal effects", Area::Statistics),
    verified("Histogram", Area::Graphs),
    verified("Density plot", Area::Graphs),
    verified("Box plot", Area::Graphs),
    verified("Bar chart", Area::Graphs),
    verified("Normal Q-Q plot", Area::Graphs),
    verified("Scatter plot", Area::Graphs),
    verified("Line plot", Area::Graphs),
    verified("Scatter plot matrix", Area::Graphs),
    verified("Heatmap", Area::Graphs),
    verified("Export plot", Area::Graphs),
    verified("Model table", Area::Reports),
    verified("Export table", Area::Reports),
    verified("Add table to report", Area::Reports),
    verified("Add plot to report", Area::Reports),
    verified("Add text to report", Area::Reports),
    verified("Report contents", Area::Reports),
    verified("Save report", Area::Reports),
    verified("Clear report", Area::Reports),
    verified("Search packages", Area::Packages),
    verified("Browse task views", Area::Packages),
    verified("Describe package", Area::Packages),
    verified("Install package", Area::Packages),
    verified("Installed packages", Area::Packages),
    verified("Update packages", Area::Packages),
    verified("Stata mode switch", Area::Stata),
    verified("Translate Stata code", Area::Stata),
    verified("Translate do-file", Area::Stata),
    verified("Open do-file and run", Area::Stata),
    verified("Do-file dry-run report", Area::Stata),
    verified("Stata and R side by side", Area::Stata),
    verified("Stata differences guide", Area::Stata),
    verified("Help topic", Area::Help),
    verified("Search help", Area::Help),
    verified("Vignettes", Area::Help),
    planned("RCraft manual", Area::Help),
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_dialog_has_a_help_page_with_an_example_and_function_links() {
        let specs = all_specs();
        assert!(specs.len() > 60, "{} dialogs", specs.len());
        let mut without_example: Vec<&str> = Vec::new();
        for spec in specs {
            let page = rcraft_rcodegen::dialog_help::for_spec(spec);
            assert!(!page.about.is_empty(), "{}: no about", spec.entry);
            assert_eq!(page.options.len(), spec.widgets.len(), "{}", spec.entry);
            assert!(
                page.options.iter().all(|o| !o.label.is_empty()),
                "{}: an option has no label",
                spec.entry
            );
            if page.example.is_none() {
                without_example.push(spec.entry);
            }
            assert!(
                !page.functions.is_empty() || page.example.is_none(),
                "{}: code with no function calls",
                spec.entry
            );
        }
        assert!(
            without_example.is_empty(),
            "no example could be made for: {without_example:?}"
        );
    }

    /// Writes every dialog's help page to `RCRAFT_HELP_PAGES` (a file), for
    /// reading them all through (P8.4). Does nothing without it.
    #[test]
    fn help_pages_can_be_written_out_for_review() {
        let Ok(path) = std::env::var("RCRAFT_HELP_PAGES") else {
            return;
        };
        let mut out = String::new();
        for spec in all_specs() {
            let p = rcraft_rcodegen::dialog_help::for_spec(spec);
            out.push_str(&format!(
                "=== {} [{}]
{}
",
                p.title, p.menu, p.about
            ));
            for o in &p.options {
                out.push_str(&format!(
                    "  - {}: {}
",
                    o.label, o.text
                ));
            }
            if !p.needs.is_empty() {
                out.push_str(&format!(
                    "  needs: {}
",
                    p.needs.join(", ")
                ));
            }
            for n in &p.notes {
                out.push_str(&format!(
                    "  note: {n}
"
                ));
            }
            out.push_str(&format!(
                "  example: {}

",
                p.example.as_deref().unwrap_or("(none)")
            ));
        }
        assert!(std::fs::write(path, out).is_ok());
    }

    #[test]
    fn names_are_unique() {
        let mut names: Vec<&str> = ENTRIES.iter().map(|e| e.name).collect();
        names.sort_unstable();
        let before = names.len();
        names.dedup();
        assert_eq!(before, names.len());
    }
}
