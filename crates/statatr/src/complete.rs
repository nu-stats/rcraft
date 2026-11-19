//! Tab completion for Stata lines (P2B.12), for the Command window and the
//! Script Editor in Stata mode.
//!
//! What Tab offers depends on where the word being typed sits in the line,
//! read the way Stata reads it:
//!
//! - in command position (after any `by ...:`, `quietly`, `capture` and
//!   `noisily`): the commands the translator knows, and those prefixes;
//! - after the comma: that command's options (`level(` with its parenthesis
//!   when the option takes an argument), and inside `vce(` its kinds;
//! - after `using`, the first word of `use`/`save`, and inside quotes: file
//!   names (folders end in `/`);
//! - after a backtick a local macro's name (with its closing quote), after
//!   `$` a global's; after `label values var` a value label's;
//! - anywhere else: the variables of the data in memory, in their order,
//!   with Stata's rules: a unique abbreviation is the name, `*`, `?` and `~`
//!   are wildcards and `a-c` the variables from `a` to `c`, so Tab on a
//!   pattern or a range writes out the variables it stands for; `if` and `in`
//!   are offered as well after a varlist.
//!
//! The words come from the translator's own tables: the commands it
//! dispatches and the options each one takes (a test translates every
//! option offered here and fails when the translator does not know it).
//! This module only reads the line; the caller says what the session holds
//! (variables, macros, labels, files), so nothing here starts R.

/// Most choices kept from one Tab.
pub const MAX_OPTIONS: usize = 60;

/// Where the word being completed starts in the line (a byte offset), and
/// what it could become.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Completion {
    pub from: usize,
    pub options: Vec<String>,
}

/// What the session holds, for the words Tab offers.
pub struct Context<'a> {
    /// The variables of the data in memory, in their order.
    pub variables: &'a [String],
    /// Local and global macro names.
    pub locals: &'a [String],
    pub globals: &'a [String],
    /// Value-label names.
    pub labels: &'a [String],
    /// The entries of a folder (`""` for the working one, or as typed, such
    /// as `data/`): file names, and folder names ending in `/`.
    pub files: &'a dyn Fn(&str) -> Vec<String>,
}

/// The commands the translator knows, the prefixes and blocks the reader
/// takes, and what the do-file runner (`crate::dofile`) does itself, offered
/// in command position.
pub const COMMANDS: &[&str] = &[
    "_pctile",
    "alpha",
    "anova",
    "anovaplot",
    "append",
    "asclogit",
    "assert",
    "avplot",
    "bitest",
    "bitesti",
    "bootstrap",
    "bsample",
    "bysort",
    "by",
    "capture",
    "cc",
    "cci",
    "cd",
    "center",
    "centile",
    "char",
    "ci",
    "cii",
    "clear",
    "clogit",
    "clonevar",
    "cluster2",
    "cmclogit",
    "cmset",
    "codebook",
    "coefplot",
    "collapse",
    "compare",
    "compress",
    "confirm",
    "contract",
    "connected",
    "continue",
    "correlate",
    "corrgram",
    "count",
    "cs",
    "csi",
    "cumul",
    "decode",
    "describe",
    "dfgls",
    "dfuller",
    "dstdize",
    "destring",
    "display",
    "do",
    "drawnorm",
    "drop",
    "ds",
    "dslogit",
    "dsregress",
    "duplicates",
    "dyndoc",
    "dyntext",
    "egen",
    "elasticnet",
    "encode",
    "erase",
    "ereturn",
    "estadd",
    "estat",
    "estimates",
    "estpost",
    "eststo",
    "esttab",
    "exit",
    "expand",
    "expandcl",
    "factor",
    "fdasave",
    "fdause",
    "file",
    "filefilter",
    "fillin",
    "foreach",
    "format",
    "forvalues",
    "frame",
    "fre",
    "frmttable",
    "fvset",
    "generate",
    "gettoken",
    "global",
    "graph",
    "grstyle",
    "gsort",
    "histogram",
    "icc",
    "import",
    "include",
    "input",
    "inspect",
    "insheet",
    "ir",
    "iri",
    "isid",
    "isvar",
    "ivprobit",
    "ivregress",
    "jackknife",
    "joinby",
    "kap",
    "kdensity",
    "keep",
    "ktau",
    "kwallis",
    "label",
    "labelbook",
    "lasso",
    "lassocoef",
    "lassoinfo",
    "lassoknots",
    "lassoselect",
    "levelsof",
    "lincom",
    "line",
    "list",
    "local",
    "lookfor",
    "log",
    "logit",
    "lowess",
    "lroc",
    "macro",
    "margins",
    "marginsplot",
    "markdown",
    "matrix",
    "mean",
    "medeff",
    "merge",
    "mhodds",
    "misstable",
    "mkdir",
    "mkmat",
    "mlogit",
    "mvdecode",
    "mvencode",
    "mvtest",
    "nbreg",
    "nestreg",
    "newey",
    "nl",
    "nlcom",
    "note",
    "notes",
    "npregress",
    "noisily",
    "ologit",
    "oneway",
    "oprobit",
    "order",
    "outfile",
    "outreg",
    "outreg2",
    "outsheet",
    "parmest",
    "pca",
    "pcorr",
    "pctile",
    "pdslasso",
    "permute",
    "pnorm",
    "poisson",
    "pologit",
    "poregress",
    "post",
    "postclose",
    "postfile",
    "postutil",
    "power",
    "pperron",
    "prais",
    "predict",
    "preserve",
    "probit",
    "program",
    "proportion",
    "prtest",
    "putdocx",
    "putexcel",
    "putpdf",
    "pwcorr",
    "pwd",
    "pwmean",
    "qnorm",
    "quietly",
    "ranksum",
    "ratio",
    "recast",
    "recode",
    "reghdfe",
    "regress",
    "rename",
    "replace",
    "reshape",
    "restore",
    "revrs",
    "rmdir",
    "rreg",
    "rvfplot",
    "robreg",
    "robvar",
    "rolling",
    "rotate",
    "return",
    "run",
    "save",
    "saveold",
    "scalar",
    "scatter",
    "screeplot",
    "sdtest",
    "sdtesti",
    "set",
    "signrank",
    "signtest",
    "simulate",
    "sktest",
    "snapshot",
    "sort",
    "spearman",
    "split",
    "spmatrix",
    "spset",
    "sqrtlasso",
    "ssc",
    "stcox",
    "statsby",
    "stepwise",
    "stset",
    "suest",
    "summarize",
    "survwgt",
    "svy",
    "svyset",
    "svmat",
    "swilk",
    "syntax",
    "tab1",
    "tab2",
    "table",
    "tabm",
    "tabodds",
    "tabstat",
    "tabulate",
    "tempfile",
    "tempname",
    "tempvar",
    "test",
    "testnl",
    "testparm",
    "tetrachoric",
    "tokenize",
    "tostring",
    "total",
    "tsappend",
    "tsreport",
    "tsset",
    "ttest",
    "ttesti",
    "twoway",
    "unab",
    "use",
    "varsoc",
    "version",
    "which",
    "while",
    "winsor",
    "wntestq",
    "xporegress",
    "xpose",
    "xtabond",
    "xtdescribe",
    "xtdpd",
    "xtdpdsys",
    "xtile",
    "xtreg",
    "xtregar",
    "xtset",
    "xtsum",
    "xtunitroot",
    "xttab",
];

