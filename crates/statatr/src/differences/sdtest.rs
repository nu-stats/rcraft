Guide {
    command: "sdtest",
    about: "Tests of standard deviations (sdtest y == 1, sdtest y == x, sdtest y, by(g))",
    points: &[
        p(
            "In R",
            "stata_sdtest prints Stata 19's table and tests: one standard deviation against a \
             value by chi2 = (n - 1)s^2/sd0^2, two (two variables, each over its own rows, or \
             the two groups of by(), named by their value labels) by the F ratio of their \
             variances, with the Combined row of every value; the one-sided and two-sided p, \
             and r() as Stata's (N, df or df_1 and df_2, chi2 or F, sd, sd_1, sd_2, p_l, p_u, p).",
        ),
        p(
            "Not yet",
            "Options other than by() and level() are refused; weights are Stata's own error.",
        ),
    ],
}
