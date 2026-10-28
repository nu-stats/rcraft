# Roadmap

RCraft is a desktop app that gives R a complete point-and-click front end and
drives a real R installation. Each phase ends when its menu entries generate
code that gives the expected result in the pinned R, not when the UI exists.
Task ids (`P1.2` and so on) go in commit messages. The current state is in
`plan/STATUS.md`.

Suggested models: the foundations (P0, P1, P4.0 catalog format) are
design-heavy, so use the strongest model; dialogs, templates, UI panes,
tests and docs are repetitive and fit a mid-size model.

## Phase 0: Setup

- P0.1 Workspace, layered crates, workspace lints
- P0.2 Contributor rules (`AGENTS.md`), licence, notices
- P0.3 `cargo xtask ci`: fmt, clippy, tests, layers, assets, coverage
- P0.4 Coverage inventory (`crates/catalog`, `docs/coverage.md`)
- P0.5 `ENVIRONMENT`: pinned R version and package snapshot date
- P0.6 CI on Windows, macOS and Linux, with R installed from the pin

## Phase 1: R host and session

- P1.1 Find an installed R (PATH, registry, standard locations); report
  version and library paths
- P1.2 Download and verify a pinned portable R on first run when none is
  found (user confirms; licence shown)
- P1.3 Start `R` as a child process with a framed protocol (`rproto`):
  run, stream output, capture warnings and errors, interrupt, restart
- P1.4 Supervision: crash, hang and out-of-memory handling; the app stays up
- P1.5 Package library basics: the user library path, dated snapshot
  repository, binary packages where available (the full install and search
  experience is Phase 1A)
- P1.6 Multiple sessions and working directories

## Phase 1A: Packages (`ssc install` and `net search` for R)

An in-app package finder and installer, as Stata users have with `ssc
install` and `net search`. Menu **Packages**, and command forms in the
Command window that expand to ordinary R and show it.

Sources, each a separate "provider" with its own trust label:

- CRAN (default): the public package database
- Bioconductor: through BiocManager's repositories
- r-universe: per-organisation package universes
- GitHub: `owner/repo`, flagged as unreviewed source code
- Local: a `.tar.gz`, `.zip` or folder

Tasks:

- P1A.1 Package index: fetch and cache the CRAN package database (names,
  titles, descriptions, versions, authors, licences, dependencies, system
  requirements, published date); refreshed on request and when stale;
  searchable offline once cached
- P1A.2 `search`: keyword search over names, titles, descriptions and
  keywords, ranked; filters by provider, task view, licence, and "has
  binary for my platform"; results show title, version, date, maintainer
  and a one-line description
- P1A.3 Browse: CRAN Task Views (Econometrics, Survival, Bayesian, Spatial
  and the rest) as a tree; "new" and "recently updated" lists
  (`ssc new`/`ssc hot` equivalents); optional download counts from the
  public download-logs service
- P1A.4 Describe: package page with description, version, authors, links,
  reverse dependencies, vignettes and the system libraries it needs
  (`ssc describe` equivalent)
- P1A.5 Install: one click or `rc install pkg` (with `version`, `provider`,
  `from()` choices); resolves dependencies, shows what will be installed
  and downloaded, prefers binaries, runs in a supervised R process with
  progress and a cancel button; the exact R call is shown and logged
- P1A.6 Installed packages: list, version, update available, update,
  remove, `which`-style "where does this function come from", load status
- P1A.7 Failures made readable: missing compiler or system library, with
  the platform-specific fix named; retry from binary or older version
- P1A.8 Reproducibility: record installed packages and versions in the
  session and project lockfile; install a project's packages in one step
- P1A.9 Command forms in the Command window, for example `rc install
  fixest`, `rc search panel data`, `rc describe fixest`, `rc update`,
  `rc installed`; Tab completion over the index
- P1A.10 "Package needed" prompt: when a menu entry or command needs a
  package that is not installed, offer to install it, with the same
  confirmation as P1A.5
- P1A.11 Tests: provider parsing and ranking on a small recorded index
  built in code; the install flow against a local test repository so CI
  needs no network

Limits:

