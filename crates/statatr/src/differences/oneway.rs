Guide {
    command: "oneway",
    about: "One-way analysis of variance (oneway y g, tabulate)",
    points: &[
        p(
            "In R",
            "stata_oneway prints Stata 19's tables: the groups' means, standard deviations and \
             counts with tabulate (labels cut at 9), the analysis of variance (between and \
             within groups, F and its p-value), and Bartlett's test of equal variances on the \
             groups with a positive variance, with Stata's note when single-observation or \
             equal-valued groups are left out; the Bonferroni, Scheffe and Sidak comparisons \
             of each pair of means; frequency and analytic weights (with analytic weights \
             Bartlett's test counts each group's rows, as Stata 19 does). nomeans, nostandard \
             and nofreq drop the summary table's columns, noobs its Obs column; missing makes \
             the rows with a missing group a group of their own (.), and wrap changes nothing \
             here. r() is Stata's: N, mss, df_m, rss, df_r, F, chi2bart, df_bart and the \
             matrix r(ANOVA).",
        ),
        p(
            "Not yet",
            "The comparisons of more groups than the line holds are refused. With weights and tabulate the summary table gains Stata's Obs column \
             (analytic weights: the weight sums as Freq., the rows as Obs, summarize's standard \
             deviation).",
        ),
    ],
}
