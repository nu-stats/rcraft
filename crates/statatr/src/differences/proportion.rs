Guide {
    command: "proportion",
    about: "Proportions of each value with logit intervals (proportion g, over(h))",
    points: &[p(
        "In R",
        "stata_meanest gives each value's share of the rows (every variable present), its \
         variance (diag(p) - pp')/n, and Stata 19's logit interval; with probability weights \
         the linearized variance; over() each group on its own. e(b) is named 1bn.g 2.g ... \
         (0bn.b@1bn.g with over()), with e(freq) and e(_N), and lincom's lines name the \
         levels so (1bn.g - 2.g = 0). prop is proportion, as in Stata (propo is not); \
         analytic weights are Stata's error (aweights not allowed), and iweights count as \
         frequency weights.",
    )],
}