/// The options of each command, as the translator takes them; a trailing
/// `(` marks one that takes an argument.
pub const OPTIONS: &[(&str, &[&str])] = &[
    ("anova", &["partial", "repeated(", "sequential"]),
    ("anovaplot", &["scatter(", "title(", "xtitle(", "ytitle("]),
    ("_pctile", &["nquantiles(", "percentiles("]),
    ("append", &["force", "generate(", "nolabel", "nonotes"]),
    (
        "asclogit",
        &[
            "alternatives(",
            "case(",
            "level(",
            "noconstant",
            "robust",
            "vce(",
        ],
    ),
    ("drawnorm", &["clear", "cov(", "double", "means(", "n("]),
    (
        "clogit",
        &[
            "baselevels",
            "group(",
            "level(",
            "or",
            "robust",
            "strata(",
            "vce(",
        ],
    ),
    ("cluster2", &["fcluster(", "tcluster("]),
    ("cmclogit", &["casevars(", "noconstant", "vce("]),
    (
        "coefplot",
        &[
            "baselevels",
            "drop(",
            "generate(",
            "headings(",
            "keep(",
            "levels(",
            "nodraw",
            "omitted",
            "order(",
            "vertical",
            "xline(",
        ],
    ),
    ("collapse", &["by(", "cw", "fast"]),
    ("connected", &["by(", "legend(", "nodraw", "sort"]),
    ("correlate", &["covariance", "means"]),
    ("describe", &["short"]),
    (
        "destring",
        &["float", "force", "generate(", "ignore(", "replace"],
    ),
    ("duplicates", &["force", "generate("]),
    ("egen", &["by(", "missing"]),
    (
        "elasticnet",
        &[
            "alphas(",
            "grid(",
            "noconstant",
            "nolog",
            "penaltywt(",
            "rseed(",
            "selection(",
            "stop(",
        ],
    ),
    ("encode", &["generate("]),
    (
        "estat",
        &[
            "correlation",
            "cutoff(",
            "detail",
            "group(",
            "lags(",
            "nomiss0",
            "rhs",
            "table",
            "uncentered",
            "white",
        ],
    ),
    ("eststo", &["title("]),
    (
        "esttab",
        &[
            "addnotes(",
            "alignment(",
            "ar2",
            "b(",
            "booktabs",
            "cells(",
            "compress",
            "drop(",
            "fragment",
            "keep(",
            "label",
            "mgroups(",
            "mlabels(",
            "mtitles(",
            "nobaselevels",
            "nogaps",
            "nomtitles",
            "nonotes",
            "nonumbers",
            "nostar",
            "not",
            "order(",
            "p",
            "postfoot(",
            "posthead(",
            "prefoot(",
            "prehead(",
            "r2",
            "replace",
            "scalars(",
            "se",
            "star(",
            "starlevels(",
            "stats(",
            "tex",
            "title(",
            "varlabels(",
        ],
    ),
    ("expand", &["generate("]),
    ("expandcl", &["cluster(", "generate("]),
    ("file", &["append", "binary", "replace", "text", "write"]),
    ("generate", &["after(", "before("]),
    ("gettoken", &["match(", "parse("]),
    (
        "graph",
        &[
            "as(", "height(", "legend(", "name(", "nodraw", "over(", "replace", "saving(", "stack",
            "width(",
        ],
    ),
    ("gsort", &["generate(", "mfirst"]),
    (
        "factor",
        &["citerate(", "factors(", "ipf", "mineigen(", "pcf", "pf"],
    ),
    ("pca", &["components(", "covariance"]),
    (
        "rotate",
        &[
            "blanks(",
            "clear",
            "equamax",
            "horst",
            "nonormalize",
            "normalize",
            "oblique",
            "orthogonal",
            "promax",
            "quartimax",
            "varimax",
        ],
    ),
    ("joinby", &["unmatched(", "_merge("]),
    ("kap", &["absolute", "tab", "wgt("]),
    (
        "icc",
        &[
            "absolute",
            "consistency",
            "format(",
            "level(",
            "mixed",
            "testvalue(",
        ],
    ),
    (
        "tetrachoric",
        &[
            "bonferroni",
            "edwards",
            "matrix",
            "notable",
            "posdef",
            "print(",
            "pw",
            "sidak",
            "star(",
            "stats(",
        ],
    ),
    ("ranksum", &["by("]),
    ("kwallis", &["by("]),
    (
        "ktau",
        &[
            "bonferroni",
            "matrix",
            "print(",
            "pw",
            "sidak",
            "star(",
            "stats(",
        ],
    ),
    ("pdslasso", &["robust"]),
    ("postfile", &["every(", "replace"]),
    (
        "power",
        &[
            "alpha(",
            "continuity",
            "corr(",
            "direction(",
            "effect(",
            "eventprob(",
            "grweights(",
            "hratio(",
            "knownsd",
            "knownsds",
            "loghazard",
            "n(",
            "n1(",
            "n2(",
            "ncontrol(",
            "ngroups(",
            "notable",
            "npergroup(",
            "nratio(",
            "ntested(",
            "onesided",
            "power(",
            "prdiscordant(",
            "r2(",
            "schoenfeld",
            "sd(",
            "sd1(",
            "sd2(",
            "sddiff(",
            "sderror(",
            "sdx(",
            "sdy(",
            "studytime(",
            "table",
            "test(",
            "unconditional",
            "varerror(",
            "varmeans(",
        ],
    ),
    ("prtest", &["by(", "level("]),
    (
        "mvtest",
        &[
            "bivariate",
            "by(",
            "compound",
            "diagonal",
            "equal",
            "lr",
            "missing",
            "spherical",
            "stats(",
            "univariate",
            "zero",
        ],
    ),
    (
        "cs",
        &["by(", "exact", "istandard", "level(", "or", "woolf"],
    ),
    ("csi", &["exact", "level("]),
    ("cc", &["by(", "level(", "tb", "woolf"]),
    ("cci", &["level(", "tb", "woolf"]),
    ("ir", &["by(", "level("]),
    (
        "tabodds",
        &["adjust(", "base(", "binomial(", "level(", "or"],
    ),
    ("iri", &["level("]),
    ("marginsplot", &["noci", "title(", "xtitle(", "ytitle("]),
    ("medeff", &["mediate(", "seed(", "sims(", "treat("]),
    ("mhodds", &["by(", "compare(", "level("]),
    (
        "putexcel",
        &[
            "colnames", "modify", "names", "replace", "rownames", "sheet(",
        ],
    ),
    (
        "histogram",
        &[
            "bin(",
            "density",
            "discrete",
            "fraction",
            "frequency",
            "kdensity",
            "legend(",
            "nodraw",
            "normal",
            "percent",
            "start(",
            "width(",
        ],
    ),
    (
        "import",
        &[
            "allstring",
            "case(",
            "cellrange(",
            "clear",
            "delimiters(",
            "encoding(",
            "firstrow",
            "sheet(",
            "varnames(",
        ],
    ),
    (
        "insheet",
        &[
            "case",
            "clear",
            "comma",
            "delimiter(",
            "double",
            "names",
            "nonames",
            "tab",
        ],
    ),
    (
        "ivregress",
        &[
            "first",
            "igmm",
            "level(",
            "noconstant",
            "robust",
            "small",
            "vce(",
            "wmatrix(",
        ],
    ),
    (
        "kdensity",
        &[
            "at(",
            "bwidth(",
            "generate(",
            "kernel(",
            "n(",
            "nograph",
            "normal",
        ],
    ),
    ("label", &["add", "modify", "replace"]),
    (
        "lasso",
        &[
            "exposure(",
            "grid(",
            "noconstant",
            "nolog",
            "nolog",
            "offset(",
            "rseed(",
            "selection(",
            "stop(",
            "tolerance(",
        ],
    ),
    ("lassocoef", &["display(", "nolegend", "sort("]),
    ("lassoknots", &["alllambdas", "display("]),
    ("levelsof", &["clean", "local(", "missing", "separate("]),
    ("lincom", &["eform", "irr", "level(", "or", "rrr"]),
    ("mean", &["level(", "over("]),
    ("proportion", &["level(", "over("]),
    ("ratio", &["level(", "over("]),
    ("total", &["level(", "over("]),
    ("line", &["by(", "legend(", "nodraw", "sort"]),
    (
        "list",
        &[
            "abbreviate(",
            "clean",
            "noobs",
            "nolabel",
            "separator(",
            "table",
        ],
    ),
    (
        "logit",
        &[
            "level(",
            "nolog",
            "noconstant",
            "offset(",
            "or",
            "robust",
            "vce(",
        ],
    ),
    ("lowess", &["bwidth(", "generate(", "nograph"]),
    ("lroc", &["all", "beta(", "nograph"]),
    (
        "margins",
        &[
            "at(",
            "atmeans",
            "contrast",
            "dydx(",
            "dyex(",
            "eydx(",
            "eyex(",
            "level(",
            "over(",
            "post",
            "predict(",
            "pwcompare",
            "vce(",
        ],
    ),
    (
        "matrix",
        &["format(", "noblank", "nohalf", "noheader", "title("],
    ),
    ("mkdir", &["public"]),
    ("mvdecode", &["mv("]),
    ("mvencode", &["mv(", "override"]),
    ("mkmat", &["matrix(", "nomissing"]),
    (
        "merge",
        &[
            "assert(",
            "generate(",
            "keep(",
            "keepusing(",
            "nogenerate",
            "nolabel",
            "noreport",
            "replace",
            "sorted",
            "update",
        ],
    ),
    (
        "mlogit",
        &[
            "baseoutcome(",
            "level(",
            "nolog",
            "noconstant",
            "robust",
            "rrr",
            "vce(",
        ],
    ),
    ("newey", &["lag(", "level(", "noconstant"]),
    ("nl", &["iterate(", "level(", "nolog", "variables("]),
    (
        "nbreg",
        &[
            "exposure(",
            "irr",
            "level(",
            "nolog",
            "noconstant",
            "offset(",
            "robust",
            "vce(",
        ],
    ),
    ("ologit", &["level(", "nolog", "or", "robust", "vce("]),
    ("oprobit", &["level(", "nolog", "robust", "vce("]),
    (
        "order",
        &["after(", "alphabetic", "before(", "first", "last"],
    ),
    (
        "outfile",
        &[
            "comma",
            "nolabel",
            "noquote",
            "replace",
            "runtogether",
            "wide",
        ],
    ),
    (
        "outsheet",
        &[
            "comma",
            "delimiter(",
            "nolabel",
            "nonames",
            "noquote",
            "replace",
        ],
    ),
    (
        "outreg2",
        &[
            "adjr2",
            "append",
            "bdec(",
            "e(",
            "keep(",
            "parenthesis(",
            "r2",
            "replace",
            "tdec(",
            "tstat",
        ],
    ),
    (
        "poisson",
        &[
            "exposure(",
            "irr",
            "level(",
            "nolog",
            "noconstant",
            "offset(",
            "robust",
            "vce(",
        ],
    ),
    (
        "predict",
        &[
            "cooksd",
            "hat",
            "hr",
            "leverage",
            "outcome(",
            "pr",
            "residuals",
            "rstandard",
            "rstudent",
            "stdf",
            "stdp",
            "stdr",
            "xb",
        ],
    ),
    (
        "prais",
        &[
            "corc",
            "level(",
            "noconstant",
            "nolog",
            "rhotype(",
            "robust",
            "twostep",
            "vce(",
        ],
    ),
    (
        "probit",
        &["level(", "nolog", "noconstant", "offset(", "robust", "vce("],
    ),
    (
        "pwcorr",
        &[
            "bonferroni",
            "casewise",
            "listwise",
            "obs",
            "print(",
            "sidak",
            "sig",
            "star(",
        ],
    ),
    ("recode", &["copyrest", "generate(", "label(", "prefix("]),
    (
        "reghdfe",
        &[
            "absorb(",
            "cluster(",
            "keepsingletons",
            "level(",
            "noabsorb",
            "vce(",
        ],
    ),
    (
        "regress",
        &["beta", "level(", "noconstant", "robust", "vce("],
    ),
    ("rename", &["lower", "upper"]),
    ("reshape", &["i(", "j(", "string"]),
    ("revrs", &["replace"]),
    ("robreg", &["efficiency(", "huber", "level("]),
    ("robvar", &["by("]),
    ("dstdize", &["by(", "level("]),
    (
        "pwmean",
        &[
            "cieffects",
            "cimeans",
            "ci",
            "effects",
            "groups",
            "level(",
            "mcompare(",
            "over(",
            "pveffects",
            "sort",
        ],
    ),
    ("save", &["nolabel", "replace", "version("]),
    ("saveold", &["nolabel", "replace", "version("]),
    ("scatter", &["by(", "legend(", "nodraw", "sort"]),
    ("sort", &["stable"]),
    ("split", &["generate(", "limit(", "notrim", "parse("]),
    (
        "sqrtlasso",
        &[
            "grid(",
            "noconstant",
            "nolog",
            "penaltywt(",
            "rseed(",
            "selection(",
            "stop(",
        ],
    ),
    (
        "stcox",
        &[
            "breslow", "efron", "exactm", "exactp", "hr", "level(", "nohr", "noshow", "robust",
            "strata(", "vce(",
        ],
    ),
    (
        "stset",
        &[
            "enter(", "exit(", "failure(", "id(", "noshow", "origin(", "scale(",
        ],
    ),
    ("summarize", &["detail", "format", "meanonly", "separator("]),
    ("suest", &["vce("]),
    ("svyset", &["vce("]),
    ("survwgt", &["by(", "generate(", "maxrep(", "totvars("]),
    ("isid", &["missok"]),
    (
        "ivprobit",
        &["first", "iterate(", "level(", "nolog", "twostep", "vce("],
    ),
    (
        "contract",
        &[
            "cfreq(",
            "cpercent(",
            "float",
            "format(",
            "freq(",
            "nomiss",
            "percent(",
            "zero",
        ],
    ),
    ("recast", &["force"]),
    ("winsor", &["generate(", "h(", "highonly", "lowonly", "p("]),
    ("alpha", &["asis", "casewise", "generate(", "min(", "std"]),
    ("frmttable", &["sdec(", "sfmt(", "statmat(", "store("]),
    ("fdasave", &["rename", "replace", "vallabfile("]),
    ("fdause", &["clear", "novallabels"]),
    ("filefilter", &["from(", "replace", "to("]),
    (
        "center",
        &[
            "casewise",
            "double",
            "inplace",
            "nolabel",
            "prefix(",
            "replace",
            "standardize",
        ],
    ),
    ("svmat", &["names("]),
    ("tab1", &["missing", "nolabel", "sort"]),
    (
        "tab2",
        &[
            "cell",
            "chi2",
            "column",
            "firstonly",
            "missing",
            "nolabel",
            "row",
        ],
    ),
    ("table", &["missing", "nformat(", "nototals", "statistic("]),
    ("tabm", &["missing"]),
    ("test", &["accumulate", "matvlc(", "mtest", "notest"]),
    ("testnl", &["iterate(", "mtest"]),
    ("testparm", &["equal"]),
    (
        "oneway",
        &[
            "bonferroni",
            "missing",
            "noanova",
            "nofreq",
            "nolabel",
            "nomeans",
            "noobs",
            "nostandard",
            "scheffe",
            "sidak",
            "tabulate",
            "wrap",
        ],
    ),
    (
        "centile",
        &["cci", "centile(", "level(", "meansd", "normal"],
    ),
    ("nlcom", &["df(", "iterate(", "level(", "noheader", "post"]),
    (
        "npregress",
        &[
            "bwidth(",
            "derivbwidth(",
            "estimator(",
            "from(",
            "iterate(",
            "kernel(",
            "meanbwidth(",
            "noderivatives",
            "nolog",
            "predict(",
            "unidentsample(",
        ],
    ),
    (
        "cii",
        &[
            "agresti", "bonett", "exact", "jeffreys", "level(", "poisson", "sd", "wald", "wilson",
        ],
    ),
    ("sdtest", &["by(", "level("]),
    ("sdtesti", &["level("]),
    ("sktest", &["noadjust"]),
    ("snapshot", &["label("]),
    (
        "ci",
        &[
            "agresti",
            "bonett",
            "exact",
            "exposure(",
            "jeffreys",
            "level(",
            "poisson",
            "sd",
            "wald",
            "wilson",
        ],
    ),
    (
        "tabstat",
        &[
            "by(",
            "casewise",
            "columns(",
            "format(",
            "labelwidth(",
            "longstub",
            "missing",
            "noseparator",
            "nototal",
            "save",
            "statistics(",
            "varwidth(",
        ],
    ),
    (
        "tabulate",
        &[
            "cell",
            "chi2",
            "column",
            "freq",
            "generate(",
            "lrchi2",
            "matcell(",
            "matcol(",
            "matrow(",
            "means",
            "missing",
            "nofreq",
            "nokey",
            "nolabel",
            "nomeans",
            "noobs",
            "nostandard",
            "obs",
            "row",
            "sort",
            "standard",
            "summarize(",
            "wrap",
        ],
    ),
    ("tostring", &["generate(", "replace"]),
    ("ttest", &["by(", "level(", "unequal", "unpaired", "welch"]),
    ("ttesti", &["level(", "unequal", "welch"]),
    ("twoway", &["by(", "legend(", "nodraw", "sort"]),
    ("use", &["clear", "nolabel"]),
    ("xpose", &["clear", "promote", "varname"]),
    (
        "xtabond",
        &[
            "artests(",
            "endogenous(",
            "inst(",
            "lags(",
            "level(",
            "maxldep(",
            "maxlags(",
            "noconstant",
            "pre(",
            "twostep",
            "vce(",
        ],
    ),
    (
        "xtdpd",
        &[
            "artests(",
            "dgmmiv(",
            "div(",
            "hascons",
            "level(",
            "lgmmiv(",
            "liv(",
            "noconstant",
            "twostep",
            "vce(",
        ],
    ),
    (
        "xtdpdsys",
        &[
            "artests(",
            "endogenous(",
            "inst(",
            "lags(",
            "level(",
            "maxldep(",
            "maxlags(",
            "noconstant",
            "pre(",
            "twostep",
            "vce(",
        ],
    ),
    ("xtile", &["altdef", "cutpoints(", "nquantiles("]),
    ("xtdescribe", &["i(", "patterns(", "t(", "width("]),
    ("xtsum", &["i("]),
    ("xttab", &["i("]),
    ("tsreport", &["casewise", "detail", "panel"]),
    (
        "dfuller",
        &["drift", "lags(", "noconstant", "regress", "trend"],
    ),
    ("pperron", &["lags(", "noconstant", "regress", "trend"]),
    ("dfgls", &["ers", "maxlag(", "notrend"]),
    ("corrgram", &["lags(", "noplot", "yw"]),
    ("wntestq", &["lags("]),
    ("varsoc", &["maxlag("]),
    ("tsappend", &["add(", "last(", "tsfmt("]),
    ("pctile", &["altdef", "genp(", "nquantiles("]),
    ("cumul", &["by(", "equal", "freq", "generate("]),
    (
        "xtregar",
        &["fe", "lbi", "level(", "re", "rhof(", "rhotype(", "twostep"],
    ),
    (
        "xtreg",
        &[
            "be", "corr(", "fe", "i(", "level(", "mle", "pa", "re", "robust", "vce(",
        ],
    ),
    (
        "tsset",
        &[
            "clear",
            "clocktime",
            "daily",
            "delta(",
            "format(",
            "generic",
            "halfyearly",
            "monthly",
            "noquery",
            "quarterly",
            "weekly",
            "yearly",
        ],
    ),
    (
        "xtset",
        &[
            "clear",
            "clocktime",
            "daily",
            "delta(",
            "format(",
            "generic",
            "halfyearly",
            "monthly",
            "noquery",
            "quarterly",
            "weekly",
            "yearly",
        ],
    ),
];

