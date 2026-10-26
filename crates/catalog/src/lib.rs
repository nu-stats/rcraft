//! The catalog of menu entries and their coverage status.
//!
//! Every user-visible action is an entry here (AGENTS.md section 3).
//! `cargo xtask coverage` renders `docs/coverage.md` from [`ENTRIES`].

/// Where an entry sits in the menus.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Area {
    File,
    Data,
    Statistics,
    Graphs,
    Reports,
    Packages,
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

const fn planned(name: &'static str, area: Area) -> Entry {
    Entry {
        name,
        area,
        status: Status::Planned,
    }
}

/// The inventory. Seeded from ROADMAP.md; grows with each phase.
pub const ENTRIES: &[Entry] = &[
    planned("Use my file", Area::File),
    planned("Save data", Area::File),
    planned("New variable", Area::Data),
    planned("Recode", Area::Data),
    planned("Filter rows", Area::Data),
    planned("Join", Area::Data),
    planned("Reshape", Area::Data),
    planned("Summary statistics", Area::Statistics),
    planned("Frequency table", Area::Statistics),
    planned("Cross-tabulation", Area::Statistics),
    planned("t-test", Area::Statistics),
    planned("Linear model", Area::Statistics),
    planned("Logistic model", Area::Statistics),
    planned("Histogram", Area::Graphs),
    planned("Scatter plot", Area::Graphs),
    planned("Model table", Area::Reports),
    planned("Search packages", Area::Packages),
    planned("Browse task views", Area::Packages),
    planned("Describe package", Area::Packages),
    planned("Install package", Area::Packages),
    planned("Installed packages", Area::Packages),
    planned("Update packages", Area::Packages),
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
