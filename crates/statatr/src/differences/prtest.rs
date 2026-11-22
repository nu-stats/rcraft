Guide {
    command: "prtest",
    about: "Two-sample test of proportions (prtest var, by(g))",
    points: &[
        p(
            "In R",
            "stata_prtest prints Stata 19's table: each group's proportion with its standard \
             error sqrt(p(1 - p)/n) and normal interval, the difference with sqrt(se1^2 + \
             se2^2), the z test under H0 with the pooled proportion and its three p-values; \
             the groups in sorted order, named by their value labels cut at 12 characters; \
             r() as Stata's (N1 ... p_u, level). Other than two groups is r(450), a mean \
             outside [0,1] r(198).",
        ),
        p(
            "Not yet",
            "The one-sample form (prtest x == #), two variables (prtest x == y) and prtesti \
             are refused.",
        ),
    ],
}
