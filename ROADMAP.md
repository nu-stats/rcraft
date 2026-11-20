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
- P1.6 Multiple sessions and working directories (done 2026-10-30: the working folder is shown and
  chosen from Session; several R sessions, each with its own objects, history, plots and tabs; sessions are not
  saved when RCraft closes, which belongs to the project files, P7.5)

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
  (done: crate `rpkg`; read from `packages.rds` and `PACKAGES.gz`, trimmed text cache,
  refresh on request only with progress and cancel, a checked HTTPS mirror setting,
  checked against the real 25,581-package file. P1A.1 is done)
- P1A.2 `search`: keyword search over names, titles, descriptions and
  keywords, ranked; filters by provider, task view, licence, and "has
  binary for my platform"; results show title, version, date, maintainer
  and a one-line description
  (done for CRAN: ranked, with licence, compiler, task view and, from 2026-11-14, "Binary for
  this R" filters; the other providers' lists are searched in Packages > Other sources, not mixed
  into Find. P1A.2 is done)
- P1A.3 Browse: CRAN Task Views (Econometrics, Survival, Bayesian, Spatial
  and the rest) as a tree; "new" and "recently updated" lists
  (`ssc new`/`ssc hot` equivalents); optional download counts from the
  public download-logs service
  (done: task views, with a filter in Find and a Browse tab, and a recently-published list;
  not done: download counts, a "new packages" list. P1A.3 is done apart from those)
  (partly done: a newest-packages list; not done: task views, download counts, and
  "new" packages, which CRAN's files do not say)
- P1A.4 Describe: package page with description, version, authors, links,
  reverse dependencies, vignettes and the system libraries it needs
  (`ssc describe` equivalent)
  (done, except vignettes: needs, who needs it, links, system requirements)
- P1A.5 Install: one click or `rc install pkg` (with `version`, `provider`,
  `from()` choices); resolves dependencies, shows what will be installed
  and downloaded, prefers binaries, runs in a supervised R process with
  progress and a cancel button; the exact R call is shown and logged
  (done for CRAN: plan, confirmation with the exact call, install through R; progress is R's own
  output and Stop cancels. Older versions, 2026-11-14: a package's page has Older versions, which
  lists CRAN's archive for it on request, with an Install for each, or takes a typed version;
  `rc install x, version()` and the failure retry use the same list. P1A.5 is done)
- P1A.6 Installed packages: list, version, update available, update,
  remove, `which`-style "where does this function come from", load status
  (done in the Packages window: list, versions, loaded, updates for RCraft's own library,
  remove, where a function comes from)
- P1A.7 Failures made readable: missing compiler or system library, with
  the platform-specific fix named; retry from binary or older version
  (done: compiler, system library, newer R, unreachable mirror, missing dependency and more,
  with retries; the Windows messages were recorded from real R, the macOS and Linux ones
  are not yet checked against real output. P1A.7 is done)
- P1A.8 Reproducibility: record installed packages and versions in the
  session and project lockfile; install a project's packages in one step
  (done: Packages > Project package list saves the loaded packages with versions to a text
  file and installs from one with a yes; older versions from CRAN's archive; not done: a list
  kept automatically, or written into the saved script. P1A.8 is done)
- P1A.9 Command forms in the Command window, for example `rc install
  fixest`, `rc search panel data`, `rc describe fixest`, `rc update`,
  `rc installed`; Tab completion over the index
  (done: install, remove and update open the usual list to say yes to; search, describe and help
  answer in Results; installed runs R; Tab completes. Not in the Script pane. P1A.9 is done)
- P1A.10 "Package needed" prompt: when a menu entry or command needs a
  package that is not installed, offer to install it, with the same
  confirmation as P1A.5 (dialogs: done earlier through `Spec::needs`; typed commands done
  2026-11-14: an error, warning or message naming a missing, too old or pre-R-4.0 package gets
  Install and Install and run again under it in Results)
- Providers (Bioconductor, r-universe, GitHub, local): done in Packages > Other sources and `rc
  install ..., provider(...)`, with trust labels and an extra tick for unreviewed code; not yet
  checked against the real services
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
  (done 2026-10-29: R completes the line itself; see STATUS. Finished 2026-11-14: at the cursor,
  argument names first and a single one as `name = `, and column names inside `df[["`, `df["`,
  `df[, "` and `df[, c("a", "`)
- P2.3 Results window: text output, errors and warnings with the failing
  call, rich tables, copy and export (rich tables done 2026-11-14: a command whose value is a data
  frame, matrix or table, or a named list of them, shows as a grid of R's own formatted cells, with
  Copy (pastes as cells) and Save as CSV; the log and script keep R's printed text. Also 2026-11-14:
  the table inside `summary(lm)`, `summary(glm)`, `anova` and `lmtest::coeftest` as a grid within R's
  own text, and Copy pastes a formatted table into Word)
- P2.4 Session log and script: every command and menu action recorded as R

## Phase 2A: Help

Help for every layer, reachable the way Stata users expect: a **Help**
menu, `help topic` in the Command window, and F1 on a word or a dialog.

- P2A.1 R help pages: render a function's help (the Rd page) in a Help
  window with usage, arguments, value, details, examples, links and "see
  also"; back and forward; open from `?topic`, `help(topic)`, F1 on a word,
  or a link (done 2026-10-29: the Help window shows R's own text for a page, with See also links,
  back and forward, `?topic`, `help topic`, F1; links inside the text are still to do)
- P2A.2 Examples: a **Run example** button on each help page, copy to the
  Command window, and output in Results
  (done 2026-10-29: Run examples and Copy examples; the examples go to the session as one typed
  command)
- P2A.3 Search help: `help search words` over installed packages' help
  titles, aliases and keywords (the `search` equivalent), plus function-name
  search (`apropos`) and a topic index
  (done 2026-10-29 for installed packages: each word must match a name, title or keyword;
  `help search words`, `??words`; no topic index yet)
- P2A.4 Vignettes and manuals: list and open a package's vignettes
  (HTML/PDF) and its news file, from the package page or the Help menu
  (done 2026-10-29: package pages with a help-topic index, vignettes with Open PDF/HTML, R code and
  source, and NEWS; R packages do not install PDF reference manuals, so the topic index stands in
  for the manual)
- P2A.5 Dialog help: every dialog has a Help button opening a page that
  says what it does, what R it generates, which package it needs, and
  links to the R functions' help pages; written for RCraft, not copied
  (done 2026-10-29: a Help button on every dialog, Use my file and Packages; each page is made
  from the dialog's own spec with the R it writes and links to R's help for its functions)
- P2A.6 RCraft manual: getting started, the menus, the Command window,
  Packages, projects and reporting, as pages inside the app, searchable
  from the same box as R help
- P2A.10 RCraft's own manuals as PDF (the user asked 2026-10-29 for our own, catered to RCraft;
  *down the road* — until then the package page opens CRAN's PDF reference manual, see P2A.4). RCraft lays out its own PDF with no LaTeX and no other program: the RCraft manual
  (P2A.6) and a **reference manual for any installed package**, built from R's own help text for
  that package (as shown in the Help window) in RCraft's style. Catered to RCraft means: each
  function's entry says which RCraft dialogs write it (Statistics > Linear model and so on, from
  `dialog_help`), the examples are marked as runnable from the Help window, and the front matter says
  which RCraft version and R version and package versions it was made from. Needs a PDF writer with
  embedded fonts under a licence we can ship (MIT, Apache or our own code; fonts attributed in
  `NOTICE`), page layout (headings, text, code, tables of arguments, a contents page and a topic
  index), and a **Save as PDF** button on the Help window's package page and on the manual.
- P2A.7 Help for missing things: if the topic is in a package that is not
  installed, say so and offer Phase 1A's install and search
- P2A.8 Offline by default: installed packages' help works with no
  network; online links open in the user's browser only on click
- P2A.9 Tests: help rendering on made-up Rd pages built in code; search
  ranking; dialog pages checked for broken function links
  (dialog pages done: every dialog has a page, an example, and function links that R has pages for;
  Rd rendering and search ranking tested with the Help window)

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
  (done: `statatr::reader`, also `while`, `if`/`else`, `frame`, `program`, `mata`, `python` and
  `input` blocks; macros recorded, not expanded; reads all 230 of StatCraft's probe do-files with
  no problems. P2B.1 is done)
- P2B.2 Expressions: Stata operators and functions to R (`==`, `&`, `|`,
  `!`, `inrange`, `inlist`, `missing()`, string, date and math functions),
  Stata missing values `.` and `.a`-`.z` mapped to `NA` (the extended
  missings kept as an attribute where it matters), `_n` and `_N` inside
  `by`, `[_n-1]` lags, and the rule that missing sorts high in comparisons.
  (done: `statatr::expr_r`, for dplyr's `mutate()`/`filter()`; 42 expressions give Stata's answers
  in real R. The extended missings get a caveat instead of an attribute; `_b[]`, `e()` and `r()` wait
  for P2B.6. P2B.2 is done)
- P2B.3 Data commands: `use`, `save`, `import delimited`, `import excel`,
  `generate`, `replace`, `drop`, `keep`, `rename`, `recode`, `label`,
  `sort`, `gsort`, `egen`, `merge`, `append`, `reshape`, `collapse`,
  `duplicates`, `encode`, `destring`, `tostring`, `order`, `describe`,
  `list`, `count`, `set seed`. Variable and value labels kept as labelled
  columns.
  (done: `statatr::data_r`; 20 do-files of these commands give the same data as StatCraft, row for
  row, in real R. `merge m:m`, `replace` that reads its own earlier rows and a few options say why
  they are not translated. P2B.3 is done)
