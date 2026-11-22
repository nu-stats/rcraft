Guide {
    command: "cc",
    about: "Odds table of a case-control study (cc case exposed, woolf)",
    points: &[
        p(
            "In R",
            "With woolf or tb: stata_epi prints Stata 19's table of cases and controls with \
             the proportion exposed, the odds ratio with Woolf's interval or the test-based \
             one (on the Mantel-Haenszel chi2, (N - 1)/N Pearson's, as Stata 19 takes it), \
             the attributable or prevented fractions and Pearson's chi2; r() as Stata's. \
             by() (with woolf or tb): each stratum's odds ratio and Mantel-Haenszel weight, \
             the crude and the Mantel-Haenszel combined ratio (Robins-Breslow-Greenland's \
             interval, or test-based on the Mantel-Haenszel chi2), the test of homogeneity \
             and the Mantel-Haenszel test.",
        ),
        p(
            "Not yet",
            "cc without woolf or tb: its default exact interval of the odds ratio comes from \
             Stata's own search, which stops short of the exact bound (by about 1e-4), so it \
             is refused (with by() too), as are cornfield, exact, bd, tarone and weights.",
        ),
    ],
}