/// The subcommands the translator takes after a command's name.
const SUBCOMMANDS: &[(&str, &[&str])] = &[
    (
        "spmatrix",
        &[
            "copy",
            "create",
            "dir",
            "drop",
            "export",
            "fromdata",
            "import",
            "note",
            "save",
            "summarize",
            "use",
        ],
    ),
    ("duplicates", &["drop", "report", "tag"]),
    ("survwgt", &["rake"]),
    (
        "mvtest",
        &["correlations", "covariances", "means", "normality"],
    ),
    (
        "power",
        &[
            "cox",
            "exponential",
            "logrank",
            "onecorrelation",
            "onemean",
            "oneproportion",
            "oneslope",
            "onevariance",
            "oneway",
            "pairedmeans",
            "pairedproportions",
            "pcorr",
            "rsquared",
            "trend",
            "twocorrelations",
            "twomeans",
            "twoproportions",
            "twovariances",
        ],
    ),
    ("ereturn", &["list"]),
    ("estadd", &["local", "scalar"]),
    (
        "estimates",
        &[
            "clear", "dir", "drop", "replay", "restore", "stats", "store", "table",
        ],
    ),
    ("estpost", &["summarize"]),
    ("frame", &["change", "create", "drop", "pwf"]),
    (
        "estat",
        &[
            "bgodfrey",
            "classification",
            "durbinalt",
            "dwatson",
            "endogenous",
            "firststage",
            "gof",
            "hettest",
            "ic",
            "imtest",
            "overid",
            "ovtest",
            "phtest",
            "vce",
            "vif",
        ],
    ),
    ("eststo", &["clear", "drop"]),
    ("file", &["close", "open", "write"]),
    (
        "graph",
        &[
            "bar", "box", "combine", "drop", "export", "hbar", "hbox", "save", "twoway",
        ],
    ),
    ("import", &["delimited", "excel"]),
    (
        "matrix",
        &["colnames", "define", "drop", "list", "rownames"],
    ),
    ("merge", &["1:1", "1:m", "m:1", "m:m"]),
    ("reshape", &["long", "wide"]),
    ("robreg", &["m"]),
    ("return", &["list"]),
    ("scalar", &["define", "dir", "drop", "list"]),
    ("ssc", &["install"]),
];