- P2B.4 Statistics commands: `summarize`, `tabulate`, `correlate`, `ttest`,
  `regress` (with `robust` and `cluster()`), `logit`, `probit`, `poisson`,
  `nbreg`, `ologit`, `mlogit`, `anova`, `predict`, `test`, `lincom`,
  `margins`, `estat`, `xtset`/`xtreg`, `ivregress`, `stset`/`stcox`. Each
  maps to a chosen R function in a named package (for example `fixest`,
  `sandwich`, `survival`) and the package is offered through Phase 1A when
  missing.
  (part done: `summarize` (with `detail`), `tabulate` (one-way and two-way,
  with `chi2`, `row`, `column`, `cell`, `missing`, `nolabel`, `sort`),
  `correlate` (with `covariance` and `means`), `ttest` (one-sample, paired,
  two-sample by `by()` or two variables, with `unequal` and `level()`) and
  `regress` (Stata's factor notation `i.`/`c.`/`ib#.`/`#`/`##`, `noconstant`,
  `robust`, `vce(robust|ols|cluster v)`, `level()`). Each figure is checked in
  real R against the same figure worked out a second way, so Stata's rules are
  the ones kept: percentiles by Stata's rule, population skewness and
  kurtosis, casewise `correlate`, chi-squared with no continuity correction,
  HC1 for `robust` and HC1 with `G / (G - 1)` for clusters, with t tests on
  `G - 1` degrees of freedom. `ttest, welch` is refused with its reason.
  Part two, also done: `logit`, `probit`, `poisson`, `nbreg`, `ologit`,
  `oprobit`, `mlogit` (with `or`/`irr`/`rrr`, `exposure()`/`offset()`,
  `baseoutcome()`, and `robust`/`vce(cluster)` for logit, probit and
  poisson), and after any model `predict`, `test`, `lincom` and `estat`
  (`ic`, `vce`, `hettest`, `ovtest`), all reading Stata's `e(b)`, `e(V)` and
  `e(df_r)` left in R. Stata's rules kept: observed-information errors (probit
  and nbreg differ from R's own), `N / (N - 1)` for robust likelihood models,
  the most frequent outcome as mlogit's base, Wald intervals, predictions in
  every row as floats, `estat ic` counting only coefficients. `stset` and
  `stcox` are done (Breslow ties by default, `efron`, `exactp`, robust and clustered errors with Stata's factors, `strata()`, `scale()`, `id()`, `origin()`, `enter()`, `exit()`, `predict, hr|xb`), matching Stata 19 to 10 digits; not yet `exactm` (no counterpart in R), `estat phtest` or the other `st` commands. With that, the commands P2B.4 names are all translated; the smaller gaps left are listed in `plan/STATUS.md`. Weights (`fweight`, `aweight`, `pweight`, `iweight`) are done for `summarize`, `tabulate`, `tab1`/`tab2`, `correlate`, `pwcorr`, `regress`, `logit`, `probit` and `poisson`, matching Stata 19; not yet in the other models or before `margins`/`estat`. `by` (and `bysort`) before the statistics commands and the models is done, as is Stata's check for perfect prediction before `logit`/`probit`. `tab1`, `tab2` (`firstonly`, `chi2`), `pwcorr` (`sig`, `obs`, `star()`, `print()`, `bonferroni`, `sidak`, `listwise`), `summarize, meanonly` and `regress, beta` are done and match Stata 19. `ivregress` is done
  (`2sls`, `liml`, `gmm`, `small`, `vce(robust|cluster)` for 2sls, `first`,
  `estat endogenous` and `overid`, `predict`, `test`), matching Stata 19 to
  10 digits; not yet robust `liml`, `gmm` with its own `vce()`/`wmatrix()`,
  `estat firststage`, or the robust forms of the `estat` tests. `xtset` and
  `xtreg` (`fe`, `re`, `be`, `vce(robust|cluster)`, then `predict`, `test`,
  `lincom`) are done and match Stata 19 to 10 digits; not yet `mle`, `pa`,
  `margins` or `estat ic` after it, or `predict, u|e|ue`. `anova` is done: categorical unless `c.x`, partial (Type
  III, an empty cell refused) or `sequential` sums of squares, postestimation
  as after regress; not yet nested (`|`) or error (`/`) terms, `repeated()`,
  or partial sums of squares with an empty cell.
  `margins` is done after regress, logit, probit, poisson and
  nbreg: predictive margins, `dydx()` (continuous and factor), a marginlist
  of factor variables, `at()`, `atmeans`, `predict()`, delta-method errors
  from the model's own variance; not yet after ologit/mlogit, nor `post`,
  `over()`, `eyex`, contrasts)
- P2B.5 Graph commands: `twoway`, `histogram`, `graph bar`, `scatter`,
  `line`, `kdensity` to ggplot2.
  (done 2026-10-31: `histogram` (`bin()`, `width()`, `start()`, `discrete`, the four scales, `normal`, `kdensity`), `kdensity` (all of Stata's kernels, `bwidth()`, `n()`, `at()`, `generate()`, `normal`), `scatter`, `line` (`sort`), `connected`, `twoway` with `scatter`/`line`/`connected`/`lfit`/`qfit`/`lowess`/`histogram`/`kdensity` layers in `( )` or `||`, `lowess` (`generate()`), `graph bar`/`hbar` with statistics and `over()`, `graph export`, titles, `legend(off)` and `by()`. The bins, densities, smooths and bar heights are worked out as Stata does and match Stata 19 to 12 digits; look-only options are left to ggplot2 and named in a note. 2026-11-03: `twoway bar` and `rarea`, plots split at `||` with their own options, `graph box`/`hbox`, `graph bar (count)` of the rows, `xline()`/`yline()`, a second y axis (`yaxis(2)`, its plots mapped onto the first axis and the right axis showing their scale), `graph drop _all`, and the look-only short forms Stata 19 accepts (`lp()`, `lc()`, `clp()`, `mlabv()`, `bc()` ...). Not yet: `pie`, `matrix`, `combine`, `twoway` `area`/`rcap`/`lfitci`/`function`, `saving()`, `by()` with fitted layers or histograms, `histogram` with `by()`, `xlabel()` as axis breaks.)
- P2B.6 Estimates and returned values: `e()`, `r()` and `_b[]`/`_se[]`
  after a translated command, with `estimates store` and `esttab`/`outreg2`
  style tables going to Phase 7.
  (done 2026-10-31: e() after every translated model with Stata's names and values (regress, the likelihood models, anova, xtreg fe/re/be, ivregress, stcox, margins post), r() after summarize, count, tabulate, correlate, ttest, test, lincom and margins, `_b[]`, `_se[]`, `_coef[]` and `[eq]_b[]` in any expression, `display` with Stata's %g/%f/%e/%s formats decoded at every width, `ereturn list` and `return list` line for line; a value Stata keeps that RCraft does not is refused with the reason, not left missing. Then the leftovers, the same day: `scalar`, `matrix list` in Stata's layout, e(b) and e(V) with Stata's columns (base levels, o., equations), r() after the other ttests, pwcorr, tab1/tab2, describe and the weighted tables, e() after xtreg mle/pa and with iweights. Not yet: e() after anova with error terms, ivregress gmm with options; margins' r(b)/r(table); matrices a do-file makes.)
- P2B.12 Tab completion in Stata mode (the user asked 2026-10-29: it must work there too):
  Stata command names and their abbreviations, `if`/`in`/`by` and option words per command,
  variable names of the open dataset (with Stata's abbreviation rules, wildcards and
  `varlist` ranges), local and global macro names, value-label names and file names after
  `using`; the same Tab behaviour as for R lines (complete as far as the choices agree, a
  second Tab lists them); tested against the translator's own command table
  (part one done 2026-10-31: `statatr::complete`, all of the above with subcommands too, every command, subcommand and option offered checked against the translator. Wired to the Tab key in Stata mode with P2B.7, the same day.)
- P2B.7 The two-pane view: Stata on the left, generated R on the right,
  line by line, with a mapping so a click on either highlights the other; a
  note on each line that has a known difference from Stata.
  (done 2026-10-31: the Stata switch on the Command and Script panes, typed lines translated after the ones before them (`statatr::live`) and run, the Script Editor's two panes with the cursor's line marked and a click selecting its Stata, the notes and reasons on every line, Copy R, Page Up and Tab in Stata mode; then Stata lines in Results, Tab in the Script Editor, and Stata's own summarize, tabulate and regress tables, checked character for character against Stata 19.)
- P2B.8 Differences guide: a page per translated command listing where R
  and Stata differ (default standard errors, degrees of freedom, how ties
  and missing values are handled, sort stability, string widths, float
  versus double). Linked from the note in P2B.7.
  (done 2026-10-31: `statatr::differences`, a page per translated command and a general one, opened from every note and reason under a Stata line and from the Help menu.)
- P2B.9 Do-file runner: see Phase 2C.
- P2B.10 Checked against Stata: a private corpus of made-up data and
  recorded Stata results, the same method as StatCraft's observation kit, so
  translated estimates are compared with Stata's to about 8 significant
  digits. A command moves to `Verified` only when its translation matches.
  (done 2026-10-31: the corpus in engine/tests/stata_corpus, recorded by `cargo xtask stata-record`, 119 of Stata 19's results matched; `statatr::verified` lists the Verified commands, shown in the differences guide.)
- P2B.11 Ado programs, Mata and `net`/`ssc` commands are out of scope for
  the first release; `ssc install x` is translated to `rc install` (Phase
  1A) when an R package of that name exists, and flagged otherwise.
  (done 2026-10-31: `ssc install` offers the R package of the same name through rc install, or says there is none; other ssc subcommands and net are refused with the reason.)

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
  (done 2026-10-31: `statatr::dofile::scan`, the do-file gone through without R: lines clean, with notes, not translated (each reason) and known only when it runs; commands, R packages, `ssc install`, every file and folder named and whether it is there.)
- P2C.2 Resolve paths and data: ask for the data folder, map Windows paths,
  keep paths relative to the do-file, follow `do` and `include`
  (done 2026-10-31: it runs in its own folder (or one chosen), `cd` is followed, a backslash in a path is read as `/`, a folder not on this machine can be pointed to without changing the file; `do`, `run` and `include` are followed, `do` with its own locals and arguments.)
- P2C.3 Front end: reuse StatCraft's do-file reader, command syntax and
  expression parsers (shared crate or vendored; see open questions)
  (done: vendored into `statatr`, not a dependency, because StatCraft is private; the
  reader, syntax and expression parsers work and are tested; the translation is not
  started. P2C.3 is done)
- P2C.4 `rcraftstata`, a small R package we ship that implements the Stata
  rules R does not share: missing values sorting high, variable
  abbreviations and wildcards, `_n`/`_N`, `by` state, `preserve`/`restore`,
  macros, storage types. Installed on first use, with the user's yes.
  (done differently 2026-10-31: there is no package to install. Stata's rules live in RCraft's helpers, written into the script itself, and in the do-file runner (`statatr::dofile`: macros, loops, `if`, `while`, files). Wildcards, ranges and `_all` in varlists use the data's names from R (part two, the same day); not yet: abbreviations in plain varlists and expressions.)
- P2C.5 Emit: idiomatic R where Stata and R agree, calls into
  `rcraftstata` where they differ; a line map kept
  (done 2026-10-31: each line keeps its file, line, text as written and loop round, `dofile::Origin`.)
- P2C.6 Run: line by line in the R session with live progress, the Stata
  line beside its R, and a choice at the first problem (stop, skip,
  continue); the log is saved
  (done 2026-10-31: File > Run do-file…, or a .do file dropped on the window; the values only R knows (`local m = r(mean)`) are worked out by R as it runs; Stop here, Go on, Go on without asking; Save log.)
- P2C.7 Report and export: what ran, differences from Stata (linked to the
  guide in P2B.8), **Save as R script** or **Save as Quarto**
  (done 2026-10-31: Save as R script, Save as Quarto, Save log, a line's notes linked to its page in the guide.)
- P2C.8 Tiers of coverage: 1 data management and control flow, 2 core
  statistics, 3 panel/survival/survey/IV, 4 graphs, 5 programs and chosen
  SSC commands; the share of lines translated per tier is shown in
  `docs/coverage.md`
  (done 2026-11-01: `statatr::coverage` puts each command in its tier and sums dry runs; the table for StatCraft's 381 probe do-files is in `docs/stata-coverage.md`, since `docs/coverage.md` is the menus' generated inventory.)
- P2C.9 Verification: StatCraft's probe do-files as the first inputs,
  recorded Stata results in a private corpus (never committed), compared to
  about 8 significant digits; a command is `Verified` only on a match
  (started 2026-10-31: `control.do` and `aware.do` in the corpus check macros, loops, include, varlists from the data, levelsof and unab against Stata 19; StatCraft's probe do-files and public replication packages are still to come.)
  (done 2026-11-01 for StatCraft's probes: `engine/tests/stata_probes.rs` runs each of them on R and sets its top-level display, return list and ereturn list against Stata 19's log, to 8 significant digits, with the counts in `docs/stata-coverage.md`; `made.do` in the corpus makes expand, preserve, restore, set obs and tokenize Verified. Public replication packages are still to come.)
  (continued 2026-11-02: 11 public replication packages from Harvard Dataverse, 46 do-files, run dry: 40.5% of their lines translated at first, 52.1% after erase, compress, append force, reshape without j(), fvset base, cluster() and label sets going with the data; `replication.do` in the corpus checks them against Stata 19.)
- P2C.10 Known limits stated up front: Stata's random draws, Mata, compiled
  plugins, interactive commands
  (done 2026-10-31: the scan lists those the do-file uses. From 2026-11-01 runiform(), rnormal() and rgamma() are no longer among them, P2C.11.)
- P2C.11 Stata's own random draws on R: `set seed`, `runiform()`, `rnormal()`
  and `rgamma()` give Stata 19's numbers, then `runiformint()`,
  `rbinomial()` and the others
  (done 2026-11-01 for the first four: Stata's generator (mt64) and StatCraft's decoding of its normal and gamma methods, written in R (`data_r/random.rs`), its state kept in R across lines, `preserve`, `by` and typed lines; draws only for the rows an `if`/`in` selects; `random.do` in the corpus checks them exactly. Design and observations in `plan/stata/random-numbers.md`. The other random functions followed the same day: `rexponential()`, `rlogistic()`, `rweibull()`, `rweibullph()`, `rcauchy()`, `rlaplace()`, `rpareto()`, `rgeometric()`, `rigaussian()`, `rchi2()`, `runiformint()`, and `rbinomial()`, `rnbinomial()` and `rhypergeometric()` in the ranges whose method is known, checked on 34 × 1,500 of Stata's draws and in `variates.do`. `rt()` (a ratio of uniforms, Stata's stream exactly and its draws to the last bit or so) and `rbeta()` (each method its manual names; equal shapes above 1 not yet) followed on 2026-11-05 (`variates2.do`). Still to come: `rpoisson()`, `rbeta()` with equal shapes above 1, KISS32 under `version` 13 or earlier, `c(rngstate)`.)
**P2C.12 Closer to Stata on the probes** is the name of a group, not a task, and is not
counted. Its goal: no compared line differs, and more lines are compared, until every
line of the probes and the replication packages that Stata runs is translated and gives
Stata's result. Until 2026-11-18 it was one task, marked done on 2026-11-01, which took in
all the Stata-matching work after that and so hid it. It is split into P2C.12a to
P2C.12al: 38 tasks, grouped by command family (a to ae) and by coverage and matching
milestones (af to al). Marks: **done** only where every command named is in `VERIFIED`
(`crates/statatr/src/verified.rs`) with a corpus file and recorded Stata 19 output;
**part done** where the commands are Verified but some forms are still refused; **not
started** otherwise; **on hold** where the user parked it (still to be built, not dropped).
Marks are as of `main` on 2026-11-18. The 95% in the milestones is a floor, not the end.
Findings per command are in `plan/stata/`; what the probes still refuse, in
`docs/stata-coverage.md`.

Command families:

- P2C.12a Foundations
  (done 2026-11-01: `capture` gives Stata's `_rc` and `noisily` its message for a name
  that is not there (111) and a variable already defined (110); a refused line that may
  fit a model hides the model before it from `estat`, `predict`, `test`, `lincom` and
  `margins`; a refused add-on or do-file program makes the globals the do-file has not
  set unknown; `invnormal()` and `normal()` give Stata's doubles; the storage types'
  `c()` constants. Errors that stop a line are printed in Stata's words (2026-11-15).
  Observations in `plan/stata/errors-and-normal.md`.)
- P2C.12b Tests, tables and intervals
  (done 2026-11-17: `ttest` and `ttesti`, `sdtest`, `prtest`, `ranksum`, `signrank`,
  `signtest`, `kwallis`, `oneway`, `ci`, `centile`, `tabstat`, `tabulate` with
  percentages, `correlate`, `pwcorr`, `mean`, `proportion`, `ratio`, `total`, `icc`,
  `kap`, `ktau`, `tetrachoric`, `mhodds`, `tabodds`, `sktest`, `alpha` and the epitab
  commands (`cs`, `cc`, `ir` with their immediate forms), each in `VERIFIED`.)
- P2C.12c Descriptive and checking commands not yet matched
  (batch C of the StatCraft port done 2026-11-18: `inspect`, `cumul`, `pcorr`, `bitest` and
  `bitesti`, `robvar`, `swilk` (12 to 2000 untied values only), `pctile` and `dstdize` (one by() and one
  stratum variable); `pwmean` done 2026-11-18 in its own session (one or several over() variables, the five
  `mcompare()` methods, the table options, `groups`, `sort`, `level()`, e() and r() with their matrices; Tukey to
  about 1e-7) and finished 2026-11-19 (`snk`, `duncan` and `dunnett`, a bare `pwmean` that shows the last results
  again, `over(i.g)` and a variable twice, an empty `mcompare()`, an option given twice, an adjusted method with
  `cimeans` and other tables, zero or one residual degrees of freedom; open: `r(groups#)`, which is refused because the
  order of the letters is not always Stata's, and Dunnett's numbers, which Stata approximates to about 1e-6 to 3e-4,
  see `plan/stata/pwmean.md`). Still open, part of it on hold for the user's go:
  `codebook`, `spearman`, and the forms of `summarize`, `list`, `ttest` (`welch`) and `tabodds` that are
  still refused.)
- P2C.12d Functions
  (part done 2026-11-17: 132 functions Verified (distribution, date, string, matrix and
  random-draw functions). Batch F of the StatCraft port done 2026-11-19: `word()` and `wordcount()`
  (blanks only), `strproper()`, `stritrim()`, `strrpos()`, `strpos()` (bytes), `strreverse()`,
  `subinword()`, `regexr()`, `regexm()` with `regexs()`, `plural()`, `bsubstr()`, `udsubstr()`,
  `matrix()`, `scalar(exp)`, `match()`, the noncentral `nt()`, `nttail()`, `nchi2()`, `nchi2tail()`,
  `nF()` and `nFtail()`, and the old names `chiprob()`, `fprob()`, `invfprob()`, `invnorm()`,
  `normprob()`, `proper()`, `itrim()`, `index()` and `reverse()` (32 more functions Verified, corpus
  `functions_f.do` and `noncentral.do`). Refused: `cofc()` in lower case (Stata: unknown function).
  Second round 2026-11-19: `nt()` and `nttail()` with a noncentrality parameter above 37.62 (integrated, to
  1e-13 of Stata), `invnchi2()`, `invnchi2tail()`, `invnt()`, `invnttail()`, `substr()` and `usubstr()` with
  any start and length (corpus `noncentral2.do`, `substr.do`), and `A[i,j]` outside the matrix is `.`. Third round
  2026-11-19: `invnF()`, `invnFtail()` and the `np` solvers `npnt()`, `npnF()`, `npnchi2()`, with Stata's edges found
  by bisection and the noncentral F summed as a Poisson mixture of betas (R's `pf` with `ncp` is good only to 1e-9;
  corpus `noncentral3.do`). Fourth round: `nF()` and `nFtail()` as Stata computes them (Stata values of 1e-20 or more
  to 1e-9; corpus `noncentral4.do`). `plan/stata/functions-f.md`.
  Probes 22,397 -> 22,430 translated lines (77.5%); packages not re-run (87.8%).
  `rpoisson()` and `rbeta()` with equal shapes stay with P2C.11.)
- P2C.12e Single-equation models
  (part done 2026-11-17: `regress`, `logit`, `probit`, `poisson`, `nbreg`, `ologit`,
  `oprobit`, `mlogit`, `clogit`, `anova`, `rreg`, `robreg`, `newey`, `prais` and `nl`
  Verified. Some forms of each are still refused (the probes' `logit`, `probit`,
  `poisson`, `nbreg`, `regress` and `ologit` groups, `e(ic)` after `logit`), and
  `poisson` to 8 digits, open since 2026-11-01, is not recorded closed.)
- P2C.12f Other single-equation models
  (not started, except that `glm` is translated by maximum likelihood (2026-11-15) and is
  not yet in the corpus: `tobit`, `cnsreg`, `fracreg`, `betareg`, `qreg`, `areg`,
  `heckman`, `intreg`, `truncreg`, `mprobit`, `slogit`, `zip`, `zinb`, `ztp`, `ztnb`,
  `tpoisson`, `tnbreg`, `cloglog`, `hetprobit`, `logistic`, `nlogit`.)
- P2C.12g Post-estimation
  (part done 2026-11-17: `predict`, `estat`, `estimates`, `test`, `testparm`, `lincom`,
  `nlcom`, `testnl`, `margins`, `marginsplot` and `suest` Verified. Still refused:
  `contrast`, `pwcompare`, `hausman`, `lrtest`, `predictnl`, forms of `margins`
  and `estat` after models not yet translated, and some `estimates` forms.)
- P2C.12h `stepwise`
  (part done 2026-11-18: `regress`, `logit`, `probit` and `poisson`, Verified
  (`stepwise.do`, `stepwise_glm.do`, `stepwise_weights.do`, `stepwise_weights_glm.do`,
  `stepwise_factors.do`, `stepwise_factors_glm.do`): weights, `vce()`, `robust`, `noconstant`, `level()` and
  factor variables included. Refused: `tobit`, interactions with factors, `ibn.`, a variable listed twice.
  `plan/stata/stepwise.md`.)
- P2C.12i `nestreg`
  (part done 2026-11-18: `regress`, `logit`, `probit` and `poisson`, Verified
  (`nestreg.do`, `nestreg_glm.do`). Refused: weights, factor variables, a variable listed
  twice, other models. `plan/stata/nestreg.md`.)
- P2C.12j Panel models, first set
  (part done 2026-11-16: `xtset`, `xtreg`, `xtregar`, `xtabond`, `xtdpd` and `xtdpdsys`
  Verified. Some `xtreg` forms are still refused.)
- P2C.12k Panel models, the rest
  (part done: `xtdescribe`, `xtsum` and `xttab` Verified 2026-11-19, batch H of the StatCraft
  port, with the printed tables and r() as Stata 19's (`xtpanel.do`); refused: `i()` and `t()`
  naming other variables than `xtset` holds, `width()` below 3, a `%tc` time,
  `plan/stata/xtdescribe.md`. Not started: `xtlogit`, `xtpoisson`, `xtnbreg`, `xtprobit`,
  `xtcloglog`, `xtologit`, `xtoprobit`, `xttobit`, `xtgls`, `xtgee`, `xtpcse`, `xtivreg`,
  `xtunitroot`; and collinear terms in the panel, survival and IV models, left from P2C.13.)
- P2C.12l Survival
  (part done: `stset` and `stcox` Verified. `stdescribe`, `stsum`, `sts` (list, generate, test), `stptime`, `stir`, `ltable` and `streg`
  (exponential, Weibull, lognormal, loglogistic, Gompertz) Verified 2026-11-19, batch K of the StatCraft port (`survival.do`, `streg.do`, to 1e-8, `streg`
  with Stata run at its tightest tolerance); `sts graph` and `stcurve` after `streg` are drawn (not compared); `stset` takes `time0()` and `clear`. Refused:
  `streg` with the generalized gamma, factor variables, `strata()`, `frailty()`, `predict` and `estat` after it; `stcurve` after `stcox`; `sts` with late entry,
  `sts test, cox`; `ltable` graphs; `stptime, smr()`, see `plan/stata/survival.md`;
  measured: probes 78.3% -> 78.6% (22,725 -> 22,804 translated source lines), packages 94.0% unchanged (3,996 of 5,601).)
- P2C.12m Survey
  (part done: `svyset` and `svy` Verified. Not started: `svyset` with strata or stages,
  `svydescribe`, `svy: mean`.)
- P2C.12n Instrumental variables
  (part done 2026-11-17: `ivregress` and `ivprobit` Verified (`ivprobit.do`, with its
  `predict pr` after `twostep` and a collinear instrument). Not started: `ivreg2`,
  `ivtobit`, `gmm`.)
- P2C.12o Lasso and double selection
  (part done 2026-11-17: `lasso`, `elasticnet`, `sqrtlasso`, `lassoselect`, `dsregress`,
  `dslogit`, `poregress`, `pologit` and `xporegress` Verified; `lassocoef`, `lassoknots`
  and `lassoinfo` translated (2026-11-15) and checked in the lasso corpus files but not
  listed in `VERIFIED` yet. Not started: `dspoisson`, `popoisson`, `xpologit`,
  `xpopoisson`, `cvplot`.)
- P2C.12p Smoothing and nonparametric
  (part done 2026-11-17: `npregress kernel` Verified. Not started: `npregress series`,
  `lpoly`, `boxcox`.)
- P2C.12q Mixed-effects models
  (on hold, not started: `mixed`, then `melogit`, `mepoisson`, `menbreg`, `meprobit` and
  `menl`, which wait for it.)
- P2C.12r Time series
  (part done: `tsreport` and `tsappend` Verified 2026-11-19, batch H of the StatCraft port
  (`tspanel.do`); `tsappend` fills the gaps as `tsfill` does, as Stata does; refused: data not
  in panel and time order, a `%tc` time, a missing panel value, `plan/stata/tsreport.md`.
  `dfuller`, `pperron`, `dfgls`, `corrgram`, `wntestq`, `varsoc` and `xtunitroot` (`ips`, `ht`,
  `breitung`, `fisher`, `hadri`) Verified 2026-11-19, batch G of the StatCraft port (`tstests.do`,
  `varsoc.do`, `xtunitroot.do`, all to 1e-8); second round the same day: `xtunitroot llc` decoded
  black box and Verified (`xtunitroot_llc.do`, 146 results, 1e-8: the adjustment table, the
  per-panel bandwidth, late-starting samples, `lags()`, `kernel()`); refused: `llc` with
  `lags(aic|bic|hqic)` or `kernel(nwest)`, `breitung` with `robust` or `lags()` (tried, not
  reproduced), `varsoc, noconstant`, `ac` and `pac` (graphs), see `plan/stata/{tstests,xtunitroot}.md`;
  measured: probes 77.8% -> 78.0% (22,513 -> 22,590 of 30,040 source lines), packages 94.0% unchanged.
  Not started: `arch` (probed: Stata's path is BHHH 5 then BFGS 10 with a step-forward search, but its
  numerical scores are not reproduced, so 8 digits fail by one; see `plan/stata/tstests.md`), `var`,
  `vargranger`, `irf`, `forecast`, `ucm`, `sspace`.)
- P2C.12s Systems and multivariate models
  (part done: `pca`, `factor`, `rotate` and `mvtest` Verified. Not started: `sureg`,
  `nlsur`, `mvreg`, `manova`, `manovatest`, `canon`, `discrim`, `mds`, `sem`, `dfactor`.)
- P2C.12t `arima` and `gsem`
  (on hold, not started: `arima` and most of `gsem`, set aside by the user.)
- P2C.12u Power and sample size
  (part done 2026-11-18: `power` Verified for `onemean`, `twomeans`, `pairedmeans`,
  `oneproportion`, `twoproportions`, `pairedproportions`, `onevariance`, `twovariances`,
  `onecorrelation`, `twocorrelations`, `oneway`, `logrank`, `cox`, `exponential`,
  `oneslope`, `rsquared` and `pcorr` (`power.do` to `power8.do`). Being built: the
  remaining methods and forms, among them `test(lrchi2)`, `test(fisher)`,
  `test(binomial)`, graphs, and lists of values for some methods. `plan/stata/power.md`.)
- P2C.12v Spatial: `spset`, `spmatrix`, `spregress`
  (part done: `spset` and `spmatrix` are Verified (corpus `spmatrix.do`, 2026-11-18);
  `spregress` is refused: Stata stops its likelihood iterations before the maximum, by a
  derivative search only partly decoded (`plan/stata/spmatrix.md`).)
- P2C.12w Graphs from a do-file
  (part done: 98.6% of the graph lines of the probes translated (2026-11-01); `coefplot`
  and `marginsplot` Verified; `lfitci`, `qfitci`, `area`, `qnorm`, `pnorm`, `rvfplot`,
  `avplot`, `screeplot`, `pie` and `graph matrix` added 2026-11-17. Not started:
  `graph export` with no graph drawn earlier in the file, the `twoway qfit` and `rcap`
  forms, `cvplot`, `stcurve`.)
- P2C.12x Tables and reports
  (part done: `esttab`, `estadd`, `eststo`, `estpost`, `outreg`, `outreg2`, `frmttable`,
  `parmest`, `putexcel`, `putdocx`, `putpdf`, `dyntext`, `dyndoc` and `markdown` Verified. Not
  started: `collect`, `etable`, `dtable`, `table`, `asdoc`, `mtable`.)
- P2C.12y Data management and files
  (part done: `merge`, `append`, `reshape`, `collapse`, `contract`, `expand`, `egen`,
  `duplicates`, `label`, `insheet`, `file`, `filefilter`, `fdasave`, `fdause`, `outfile`,
  `outsheet`, `mkdir`, `rmdir`, `fillin`, `expandcl`, `joinby`, `mvdecode`, `mvencode`, `clonevar`, `char`, `note`, `notes`,
  `labelbook`, `compare`, `ds`, `lookfor` and `misstable summarize` Verified (batch A2 of the StatCraft port, 2026-11-19; refused:
  `char rename`, `notes` with `TS` or quotes, `labelbook` after a data file was read, `ds` with storage types or formats,
  `misstable patterns`, `tree` and `nested`, see `plan/stata/{char,notes,labelbook,compare,ds,lookfor,misstable}.md`). Still refused:
  forms of `egen`; `format`, `sysuse`, `import`, `export`,
  `mark`, `markout`.)
- P2C.12z Programs, macros and matrices
  (part done: `program`, `syntax`, `tokenize`, `gettoken`, `local`, `global`, `matrix`
  (many forms), `mkmat`, `svmat`, `ereturn` and `frame` Verified. Still refused: other
  `matrix` forms, the macro functions (`colnames`, `rownames`, `coleq`, `colfullnames`,
  `copy`, `sortedby`, `data label`), `constraint`, `ml`, `c_local`, `sreturn`,
  and the ado helpers (`opts_exclusive`, `_get_diopts`, `_robust`, `_ms_extract_varlist`,
  `_ms_omit_info`).)
- P2C.12aa Resampling and simulation
  (part done: `bootstrap` and `bsample` Verified (P2C.16). `jackknife` (also as `vce(jackknife)` of `regress`, `logit` and
  `probit`), `statsby`, `rolling`, `permute` and `simulate` Verified 2026-11-19, batch I of the StatCraft port (`jackknife.do`,
  `statsby.do`, `permute.do` (146 results), `simulate.do`, all to 1e-8, `permute` with Stata's own permutations and draws);
  `permute` and `simulate` run programs the do-file defines, one replication after another. Follow-up 2026-11-19 (the refused
  lines of the batch): `quietly spearman y x` (r(rho), r(N)), `r(nosuch)` (r(322)), e-class programs in `permute`, `_b` after
  `logit` and `probit` (named after the equation), `saving()` of `statsby` and `rolling`, `rolling` over gaps in time, `jackknife` with
  `if` and its r(459); `preserve`, `restore`, `post`, `postfile` and `postclose` leave r(), `format c*`; every probe line of batch I
  now translates. Round 3 (2026-11-19): printed `spearman` of two variables (Stata's beta p-value), `jackknife` with `in`, `statsby` and
  `rolling` with neither `clear` nor `saving()` (r(4) as Stata decides it), `rrecursive`, replications that stop with an error
  (missing results), any number of replications (one R loop). Still refused: `spearman` with ties printed and its r(p), and the rare
  forms in `plan/stata/resampling.md`;
  measured: probes 78.0% -> 78.4% (22,588 -> 22,734 translated source lines) before batch K; 78.7% (22,817) with K and round 3;
  packages 94.0% unchanged (3,996 of 5,601).
  Not started: `bca` and `vce(bootstrap)`.)
- P2C.12an Expressions Stata does not read (fix batch 2 of the StatCraft port, 2026-11-19)
  (done: under `capture`, a line whose expression Stata rejects (an operator at the end, a token left after the expression,
  unpaired parentheses, `+x`, a function with no argument, `generate y = x , 2`, `generate y = x [2]`) stops with Stata's
  r(198), r(132), r(133) or r(101); `_rc` is set, nothing else changes and the do-file goes on. `display`, `scalar`, `generate`,
  `replace`, `assert`, `count if`, `list if`, `local` and `global`; `display` shows the items before the bad one. Corpus
  `expression_errors.do` (187 return codes, all Stata 19's); note `plan/stata/expression-errors.md`. Messages are RCraft's own words.
  Measured: probes 78.7% -> 79.8% (22,823 -> 23,159 of 29,018 judged source lines), packages 94.7% unchanged. Still refused: `x[1] + +2`
  (r(303)), `"a" * +2`, and the same errors outside `capture` (Stata ends the do-file).)
- P2C.12ab Multiple imputation
  (not started: `mi`.)
- P2C.12ac Treatment effects and difference in differences
  (not started: `teffects`, `tebalance`, `didregress`, `xtdidregress`; `medeff` is
  Verified.)
- P2C.12ad Mata and compiled plugins
  (on hold: outside the first release (P2B.11, P2C.10); the do-files' Mata blocks stay
  refused and counted here until it is taken up.)
- P2C.12ae `_natscale`'s last bits
  (on hold: translated and Verified wherever no end label is 0 (14,841 results bit for bit
  and 9,119 of 9,120 to 8 digits on 2026-11-16); declined with the reason where an end
  label is 0 with a step below 1 (6,125 of 15,245 swept results). The bit-for-bit search
  was parked by the user on 2026-11-16 (`plan/stata/natscale-residue.md`). Before the
  narrowing it held 3,018 probe lines, 10.9 points of the share: no milestone from 95% on
  is reachable without it or without a decision to translate the zero-label cases.)

Coverage and matching milestones (the share is of the lines a dry run judges, each source
line once; **two measures count equally** for the 95% minimum: probe lines and replication-package lines.
Last measured 2026-11-19 (`main` at 271f164 plus the `#delimit` resync): probes 77.5% (544 do-files, 22,439 of 28,946
judged lines), packages 94.0% (3,996 of 4,249 judged source lines of the 46 replication do-files). Packages were 94.9%
(3,715 of 3,915) with the old reader, which saw one CR-only file (6J6BAW, 572 lines with no LF) as a single line, then
87.8% (3,750 of 4,272) with the resynced reader, whose 322 `# delimit ;` lines of that file did not translate until the
reader took the blank after the `#` (P2C.12am). 253 package lines are still not translated. Each milestone is closed
by measuring again and writing the figure into `docs/stata-coverage.md`:

- P2C.12af Probes 80% of lines translated (not started)
- P2C.12ag Probes 90% of lines translated (not started)
- P2C.12ah Probes 95% of lines translated: the user's minimum, together with the packages at 95%
  (P2C.12al); neither measure counts for the minimum without the other (not started)
- P2C.12ai Probes 98% of lines translated (not started)
- P2C.12aj Probes 100% of the judged lines translated, or refused only where Stata itself
  stops; the target (not started)
- P2C.12ak More lines compared, none differing: at each milestone above, the probes are
  run against Stata 19 again and every compared line matches to 8 digits (5,508 of 5,626
  on 2026-11-01, the rest `_natscale`; not measured again since) (not started)
- P2C.12al Replication packages: the 46 do-files of the 11 public packages to 100% of
  their lines translated, and every compared result matching; 95% of source lines is the
  user's minimum, equal to the probes' (P2C.12ah) (not started; 94.7% of source lines on
  2026-11-19 after the package quick wins (4,025 of 4,249: `bs`, `cd ""`, `graph combine` `l1()`/`l2()`, `outreg2 onecol`;
  `bs` of a program the do-file defines and the outreg2 lines after `tobit` still refused), 94.0% before them; 87.8% on 2026-11-18 while the resynced reader still missed `# delimit ;`, 94.9% with the old one that
  read 6J6BAW as one line)
- P2C.12am `# delimit ;` with a blank after the `#` (done 2026-11-19): the reader took `#delimit` and `#d` but not
  `# delimit`, so 322 lines of 6J6BAW (CR-only, run in `;` mode) did not translate. Stata 19 accepts blanks or a tab
  after the `#`, any abbreviation from `#d`, and `#delimit;` (StatCraft probe 754); StatCraft PR 29 fixed the reader and
  the vendored copy was resynced (b7d392e). Packages 87.8% -> 94.0% (3,750 -> 3,996 translated, 522 -> 253 not).
- P2C.12ao Least squares family (batch E of the StatCraft port, done 2026-11-19, in part)
  (`constraint`, `cnsreg` (constraints by projection, weights, robust and clustered errors, `predict`, `test`), `areg` (the
  within fit with the grand means, N - k - G, robust, clustered, fw/aw/pw, factor variables, `predict` xb, residuals, d,
  xbd), `sureg` and `mvreg` (two-step GLS, `isure`, `small`, `dfk`, `corr`, fw/aw, `predict` with `equation()`, `test`
  across equations), `qreg` (`nolog`, any quantile, fweights, the iid variance, `predict`), `logistic` and `xi` (round 2, 2026-11-19: `xi: cmd`, `xi i.g`, `prefix()`, `i.g*i.h`, `i.g*x`, `i.g|x`, the omit characteristics) Verified (corpus
  `cnsreg.do`, `areg.do`, `sureg.do`, `qreg.do`, `logistic.do`, `xi.do`; notes `plan/stata/{constraint,areg,sureg,qreg,logistic,xi}.md`).
  `nlsur` and the `qreg` ties and iteration log were built in round 3, the `nlsur` gaps in rounds 4 to 6 (below).
  Still refused: `xi` with a factor in two terms or `|` between factors (round 5, below: other levels and `noomit` are built); `constraints()` of sspace and spregress
  (`constraints()` of logit, probit and poisson, `areg` with collinear regressors and `logistic, noconstant` were built in round 4b, below).
  Measured on top of P2C.12an: probes 79.8% -> 80.2% (23,159 -> 23,278 translated source lines of 29,018 judged); with `xi` the 7 probe lines move from refused to known only when run (the dry run cannot ask R for the levels; the corpus shows them translating): 23,278 translated, 5,733 not, 1,104 when run; packages 94.7% unchanged (4,025 of 4,249).)
  Round 3 (2026-11-19, the on-hold `qreg` item): `qreg` where the minimum is not unique and `qreg` without `nolog` Verified (corpus `qreg.do`, 234 results to 1e-8, was 51; note
  `plan/stata/qreg.md`). Stata's own path rebuilt from its iteration log: one weighted least squares step (check-loss weights, zero residuals dropped), the k rows
  with the smallest weighted deviation as the first vertex, a descent over vertices priced per unit of the basic row's weight, breakpoints passed until the slope
  reaches zero, zero-residual rows keeping a side; the log prints (`Iteration k:` lines, `note: alternate solutions exist.`). On 140 Stata runs over 10 data
  sets (84 held out): WLS value 140/140, final e(b) 123 of the 126 Stata finishes, logged sequence 114/140 (all 14 continuous runs). Also found: e(q_v) is the
  floor(q (N + 1))-th value, and r(498) `VCE computation failed; try a different bandwidth` is exactly q -/+ h outside (0, 1). **Not reproduced:** Stata's cycling
  (r(430), r(498) `try increasing the maximum number of iterations`) on two-integer-dummy data, and the three e(b) endings above; the log can differ on heavily tied data.
  Measured: probes 80.2% and packages 94.7% unchanged (the probe lines already ran `nolog` or fit continuous data).
  Round 4 (2026-11-20, closer match on tied data): ties in the breakpoint order go by the row's place in the first sort (smallest weighted deviation under the WLS fit), not
  row number: on the 140 Stata runs the logged sequence 114 -> 124, the notes 128 -> 134, final e(b) 123 -> 124 of 126 (tuned on all 140: no longer held out). Corpus `qreg.do`
  344 results (was 234), 115 more with quantiles at the ends of flat pieces. Tried and not better, with the numbers, in `plan/stata/qreg.md`; checked by running Stata at the q +/- h of
  the failing fits that R's paths are Stata's (5 of 6 logs identical). **Still not reproduced:** Stata's give-up (r(430), r(498) `increasing the maximum number of iterations`) on
  two-integer-dummy data, and the tied-breakpoint order that rounding noise decides (16 of 140 logs, 2 of 126 e(b)).
Round 5 (2026-11-20, `xi`): factors whose values are not whole numbers from 0 up (negative, fractional, strings) are named by rank with Stata's header and `g==label` labels, `char s[omit]` for strings, `noomit` (before the terms or the colon), the product's type for `i.g*x`. Verified (corpus `xi.do`, 126 results to 1e-8, was 54; note `plan/stata/xi.md`). Still refused: a factor in two terms, `|` between factors, other options. Left for later and listed: `areg` with interactions and `ib.` bases, `cnsreg` with factor variables (both need `regress`'s e(b) naming and the base-level columns), `constraints()` of sspace and spregress.
  Round 4b (2026-11-20, the batch E leftovers): `constraints()` in `logit`, `probit`, `poisson` and `logistic` (Stata's Newton-Raphson over what the
  constraints leave free from its usual start projected onto them, the iteration log, the Wald header and constraint lines, `(constrained)` rows,
  e(Cns), robust, clustered, fw, pw, `or`/`irr`, `if`); `logit`/`probit`/`logistic` with `noconstant` (the same Wald header, the log from 0, no e(ll_0));
  `areg` with regressors the absorbed groups or the others make collinear (omitted with Stata's note and an o. column); `predict`'s `(option pr assumed; ...)`
  and `(option n assumed; ...)` notes after logit, probit and poisson. Verified (corpus `ml_cns.do`, 263 results to 1e-6 and the printed output line for
  line; `areg.do`, 137 results, was 77; notes `plan/stata/{constraint,areg,logistic}.md`). Still refused: `constraints()` with offset/exposure, iweights or a
  regressor omitted for collinearity; poisson with `constraints()` and `noconstant`; areg with a factor level omitted.
  Measured on top of round 3 (nlsur): probes 80.5% (23,356 -> 23,358 translated; the two `constraints()` probe lines), packages 94.7% unchanged.
Round 6 (2026-11-20, the factor-variable leftovers): `areg` with interactions (`i.g#c.z`, `c.z##i.h`, `i.g#i.h`, `c.x#c.z`) and `ib.` bases, `cnsreg` with factor variables (the base level a fixed column and the first rows of e(Cns), `1.g` reading `1b.g`, a coefficient fixed at 0 printed omitted, a plain `x = 0` too). Verified (corpus `areg.do` 205 results, `cnsreg.do` 523 results, printed output line for line; notes `plan/stata/{areg,constraint}.md`). Still refused: `cnsreg` with interactions, noconstant or collinear factors; `areg` with powers or a factor level omitted for collinearity.
- P2C.12aq Survey design (batch 5 of the StatCraft port, done 2026-11-19, in part)
  (`svyset` with `strata()`, `fpc()`, `singleunit()`, later stages, `[iweight=]`, no PSU, `svyset, clear` and bare `svyset`, with the r()
  it leaves; `svydescribe` (stage 1 with and without a varlist, `finalstage`, `if`/`in`, r()); `svy: mean`, `total`, `ratio` and `proportion` with
  the stratified linearized variance, fpc, the four `singleunit()` rules, `subpop()` as a variable or an `if`, `level()`, `over()` of one variable,
  the design header and e() as Stata's; the one-stage designs also reach `svy: clogit`'s tests as before) Verified (corpus `svy_design.do`, `svy_mean.do`; note `plan/stata/svy.md`).
  Still refused: `svy: regress`, `logit`, `probit`, `poisson` (the F header and R-squared, the iteration log, the adjusted Wald `test`); `svy:` under
  a design with later stages; `over()` of several variables or with ratio, proportion or `subpop()`; replicate-weight variances and poststrata in `svyset`;
  `svydescribe` for later stages, `single`, `generate()`; a stratum sampled completely; `e(V_srs)` and the other SRS matrices.
  Measured on top of P2C.12ao (probes only; the probe folder is StatCraft's `observe/dofiles` at b7d392e): 80.2% -> 80.5% (23,278 -> 23,343 translated
  source lines of 29,018 judged; 5,733 -> 5,668 not translated); packages unchanged at 94.7%.)

- P2C.12ao round 3: `nlsur` (done 2026-11-19, the on-hold item of batch E)
  (`nlsur (y1 = {a} + {b}*x) (y2 = ...)` with `{name}` and `{name = #}` parameters: NLS, FGNLS and `ifgnls`, Stata's Gauss-Newton
  log line for line (forward differences, step halving, stop at 1e-5 relative to the old size), the equation table with
  constants and uncentred R-squared, the z table with `(constrained)` rows, e(), `predict` with `equation(#k)`; options
  `nolog`, `level()`, `iterate()`, `eps()`, `noconstant`, `initial()`, if and in) Verified (corpus `nlsur.do`, 279 results to 1e-6;
  note `plan/stata/nlsur.md`, which also records what was found beyond StatCraft's decoding: the stopping rule, the
  Jacobian Stata's variance keeps, the constant, and that a right-hand-side missing value is r(480), not a dropped row).
  Still refused: weights, `vce()`, `variables()`, the evaluator form, `{name: varlist}`, `delta()`, `nls` with a parameter
  shared by two equations (Stata's estimates there are not the stacked least squares), `ifgnls` with `iterate()`; a run
  that has not converged stops with a message after its log (Stata prints a table, then r(430)). The cap on steps
  (100) is an assumption. Measured: probes 80.2% -> 80.3% (23,278 -> 23,291 translated of 30,115 source lines, 5,733 -> 5,720 not);
  packages 94.7% unchanged (4,025 of 4,249).)
- P2C.12ao round 4: `nlsur` gaps (done 2026-11-20)
  (fweights, aweights and pweights with `(sum of wgt is #)`, `vce(robust)` and `vce(cluster v)` (scores at the final
  estimates, clusters times G/(G-1) (N-1)/(N-k)), `nls` with a parameter shared by two equations (Stata's hidden pass
  weighted by each equation's variance, decoded; it also makes the cross-equation covariances 0, which round 3 printed
  from the full Sigma), `delta()` and the derivative step d = delta (|b| + delta) (the round-3 rule missed nonlinear zero
  starts and larger deltas), the default cap of 300 steps, the table of a run that runs out of steps then r(430),
  `ifgnls` with `iterate()`, `{name: varlist}` (`xb_x1`), `variables()`, the covariance change against the new Sigma;
  `test`, `lincom`, `nlcom` with `/a`, `[a]_cons`, `_b[/a]`; `estat ic`, `estat vce`) Verified (corpus `nlsur.do`, 680
  results to 1e-6, 401 new; note `plan/stata/nlsur.md`). Still refused: iweights, the evaluator form, `parameters()`,
  `margins` and other `estat` after `nlsur`, clusters with missing values; e() after a captured r(430) is not kept.
  Measured: probes 80.5% (23,356 translated of 30,115 source lines, 5,655 not; round 3 had 23,291 and 5,720, before
  the svy batch also landed).)
- P2C.12ao round 5: what round 4 left in `nlsur` (done 2026-11-20)
  (iweights (as fweights, `(obs = N)`; with `vce(robust)` Stata's r(101)), `vce(cluster v)` with v missing on some rows
  (left out), a linear combination written twice (the same parameters), `parameters()` without the evaluator form (Stata's
  r(198)), e() kept after a captured r(430), `margins` after nlsur (Stata's predictive margin of an equation's fitted
  values with delta-method errors; `dydx()`, `at()` and `predict(residuals)` are its r(322) and r(498)), `estat summarize`,
  `estat vce` in Stata's layout, lincom's table without a variable over it, and the function-evaluator form
  (`nlsur name @ varlist, parameters() neq()`: the do-file's program `nlsurname` runs at each step, compiled by the runner
  into an R function from its lines) Verified (corpus `nlsur.do`, 853 results to 1e-6, 173 new; note
  `plan/stata/nlsur.md`). Still refused: `hasconstants()`, `nparameters()`, the evaluator form with weights,
  `variables()` or `noconstant` and `predict`/`margins` after it, `margins` with terms, `over()` or `post`.
  Measured: probes 80.5% unchanged (23,356 translated of 30,115 source lines; the probe do-files have none of these forms).)
- P2C.12ao round 6: what round 5 left in `nlsur` (done 2026-11-20)
  (the evaluator form's `nparameters()` (b1, b2, ...), `hasconstants()` (centred R-squared, the constants named),
  weights, `variables()` and `noconstant`; `predict` and `margins` after it (the program run again on the data);
  `margins` terms (Stata's r(452) or r(322): the model has no covariates), `over()` with one or more variables (one
  margin per group, Stata's layout and r() names) and `post` (Stata 19's e() after it, with e(_N), e(error) and
  e(Jacobian))) Verified (corpus `nlsur.do`, 978 results to 1e-6, 125 new; note `plan/stata/nlsur.md`). Still refused,
  none of them seen in Stata: `hasconstants()` naming no constant for an equation, `over()` with value labels or missing
  values, `margins, post` after `vce(cluster)`. Probes unchanged at 80.5%.)
- P2C.12av Post-estimation (batch L of the StatCraft port, done 2026-11-20, in part)
  (`contrast` and `pwcompare` (after regress, anova, logit, probit, poisson and xtreg: plain, `r.`, `ar.`, `a.`, `gw.`, `g.`, `g#h`, `g@h`, `r.g@h`, several terms,
  `effects`, `mcompare(bonferroni sidak scheffe)`, `level()`, `cimargins`, `(not testable)` / `(not estimable)` where the design cannot tell the margins apart, weights and robust
  or clustered variances; r(b), r(V), r(table), r(F)/r(chi2), r(df), r(p), r(df2) and for `pwcompare` the `_vs` set), `hausman` (stored results or `.`, `constant`, `sigmamore`, `sigmaless`, a negative chi2),
  `lrtest` (nested logit, probit, poisson and regress fits), `predictnl` (`_b[]`, `xb()`, `predict()`, `if`, `se() variance() wald() p() ci() level()`, `double`)) Verified (corpus `contrast.do`,
  `contrast_glm.do`, `contrast_omitted.do`, `hausman.do`, `lrtest.do`, `predictnl.do`; note `plan/stata/postest.md`).
  **On hold** (listed, to be built): `manovatest` (3 probe lines; needs `manova`, batch MV), `tebalance` (1; needs `teffects`, batch CA), `contrast`/`pwcompare` after `areg` and `xtregar`, `hausman`/`predictnl` after `xtregar` (their results are not held as a model).
  Still refused: `contrast` with `post`, `noestimcheck`, `pveffects`, `cieffects`, `groups`, `tukey`, `snk`, `duncan`, `dunnett`; `gw.` after a weighted model; `lrtest` after a robust model.
  Measured on top of P2C.12aq (probes only; the probe folder is StatCraft's `observe/dofiles` at b7d392e): 80.5% -> 80.7% (23,343 -> 23,399 translated source lines of 29,018 judged; 5,668 -> 5,612 not translated);
  packages unchanged at 94.7% (4,025 of 4,249; none of these commands is in the 46 do-files).)
- P2C.12as arima (batch AR of the StatCraft port, arima done 2026-11-20; sspace, ucm and dfactor follow)
  (`arima depvar [indepvars], ar() ma() arima(p,d,q) mar() mma() sarima(P,0,Q,s) noconstant level() nolog trace gradient tolerance() ltolerance()
  nrtolerance() iterate()`: the exact likelihood by the Kalman filter, Stata's starting values (least squares; AR by least squares of the residuals;
  MA by Hannan and Rissanen's two regressions), its BHHH-then-BFGS path and `ml` stopping rule, OPG standard errors, e() with Stata's equation names, `_b[ARMA:L.ar]`;
  `predict` with `xb`, `y`, `residuals`, `yresiduals`, `mse`, `structural` and `dynamic(t)`) Verified (corpus `arima.do`, `arima_path.do`, `arima_more.do`; note `plan/stata/arima.md`).
  Agreement with Stata 19: tight tolerances to 3e-5 (OPG chi2 to 4e-7); every AR path step, `e(ic)` and stop to 1e-6; MA(1) and ARMA(1,1) default stops to 2e-4
  (Stata's line search past the full step is not decoded); `ar(1 2) ma(1)` and `ma(2)` reach the optimum, their paths are not Stata's.
  Still refused: time-series operators in the varlist, weights, `constraints()`, `vce()`, seasonal differencing, gaps or missing values inside the sample, collinear
  regressors, `predict, stdp` and `t0()`, forecasts beyond the data, `estat` and the other post-estimation commands.
  Measured on top of P2C.12aq (probes only; the probe folder is StatCraft's `observe/dofiles` at b7d392e): 80.5% -> 80.9% (23,343 -> 23,464 translated source lines of 29,011
  judged; 5,668 -> 5,547 not translated); packages unchanged at 94.7% (none uses arima).)
- P2C.12at sspace, ucm and dfactor (batch AR of the StatCraft port, part 2, done 2026-11-20)
  (`sspace (state eqs) (observation eqs), constraints()`: first-order state equations (`state noconstant`), observation equations with states, a constant and
  exogenous variables, `[eq]term = #` constraints; `ucm y [x], model(llevel | rwalk | lltrend)`; `dfactor (y1 y2 y3 = , noconstant) (f = , ar(1))` and
  `dfactor (y1 y2 = , noconstant ar(1))`; the exact Kalman likelihood (stationary start, or the exact diffuse start for a unit root, each diffuse step adding
  -ln(2 pi)/2), a BFGS-then-Newton search of RCraft's own, observed-information standard errors by Richardson second differences, the Wald test, e() and
  `_b[/state:var(u)]`, `_b[/:var(level)]` as Stata names them; `predict` with `xb`, `states, equation()`, `trend` and `factors`, `smethod(onestep|filter)`)
  Verified (corpus `sspace.do`, `ucm.do`, `dfactor.do`; note `plan/stata/sspace.md`). Agreement with Stata 19: estimates, standard errors, log likelihood, chi2
  and predictions to 3e-7 (corpus tolerance 1e-5); the p-value of a very large chi2 to 2e-6. Stata's iteration log and `e(ic)`, `e(tech_steps)`,
  `e(technique)`, `e(ilog)`, `e(gradient)`, the data signature are not reproduced.
  Still refused: constants or exogenous variables in state equations, lags beyond the first, `covstate()`, `covobserved()`, `from()`, `technique()`, `ucm` cycles,
  seasonals and other models, `dfactor` with constants, exogenous variables, more than one factor or `ar()` above 1, gaps in the sample, `estat` and the other
  post-estimation commands, `predict` with smoothing, `rmse()`, `dynamic()`; a variance at zero stops with r(430) (Stata prints it with no standard error).
  Measured on top of P2C.12as (probes only; the probe folder is StatCraft's `observe/dofiles` at origin/main): 80.9% -> 81.0% (23,477 -> 23,498 translated
  source lines of 29,011 judged; 5,534 -> 5,513 not translated; probes 549, 550 and 551 now translate in full); packages unchanged at 94.7% (4,025 of 4,249).)
- P2C.13 Collinear regressors as Stata omits them
  (done 2026-11-01:
  - `regress` sweeps the largest column left first, so the smallest of a collinear set goes.
  - `logit`, `probit` and `poisson` drop the later one in the order written.
  - Stata's 1e-13 tolerance, the `note:` lines, `0  (omitted)` rows, `o.` columns in `e(b)`, `_b[]` of 0, and `e(rank)`/`e(df_m)` counting only what is estimated.
  - `collinear.do` in the corpus.
  Two factors interacted followed in P2C.15. `nbreg`, `ologit`, `oprobit` and `mlogit` followed later the same day: the terms are found before the fit, and `mlogit` has `o.` in every equation. Still to come: the panel, survival and IV models.)
- P2C.14 Stata's date and time functions on R: `clock()`, `Clock()`,
  `monthly()`, `quarterly()`, `halfyearly()`, `weekly()`, `yearly()`, `yw()`
  and the others, with Stata's masks and missing values (StatCraft's decoding,
  checked against Stata 19)
  (done 2026-11-01 except the leap-second family (`Clock()`, `Cofd()`, ...):
  - the R helper `stata_dates` runs on columns;
  - 13,172 of 13,177 of Stata's recorded answers match bit for bit;
  - `dates.do` in the corpus.
  See `plan/stata/dates.md`.)
- P2C.15 Two factors interacted in a model (`i.g#i.h`, `i.g##i.h`): Stata's
  cells, base and empty ones, its table and `e(b)`
  (done 2026-11-01:
  - a bare variable in an interaction is a factor;
  - `i.g#i.h` alone leaves out only the all-base cell, as Stata does;
  - Stata's cell names in `e(b)`;
  - `regress`'s table with value labels, long labels and empty cells;
  - `matrix list` follows `set linesize`;
  - `interactions.do` in the corpus.
  See `plan/stata/interactions.md`.)
- P2C.16 `bsample` and `bootstrap` with Stata's own draws (StatCraft's
  decoding), with `cluster()`
  (done 2026-11-01:
  - `bsample` (plain, `#`, `if`/`in`, `cluster()`, `idcluster()`) draws Stata's rows in its order, checked on 280 of Stata 19's samples and in `bsample.do`;
  - the `bootstrap` prefix (`_b` after regress/logit/probit, `r()`/`e()` statistics, `reps()`, `seed()`, `nodots`, `cluster()`, `level()`) gives Stata's table and standard errors, checked in `bootstrap.do`;
  - `el()` of e()/r() matrices.
  `strata()` (in `bsample` too), `saving()` and `estat bootstrap` followed on 2026-11-05 (`bsopts.do`). Still to come: `bca`, `vce(bootstrap)`.)

## Phase 3: Data

- P3.1 Data Editor and Browser, virtualised, for millions of rows, backed by
  the RCraft columnar bridge from R (`rdata`, see `plan/data-bridge.md`)
- P3.2 **Use my file**: done for csv/tsv/txt, xlsx/xls (sheet and cell range),
  `.dta`, `.sav`/`.zsav`/`.por`, `.xpt`, `.sas7bdat`, `.rds` and `.RData`, with R's own
  preview and the generated R shown; packages `readxl`/`haven`/`arrow` are installed on
  request into RCraft's own library; parquet and feather; a Browse button; the ten most
  recent files remembered; a `.sas7bdat` file's format catalogue (`.sas7bcat`, its value
  labels) is found next to it or chosen (done 2026-11-13). P3.2 is done
- P3.3 Labels (shown in the grid and Variables pane with a conversion report; variable
  labels and value labels kept as labelled vectors; conversion report of what was kept,
  changed or dropped; Data menu dialogs to set a variable label, set value labels and turn
  a labelled variable into a factor)
- P3.4 Variables pane (type, label, missing count, factor level count, filter by type) and
  an Objects pane for the whole workspace, with a right-click menu to rename, remove and
  save (as .rds) any object. The Variables pane lists each factor's levels with their counts (done 2026-10-30)
- P3.5 Data management dialogs (done: the dialog-spec format, plan/dialog-spec.md; new
  variable, filter, sort and rename in base and tidyverse style; recode, convert type,
  factor levels (reference, combine, reorder), text (case and spaces, find and replace,
  extract) and dates (text to date, date part) in base R, each run against real R; a
  missing-package panel in the dialogs (the tidyverse style asks R for dplyr and offers to
  install it); CI installs dplyr and checks the tidyverse styles give the base styles'
  tables. Still to do: the per-factor level list in the Variables pane (P3.4))
- P3.6 Combine and reshape: join, append, pivot long/wide, aggregate, sample,
  duplicates and missing-value tools (done, each run against real R: the other data frame is
  chosen from a list, joins can match variables that have different names in the two data
  frames, and join, append, aggregate, duplicates, random sample and drop missing values have
  a tidyverse style that gives the base style's table. The two reshape dialogs stay base R,
  as the tidyverse way needs tidyr. P3.6 is done)
- P3.7 Save and export (the same formats), plus "save workspace" (done: File > Save data as
  text (comma, tab or semicolon), Excel, Stata, SPSS and SAS transport, R data (.rds, .RData)
  and Parquet or Feather, each read back in real R; nothing is overwritten unless Replace is
  ticked; a Browse button opens the system's save dialog; File > Save workspace. Not
  written: `.sas7bdat` and SPSS portable `.por`, which haven cannot write. P3.7 is done)
- P3.8 Frames-style workspace: several data frames open at once with a
  switcher (done 2026-10-30: tabs above the Data grid)

## Phase 4: Catalog and core statistics

- P4.0 Dialog-spec format and renderer: widgets, variable-type filters,
  defaults, validation, code templates in tidyverse and base styles; one spec
  gives the dialog and the code; snapshot tests of generated R
- P4.1 Descriptives: summary statistics, frequency tables, cross-tabs,
  correlation, normality, missing-data patterns (done in base R, each run against real R;
  a multi-variable widget was added to the dialog-spec format. The Q-Q plot came with Phase 5
  (Graphs > Normal Q-Q plot); on 2026-11-14 frequency tables, cross-tabs and correlations return
  their tables, so Results shows them as grids. P4.1 is done)
- P4.2 Tests and intervals: t-tests, proportions, nonparametrics, variance
  tests, exact tests, agreement measures (done in base R, each checked against R's own
  report: one-sample, two-sample and paired t-tests, test of proportions, exact binomial,
  Fisher's exact, Mann-Whitney, Wilcoxon signed-rank, Kruskal-Wallis, variance tests. Cohen's kappa, weighted or not, was added 2026-10-30;
  Fleiss' kappa and the intraclass correlations were added 2026-10-30)
- P4.3 ANOVA and linear models: `lm`, anova tables, contrasts, post-hoc (done in base R:
  linear model with categorical predictors, interactions, weights and confidence
  intervals; one-way and factorial ANOVA with Tukey comparisons; each saves its model
  under a chosen name. Type II and III sums of squares were added 2026-10-30, in base R, in the Objects pane's Model menu. Custom contrasts, for one factor of a linear model, were added 2026-10-30)
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
  together) and average marginal effects (marginaleffects package); the residual plots followed with Phase 5
  as Objects > Model > Diagnostic plots. P4.5 is done)

## Phase 5: Graphs

- P5.1 Graph window: capture R devices, history of plots, zoom (done: plots are captured as
  PNG snapshots from one live device that stays open between commands, so a later command can
  add to a plot; a Graph tab with older and newer, the command that drew each, fitted to the
  pane, and Save as PNG; zoom (Fit, smaller, 100%, larger, Ctrl and the wheel over the plot) and
  Copy, which puts the plot on the clipboard as a picture (done 2026-11-14). P5.1 is done)
- P5.2 ggplot2 builder: pick data, geoms and aesthetics; facets and themes
- P5.3 Plot menus: histogram, density, box, bar, scatter with fit, line,
  heatmap, QQ, diagnostics, survival curves (done in base graphics under Graphs: histogram,
  density, box plot, bar chart, normal Q-Q, scatter with a fit and smooth, line plot and a
  scatter plot matrix, each run against real R and required to draw exactly one picture; and a
  four-panel diagnostic plot for a fitted model in the Objects pane's Model menu. Survival curves
  came with P6.3 (Statistics > Survival > Kaplan-Meier survival curves). A heatmap (Graphs > Several variables > Heatmap: correlations, or each row's
  standardised values, optionally ordered by hclust) followed on 2026-11-14. P5.3 is done)
- P5.4 Export: SVG, PDF, PNG at chosen size and dpi (done: Graphs > Export plot, and an Export button
  on the Graph tab, draw a captured plot again on a PNG, SVG or PDF file of the size given in
  inches, centimetres or pixels, at a chosen resolution; R keeps the newest 30 plots of a session
  for it, an older one says so. P5.4 is done)

## Phase 6: Depth modules

Each family is its own set of dialogs over a named CRAN package.

- P6.1 Panel and fixed effects (`fixest`, `plm`)
  (first part done 2026-11-12: Statistics > Panel models, four dialogs: fixed-effects regression
  (`fixest::feols`, any number of absorbed fixed effects, conventional, robust, clustered or two-way
  clustered errors), a fixed-effects Poisson or logit model (`fepois`, `feglm`, with an exposure),
  the classic panel estimators (`plm::plm`: within, random, between, pooled, first differences,
  effects for units, periods or both, errors clustered by unit or period) and the Hausman test
  (`plm::phtest`); each checked in real R against the formulas written out by hand and against
  Stata 19 (`xtreg`, `areg`, `poisson`, `logit`, `regress D.`, `hausman`), Verified 2026-11-12;
  first differences go by time, clustered plm errors carry Stata's small-sample factor and the
  Poisson and logit errors Stata's conventions. Not yet: dynamic panels (GMM),
  instrumental variables in panels, unit-root and serial-correlation tests, and post-estimation
  from the Model menu on these models)
- P6.2 Multilevel models (`lme4`, `glmmTMB`) (first part done 2026-11-12: Statistics > Multilevel
  models, a linear mixed model (`lme4::lmer`: random intercepts, random slopes independent or
  correlated, a nested level, ML or REML) and a logistic or Poisson mixed model (`lme4::glmer`,
  a random intercept, 7-point adaptive quadrature, an exposure), with Stata's defaults and fitted
  to the maximum. Checked against the likelihoods written out by hand and Stata 19's `mixed`,
  `melogit` and `mepoisson`; Verified. Not yet: glmmTMB, random slopes in the logistic and Poisson
  models, crossed effects, the intraclass correlation and predictions of the random effects)
- P6.3 Survival (`survival`)
  (first part done 2026-11-12: Statistics > Survival, four dialogs: Kaplan-Meier survival curves
  (`survfit`, overall or by group, with the plot), the log-rank test (`survdiff`, log-rank or
  Peto-Peto), the Cox model (`coxph`: Efron, Breslow or exact ties, an entry time, strata, a cluster
  for robust errors, hazard ratios, and the test of proportional hazards with `cox.zph`) and
  parametric survival models (`survreg`: Weibull, exponential, log-normal, log-logistic, with time
  ratios); each checked in real R against the formulas written out by hand and against Stata 19
  (`sts`, `sts test`, `stcox`, `streg`), Verified 2026-11-12, with log-log intervals as Stata's.
  Not yet: competing risks,
  time-varying covariates set up from the data, survival curves from a fitted Cox model, and the
  Model menu on these models)
- P6.4 Survey data (`survey`) (first part done 2026-11-12: Statistics > Survey data, with a design
  of sampling weights, primary sampling units, strata and a finite-population correction; means
  and totals, overall or by group; proportions, two-way tables and the Rao-Scott test; linear,
  logistic and Poisson regression with odds or rate ratios. Each checked in real R against the
  [SVY] manual's linearization formulas written out by hand and against Stata 19's svy results,
  to 1e-9, with the R and survey versions recorded: the catalog's first Verified entries. Not yet:
  replicate weights, multi-stage designs, subpopulations, ratios, quantiles, and the Model menu on
  survey models)
- P6.5 Time series (`forecast`, `fable`) (first part done 2026-11-12, with base R's stats: Statistics >
  Time series > ARIMA model (`stats::arima` by exact maximum likelihood, AR and MA orders, 0 to 2
  differences with Stata's constant on the differenced series, level forecasts with errors from
  the psi weights) and Autocorrelations (AC, Yule-Walker and regression PAC, Ljung-Box Q, the
  correlograms). Checked against the exact likelihood and the formulas written out by hand and
  Stata 19's `arima`, `predict` and `corrgram`; Verified. Not yet: forecast and fable, seasonal
  models, unit-root tests, exponential smoothing, ARIMAX, VAR and the time-series plot)
- P6.6 Instrumental variables and selection models (first part done 2026-11-12: Statistics >
  Instrumental variables > Two-stage least squares (`fixest::feols` with instruments, optional
  fixed effects, conventional, robust or clustered errors, with or without Stata's small-sample
  statistics, the first stage, its F and robust Wald, Wu-Hausman and Sargan) and Statistics >
  Selection models > Heckman selection model (`sampleSelection::selection`, maximum likelihood
  fitted to Stata's precision or Heckman's two steps, the selection taken from a 0/1 variable or
  from the outcome being seen). Checked against the formulas by hand and Stata 19's `ivregress`,
  `estat`, `xtivreg, fe` and `heckman`; Verified. Not yet: LIML and GMM, IV probit and tobit,
  treatment-effects models, and the Model menu on these models)
- P6.7 Multivariate: PCA, factor analysis, clustering, discriminant
- P6.8 Resampling and permutation tests
- P6.9 Bayesian (`brms`, `rstanarm`)
- P6.10 Structural equation models (`lavaan`)
- P6.11 Meta-analysis (`metafor`)
- P6.12 Machine learning basics (`tidymodels`): split, fit, tune, evaluate

## Phase 7: Reporting and reproducibility

- P7.1 Publication tables (`modelsummary`/`gt`): side-by-side models, stars,
  chosen statistics, notes, custom rows
  (first part 2026-11-15: Model table, from a model's Model menu, in base R with no package: linear
  and generalised linear models side by side, each coefficient with stars and its standard error,
  statistic or p-value below, observations, R-squared, AIC, BIC and log-likelihood, a note line;
  a data frame, so Results shows it as a grid that copies into Word or Excel. Verified against the
  table built by hand. modelsummary and gt are installed for the export of P7.2. Added 2026-11-17:
  95% intervals in brackets, robust (HC1, HC3) standard errors, a blank header over the terms and
  "Also add it to the report". Not yet: custom rows, other model classes)
- P7.2 Export to docx, xlsx, HTML, LaTeX and Markdown; one document built up
  across a session
  (first part 2026-11-15: Export table, for any data frame in the Objects pane (a Model table, say):
  .html, .rtf (which Word opens), .tex through gt, .xlsx through openxlsx2, .md in base R, chosen by
  the file's ending, with a title and the Model table's note; .docx through flextable and officer,
  installed with the user's yes the same day; and the session's report, built up from the Reports
  menu (text, plots) and a data frame's menu (tables), saved as .docx or .html. P7.2 is done)
- P7.3 Script Editor: run selection, outline, find and replace
  (run selection was done with P2.4; 2026-11-15: File > Open script (Ctrl+O; a do-file opens in Stata
  mode), Save script (Ctrl+S) and Save script as, the file's name in the pane with a dot for changes
  not saved, and a question before they would be lost; find and replace (Ctrl+F, Ctrl+H; match case,
  whole word, wrapping) and an Outline of sections and functions. P7.3 is done)
- P7.4 Export the session as an R script, an R Markdown or Quarto document,
  or a Quarto project
  (done 2026-11-15: File > Save session as… writes the commands that ran without error as an R
  script, a Quarto document or an R Markdown document, one chunk a command with its menu entry above
  it; Save session as Quarto project… writes _quarto.yml and the document into a folder, never over
  files there. Each runs again in a fresh R to the same objects. P7.4 is done)
- P7.5 Project files: data, script, log and package snapshot together
  (renv-style lockfile)
  (done 2026-11-15: File > Save project… writes the script, R's objects (workspace.RData), the log and
  the package list (loaded packages, versions, base, CRAN or other) into a folder with `<folder>.rcraft`
  naming them; File > Open project… moves R there, opens the script, loads the objects and puts the
  package list in Packages > Project package list. A saved project opens in a fresh session to identical
  objects. P7.5 is done; with it Phase 7)

## Phase 8: Interface and packaging

- P8.1 Pane layout: Command, Results, History, Variables, Objects,
  Properties, Graph, Script
  (done 2026-11-15: Properties joined the others, under Variables: the data frame shown, or the
  variable picked, its label, type, class, values, missing, distinct values, range and mean, levels or
  value labels. P8.1 is done)
- P8.2 Installers: Windows MSI and portable zip (signed), macOS universal DMG
  (signed and notarised), Linux AppImage, deb, rpm, tar.gz
- P8.3 Example datasets that RCraft may redistribute, with attribution
  (done 2026-11-15, without redistributing anything: File > Example data… copies one of 20 data frames
  from R's own datasets package, which comes with every R, into the workspace and shows it; R's help
  page on each gives its source, and the descriptions are RCraft's own. P8.3 is done)
- P8.4 Help polish: see Phase 2A; here only the final review of every
  dialog's help page and the manual
  (dialogs' pages reviewed 2026-11-15: every page read through, the wrong or unclear statements
  fixed, a sentence added where an option needed one; the manual waits for RCraft's own manuals)
- P8.5 Browser build, if feasible
  (feasibility studied 2026-11-17, `plan/P8.5-browser-feasibility.md`: feasible as a reduced edition
  through webR, about two weeks of work, best done after P8.2; the shared crates build for wasm now.
  The build itself is not started)

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
