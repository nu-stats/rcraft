//! How much of a set of do-files RCraft runs on R (P2C.8), by tier of
//! Stata: 1 data management and control flow, 2 core statistics, 3 panel,
//! survival, survey, IV and the other models, 4 graphs, 5 programs and chosen
//! SSC commands. Lines whose command is in none of them are counted apart.
//!
//! The numbers come from dry runs ([`crate::dofile::dry_run`]): a line is
//! translated, not translated (with the reason), or known only when the
//! do-file runs, which a dry run cannot say. `docs/coverage.md` holds them
//! for StatCraft's probe do-files.

use std::collections::BTreeMap;

use crate::dofile::{status, DoFile, Status, What};
use crate::reader::read;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Tier {
    Data = 1,
    Statistics = 2,
    Models = 3,
    Graphs = 4,
    Programs = 5,
    /// In none of the tiers.
    Other = 6,
}

impl Tier {
    pub const ALL: [Tier; 6] = [
        Tier::Data,
        Tier::Statistics,
        Tier::Models,
        Tier::Graphs,
        Tier::Programs,
        Tier::Other,
    ];

    pub fn title(self) -> &'static str {
        match self {
            Tier::Data => "1. Data management and control flow",
            Tier::Statistics => "2. Core statistics",
            Tier::Models => "3. Panel, survival, survey, IV and other models",
            Tier::Graphs => "4. Graphs",
            Tier::Programs => "5. Programs and SSC commands",
            Tier::Other => "Other commands",
        }
    }
}

const DATA: &[&str] = &[
    "append",
    "assert",
    "browse",
    "by",
    "bysort",
    "capture",
    "cd",
    "center",
    "char",
    "clear",
    "clonevar",
    "codebook",
    "cmdlog",
    "codebook",
    "collapse",
    "compress",
    "continue",
    "contract",
    "count",
    "cross",
    "datasignature",
    "decode",
    "describe",
    "destring",
    "discard",
    "display",
    "do",
    "drawnorm",
    "drop",
    "ds",
    "duplicates",
    "edit",
    "egen",
    "else",
    "encode",
    "ereturn",
    "exit",
    "expand",
    "export",
    "filefilter",
    "fillin",
    "foreach",
    "format",
    "forvalues",
    "frame",
    "frames",
    "generate",
    "global",
    "gsort",
    "if",
    "import",
    "include",
    "input",
    "insheet",
    "inspect",
    "isid",
    "isvar",
    "joinby",
    "keep",
    "label",
    "levelsof",
    "list",
    "local",
    "log",
    "lookfor",
    "macro",
    "matrix",
    "merge",
    "more",
    "mvdecode",
    "mvencode",
    "noisily",
    "note",
    "notes",
    "order",
    "outsheet",
    "pause",
    "post",
    "postclose",
    "postfile",
    "postutil",
    "preserve",
    "putexcel",
    "pwd",
    "query",
    "quietly",
    "recast",
    "recode",
    "rename",
    "replace",
    "reshape",
    "restore",
    "return",
    "run",
    "sample",
    "save",
    "scalar",
    "separate",
    "set",
    "sleep",
    "sort",
    "split",
    "stack",
    "sysuse",
    "tempfile",
    "tempname",
    "tempvar",
    "tostring",
    "unab",
    "use",
    "version",
    "webuse",
    "which",
    "while",
    "winsor",
    "xpose",
];

const STATISTICS: &[&str] = &[
    "_pctile",
    "alpha",
    "anova",
    "centile",
    "ci",
    "correlate",
    "estat",
    "estimates",
    "factor",
    "glm",
    "kwallis",
    "lincom",
    "logistic",
    "logit",
    "margins",
    "mean",
    "mlogit",
    "nbreg",
    "nlcom",
    "ologit",
    "oneway",
    "oprobit",
    "pca",
    "pctile",
    "poisson",
    "predict",
    "probit",
    "proportion",
    "prtest",
    "pwcorr",
    "qreg",
    "ranksum",
    "ratio",
    "regress",
    "sdtest",
    "signrank",
    "sktest",
    "spearman",
    "summarize",
    "swilk",
    "tabstat",
    "table",
    "tabulate",
    "test",
    "testparm",
    "tobit",
    "total",
    "ttest",
    "xtile",
];

const MODELS: &[&str] = &[
    "areg",
    "asclogit",
    "arima",
    "betareg",
    "biprobit",
    "bootstrap",
    "bsample",
    "clogit",
    "cluster2",
    "cloglog",
    "cmclogit",
    "dfgls",
    "dfuller",
    "didregress",
    "fracreg",
    "gsem",
    "heckman",
    "hetprobit",
    "intreg",
    "ivprobit",
    "ivregress",
    "ivtobit",
    "jackknife",
    "lasso",
    "ltable",
    "meglm",
    "melogit",
    "menbreg",
    "mepoisson",
    "mi",
    "mixed",
    "mlexp",
    "mprobit",
    "mvreg",
    "newey",
    "nl",
    "nlogit",
    "nlsur",
    "permute",
    "prais",
    "rreg",
    "sem",
    "simulate",
    "slogit",
    "statsby",
    "stcox",
    "stcurve",
    "stir",
    "stptime",
    "streg",
    "sts",
    "stset",
    "sureg",
    "svy",
    "svyset",
    "teffects",
    "tnbreg",
    "tpoisson",
    "truncreg",
    "tsfill",
    "tsset",
    "ucm",
    "var",
    "xtabond",
    "xtcloglog",
    "xtdescribe",
    "xtdidregress",
    "xtdpd",
    "xtdpdsys",
    "xtgee",
    "xtgls",
    "xtivreg",
    "xtlogit",
    "xtnbreg",
    "xtologit",
    "xtoprobit",
    "xtpcse",
    "xtpoisson",
    "xtprobit",
    "xtreg",
    "xtregar",
    "xtset",
    "xtsum",
    "xttab",
    "xttobit",
    "xtunitroot",
    "zinb",
    "zip",
];

