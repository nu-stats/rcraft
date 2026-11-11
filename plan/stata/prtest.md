# prtest var, by(g) and graph bar (asis) (2026-11-10)

Observed on Stata 19 SE with made-up data (probes `prt/p1.do`, `p2.do`, `gb/a1.do` in the session
scratchpad) and AXYMDI's own Stata log; corpus `prtest.do`.

## prtest var [if] [in], by(g) [level(#)]

- Each group: the mean p, its standard error sqrt(p(1 - p)/n), the normal interval p ± z·se. The
  difference p1 - p2 with sqrt(se1² + se2²) and its interval; under H0 the pooled proportion's standard
  error sqrt(pp(1 - pp)(1/n1 + 1/n2)), z, and Pr(Z < z), Pr(|Z| > |z|), Pr(Z > z).
- The table: "Two-sample test of proportions" with each group's "Number of obs" (the label right-aligned
  to column 52, the count %8.0f, no commas at 14980), the 78-wide rules, values in %9.0g, z %8.2f, P
  %7.3f, "diff = prop(a) - prop(b)" and "z = " %8.4f at the right edge, the three Ha lines in %6.4f.
- The groups in sorted order (numbers ascending, strings in byte order), named by their value labels,
  else their values, cut at 12 characters everywhere ("A rather lon").
- r(): N1 N2 P1 P2 P_diff se1 se2 se_diff0 se_diff lb1 ub1 lb2 ub2 lb_diff ub_diff z p_l p p_u level.
- `by()` with other than two values: "h takes on 1 values, not 2", r(450). A group's mean outside [0,1]
  (the variable need not be 0/1): "1.302269692923899 not in [0,1]", r(198).
- AXYMDI's two prtest tables come out as Stata's log has them.

## graph bar (asis) vars, over(g) stack legend(order(...)) b1title()

- `(asis)`: each group's one value as it is; a group with more than one row is "variables g and
  _variables do not uniquely identify the observations", r(459). The series are named by the variables'
  labels, else their names.
- `stack`: the first variable at the bottom (RCraft had stacked it on top).
- `legend(order(3 "Unreported" 2 "..." 1 "..."))`: those series, in that order, with those texts.
- `b1title(Congress)`: the title below the x axis.
