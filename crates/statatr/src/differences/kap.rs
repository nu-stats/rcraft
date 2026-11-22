Guide {
    command: "kap",
    about: "Interrater agreement (kap rater1 rater2, kap r1 r2 r3, wgt(w), wgt(w2), tab, absolute)",
    points: &[
        p(
            "In R",
            "stata_kap gives Cohen's kappa for two raters over the ratings either used, \
             unweighted or weighted by w (1 - |i-j|/(k-1)) or w2 (1 - ((i-j)/(k-1))^2), on \
             the ratings' ranks or with absolute their values, with frequency weights, and its \
             standard error under no agreement beyond chance; for three or more raters, \
             Fleiss's kappa for each outcome and combined with their z tests. Stata 19's \
             tables (tab's two-way table of the ratings' values) and r(); Stata's errors.",
        ),
        p(
            "Not yet",
            "kapwgt and its weights, kappa (the frequency form), raters per subject that vary \
             (missing ratings with three or more raters), a chance agreement that leaves no \
             test (Stata's dots), tab with more than five ratings, and weights with three or \
             more raters are not translated. r() can differ from Stata's in the 16th digit.",
        ),
    ],
}
