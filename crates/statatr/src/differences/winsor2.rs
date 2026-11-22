Guide {
    command: "winsor2",
    about: "Winsorizes or trims variables at percentiles (winsor2, SSC)",
    points: &[
        p(
            "As Stata 19",
            "cuts(# #) at Stata's own percentile (the one _pctile gives) among the nonmissing \
             rows of if and in, by group with by() (a missing group is a group); a 0 or a 100 \
             leaves that end alone; winsor (the default) sets the values beyond the cuts to \
             them and trim sets them to missing. Without replace the new variable x_w (x_tr \
             with trim, or suffix()) has the old one's storage type (a byte, int or long \
             takes the cut cut short) and the default display format, and the old label (the \
             name when there is none, or label's x-Winsor(p1,p99)); replace keeps the \
             variable's type (a whole-number type taking a cut that is not whole becomes \
             float), format and labels, and changes only the rows of if and in. Its error \
             messages and return codes, and the warning for a cut below 1, are the \
             add-on's. Observed on made-up data; the add-on's program was not read.",
        ),
        p(
            "Not yet",
            "Variable patterns and ranges in the varlist, more than two numbers in cuts(), \
             and replace of a long variable with a cut that is not whole.",
        ),
    ],
}
