# Run my do-file: findings and plan

Written 2026-10-26 after looking at StatCraft's do-file runner and searching
for existing Stata-to-R tools. Feeds Phase 2C in `ROADMAP.md`.

## The request

A user uploads an existing Stata do-file into RCraft and runs it "as is", on
R, without a Stata licence.

## What exists already

**Other tools (from a web search; not tried, so treat as unverified):**
`translate.do` (Kranz, 2014, "preliminary", output needs hand edits),
`stata2r` and `regtranslate` (Kranz, version 0.1, data manipulation and
regression commands, `stata2r` said to be built with an AI agent),
`metasurvey`'s transpiler (survey recipes only), `RStata` (needs a Stata
licence: it calls Stata), and `ipz-stata2r` (side-by-side snippets, not a
translator). None is a mature, general do-file runner, so the gap is real.
Their licences were not checked. Under `AGENTS.md` we study what they do and
do not copy their code.

**Our own StatCraft** (`C:\pinokio\api\statcraft`) already runs do-files
natively: `crates/engine/src/script.rs` reads do-files (comments, `///`
continuations, `#delimit`), `syntax.rs` parses command syntax (varlists with
abbreviations, `if`, `in`, weights, `by:`), `crates/lang/src/expr.rs` parses
and evaluates expressions, and the engine has macros, loops, `capture`,
`program`. About 235 commands are catalogued, all `partial` and none yet
`Matches`, so it is a strong parser to reuse but not yet a verified
reference for results. It is our own MIT/Apache code, so reuse is allowed.

## What "as is" can honestly mean

1. **The file is not edited.** The user's `.do` goes in unchanged. Achievable.
2. **Every command runs.** Not achievable at first, and not for ado programs
   and Mata. The product promise is therefore: unchanged file in, a clear
   report of what ran, what was translated with caveats, and what was
   skipped, never a silent wrong answer.
3. **Same numbers as Stata.** Achievable command by command, verified against
   recorded Stata output, with documented exceptions (see risks).

## Approaches considered

| Approach | Runs on R | Readable R out | Faithful | Verdict |
|---|---|---|---|---|
| A. Static translation to idiomatic R | yes | best | weakest: dynamic Stata features (macros, abbreviations, `by` state, `preserve`) are hard to emit statically | good for the safe subset only |
| B. Translate to calls into an R runtime package (`st_gen()`, `st_replace()`, ...) that implements Stata's rules | yes | fair | strongest | best for fidelity |
| C. Run the do-file in StatCraft's engine, use R only for display | no | none | depends on StatCraft | not what was asked |
| D. Call a real Stata | no | none | exact | needs a licence; RStata already does this |

**Recommendation: A plus B.** Emit idiomatic R (dplyr, base) where Stata and
R agree; emit calls to a small RCraft-owned R package for the rules that
differ. The R package ships with RCraft under our licence and is installed
into the user's library on first use, with the user's yes (AGENTS.md
section 1). Reuse StatCraft's do-file reader, syntax and expression parsers
for the front end.

## Pipeline

1. **Upload**: file picker or drag and drop for `.do` (and a folder, for
   `include`/`do` of other files). Never uploaded anywhere: it is read
   locally.
2. **Scan (dry run)**: list every command used, how many lines translate
   cleanly, with caveats, or not at all, the packages needed, the data files
   and paths the script refers to, `ssc install` lines, `cd`, absolute paths
   and `log using` lines.
3. **Resolve**: ask where the data folder is; map Windows paths and `cd`;
   keep relative paths relative to the do-file.
4. **Translate**: parse to a command tree, then to R, keeping a line map.
5. **Run**: in the R session, line by line, with live progress, the Stata
   line beside the R it became, and a choice at the first problem: stop,
   skip, or continue.
6. **Report**: log, what ran, differences from Stata (linked to the guide),
   and **Save as R script** or Quarto.

## Main risks and how each is handled

- **Variable abbreviations** (`sum pri` for `price`) and wildcards: resolved
  at run time against the data in memory, in the runtime package.
- **Missing values**: `.` is larger than every number in comparisons, so
  `if x > 5` keeps missings. Emit explicit rules, not R's `NA` behaviour.
- **Storage types**: `generate` makes `float` by default. Matters for exact
  comparisons and merges; the runtime package tracks types.
- **Sort and `by`**: Stata's sort is not stable by default and `by` needs a
  sort; `_n`, `_N` and `[_n-1]` depend on the current order.
- **Standard errors**: `regress, robust` is HC1, cluster-robust uses Stata's
  small-sample factors; R packages default differently. Choose and document
  per command.
- **Random numbers**: `set seed` and `runiform()` will not reproduce
  Stata's draws unless the generator is implemented; bootstrap and
  simulation results will differ at first. Say so in the report.
- **Programs and ado**: user `program define` is translated to R functions
  where simple; SSC commands map to R packages where an equivalent exists
  (`esttab` to `modelsummary`, `reghdfe` to `fixest`) and are flagged
  otherwise. Mata is out of scope first.
- **Interactive commands** (`browse`, `pause`, `more`): ignored with a note.

## Proof it works

Reuse StatCraft's method. The maintainer has a licensed Stata (Stata 19 SE).
Probe do-files on made-up data are run in Stata to record results in a
private corpus (never committed). The same do-files are translated and run in
R and compared to about 8 significant digits, command by command. A command
becomes `Verified` only when it matches. StatCraft's 81 probe do-files in
`observe/dofiles/` are the first test inputs. To measure real-world reach,
keep a private set of public replication-package do-files (kept out of the
repo) and track the share of lines that translate.

## Order of work

1. Tier 1: data management scripts (generate, replace, keep, drop, sort,
   merge, append, collapse, reshape, labels, macros, loops).
2. Tier 2: core statistics (summarize, tabulate, regress, logit, probit,
   tests, predict).
3. Tier 3: panel, survival, survey, IV.
4. Tier 4: graphs.
5. Tier 5: programs, selected SSC commands.

## Open questions

- Share StatCraft's parser by publishing a shared crate from that repo, or
  vendor a copy here? A shared crate avoids drift.
- Where does the runtime R package live (a folder in this repo, installed
  from it)? It is R code, which `AGENTS.md` allows as a shipped helper.
- Which public replication packages to use for the private reach corpus.
