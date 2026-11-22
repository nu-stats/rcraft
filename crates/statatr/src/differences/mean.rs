Guide {
    command: "mean",
    about: "Means with their standard errors (mean y x, over(g))",
    points: &[
        p(
            "In R",
            "stata_meanest estimates as Stata 19 does and prints its table: the rows with \
             every variable (and over()'s) present, each mean's standard error s/sqrt(n) \
             with frequency weights counting rows and analytic ones scaled to the rows (in \
             each group of over()), and with probability weights the linearized (sandwich) \
             variance n/(n-1) sum z z', each group of over() on its own. e(b) and e(V) are \
             named as Stata names them (y, c.y@0bn.b), with e(N), e(df_r), e(sd), e(_N) and \
             r(table); test and lincom read them, their lines naming the levels as Stata's \
             do (c.y@0bn.b - c.y@1.b = 0).",
        ),
        p(
            "Not yet",
            "vce() and the display options are refused. Several variables in over() make \
             cells of their values (c.y@1bn.g#0bn.b), iweights count as frequency weights, as \
             Stata 19's, and _b[] and _se[] read over()'s names (_b[c.y@0.b], with or without \
             bn).",
        ),
    ],
}
