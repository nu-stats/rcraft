# pwmean (2026-11-18)

Observed on Stata 19 SE with made-up data (corpus `pwmean.do`, 102 results to 1e-8, and `pwmean_tukey.do`, 15 results to
1e-7). StatCraft's note (`constraints.md`, probe 530) had the formulas and the main layout; the rest below was decoded here
by running Stata. Translated in `crates/statatr/src/data_r/pwmean.rs`. The last piece of batch C of the StatCraft port.
The printed tables of every case below were compared line by line with Stata's log (probes of our own, 70 tables): all
identical except where a value lands on the other side of a rounding in the seventh digit (Tukey, below) and where a
`label define ..., add` after `label values` is not followed (a gap of the label model, not of pwmean).

## What it computes

- `pwmean y [if] [in], over(vars) [mcompare(method) effects pveffects cieffects ci cimeans groups sort level(#)]`.
  Groups are the cells of the over() variables, in the order of their values, those with a value of y and of every over()
  variable. N - G degrees of freedom, s2 the pooled within-group variance, each pair `j vs i` for i < j (ordered by i, then
  j) with the difference of the means and s * sqrt(1/n_i + 1/n_j).
- Methods (critical value, p): noadjust t(a/2) and 2P(t); bonferroni a/(2m) and min(m p, 1); sidak a' = 1 - (1 - a)^(1/m)
  and 1 - (1 - p)^m; scheffe sqrt((G-1) F(a; G-1, df)) and the F tail of t^2/(G-1); tukey q(1-a; G, df)/sqrt(2) and
  P(Q > |t| sqrt(2)); m = G(G-1)/2. With one comparison (G = 2) the method is ignored: `note: option tukey ignored since
  there is only one comparison` before the table, and r(mcmethod_vs) is noadjust.
- Abbreviations Stata takes: `mcomp`, `eff`, `cim`, `pv`, `cie`, `ci`, `group` (not `groups` shorter), `sort`, `l`;
  mcompare() `noadj`, `bon`, `sid`, `sch`, `tuk`. Options are case sensitive (`EFFECTS` is r(198)).

## Tables

- Order when several are asked: groups, cimeans, pveffects, cieffects (and `ci`), effects; none asked means cieffects.
  `groups` and `cimeans` alone give only their table; a table of contrasts has Contrast, Std. err. and the interval
  (cieffects), plus t and P>|t| (effects, all columns), or only t and P>|t| (pveffects). The title of a table is the method
  (Unadjusted, Bonferroni, Sidak, Scheffe, Tukey), always Unadjusted over the means of cimeans; effects repeats it over the
  p-value and over the interval.
- An adjusted method prints "Number of comparisons" before each table that uses it (groups, pveffects, cieffects, effects),
  not before cimeans. Its label column is as wide as the over() names joined by `#` (plus one), at least 13.
- The label column is the longest label + 2, at least 13 and at least the over() names joined by # plus one; the
  dependent variable's name is cut with `~` to fit (Stata's abbreviation of the name, 12 characters at the least width).
- Level labels: a value label if there is one, else the value. With several variables a cell is `g#w` in the title, in a
  table of means `one#2` when any part has a label and `2 1` (a space) when none has; a contrast row is
  `(one#2) vs (one#1)`, the separator chosen by the left cell: `#` if it has a label, else a space, for both sides.
