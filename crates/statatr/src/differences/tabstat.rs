Guide {
    command: "tabstat",
    about: "Table of summary statistics (tabstat varlist, statistics() by())",
    points: &[
        p(
            "In R",
            "stata_tabstat computes summarize's statistics (Stata's percentile rule, its \
             frequency and analytic weights, skewness and kurtosis over the whole sample) and \
             prints Stata 19's table: 10-wide columns in %9.0g or format(), the statistics' or \
             variables' stub, the groups by their value labels cut to labelwidth(), rules \
             between groups of more than one row, the table cut into blocks at the line size. \
             Rows with a missing group are left out of the groups and the total unless \
             missing; each variable uses its own rows unless casewise. save leaves r() as \
             Stata's: StatTotal, Stat1, ... (statistics by variables) and name1, ...",
        ),
        p(
            "Not quite",
            "A string group's stub is as wide as its longest value; Stata's is the string's \
             storage width (str#), wider when the variable was made wider than its values.",
        ),
        p(
            "Not yet",
            "format alone takes each variable's own display format: one set by format, read \
             with the data from a .dta, or the default of the storage type generate made \
             (float %9.0g, double %10.0g, long %12.0g); a variable whose format is none of \
             these stops with that reason. Formats wider than 30, widths above 32 \
             and patterns in the varlist that the data's names do not settle are refused; \
             an unknown statistic, a string variable and pweights or iweights are Stata's \
             errors (r(198), r(109), r(101)).",
        ),
    ],
}