- Installing a package can run arbitrary code (compilation, install hooks).
  RCraft always shows the source, version and provider and asks before
  installing, and never installs anything on its own.
- GitHub and local sources are labelled unreviewed.
- RCraft talks to the package services only when the user asks, and sends
  nothing but the search text or package names.

## Phase 2: Command window and Results

- P2.1 Command window: history, Page Up/Down and arrow recall, click a
  History entry to put it back
- P2.2 Tab completion: functions, objects, data-frame columns, file names,
  package names; a second Tab lists the choices
- P2.3 Results window: text output, errors and warnings with the failing
  call, rich tables, copy and export
- P2.4 Session log and script: every command and menu action recorded as R

## Phase 2A: Help

Help for every layer, reachable the way Stata users expect: a **Help**
menu, `help topic` in the Command window, and F1 on a word or a dialog.

- P2A.1 R help pages: render a function's help (the Rd page) in a Help
  window with usage, arguments, value, details, examples, links and "see
  also"; back and forward; open from `?topic`, `help(topic)`, F1 on a word,
  or a link
- P2A.2 Examples: a **Run example** button on each help page, copy to the
  Command window, and output in Results
- P2A.3 Search help: `help search words` over installed packages' help
  titles, aliases and keywords (the `search` equivalent), plus function-name
  search (`apropos`) and a topic index
- P2A.4 Vignettes and manuals: list and open a package's vignettes
  (HTML/PDF) and its news file, from the package page or the Help menu
- P2A.5 Dialog help: every dialog has a Help button opening a page that
  says what it does, what R it generates, which package it needs, and
  links to the R functions' help pages; written for RCraft, not copied
- P2A.6 RCraft manual: getting started, the menus, the Command window,
  Packages, projects and reporting, as pages inside the app, searchable
  from the same box as R help
- P2A.7 Help for missing things: if the topic is in a package that is not
  installed, say so and offer Phase 1A's install and search
- P2A.8 Offline by default: installed packages' help works with no
  network; online links open in the user's browser only on click
- P2A.9 Tests: help rendering on made-up Rd pages built in code; search
  ranking; dialog pages checked for broken function links

## Phase 2B: Stata mode (Stata code in, R code out, R runs)

A **Stata mode** switch (a toggle on the Command window and the Script
Editor, and a **Translate Stata** button) for people who know Stata. The user
types or pastes Stata commands; RCraft translates them to R, shows the R it
made beside the Stata, and runs it in the R session. The same translator
reads whole do-files.

What it is and is not:

- It is a translator, not Stata. It aims at the same *meaning* (same
  estimates, same rows kept, same variables made), written as readable R the
  user can keep. It does not promise Stata's exact printed layout; the
  Results window shows R's output, and a **Stata-style output** option for the
  commonest commands (`summarize`, `regress`, `tabulate`) comes later.
- Anything it cannot translate is marked, never guessed: the line is kept as a
  comment, the reason is given, and the run stops or skips by the user's
  choice. Silent wrong answers are the failure to avoid.
- Related, separate project: StatCraft runs Stata code natively. Stata mode
  here does not need StatCraft; the two may share a parser later (both are
  our own permissively licensed code).

Tasks:

- P2B.1 Stata syntax reader (`statatr` crate, L3): commands with `if`,
  `in`, `by:`, `[weight]`, `using`, options after the comma, abbreviations
  (`reg`, `gen`, `sum`), local and global macros, comments and line
  continuation (`///`), `foreach`, `forvalues`, `capture`, `quietly`. Never
  crashes on bad input; reports line and column.
- P2B.2 Expressions: Stata operators and functions to R (`==`, `&`, `|`,
  `!`, `inrange`, `inlist`, `missing()`, string, date and math functions),
  Stata missing values `.` and `.a`-`.z` mapped to `NA` (the extended
  missings kept as an attribute where it matters), `_n` and `_N` inside
  `by`, `[_n-1]` lags, and the rule that missing sorts high in comparisons.