/// What `vce()` takes.
const VCE: &[&str] = &["cluster", "oim", "ols", "robust"];

/// The prefixes in command position, and their shortest forms.
const PREFIXES: &[(&str, usize)] = &[("quietly", 3), ("capture", 3), ("noisily", 3)];

/// The commands whose first word is a file name.
const FILE_FIRST: &[&str] = &["use", "save", "saveold"];

/// What Tab offers at the end of `line`, or `None` when there is nothing.
pub fn complete(line: &str, ctx: &Context) -> Option<Completion> {
    // inside quotes: a file name
    if let Some(q) = open_quote(line) {
        return files(line, q + 1, ctx);
    }
    let from = token_start(line);
    let token = line.get(from..).unwrap_or("");
    let before = line.get(..from).unwrap_or("");
    // a macro's name
    if before.ends_with('`') {
        return offer(from, token, ctx.locals.iter().map(|n| format!("{n}'")));
    }
    if before.ends_with("${") {
        return offer(from, token, ctx.globals.iter().map(|n| format!("{n}}}")));
    }
    if before.ends_with('$') {
        return offer(from, token, ctx.globals.iter().cloned());
    }
    let (start, by) = command_start(before);
    if by {
        return variables(from, token, ctx, false);
    }
    let rest = before.get(start..).unwrap_or("");
    let (word, after_word) = split_word(rest);
    // the command itself
    if word.is_empty() && after_word.is_empty() {
        let prefixes = PREFIXES.iter().map(|(p, _)| (*p).to_string());
        return offer(
            from,
            token,
            COMMANDS.iter().map(|c| (*c).to_string()).chain(prefixes),
        );
    }
    let cmd = full_name(word);
    // the options, and inside one's parentheses
    if let Some(after_comma) = options_part(after_word) {
        return match open_option(after_comma) {
            // vce(cluster var): its kinds first, then a variable
            Some(opt) if abbrev_of(&opt, "vce", 3) && !after_open(after_comma).contains(' ') => {
                offer(from, token, VCE.iter().map(|v| (*v).to_string()))
            }
            Some(_) => variables(from, token, ctx, false),
            None => {
                let known = OPTIONS
                    .iter()
                    .find(|(c, _)| *c == cmd)
                    .map_or(&[][..], |(_, o)| *o);
                offer(from, token, known.iter().map(|o| (*o).to_string()))
            }
        };
    }
    let words: Vec<&str> = after_word.split_whitespace().collect();
    // a file name: after using, or the first word of use and save (the
    // words before the path being typed, which may hold a / of its own)
    let prior: Vec<&str> = line
        .get(..path_start(line))
        .and_then(|h| h.get(start..))
        .map(|h| split_word(h).1.split_whitespace().collect())
        .unwrap_or_default();
    if prior.last() == Some(&"using") || (FILE_FIRST.contains(&cmd.as_str()) && prior.is_empty()) {
        return files(line, path_start(line), ctx);
    }
    // a subcommand (graph bar, import delimited, merge 1:1, ...)
    if words.is_empty() {
        if let Some((_, subs)) = SUBCOMMANDS.iter().find(|(c, _)| *c == cmd) {
            return offer(from, token, subs.iter().map(|s| (*s).to_string()));
        }
    }
    // label values var LABEL, and the label subcommands that name one
    if cmd == "label" {
        match words.as_slice() {
            [] => {
                return offer(
                    from,
                    token,
                    ["data", "define", "drop", "list", "values", "variable"]
                        .iter()
                        .map(|s| (*s).to_string()),
                )
            }
            [sub, _var] if abbrev_of(sub, "values", 3) => {
                return offer(from, token, ctx.labels.iter().cloned())
            }
            [sub] if abbrev_of(sub, "define", 3) => {
                return offer(from, token, ctx.labels.iter().cloned())
            }
            [sub, ..] if abbrev_of(sub, "list", 1) || abbrev_of(sub, "drop", 4) => {
                return offer(from, token, ctx.labels.iter().cloned())
            }
            _ => {}
        }
    }
    // a varlist; if and in once it has a word, unless an if is already there
    let keywords = !words.is_empty() && !words.iter().any(|w| *w == "if" || *w == "in");
    variables(from, token, ctx, keywords)
}

/// The line after Tab: the word completed as far as the choices agree, and
/// the choices to show when there are several (as for R lines).
pub fn apply(line: &str, c: &Completion) -> (String, Vec<String>) {
    let typed = line.get(c.from..).unwrap_or("");
    let common = common_prefix(&c.options);
    let mut out = line.get(..c.from).unwrap_or("").to_string();
    out.push_str(if common.len() > typed.len() {
        &common
    } else {
        typed
    });
    if c.options.len() == 1 {
        return (out, Vec::new());
    }
    (out, c.options.clone())
}

