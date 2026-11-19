# graph combine: l1() and l2() (2026-11-19)

Observed on Stata 19.0 SE with made-up data, for BraderTuckerDuell's
`gr combine plot0 plot1 plot2 plot3, graphr(...) ycommon xcommon imargin(zero) rows(1) l2(Mean of level of support, c(black) si(medium))`
(5 package lines).

- **Rule:** `graph combine` takes `l1()`, `l1t()`, `l1ti()`, `l1title()` and the same for `l2`, `r1`, `r2`, `b1`, `b2`,
  `t1`, `t2` (all ran with rc 0). `l1title()` and `l2title()` are titles on the left of the combined graph. The text
  comes before the first comma outside quotes; what follows is the text's look (`c(black)`, `si(medium)`).
- **RCraft:** `l1()` and `l2()` (from 2 letters on) become upright texts in columns on the left of the grid, l1 next
  to the graphs and l2 outside it; their look options are not drawn. `title()` of `graph combine` also takes only the
  text before its comma now (it kept `, si(large)` as part of the text). The layout is R's (ggplot2 and gtable), as the
  caveat says; nothing of a graph is compared with Stata's numbers here. Test: `engine/tests/stata_graphs.rs`
  (`graph_combine_puts_left_titles_beside_the_graphs`).
- **Open:** `r1()`, `r2()`, `b1()`, `b2()`, `t1()`, `t2()` on `graph combine` are still refused (no package uses them);
  `twoway` titles still keep the text after a comma (`ti(Full, c(black) si(large))`), a follow-up.
