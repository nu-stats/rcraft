# anovaplot (Nick Cox, Stata Journal gr0009_2, SJ23-1), 2026-11-10

Installed that day with the user's yes (`net install gr0009_2, from(http://www.stata-journal.com/software/sj23-1)`;
not on SSC, whose `modeldiag` is an old version); observed on Stata 19 SE with made-up data, its graphs
exported as SVG and PNG (probes `anovap/a1.do`, `a3.do`, `a4.do` in the session scratchpad); its code and
help were not read.

- `anova y g x g##x` then `anovaplot x g`: the model's fitted values (predict's xb), joined along x for
  each value of g, a marker at each cell: after the full factorial the cell means (1.917574 ... 2.874856),
  after `anova y g x` the additive fit (2.251242 ...), as the SVG's line coordinates show at the axis's
  scale (355 units a point).
- Only the model's rows: after `anova ... if cond != 3` no `cond` 3 column; after `if z == 1` the lines run
  through the sample's rows only.
- The data as points; `scatter(ms(i))` hides them, but the y axis still spans them (0 to 4 for cell
  means near 2).
- x's values as its ticks with x's value labels (Control, Gov, Opp); the x title x's variable label, else
  its name; the y title y's label (`Opinion`), or `ytitle()`; `title()`, `xtitle()`; the legend g's value
  labels, else its values (0, 1); "Profiles by" and g's variable label, else its name ("Profiles by
  Binary vote intention" in ASKO93), as a subtitle above the plot.
- r() is cleared; the data is left as it was (no new variables).
