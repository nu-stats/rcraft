Guide {
    command: "tabulate",
    about: "One-way and two-way tables (and tab1, tab2)",
    points: &[
        p(
            "Missing values",
            "Left out unless missing is given, as in Stata; R's table() leaves them out \
             too unless useNA says otherwise.",
        ),
        p(
            "Order",
            "Categories in the order of the values underneath, with their labels, as \
             Stata's.",
        ),
        p(
            "generate()",
            "One indicator per value, labelled as Stata labels them (cat3== followed by the \
             number in 11 characters, 1.0000 padded on the left; s==a; f==one); rows the \
             table leaves out are missing, and with missing a missing value has an \
             indicator of its own.",
        ),
        p(
            "chi2",
            "Pearson's chi-squared with no continuity correction, as Stata's; R's \
             chisq.test() corrects 2 x 2 tables unless told not to.",
        ),
        p(
            "Printing",
            "Stata's own tables for counts, and for two-way tables with row, column and \
             cell percentages (with nofreq too: the Key box, each cell's lines, the rules \
             between rows), checked character for character; analytic weights and string \
             variables keep R's layout. nofreq alone prints nothing and leaves r() empty, as \
             in Stata 19; with chi2 it prints the chi2 line only and leaves r() whole.",
        ),
        p(
            "More options",
            "lrchi2 (2 sum O ln(O/E), its line after Pearson's, r(chi2_lr) and r(p_lr); not \
             with analytic weights yet), nokey, and matcell(), matrow() and matcol(): the \
             counts as the table has them (analytic weights scaled to the rows) and the \
             values of the rows and columns, r1 r2 ... and c1 c2 ..., as Stata 19 leaves \
             them; matrow() of a string variable is Stata's error.",
        ),
        p(
            "summarize()",
            "Stata 19's tables of the summarized variable's mean, standard deviation, Freq. \
             and (with weights or obs) Obs: one way oneway's summary table, two ways a line \
             for each statistic in each cell (%10.0g, . for an empty cell, the title naming \
             the statistics shown, columns in blocks of five); means, standard, freq, obs and \
             their no forms choose the lines, with frequency and analytic weights as oneway's. \
             r() is left empty, as in Stata.",
        ),
    ],
}
