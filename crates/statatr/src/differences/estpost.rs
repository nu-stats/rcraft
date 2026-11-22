Guide {
    command: "estpost",
    about: "Posts summary statistics for esttab (estpost summarize, the SSC estout package)",
    points: &[
        p(
            "In R",
            "stata_estpost_summarize posts e(count), e(sum_w), e(mean), e(Var), e(sd), e(min), \
             e(max) and e(sum) for each variable, e(N) (the rows where any is nonmissing) and \
             e(k), and prints its table as Stata 19 does, split to the linesize.",
        ),
        p(
            "Not yet",
            "Its other subcommands (tabulate, correlate, ttest, ...), weights, options and \
             string variables are refused.",
        ),
    ],
}
