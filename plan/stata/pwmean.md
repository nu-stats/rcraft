# pwmean (2026-11-18)

Observed on Stata 19 SE with made-up data (corpus `pwmean.do`, 102 results to 1e-8, `pwmean_tukey.do`, 15 results to
1e-7, and, from the second round of 2026-11-19, `pwmean_more.do`, 47 results to 1e-8, `pwmean_range.do`, 77 results to
1e-6, and `pwmean_dunnett.do`, 24 results to 1e-3). StatCraft's note (`constraints.md`, probe 530) had the formulas and the
main layout; the rest below was decoded here by running Stata. Translated in `crates/statatr/src/data_r/pwmean.rs`. The
last piece of batch C of the StatCraft port. The second round took the refusals of the first one away except two (the
order of the letters of `groups`, below, and the exact Dunnett numbers).
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
- **SNK, Duncan, Dunnett (second round).** A pair of means that spans r means in the sorted means (the sorted position
  difference plus one): SNK has the critical value q(1 - a; r, df)/sqrt(2) and the p-value P(Q_r > |t| sqrt(2)); Duncan
  uses a_r = 1 - (1 - a)^(r - 1) in place of a and the p-value 1 - (1 - P(Q_r > |t| sqrt(2)))^(1/(r - 1)); both agree with
  Stata's printed values to the digit (pairs of the same standard error or not). Dunnett compares each group with the
  first (m = G - 1 contrasts `jvs1`, so b_vs, V_vs and r(table_vs) have G - 1 columns and e(V_vs) is not diagonal), with
  the two-sided critical value and p-value of the maximum of G - 1 equicorrelated t variables **with correlation one half
  whatever the group sizes** (Stata's dunnettprob has no unequal-n form; its output with n = 1, 3, 5, 7, 9, 11, 4 matches
  rho = 1/2, not the true correlations). r(k_groups) does not exist after dunnett (also when it is ignored for two
  groups); `groups` with dunnett is r(198) ("only one of options dunnett or groups is allowed", printed after the heading,
  also with two groups). With one comparison all three are ignored with the note, as the older methods are.
- Abbreviations Stata takes: `mcomp`, `eff`, `cim`, `pv`, `cie`, `ci`, `group` (not `groups` shorter), `sort`, `l`;
  mcompare() `noadj`, `bon`, `sid`, `sch`, `tuk`, `snk` (no shorter), `dunc`, `dunn`; an empty `mcompare()` is noadjust.
  Options are case sensitive (`EFFECTS` is r(198)).
- `over()` takes `i.g`, `ib2.g` and `c.g` as the variable g, a variable twice (`over(g g)`) once, and refuses `g#w` with
  r(198) ("interactions may not be specified in the over() option"); the variable list takes no factor variables (`i.g`,
  `1.g`: r(101)).

## Tables

- Order when several are asked: groups, cimeans, pveffects, cieffects (and `ci`), effects; none asked means cieffects.
  `groups` and `cimeans` alone give only their table; a table of contrasts has Contrast, Std. err. and the interval
  (cieffects), plus t and P>|t| (effects, all columns), or only t and P>|t| (pveffects). The title of a table is the method
  (Unadjusted, Bonferroni, Sidak, Scheffe, Tukey, SNK, Duncan, Dunnett), always Unadjusted over the means of cimeans;
  effects repeats it over the p-value and over the interval.
- An adjusted method prints "Number of comparisons" before the `groups` table and before the first table of contrasts
  (pveffects, cieffects, effects: several in a row share one), not before cimeans (observed with all five together: the
  comparisons, groups, the unadjusted means, the comparisons again, the three contrast tables). The means of cimeans are
  always titled Unadjusted. Its label column is as wide as the over() names joined by `#` (plus one), at least 13.
- The label column is the longest label + 2, at least 13 and at least the over() names joined by # plus one; the
  dependent variable's name is cut with `~` to fit (Stata's abbreviation of the name, 12 characters at the least width).
- Level labels: a value label if there is one, else the value. With several variables a cell is `g#w` in the title, in a
  table of means `one#2` when any part has a label and `2 1` (a space) when none has; a contrast row is
  `(one#2) vs (one#1)`, the separator chosen by the left cell: `#` if it has a label, else a space, for both sides.