fn common_prefix(options: &[String]) -> String {
    let Some(first) = options.first() else {
        return String::new();
    };
    let mut end = first.len();
    for o in options.iter().skip(1) {
        end = end.min(
            first
                .char_indices()
                .zip(o.chars())
                .take_while(|((_, a), b)| a == b)
                .last()
                .map_or(0, |((i, a), _)| i + a.len_utf8()),
        );
    }
    first.get(..end).unwrap_or("").to_string()
}

/// The choices that start with `token`, sorted, at most [`MAX_OPTIONS`].
fn offer(from: usize, token: &str, words: impl Iterator<Item = String>) -> Option<Completion> {
    let mut options: Vec<String> = words.filter(|w| w.starts_with(token)).collect();
    options.sort();
    options.dedup();
    options.truncate(MAX_OPTIONS);
    if options.is_empty() {
        return None;
    }
    Some(Completion { from, options })
}

/// The variables a word could be: a pattern or a range written out, or the
/// names it begins.
fn variables(from: usize, token: &str, ctx: &Context, keywords: bool) -> Option<Completion> {
    let vars = ctx.variables;
    if token.contains(['*', '?', '~']) {
        let matched: Vec<&str> = vars
            .iter()
            .filter(|v| wildcard(token, v))
            .map(String::as_str)
            .collect();
        // ~ stands for exactly one variable, as in Stata
        if matched.is_empty() || (token.contains('~') && matched.len() > 1) {
            return None;
        }
        return Some(Completion {
            from,
            options: vec![matched.join(" ")],
        });
    }
    if let Some((a, b)) = token.split_once('-') {
        if b.is_empty() {
            // the end of a range: any variable after its start
            let first = unique(vars, a)?;
            let after = vars.iter().skip_while(|v| v.as_str() != first).skip(1);
            return offer(from + a.len() + 1, "", after.cloned());
        }
        let (first, last) = (unique(vars, a)?, unique(vars, b)?);
        let i = vars.iter().position(|v| v == first)?;
        let j = vars.iter().position(|v| v == last)?;
        if j < i {
            return None;
        }
        let range: Vec<&str> = vars
            .get(i..=j)
            .unwrap_or_default()
            .iter()
            .map(String::as_str)
            .collect();
        return Some(Completion {
            from,
            options: vec![range.join(" ")],
        });
    }
    let words = vars.iter().cloned().chain(
        ["if", "in"]
            .iter()
            .filter(|_| keywords)
            .map(|k| (*k).to_string()),
    );
    offer(from, token, words)
}

/// The variable a name or a unique abbreviation of one means.
fn unique<'v>(vars: &'v [String], word: &str) -> Option<&'v str> {
    if let Some(v) = vars.iter().find(|v| v.as_str() == word) {
        return Some(v);
    }
    let mut found = vars.iter().filter(|v| v.starts_with(word));
    let one = found.next()?;
    if found.next().is_some() {
        return None;
    }
    Some(one)
}

/// Stata's wildcards: `*` and `~` any characters, `?` one.
fn wildcard(pattern: &str, name: &str) -> bool {
    fn go(p: &[char], n: &[char]) -> bool {
        match p.split_first() {
            None => n.is_empty(),
            Some(('*' | '~', rest)) => (0..=n.len()).any(|k| go(rest, n.get(k..).unwrap_or(&[]))),
            Some(('?', rest)) => n.split_first().is_some_and(|(_, tail)| go(rest, tail)),
            Some((c, rest)) => n
                .split_first()
                .is_some_and(|(d, tail)| c == d && go(rest, tail)),
        }
    }
    let p: Vec<char> = pattern.chars().collect();
    let n: Vec<char> = name.chars().collect();
    go(&p, &n)
}

/// The entries of the folder typed so far that begin with the rest.
fn files(line: &str, from: usize, ctx: &Context) -> Option<Completion> {
    let typed = line.get(from..).unwrap_or("");
    let cut = typed.rfind(['/', '\\']).map_or(0, |i| i + 1);
    let (dir, name) = typed.split_at(cut);
    offer(from + cut, name, (ctx.files)(dir).into_iter())
}

/// Where a file name at the end of the line begins: after the last space.
fn path_start(line: &str) -> usize {
    line.rfind(char::is_whitespace).map_or(0, |i| i + 1)
}

/// The text inside the parentheses still open at the end.
fn after_open(text: &str) -> &str {
    text.rfind('(')
        .and_then(|i| text.get(i + 1..))
        .unwrap_or("")
}

/// Where the word at the end of the line begins: letters, digits, `_` and
/// the wildcard and range characters.
fn token_start(line: &str) -> usize {
    line.char_indices()
        .rev()
        .take_while(|(_, c)| c.is_alphanumeric() || matches!(c, '_' | '*' | '?' | '~' | '-'))
        .last()
        .map_or(line.len(), |(i, _)| i)
}

/// The opening `"` of a quote still open at the end of the line.
fn open_quote(line: &str) -> Option<usize> {
    let mut open = None;
    let mut chars = line.char_indices().peekable();
    while let Some((i, c)) = chars.next() {
        match c {
            // a compound quote `" ... "' is a quote too
            '`' if chars.peek().is_some_and(|(_, d)| *d == '"') => {
                if let Some((j, _)) = chars.next() {
                    open = Some(j);
                }
            }
            '"' => {
                if open.is_some() {
                    open = None;
                    if chars.peek().is_some_and(|(_, d)| *d == '\'') {
                        chars.next();
                    }
                } else {
                    open = Some(i);
                }
            }
            _ => {}
        }
    }
    open
}

/// Past the prefixes: where the command starts, and whether the word being
/// typed is still in a `by` prefix's varlist (before its colon).
fn command_start(before: &str) -> (usize, bool) {
    let mut at = 0;
    loop {
        let rest = before.get(at..).unwrap_or("");
        let lead = rest.len() - rest.trim_start().len();
        let (word, _) = split_word(rest);
        let w = word.trim_end_matches(':');
        if abbrev_of(w, "bysort", 3) || abbrev_of(w, "by", 2) {
            // by varlist: command
            match rest.find(':') {
                Some(colon) => at += colon + 1,
                // no colon yet: the word is in its varlist
                None => return (at, true),
            }
            continue;
        }
        if PREFIXES.iter().any(|(p, min)| abbrev_of(w, p, *min)) && rest.len() > lead + word.len() {
            at += lead + word.len();
            continue;
        }
        return (at, false);
    }
}

/// The first word and the text after it.
fn split_word(text: &str) -> (&str, &str) {
    let t = text.trim_start();
    let end = t
        .find(|c: char| c.is_whitespace() || c == ',')
        .unwrap_or(t.len());
    let (w, r) = t.split_at(end);
    (w, r)
}

/// The full name of a command word (`reg` is `regress`).
fn full_name(word: &str) -> String {
    let w = crate::lang::commands::resolve(word);
    if w != word {
        return w.to_string();
    }
    // the translator's own abbreviations, as Stata's
    const MORE: &[(&str, usize)] = &[
        ("histogram", 4),
        ("kdensity", 5),
        ("scatter", 2),
        ("connected", 4),
        ("twoway", 2),
        ("graph", 2),
        ("logit", 4),
        ("probit", 4),
        ("poisson", 4),
        ("xtreg", 5),
        ("ttest", 5),
        ("pwcorr", 6),
        ("predict", 4),
        ("margins", 6),
    ];
    MORE.iter()
        .find(|(n, min)| abbrev_of(word, n, *min))
        .map_or_else(|| word.to_string(), |(n, _)| (*n).to_string())
}

fn abbrev_of(word: &str, full: &str, min: usize) -> bool {
    crate::lang::syntax::abbrev(word, full, min)
}

/// The text after a comma outside parentheses and quotes, if there is one.
fn options_part(text: &str) -> Option<&str> {
    let mut depth = 0i32;
    let mut quoted = false;
    for (i, c) in text.char_indices() {
        match c {
            '"' => quoted = !quoted,
            '(' | '[' if !quoted => depth += 1,
            ')' | ']' if !quoted => depth -= 1,
            ',' if !quoted && depth == 0 => return text.get(i + 1..),
            _ => {}
        }
    }
    None
}

