Guide {
    command: "ranksum",
    about: "Wilcoxon rank-sum (Mann-Whitney) test (ranksum x, by(g))",
    points: &[
        p(
            "In R",
            "stata_ranksum prints Stata 19's table of the two groups' observed and expected \
             rank sums (named by their value labels), the unadjusted variance, the ties' \
             adjustment and the adjusted variance, the H0 line, z and Prob > |z|, and the \
             exact p when there are 200 observations or fewer, worked out over every way of \
             sharing the midranks between the groups; r() as Stata's (N_1, N_2, z, Var_a, \
             sum_obs, sum_exp, p, p_l, p_u, p_exact, p_l_exact, p_u_exact).",
        ),
        p(
            "Not yet",
            "Options other than by() (porder, exact) and weights are refused.",
        ),
    ],
}