- `groups`: letters for the means that cannot be told apart at level(): every maximal set of means with no significant
  pair, of two or more (with groups of unequal size these need not be runs of neighbours: Duncan on groups of 1, 3, 5, 7, 9,
  11 and 4 rows gave A = means 1 to 6 and B = means 1, 5, 6 and 7); each set is a letter, the sets listed by their lowest
  means first (the sorted member lists in lexicographic order). A group in no set has no letter, a missing p-value
  (no or one residual degree of freedom for a range method) is no difference, so every mean gets A. r(k_groups) is the
  number of letters (0 when none; then no r(groups#) exists) and follows level() only when the `groups` table is shown,
  like the two matrices (else 95%). The column is as wide as the
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
  Tukey/tukey, SNK/snk, Duncan/duncan, Dunnett/dunnett); r(groups1) ... r(groupsG) only when there are letters (each padded
  with spaces to the number of letters, a group with none is all spaces); matrices table_vs (9 x m: b se t pvalue ll ul df
  crit eform), table (9 x G, the same for the means) and k_groups (1 x 1, row r1, column c1; none after dunnett).
- With several over() variables and unequal cells Stata's e(V) holds rounding noise (about 1e-16) where it is 0; RCraft
  holds 0.
- **No residual degrees of freedom** (every group of one row): se, t, p, limits and crit are missing everywhere (r(table),
  r(table_vs), the tables print `.`), df is 0, e(V) and e(V_vs) are 0 (not missing), e(df_r) is 0; the `groups` table
  prints the standard error as 0 (from e(V)) while cimeans prints `.`. **One residual degree of freedom**: Tukey, SNK,
  Duncan and Dunnett have no p-value, limits or crit (`.`; t is printed), the other methods are computed (r(table) at 1 df,
  crit 12.706 for 95%).
- **A bare `pwmean`** (new options allowed) shows the groups kept in e() again: it needs no data (it also works after
  `clear`), uses defaults for everything it is not given (a level(90) or mcompare(bon) of the run is forgotten), leaves
  e() as it was (e(cmdline) too) and sets r() again. After another estimation command it is r(301); with an if or in
  r(102), weights r(101), over() r(198) ("option over() not allowed"), a variable and no over() r(198). RCraft keeps the
  groups in an attribute of e() (`stata_e`).

## Errors (Stata's return codes)

No over() or `over()` empty or an unknown option or method: 198; weights: 101; more than one variable: 103; a factor
variable in the variable list (`z i.g`, `1.g`): 101; `over(g#w)`: 198; a string variable or a string over(): 109; a name
the data lack: 111; an over() value negative or not whole (including a continuous variable): 452; no rows: 2000; one
group: 322; `by:` 190; level() outside 10 to 99.99: 198; dunnett with groups: 198. **An option twice: 198, for every
option (`ci` and `cieffects` are one option, so are `effects` and `eff`); a second level() is r(198) too unless it says
exactly 95, and then the first counts** (`level(90) level(95)` is 90, `level(95) level(90)` and `level(90) level(90)` are
r(198), `level(95) level(95)` and `level(95.5) level(95)` are fine).

## Precision

- Everything but the range methods agrees to 1e-8 (the corpus compares at 1e-8). Tukey, SNK and Duncan use R's `ptukey`
  and a root of it for the critical value. Stata's `tukeyprob()` is about 1e-10 from the exact integral and R's is about
  4e-12, so the two differ by about 1e-10 in a p-value (3e-9 in the critical value, 7e-9 in an interval limit); the
  Tukey corpus is checked at 1e-7 and keeps to p-values above 0.003 (a tiny p-value is compared relatively and the 1e-10
  shows). SNK and Duncan on spans of five means or more differ in the critical value by about 3e-8 relative, which a
  limit near zero shows as 1e-6: `pwmean_range.do` is compared at 1e-6.
- **Dunnett is only approximate**, on Stata's side: R's nested integral (rel.tol 1e-13) of the equicorrelated maximum is
  exact, and Stata's critical value is off from it by 1.5e-6 (3 comparisons, 36 df) to 6e-6 (6 comparisons, 33 df) and
  its p-values by 1e-6 to 2.2e-4 absolute (p(5 vs 1) .786480 against the exact .786264), with no pattern in the sign; so
  a printed Dunnett interval can differ in the fifth digit and a p-value in the third. The Dunnett corpus is compared at
  1e-3. Finding Stata's quadrature (it probably is a fixed-order Gauss rule) would close this.
- `invtukeyprob()` of Stata is missing for 1 degree of freedom, as are the other range quantiles: Stata prints missing
  and so does RCraft.

## Open / refused

- **The order of the letters of `groups`** when the sets overlap. The sets of means that share a letter are right in
  all cases tried, but Stata deals the letters in an order that is not the lowest set first in 24 of the 134 designs
  with letters. Looked at again 2026-11-19: in every one of the 24 the sets are {2..k}-type (only the lowest mean differs
  from the others) and Stata puts the large set first; but the same family with k = 4 and sets {1,2}, {2,3,4} comes out
  lowest first, a size order breaks 10 of the cases that are lowest first, and an insert-and-absorb build or an
  incremental clique build in the original group order or the sorted order (24 variants) and the earlier 3,000 variants
  do not reproduce all of it. So `r(groups#)` is not kept (a line reading it is refused) and `groups`
  carries a caveat. `r(k_groups)` and the shape of the table are right.
- Two means that are equal to the last bit: SNK and Duncan span them in the order Stata's own rounding gives (a
  difference of -1.14e-15 printed for two means of 5), which cannot be matched.
- Dunnett's numbers to Stata's digits (above).
- `: colnames` and `: rownames` macro functions are not translated yet, so the column names with bn are checked by hand
  against Stata, not in the corpus.