- `groups`: letters for the means that cannot be told apart at level(): sort the means, take every run of neighbours with no
  significant pair, of two or more; each run is a letter, the run of the lowest means first. A group in no run has no
  letter. r(k_groups) is the number of letters (0 when none; then no r(groups#) exists). The column is as wide as the
  title + 3 (at least 10), the letters right-aligned (a missing letter is a space). The note under the table is wrapped at
  the table's width, six spaces on the following lines.
- `sort` lists the contrasts by the difference, the means by their value, ascending. Only the display moves: e() and r()
  keep the original order, and r(groups#) too.
- Cells are `%9.0g` (`.` for a missing value), t `%9.2f`, p `%8.3f`.
- level(): 10 to 99.99 (r(198) outside). **r(table) holds the confidence limits at level() only when `cimeans` is
  shown, r(table_vs) only when a table of the contrasts with an interval or p is shown (cieffects, effects, pveffects and
  the default); otherwise they are the 95% ones.** r(level) is always level().

## e() and r()

- e(): scalars balanced (all groups of one size) and df_r; macros cmdline, cmd, title, depvar, over, properties (`b V`);
  matrices b (1 x G, row y1), V (diagonal s2/n), error (1 x G, `c1 ...`, row r1), b_vs (1 x m, row r1), V_vs, error_vs. No
  e(sample).
- Column names are factor-variable style: `1bn.g 2.g 3.g`, `1bn.g#1bn.w 1bn.g#2.w ...` (the base level, written bn, is each
  variable's lowest level) and for the contrasts `2vs1bn.g 3vs1.g 3vs2.g` (every term equal to the first column's carries
  bn: `1vs1bn.w#2vs1bn.g`, `1vs1bn.w#3vs1.g`, `2vs1.w#2vs1bn.g`). Level values print as they are (`200000vs100000bn.gl`).
  `matrix list` shows them without the bn.
- r(): level; mctitle_vs, mcmethod_vs (Unadjusted/noadjust, Bonferroni/bonferroni, Sidak/sidak, Scheffe/scheffe,
  Tukey/tukey); r(groups1) ... r(groupsG) only when there are letters (each padded with spaces to the number of letters,
  a group with none is all spaces); matrices table_vs (9 x m: b se t pvalue ll ul df crit eform), table (9 x G, the same for
  the means) and k_groups (1 x 1, row r1, column c1).
- With several over() variables and unequal cells Stata's e(V) holds rounding noise (about 1e-16) where it is 0; RCraft
  holds 0.

## Errors (Stata's return codes)

No over() or `over()` empty or an unknown option or method: 198; weights: 101; more than one variable (or `i.g`): 103 and
101; a string variable or a string over(): 109; a name the data lack: 111; an over() value negative or not whole
(including a continuous variable): 452; no rows: 2000; one group: 322; `by:` 190; level() outside 10 to 99.99: 198; an
option twice: 198 (a repeated level() takes the last).

## Precision

- Everything but Tukey agrees to 1e-8 (the corpus compares at 1e-8). Tukey uses R's `ptukey` and a root of it for the critical
  value. Stata's `tukeyprob()` is about 1e-10 from the exact integral and R's is about 4e-12, so the two differ by about
  1e-10 in a p-value (3e-9 in the critical value, 7e-9 in an interval limit); the Tukey corpus is checked at 1e-7 and keeps
  to p-values above 0.003 (a tiny p-value is compared relatively and the 1e-10 shows).
- `invtukeyprob()` of Stata is missing for 1 degree of freedom; RCraft stops there (below).

## Open / refused

- `mcompare(snk)` and `mcompare(dunnett)` (Stata has them; not decoded). A bare `pwmean` (it shows the last results again),
  `over(i.g)`, a variable twice in over(), an empty `mcompare()`, an option given twice, abbreviations the notes above do not list.
- An adjusted method with `cimeans` and another table together (the order of the comparison tables is not observed).
- Residual degrees of freedom 0 (every group of one row, Stata prints missing standard errors) and, for Tukey, 1: R stops with a
  message instead of printing Stata's tables.
- **The order of the letters of `groups`.** The sets of means that share a letter are right in all 134 cases with letters tried (of 380 designs), but when
  the runs overlap Stata deals the letters in an order that is not the lowest run first in 24 of those 134 (for
  example runs {1,2}, {2,3,4}, {4,5} of the sorted means come out as B, A, C). Not found by an insert-and-absorb
  search, a greedy clique build or a size order over about 3,000 variants; the two orders agree whenever there is one run or the
  runs chain with no run inside two others. So `r(groups#)` is not kept (a line reading it is refused) and `groups`
  carries a caveat. `r(k_groups)` and the shape of the table are right.
- `: colnames` and `: rownames` macro functions are not translated yet, so the column names with bn are checked by hand
  against Stata, not in the corpus.