- P2B.3 Data commands: `use`, `save`, `import delimited`, `import excel`,
  `generate`, `replace`, `drop`, `keep`, `rename`, `recode`, `label`,
  `sort`, `gsort`, `egen`, `merge`, `append`, `reshape`, `collapse`,
  `duplicates`, `encode`, `destring`, `tostring`, `order`, `describe`,
  `list`, `count`, `set seed`. Variable and value labels kept as labelled
  columns.
- P2B.4 Statistics commands: `summarize`, `tabulate`, `correlate`, `ttest`,
  `regress` (with `robust` and `cluster()`), `logit`, `probit`, `poisson`,
  `nbreg`, `ologit`, `mlogit`, `anova`, `predict`, `test`, `lincom`,
  `margins`, `estat`, `xtset`/`xtreg`, `ivregress`, `stset`/`stcox`. Each
  maps to a chosen R function in a named package (for example `fixest`,
  `sandwich`, `survival`) and the package is offered through Phase 1A when
  missing.
- P2B.5 Graph commands: `twoway`, `histogram`, `graph bar`, `scatter`,
  `line`, `kdensity` to ggplot2.
- P2B.6 Estimates and returned values: `e()`, `r()` and `_b[]`/`_se[]`
  after a translated command, with `estimates store` and `esttab`/`outreg2`
  style tables going to Phase 7.
- P2B.7 The two-pane view: Stata on the left, generated R on the right,
  line by line, with a mapping so a click on either highlights the other; a
  note on each line that has a known difference from Stata.
- P2B.8 Differences guide: a page per translated command listing where R
  and Stata differ (default standard errors, degrees of freedom, how ties
  and missing values are handled, sort stability, string widths, float
  versus double). Linked from the note in P2B.7.
- P2B.9 Do-file runner: see Phase 2C.
- P2B.10 Checked against Stata: a private corpus of made-up data and
  recorded Stata results, the same method as StatCraft's observation kit, so
  translated estimates are compared with Stata's to about 8 significant
  digits. A command moves to `Verified` only when its translation matches.
- P2B.11 Ado programs, Mata and `net`/`ssc` commands are out of scope for
  the first release; `ssc install x` is translated to `rc install` (Phase
  1A) when an R package of that name exists, and flagged otherwise.

Rules (also in AGENTS.md section 1): translations are written from Stata's
public documentation and from observing a licensed Stata with made-up data.
Stata's program files are never read. Stata's own ado code, help files,
datasets and wording are never copied. "Stata" is named only to describe
compatibility, with the trademark notice in `NOTICE`.

## Phase 2C: Run my do-file

An **Open do-file** button (and drag and drop) that takes an existing `.do`
file, unedited, and runs it on R. Findings, approach and risks are in
`plan/do-file-run.md`. The promise is an honest one: the file is not edited,
and the report says exactly what ran, what ran with a caveat and what was
skipped. It never gives a silent wrong answer.

- P2C.1 Open and scan (dry run): commands used, lines that translate
  cleanly, with caveats or not at all, packages needed, data files and paths
  referred to, `cd`, `log using`, `ssc install`
- P2C.2 Resolve paths and data: ask for the data folder, map Windows paths,
  keep paths relative to the do-file, follow `do` and `include`
- P2C.3 Front end: reuse StatCraft's do-file reader, command syntax and
  expression parsers (shared crate or vendored; see open questions)
- P2C.4 `rcraftstata`, a small R package we ship that implements the Stata
  rules R does not share: missing values sorting high, variable
  abbreviations and wildcards, `_n`/`_N`, `by` state, `preserve`/`restore`,
  macros, storage types. Installed on first use, with the user's yes.
- P2C.5 Emit: idiomatic R where Stata and R agree, calls into
  `rcraftstata` where they differ; a line map kept
- P2C.6 Run: line by line in the R session with live progress, the Stata
  line beside its R, and a choice at the first problem (stop, skip,
  continue); the log is saved
- P2C.7 Report and export: what ran, differences from Stata (linked to the
  guide in P2B.8), **Save as R script** or **Save as Quarto**
- P2C.8 Tiers of coverage: 1 data management and control flow, 2 core
  statistics, 3 panel/survival/survey/IV, 4 graphs, 5 programs and chosen
  SSC commands; the share of lines translated per tier is shown in
  `docs/coverage.md`
