Guide {
    command: "cs",
    about: "Risk table of a cohort study (cs case exposed, csi 20 15 80 120)",
    points: &[
        p(
            "In R",
            "stata_epi prints Stata 19's table: cases and noncases of the exposed and the \
             unexposed (a nonzero value counts as one; the exposure's label heads the \
             columns), the risks, the risk difference with its Wald interval, the risk ratio \
             with its log interval, the attributable fraction among the exposed and in the \
             population (prevented fractions when the ratio is below 1), Pearson's chi2, or \
             with exact Fisher's one- and two-sided p; r() as Stata's.",
        ),
        p(
            "Strata",
            "stata_epi_by prints each stratum's risk ratio with its log interval and \
             Mantel-Haenszel weight, the crude ratio, the Mantel-Haenszel combined one with \
             Greenland and Robins's interval and the test of homogeneity (worked out in float \
             precision, as Stata keeps it); istandard the internally standardized ratio (the \
             exposed as the standard); or woolf the odds ratios with Woolf's intervals, the \
             Robins-Breslow-Greenland interval of the combined one and the Mantel-Haenszel \
             test. Strata are named by their value labels.",
        ),
        p(
            "Not yet",
            "or (its Cornfield interval comes from Stata's own search, which R's root of \
             Cornfield's equation matches only to about 5 digits; with by() or woolf is \
             translated), rd, binomial(), estandard, ustandard(), weights and the other \
             options are refused.",
        ),
    ],
}