/// The option whose parentheses are still open at the end, if any.
fn open_option(options: &str) -> Option<String> {
    let mut stack: Vec<usize> = Vec::new();
    for (i, c) in options.char_indices() {
        match c {
            '(' => stack.push(i),
            ')' => {
                stack.pop();
            }
            _ => {}
        }
    }
    let open = *stack.first()?;
    let head = options.get(..open).unwrap_or("");
    let name = head
        .rsplit(|c: char| c.is_whitespace() || c == ',')
        .next()
        .unwrap_or("");
    Some(name.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn names(v: &[&str]) -> Vec<String> {
        v.iter().map(|s| (*s).to_string()).collect()
    }

    fn at(line: &str, vars: &[&str]) -> Option<Completion> {
        let variables = names(vars);
        let locals = names(&["controls", "cutoff"]);
        let globals = names(&["root"]);
        let labels = names(&["yesno", "sexlbl"]);
        let files = |dir: &str| -> Vec<String> {
            match dir {
                "" => names(&["auto.dta", "analysis.do", "data/"]),
                "data/" => names(&["panel.dta", "people.csv"]),
                _ => Vec::new(),
            }
        };
        complete(
            line,
            &Context {
                variables: &variables,
                locals: &locals,
                globals: &globals,
                labels: &labels,
                files: &files,
            },
        )
    }

    fn options(line: &str, vars: &[&str]) -> Vec<String> {
        at(line, vars).map(|c| c.options).unwrap_or_default()
    }

    const VARS: &[&str] = &["id", "income", "incwage", "age", "sex", "educ", "y"];

    #[test]
    fn commands_after_the_prefixes() {
        assert_eq!(options("reg", VARS), names(&["reghdfe", "regress"]));
        assert_eq!(options("regr", VARS), names(&["regress"]));
        assert_eq!(
            options("su", VARS),
            names(&["suest", "summarize", "survwgt"])
        );
        assert_eq!(
            options("ta", VARS),
            names(&["tab1", "tab2", "table", "tabm", "tabodds", "tabstat", "tabulate"])
        );
        assert_eq!(options("q", VARS), names(&["qnorm", "quietly"]));
        assert_eq!(options("quietly regr", VARS), names(&["regress"]));
        assert_eq!(options("qui: reg", VARS), names(&["reghdfe", "regress"]));
        assert_eq!(options("by sex: summ", VARS), names(&["summarize"]));
        assert_eq!(options("bysort sex age: gen", VARS), names(&["generate"]));
        assert_eq!(
            options("capture noisily tt", VARS),
            names(&["ttest", "ttesti"])
        );
        // a subcommand
        assert_eq!(options("graph b", VARS), names(&["bar", "box"]));
        assert_eq!(options("import d", VARS), names(&["delimited"]));
        assert_eq!(options("merge 1", VARS), names(&["1:1", "1:m"]));
        assert_eq!(options("scalar d", VARS), names(&["define", "dir", "drop"]));
        // where the word being completed begins
        assert_eq!(at("quietly reg", VARS).map(|c| c.from), Some(8));
    }

    #[test]
    fn variables_with_statas_rules() {
        assert_eq!(
            options("summarize inc", VARS),
            names(&["income", "incwage"])
        );
        assert_eq!(options("regress y a", VARS), names(&["age"]));
        // a pattern or a range written out
        assert_eq!(options("summarize inc*", VARS), names(&["income incwage"]));
        assert_eq!(options("summarize ?ge", VARS), names(&["age"]));
        assert_eq!(options("summarize inco~", VARS), names(&["income"]));
        assert_eq!(options("summarize inc~", VARS), Vec::<String>::new());
        assert_eq!(
            options("list id-age", VARS),
            names(&["id income incwage age"])
        );
        assert_eq!(
            options("list incw-ed", VARS),
            names(&["incwage age sex educ"])
        );
        assert_eq!(options("list sex-", VARS), names(&["educ", "y"]));
        // the by prefix's varlist, factor variables, an if
        assert_eq!(options("by se", VARS), names(&["sex"]));
        assert_eq!(options("regress y i.se", VARS), names(&["sex"]));
        assert_eq!(options("summarize y if ag", VARS), names(&["age"]));
        // if and in are offered after a varlist
        assert_eq!(
            options("summarize y i", VARS),
            names(&["id", "if", "in", "income", "incwage"])
        );
        assert_eq!(
            options("summarize y if a > 1 i", VARS),
            names(&["id", "income", "incwage"])
        );
    }

    #[test]
    fn options_and_their_arguments() {
        assert_eq!(options("regress y x, r", VARS), names(&["robust"]));
        assert_eq!(
            options("reg y x, ", VARS),
            names(&["beta", "level(", "noconstant", "robust", "vce("])
        );
        assert_eq!(options("regress y x, vce(cl", VARS), names(&["cluster"]));
        assert_eq!(
            options("regress y x, vce(cluster se", VARS),
            names(&["sex"])
        );
        assert_eq!(options("ttest y, by(se", VARS), names(&["sex"]));
        assert_eq!(options("tabulate sex age, ch", VARS), names(&["chi2"]));
        assert_eq!(options("estat h", VARS), names(&["hettest"]));
        // a comma inside parentheses is not the options' comma
        assert_eq!(
            options("generate z = max(age, inc", VARS),
            names(&["income", "incwage"])
        );
    }

    #[test]
    fn files_macros_and_labels() {
        assert_eq!(options("use a", VARS), names(&["analysis.do", "auto.dta"]));
        assert_eq!(
            options("use data/p", VARS),
            names(&["panel.dta", "people.csv"])
        );
        assert_eq!(at("use data/p", VARS).map(|c| c.from), Some(9));
        assert_eq!(options("merge 1:1 id using da", VARS), names(&["data/"]));
        assert_eq!(
            options("import delimited \"data/pe", VARS),
            names(&["people.csv"])
        );
        assert_eq!(
            options("summarize `c", VARS),
            names(&["controls'", "cutoff'"])
        );
        assert_eq!(options("cd $r", VARS), names(&["root"]));
        assert_eq!(options("cd ${r", VARS), names(&["root}"]));
        assert_eq!(options("label values sex y", VARS), names(&["yesno"]));
        assert_eq!(options("label define s", VARS), names(&["sexlbl"]));
        assert_eq!(options("label v", VARS), names(&["values", "variable"]));
    }

    /// Why each line was not translated (empty when it was).
    fn reasons(source: &str) -> Vec<String> {
        crate::data_r::translate(&crate::reader::read(source))
            .lines
            .into_iter()
            .map(|l| l.result.err().map(|e| e.reason).unwrap_or_default())
            .collect()
    }

    /// Every command Tab offers is one the translator dispatches (the
    /// prefixes, loops and macros the reader takes aside).
    #[test]
    fn the_commands_offered_are_the_translators() {
        let reader = [
            "by",
            "bysort",
            "capture",
            "quietly",
            "noisily",
            "foreach",
            "forvalues",
            "local",
            "global",
            // and what the do-file runner (crate::dofile) does itself
            "continue",
            "dyndoc",
            "dyntext",
            "do",
            "exit",
            "include",
            "macro",
            "program",
            "run",
            "syntax",
            "tempfile",
            "tempname",
            "tempvar",
            "tokenize",
            "gettoken",
            "unab",
            "while",
        ];
        for cmd in COMMANDS.iter().filter(|c| !reader.contains(c)) {
            let why = reasons(&format!("use s\n{cmd}\n"));
            let last = why.last().cloned().unwrap_or_default();
            assert!(
                !last.contains(&format!("{cmd} is not translated yet")),
                "{cmd}: {last}"
            );
        }
        // and each subcommand offered is one it takes
        for (cmd, subs) in SUBCOMMANDS {
            for sub in *subs {
                let why = reasons(&format!("use s\nregress y x\n{cmd} {sub} y\n"));
                let last = why.last().cloned().unwrap_or_default();
                assert!(
                    !last.contains(&format!("{cmd} {sub} is not translated")),
                    "{cmd} {sub}: {last}"
                );
            }
        }
    }

    /// A line that uses the option, for each command the table names.
    fn with_option(cmd: &str, opt: &str) -> String {
        let name = opt.trim_end_matches('(');
        let arg = match name {
            "vce" if cmd == "svyset" => "linearized",
            "level" => "90",
            "df" => "30",
            "iterate" => "50",
            "vce" => "robust",
            "by" | "over" | "strata" => "g",
            "generate" => "z",
            "corr" => "exchangeable",
            "baseoutcome" => "1",
            "failure" => "d",
            "id" | "i" => "id",
            "j" => "t",
            "at" => "x=1",
            "keep" => "match",
            "format" => "%9.2f",
            "rhotype" => "dw",
            "rhof" => ".5",
            "lag" => "1",
            "n" => "5",
            "percentiles" => "50",
            "nquantiles" => "4",
            "means" => "J(1,2,0)",
            "cov" => "I(2)",
            "stats" => "all",
            "case" => "g",
            "alternatives" => "t",
            "mv" => "5",
            _ => "x",
        };
        let o = if opt.ends_with('(') {
            format!("{name}({arg})")
        } else {
            name.to_string()
        };
        let line = match cmd {
            "anova" => format!("anova y g, {o}"),
            "asclogit" => {
                let need = ["case(g)", "alternatives(t)", "noconstant"];
                let rest: Vec<&str> = need
                    .iter()
                    .copied()
                    .filter(|r| !r.starts_with(name))
                    .collect();
                format!("asclogit y x, {} {o}", rest.join(" "))
            }
            "drawnorm" if name == "n" => format!("drawnorm z1 z2, {o} clear"),
            "drawnorm" => format!("drawnorm z1 z2, {o}"),
            "_pctile" => format!("_pctile y, {o}"),
            "svyset" => format!("svyset g [pweight=w], {o}"),
            "survwgt" => {
                let need = ["by(g)", "totvars(t)", "generate(z)"];
                let rest: Vec<&str> = need
                    .iter()
                    .copied()
                    .filter(|r| !r.starts_with(name))
                    .collect();
                format!("survwgt rake w, {} {o}", rest.join(" "))
            }
            "append" => format!("append using t, {o}"),
            "ci" if matches!(name, "poisson" | "exposure") => format!("ci means c, {o}"),
            "ci" if matches!(name, "bonett" | "sd") => format!("ci variances y, {o}"),
            "ci" => format!("ci proportions d, {o}"),
            "cii" if name == "poisson" => format!("cii means 20 5, {o}"),
            "cii" if name == "bonett" => format!("cii variances 20 4 3, {o}"),
            "cii" if name == "sd" => format!("cii variances 20 4, {o}"),
            "cii" => format!("cii proportions 20 5, {o}"),
            "clogit" => format!("clogit y x, group(g) {o}"),
            "cmclogit" => format!("cmset g t\ncmclogit d x, {o}"),
            "cluster2" => "cluster2 y x, fcluster(g) tcluster(t)".to_string(),
            "coefplot" => format!("regress y x\ncoefplot, {o}"),
            "nlcom" => format!("regress y x\nnlcom _b[x], {o}"),
            "nl" => format!("nl (y = {{a = 1}} * exp({{b = 0.1}} * x)), {o}"),
            "collapse" => format!("collapse (mean) y, {o}"),
            "contract" if name == "format" => format!("contract g, percent(p) {o}"),
            "contract" => format!("contract g, {o}"),
            "recast" => format!("recast float y, {o}"),
            "center" => format!("center y, {o}"),
            "alpha" => format!("alpha y x, {o}"),
            "filefilter" if name == "replace" => {
                format!("filefilter a.txt b.txt, from(x) to(y) {o}")
            }
            "filefilter" if name == "from" => format!("filefilter a.txt b.txt, {o} to(y)"),
            "filefilter" => format!("filefilter a.txt b.txt, from(x) {o}"),
            "putexcel" if matches!(name, "names" | "rownames" | "colnames") => format!(
                "regress y x\nputexcel set \"a.xlsx\", replace\nputexcel A1 = matrix(e(b)), {o}"
            ),
            "putexcel" => format!("putexcel set \"a.xlsx\", {o}"),
            "marginsplot" => format!("regress y x\nmargins, at(x=(0 1))\nmarginsplot, {o}"),
            "winsor" if name == "generate" => format!("winsor y, {o} p(.1)"),
            "winsor" if name == "p" || name == "h" => format!("winsor y, gen(z) {o}"),
            "winsor" => format!("winsor y, gen(z) p(.1) {o}"),
            "connected" | "line" | "scatter" => format!("{cmd} y x, {o}"),
            "twoway" => format!("twoway scatter y x, {o}"),
            "describe" => format!("describe, {o}"),
            "destring" | "encode" | "tostring" => format!("{cmd} s, {o}"),
            "duplicates" => format!("duplicates drop, {o}"),
            "egen" => format!("egen z = mean(y), {o}"),
            "generate" => format!("generate z = 1, {o}"),
            "gettoken" => format!(
                "local s \"a b\"
gettoken a b : s, {o}"
            ),
            "graph" if matches!(name, "replace" | "width" | "height" | "as" | "name") => {
                format!("scatter y x\ngraph export f.png, {o}")
            }
            "graph" => format!("graph bar y, {o}"),
            "gsort" => format!("gsort -y, {o}"),
            "import" if matches!(name, "sheet" | "firstrow" | "cellrange" | "allstring") => {
                format!("import excel f.xlsx, {o}")
            }
            "import" => format!("import delimited f.csv, {o}"),
            "insheet" => format!("insheet using f.csv, {o}"),
            "postfile" => format!("postfile h z using f, {o}"),
            "cs" if name == "or" => format!("cs y x, by(g) woolf {o}"),
            "cs" if name == "woolf" => format!("cs y x, by(g) or {o}"),
            "cs" if name == "istandard" => format!("cs y x, by(g) {o}"),
            "cs" => format!("cs y x, {o}"),
            "cc" if name == "level" => format!("cc y x, woolf {o}"),
            "cc" if name == "by" => format!("cc y x, woolf {o}"),
            "cc" => format!("cc y x, {o}"),
            "csi" => format!("csi 1 2 3 4, {o}"),
            "cci" if name == "level" => format!("cci 1 2 3 4, woolf {o}"),
            "cci" | "iri" => format!("{cmd} 1 2 3 4, {o}"),
            "ir" => format!("ir y x t, {o}"),
            "tabodds" if name == "base" => "tabodds y x, or base(2)".to_string(),
            "tabodds" if name == "adjust" => "tabodds y x, adjust(g)".to_string(),
            "tabodds" if name == "binomial" => "tabodds y x, binomial(n)".to_string(),
            "tabodds" => format!("tabodds y x, {o}"),
            "ktau" if name == "stats" => "ktau x y z, stats(taua p)".to_string(),
            "ktau" if matches!(name, "print" | "star") => format!("ktau x y z, {name}(.05)"),
            "ktau" => format!("ktau x y z, {o}"),
            "pwmean" if name == "over" => "pwmean y, over(g)".to_string(),
            "pwmean" if name == "mcompare" => "pwmean y, over(g) mcompare(tukey)".to_string(),
            "pwmean" if name == "level" => "pwmean y, over(g) level(90)".to_string(),
            "pwmean" => format!("pwmean y, over(g) {o}"),
            "testnl" => format!("regress y x\ntestnl _b[x] = 0, {o}"),
            "rotate" if name == "blanks" => "factor x y z\nrotate, blanks(.3)".to_string(),
            "rotate" if name == "oblique" => "factor x y z\nrotate, oblique promax".to_string(),
            "rotate" => format!("factor x y z\nrotate, {o}"),
            "kap" if name == "wgt" => "kap x y, wgt(w)".to_string(),
            "kap" => format!("kap x y, {o}"),
            "lroc" if name == "beta" => {
                "logit y x\nmatrix b = (1, 0)\nmatrix colnames b = x _cons\nlroc y, beta(b)"
                    .to_string()
            }
            "lroc" => format!("logit y x\nlroc, {o}"),
            "mhodds" if name == "by" => "mhodds y x, by(g)".to_string(),
            "mhodds" if name == "compare" => "mhodds y x, compare(2,1)".to_string(),
            "mhodds" if name == "level" => "mhodds y x, level(90)".to_string(),
            "icc" if name == "testvalue" => "icc y g t, testvalue(.2)".to_string(),
            "icc" if name == "level" => "icc y g t, level(90)".to_string(),
            "icc" if name == "format" => "icc y g t, format(%9.4f)".to_string(),
            "icc" => format!("icc y g t, {o}"),
            "tetrachoric" if name == "stats" => "tetrachoric x y z, stats(rho p)".to_string(),
            "tetrachoric" if matches!(name, "print" | "star") => {
                format!("tetrachoric x y z, {name}(.05)")
            }
            "tetrachoric" => format!("tetrachoric x y z, {o}"),
            "power" if name == "notable" => "power twomeans 0 0.5, n(50 100) notable".to_string(),
            "power" if matches!(name, "loghazard" | "unconditional" | "studytime") => {
                format!("power exponential 0.5 0.4, {o}")
            }
            "power" if matches!(name, "sdx" | "sderror" | "sdy") => {
                format!("power oneslope 0 0.5, {o}")
            }
            "power" if matches!(name, "ncontrol" | "ntested") => {
                format!("power pcorr 0.3, ncontrol(2) {o}")
            }
            "power" if matches!(name, "hratio" | "schoenfeld") => format!("power logrank 0.5, {o}"),
            "power" if matches!(name, "eventprob" | "r2") => format!("power cox, hratio(0.6) {o}"),
            "power" if matches!(name, "grweights" | "npergroup" | "varerror") => {
                format!("power oneway 1 2 3, {o}")
            }
            "power" if matches!(name, "ngroups" | "varmeans") => {
                "power oneway, ngroups(3) varmeans(0.5)".to_string()
            }
            "power" if name == "prdiscordant" => {
                "power pairedproportions, n(100) prdiscordant(0.3) power(0.8)".to_string()
            }
            "power" if matches!(name, "continuity" | "effect" | "test") => {
                format!("power twoproportions 0.3 0.5, {o}")
            }
            "power" if matches!(name, "sddiff" | "corr" | "knownsd") => {
                format!("power pairedmeans 0 0.5, {o}")
            }
            "power" => format!("power twomeans 0 0.5, {o}"),
            "mvtest" if matches!(name, "lr" | "missing") => format!("mvtest means x y, by(g) {o}"),
            "mvtest" if matches!(name, "by" | "equal" | "zero") => format!("mvtest means x y, {o}"),
            "mvtest" if matches!(name, "stats" | "univariate" | "bivariate") => {
                format!("mvtest normality x y, {o}")
            }
            "mvtest" => format!("mvtest covariances x y, {o}"),
            "file" => format!("file open h using f.txt, write {o}"),
            "mkdir" => format!("mkdir a, {o}"),
            "expandcl" => "expandcl 2, cluster(g) generate(zz)".to_string(),
            "fdasave" if name == "vallabfile" => "fdasave a.xpt, vallabfile(none)".to_string(),
            "fdasave" => format!("fdasave a.xpt, {o}"),
            "fdause" => format!("fdause a.xpt, {o}"),
            "snapshot" => "snapshot save, label(x)".to_string(),
            "outfile" if name == "runtogether" => "outfile using a.raw, runtogether".to_string(),
            "outfile" => format!("outfile using a.raw, {o}"),
            "outsheet" if name == "delimiter" => {
                "outsheet using a.out, delimiter(\";\")".to_string()
            }
            "outsheet" => format!("outsheet using a.out, {o}"),
            "svmat" => format!("matrix A = (1, 2)\nsvmat A, {o}"),
            "ivregress" => format!("ivregress 2sls y (x = w), {o}"),
            "joinby" => format!("joinby k using t, {o}"),
            "ivprobit" => format!("ivprobit d y (x = w), {o}"),
            "label" => format!("label define l 1 \"a\", {o}"),
            // what predict can make depends on the model: hr after stcox, pr after
            // logit, outcome() after mlogit
            "predict" if name == "hr" => format!("stset t, failure(d)\nstcox x\npredict z, {o}"),
            "predict" if name == "pr" => format!("logit d x\npredict z, {o}"),
            "predict" if name == "outcome" => "mlogit o x\npredict z, outcome(1)".to_string(),
            "lincom" | "margins" | "predict" => {
                let what = match cmd {
                    "lincom" => "lincom x",
                    "margins" => "margins",
                    _ => "predict z",
                };
                format!("regress y x\n{what}, {o}")
            }
            "matrix" => format!("regress y x\nmatrix list e(b), {o}"),
            "logit" | "probit" => format!("{cmd} d x, {o}"),
            "poisson" | "nbreg" => format!("{cmd} c x, {o}"),
            "ologit" | "oprobit" | "mlogit" => format!("{cmd} o x, {o}"),
            "lowess" | "pwcorr" | "correlate" | "regress" | "tab2" | "oneway" => {
                format!("{cmd} y x, {o}")
            }
            // the columns summarize() shows
            "tabulate"
                if matches!(
                    o.as_str(),
                    "freq"
                        | "means"
                        | "nomeans"
                        | "noobs"
                        | "nostandard"
                        | "obs"
                        | "standard"
                        | "wrap"
                ) =>
            {
                format!("tabulate y, summarize(x) {o}")
            }
            "tabulate" if matches!(name, "generate" | "missing" | "nolabel" | "sort") => {
                format!("tabulate y, {o}")
            }
            "tabulate" | "tab1" => format!("{cmd} y x, {o}"),
            "merge" => format!("merge 1:1 id using t, {o}"),
            "recode" => format!("recode y (1=2), {o}"),
            "rename" => format!("rename *, {o}"),
            "reshape" => format!("reshape long y, {o}"),
            "robreg" => format!("robreg m y x, {o}"),
            "save" | "saveold" | "use" => format!("{cmd} f, {o}"),
            "stcox" => format!("stset t, failure(d)\nstcox x, {o}"),
            "stset" => format!("stset t, {o}"),
            "ttest" if name == "by" => format!("ttest y, {o}"),
            "ttest" => format!("ttest y == x, {o}"),
            "ttesti" => format!("ttesti 20 1 1 30 2 2, {o}"),
            "reghdfe" if name == "absorb" || name == "noabsorb" => format!("reghdfe y x, {o}"),
            "reghdfe" => format!("reghdfe y x, absorb(g) {o}"),
            "outreg2" => format!("regress y x\noutreg2 using t.xls, tstat bdec(3) tdec(2) {o}"),
            "xtile" => format!("xtile q = y, {o}"),
            "pctile" => format!("pctile q = y, {o}"),
            "cumul" if name == "generate" => format!("cumul y, {o}"),
            "cumul" => format!("cumul y, generate(cy) {o}"),
            "xpose" => format!("xpose, clear {o}"),
            "xtreg" => format!("xtset id t\nxtreg y x, {o}"),
            "npregress" => format!("npregress kernel y x, {o}"),
            "xtset" => format!("xtset id t, {o}"),
            "xtdescribe" if name == "patterns" => "xtset id t\nxtdescribe, patterns(3)".to_string(),
            "xtdescribe" if name == "width" => "xtset id t\nxtdescribe, width(5)".to_string(),
            "xtdescribe" => format!("xtset id t\nxtdescribe, {o}"),
            "xtsum" | "xttab" => format!("xtset id t\n{cmd} y, {o}"),
            "tsreport" if name == "casewise" => "xtset id t\ntsreport x y, casewise".to_string(),
            "tsreport" => format!("xtset id t\ntsreport, {o}"),
            "tsappend" if name == "add" => "xtset id t\ntsappend, add(1)".to_string(),
            "tsappend" => "xtset id t\ntsappend, last(25jun1965) tsfmt(td)".to_string(),
            "tsset" => format!("tsset t, {o}"),
            "dfuller" | "pperron" | "dfgls" | "corrgram" | "wntestq" => {
                format!("tsset t\n{cmd} y, {o}")
            }
            _ => format!("{cmd} y, {o}"),
        };
        format!("use s\n{line}\n")
    }

    /// Every option Tab offers is one the translator takes: no line using it
    /// is refused for that option.
    #[test]
    fn the_options_offered_are_the_translators() {
        let mut wrong = Vec::new();
        for (cmd, opts) in OPTIONS {
            assert!(
                COMMANDS.contains(cmd),
                "{cmd} has options but is not offered"
            );
            for opt in *opts {
                let name = opt.trim_end_matches('(');
                let source = with_option(cmd, opt);
                for why in reasons(&source) {
                    if why.contains(&format!("option {name}")) || why.contains("not allowed") {
                        wrong.push(format!("{cmd}, {opt}: {why}"));
                    }
                }
            }
        }
        assert!(wrong.is_empty(), "{}", wrong.join("\n"));
    }

    #[test]
    fn tab_completes_as_far_as_the_choices_agree() {
        let c = at("summarize inc", VARS).unwrap_or(Completion {
            from: 0,
            options: vec![],
        });
        assert_eq!(
            apply("summarize inc", &c),
            ("summarize inc".to_string(), names(&["income", "incwage"]))
        );
        let c = at("summarize ag", VARS).unwrap_or(Completion {
            from: 0,
            options: vec![],
        });
        assert_eq!(
            apply("summarize ag", &c),
            ("summarize age".to_string(), Vec::new())
        );
        let c = at("list id-age", VARS).unwrap_or(Completion {
            from: 0,
            options: vec![],
        });
        assert_eq!(
            apply("list id-age", &c),
            ("list id income incwage age".to_string(), Vec::new())
        );
        assert_eq!(at("summarize zz", VARS), None);
        assert_eq!(at("", VARS).map(|c| c.options.len() > 20), Some(true));
    }
}