- P2C.9 Verification: StatCraft's probe do-files as the first inputs,
  recorded Stata results in a private corpus (never committed), compared to
  about 8 significant digits; a command is `Verified` only on a match
- P2C.10 Known limits stated up front: Stata's random draws, Mata, compiled
  plugins, interactive commands

## Phase 3: Data

- P3.1 Data Editor and Browser, virtualised, for millions of rows, backed by
  the RCraft columnar bridge from R (`rdata`, see `plan/data-bridge.md`)
- P3.2 **Use my file**: done for csv/tsv/txt, xlsx/xls (sheet and cell range),
  `.dta`, `.sav`/`.zsav`/`.por`, `.xpt`, `.sas7bdat`, `.rds` and `.RData`, with R's own
  preview and the generated R shown; packages `readxl`/`haven`/`arrow` are installed on
  request into RCraft's own library; parquet and feather; a Browse button; the ten most
  recent files remembered. Still to do: `.sas7bdat` catalogue files
- P3.3 Labels (shown in the grid and Variables pane with a conversion report; variable
  labels and value labels kept as labelled vectors; conversion report of what was kept,
  changed or dropped; Data menu dialogs to set a variable label, set value labels and turn
  a labelled variable into a factor)
- P3.4 Variables pane (type, label, missing count, factor level count, filter by type) and
  an Objects pane for the whole workspace, with a right-click menu to rename, remove and
  save (as .rds) any object. Still to do: a level list per factor
- P3.5 Data management dialogs (done: the dialog-spec format, plan/dialog-spec.md; new
  variable, filter, sort and rename in base and tidyverse style; recode, convert type,
  factor levels (reference, combine, reorder), text (case and spaces, find and replace,
  extract) and dates (text to date, date part) in base R, each run against real R; a
  missing-package panel in the dialogs (the tidyverse style asks R for dplyr and offers to
  install it); CI installs dplyr and checks the tidyverse styles give the base styles'
  tables. Still to do: the per-factor level list in the Variables pane (P3.4))
- P3.6 Combine and reshape: join, append, pivot long/wide, aggregate, sample,
  duplicates and missing-value tools
- P3.7 Save and export (the same formats), plus "save workspace"
- P3.8 Frames-style workspace: several data frames open at once with a
  switcher

## Phase 4: Catalog and core statistics

- P4.0 Dialog-spec format and renderer: widgets, variable-type filters,
  defaults, validation, code templates in tidyverse and base styles; one spec
  gives the dialog and the code; snapshot tests of generated R
- P4.1 Descriptives: summary statistics, frequency tables, cross-tabs,
  correlation, normality, missing-data patterns (done in base R, each run against real R;
  a multi-variable widget was added to the dialog-spec format. Still to do: a Q-Q plot with
  Phase 5, and rich tables in Results with P2.3)
