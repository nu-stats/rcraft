Guide {
    command: "bitest",
    about: "Exact binomial test of a 0/1 variable",
    points: &[
        p(
            "As Stata 19",
            "bitest var == #p counts the successes (frequency weights count rows several \
             times) and prints the one-sided tails Pr(k >= observed) and Pr(k <= observed), \
             and the two-sided probability: the tail on k's side plus the opposite tail from \
             the first outcome, past the expected count, that is no more likely than k \
             (equal probabilities count). When k is the expected count the two-sided p is 1; \
             when no outcome on the other side is as unlikely the other tail is empty, \
             r(k_opp) is missing and a note says so. r() holds k_opp, p, k, P_p, N, p_l and \
             p_u. The probabilities are exact (pbinom and dbinom) and agree to 9 digits.",
        ),
        p(
            "Errors",
            "Not a 0/1 variable, or #p outside (0,1), r(450); weights other than fweights \
             r(101); no rows r(2000). The messages were read off Stata's log by eye.",
        ),
        p("Not yet", "The detail option, by:, and a #p that is not a number written out."),
    ],
}