const GRAPHS: &[&str] = &[
    "_natscale",
    "connected",
    "graph",
    "grstyle",
    "histogram",
    "kdensity",
    "line",
    "lowess",
    "marginsplot",
    "scatter",
    "twoway",
];

const PROGRAMS: &[&str] = &[
    "ado",
    "args",
    "distinct",
    "end",
    "estout",
    "esttab",
    "eststo",
    "fre",
    "frmttable",
    "gettoken",
    "ivreg2",
    "mata",
    "net",
    "outreg",
    "outreg2",
    "plugin",
    "program",
    "python",
    "reghdfe",
    "ssc",
    "syntax",
    "tokenize",
    "unique",
    "winsor2",
];

/// Which tier a command is in, by its full name or a short form of one
/// (`reg`, `gen`, `su`): the full name comes from the reader's table when
/// it knows the word, else the one name in the tiers the word starts.
pub fn tier(command: &str) -> Tier {
    let word = command.trim();
    let full = read(word)
        .commands()
        .first()
        .map(|c| c.name.clone())
        .unwrap_or_else(|| word.to_string());
    let tables = [
        (Tier::Data, DATA),
        (Tier::Statistics, STATISTICS),
        (Tier::Models, MODELS),
        (Tier::Graphs, GRAPHS),
        (Tier::Programs, PROGRAMS),
    ];
    for (t, names) in tables {
        if names.contains(&full.as_str()) {
            return t;
        }
    }
    if word.len() >= 2 {
        let starts: Vec<Tier> = tables
            .iter()
            .flat_map(|(t, names)| names.iter().filter(|n| n.starts_with(word)).map(|_| *t))
            .collect();
        if let [one] = starts.as_slice() {
            return *one;
        }
    }
    Tier::Other
}

/// Lines of one tier, or of one command.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Count {
    pub lines: usize,
    /// Translated, or done by RCraft itself (a macro, a loop).
    pub translated: usize,
    pub not: usize,
    /// Known only when the do-file runs.
    pub later: usize,
}

impl Count {
    fn add(&mut self, st: Status) {
        self.lines += 1;
        match st {
            Status::Clean | Status::Notes | Status::ByRcraft => self.translated += 1,
            Status::Not => self.not += 1,
            Status::Later => self.later += 1,
        }
    }

    /// The share translated of the lines a dry run can judge, in percent.
    pub fn share(&self) -> f64 {
        let judged = self.translated + self.not;
        if judged == 0 {
            return 0.0;
        }
        100.0 * self.translated as f64 / judged as f64
    }
}

/// Several do-files' dry runs, summed up.
#[derive(Debug, Clone, Default)]
pub struct Coverage {
    pub files: usize,
    pub tiers: BTreeMap<Tier, Count>,
    pub commands: BTreeMap<String, Count>,
    /// Why lines are not translated, with how often.
    pub reasons: BTreeMap<String, usize>,
}

impl Coverage {
    pub fn add(&mut self, d: &DoFile) {
        self.files += 1;
        for (line, origin) in d.translation.lines.iter().zip(&d.origins) {
            let st = status(line, origin);
            let word = crate::dofile::command_name(line.stata.trim());
            // a macro set or a loop line is control flow
            let t = match origin.what {
                What::Macro | What::Control => Tier::Data,
                _ => tier(&word),
            };
            self.tiers.entry(t).or_default().add(st);
            if !word.is_empty() {
                self.commands.entry(word).or_default().add(st);
            }
            if let (Status::Not, Err(e)) = (st, &line.result) {
                *self.reasons.entry(e.reason.clone()).or_default() += 1;
            }
        }
    }

    /// Every tier together.
    pub fn total(&self) -> Count {
        let mut all = Count::default();
        for c in self.tiers.values() {
            all.lines += c.lines;
            all.translated += c.translated;
            all.not += c.not;
            all.later += c.later;
        }
        all
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dofile::{dry_run, NoFiles, Options};

    #[test]
    fn commands_fall_in_their_tiers() {
        assert_eq!(tier("gen"), Tier::Data);
        assert_eq!(tier("reg"), Tier::Statistics);
        assert_eq!(tier("su"), Tier::Statistics);
        assert_eq!(tier("xtreg"), Tier::Models);
        assert_eq!(tier("tw"), Tier::Graphs);
        assert_eq!(tier("esttab"), Tier::Programs);
        assert_eq!(tier("frobnicate"), Tier::Other);
    }

    #[test]
    fn a_dry_run_is_counted_by_tier() {
        let d = dry_run(
            "clear\nset obs 3\ngenerate x = _n\nforeach v in x {\nsummarize `v'\n}\nlocal m = r(mean)\nprogram define f\nend\n",
            &NoFiles,
            Options::default(),
        );
        let mut c = Coverage::default();
        c.add(&d);
        assert_eq!(c.files, 1);
        let stats = c.tiers[&Tier::Statistics];
        assert_eq!((stats.lines, stats.translated), (1, 1), "{c:?}");
        assert!(c.tiers[&Tier::Data].later >= 1, "{c:?}");
        assert_eq!(c.total().lines, d.translation.lines.len());
        assert!(c.commands.contains_key("summarize"));
    }
}