- P4.2 Tests and intervals: t-tests, proportions, nonparametrics, variance
  tests, exact tests, agreement measures (done in base R, each checked against R's own
  report: one-sample, two-sample and paired t-tests, test of proportions, exact binomial,
  Fisher's exact, Mann-Whitney, Wilcoxon signed-rank, Kruskal-Wallis, variance tests. Still
  to do: agreement measures such as Cohen's kappa)
- P4.3 ANOVA and linear models: `lm`, anova tables, contrasts, post-hoc (done in base R:
  linear model with categorical predictors, interactions, weights and confidence
  intervals; one-way and factorial ANOVA with Tukey comparisons; each saves its model
  under a chosen name. Still to do: Type II and III sums of squares and custom contrasts,
  which need a package, and diagnostics with P4.5)
- P4.4 Generalised models: logit, probit, Poisson, negative binomial,
  ordinal, multinomial (logistic with logit, probit or complementary log-log links and odds
  ratios, and Poisson or quasi-Poisson with an exposure offset and rate ratios are done in
  base R; negative binomial through MASS, the first dialog with a package requirement of its
  own (`Spec::needs`, which the package panel reads); ordinal (MASS::polr, with the outcome's
  order printed) and multinomial (nnet::multinom, with a chosen reference category) models.
  P4.4 is done)
- P4.5 Post-estimation context menu on the last model: predict, residuals,
  diagnostics, marginal effects, hypothesis tests, robust and clustered
  standard errors, model comparison (first part done: a Model submenu in the Objects pane's
  right-click menu on a fitted model, with fit statistics, comparison of models, predicted
  values and residuals added to a data frame, diagnostics, and robust standard errors (needs
  sandwich and lmtest), with clustered standard errors, tests of terms (each, or several
  together) and average marginal effects (marginaleffects package). P4.5 is done apart from
  residual plots, which come with Phase 5)

## Phase 5: Graphs

- P5.1 Graph window: capture R devices, history of plots, zoom (done: plots are captured as
  PNG snapshots from one live device that stays open between commands, so a later command can
  add to a plot; a Graph tab with older and newer, the command that drew each, fitted to the
  pane, and Save as PNG. Still to do: zoom, and copy to the clipboard)
- P5.2 ggplot2 builder: pick data, geoms and aesthetics; facets and themes
- P5.3 Plot menus: histogram, density, box, bar, scatter with fit, line,
  heatmap, QQ, diagnostics, survival curves (done in base graphics under Graphs: histogram,
  density, box plot, bar chart, normal Q-Q, scatter with a fit and smooth, line plot and a
  scatter plot matrix, each run against real R and required to draw exactly one picture; and a
  four-panel diagnostic plot for a fitted model in the Objects pane's Model menu. Still to do:
  heatmap and survival curves)
- P5.4 Export: SVG, PDF, PNG at chosen size and dpi

## Phase 6: Depth modules

Each family is its own set of dialogs over a named CRAN package.

- P6.1 Panel and fixed effects (`fixest`, `plm`)
- P6.2 Multilevel models (`lme4`, `glmmTMB`)
- P6.3 Survival (`survival`)
- P6.4 Survey data (`survey`)
- P6.5 Time series (`forecast`, `fable`)
- P6.6 Instrumental variables and selection models
- P6.7 Multivariate: PCA, factor analysis, clustering, discriminant
- P6.8 Resampling and permutation tests
- P6.9 Bayesian (`brms`, `rstanarm`)
- P6.10 Structural equation models (`lavaan`)
- P6.11 Meta-analysis (`metafor`)
- P6.12 Machine learning basics (`tidymodels`): split, fit, tune, evaluate

## Phase 7: Reporting and reproducibility

- P7.1 Publication tables (`modelsummary`/`gt`): side-by-side models, stars,
  chosen statistics, notes, custom rows
- P7.2 Export to docx, xlsx, HTML, LaTeX and Markdown; one document built up
  across a session
- P7.3 Script Editor: run selection, outline, find and replace
- P7.4 Export the session as an R script, an R Markdown or Quarto document,
  or a Quarto project
- P7.5 Project files: data, script, log and package snapshot together
  (renv-style lockfile)

## Phase 8: Interface and packaging

- P8.1 Pane layout: Command, Results, History, Variables, Objects,
  Properties, Graph, Script
- P8.2 Installers: Windows MSI and portable zip (signed), macOS universal DMG
  (signed and notarised), Linux AppImage, deb, rpm, tar.gz
- P8.3 Example datasets that RCraft may redistribute, with attribution
- P8.4 Help polish: see Phase 2A; here only the final review of every
  dialog's help page and the manual
- P8.5 Browser build, if feasible

## Phase 9: Extensibility

- P9.1 Dialog-spec plugin format: a user adds a menu for any CRAN function
  with a spec file, no Rust
- P9.2 Spec sharing and a compatibility list in `docs/coverage.md`
- P9.3 Preferences: code style (tidyverse or base), package snapshot, themes

## Limits

- RCraft needs R. It downloads or finds one; it does not reimplement R.
- Packages are third-party code under their authors' terms. RCraft installs
  them on request and never bundles them in its binary.
- Packages with system-library requirements can fail to install on some
  platforms; RCraft reports what is missing.
