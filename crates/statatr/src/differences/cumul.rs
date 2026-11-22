Guide {
    command: "cumul",
    about: "Stores the empirical distribution function of a variable",
    points: &[
        p(
            "As Stata 19",
            "cumul x, generate(new) makes a float variable, labelled \"ECDF of x\", holding the \
             running weight over the total (the running weight itself with freq), per by() \
             group (a missing value is a group of its own). equal gives tied values the value \
             after the last of them. Frequency and analytic weights are taken, with the \
             running weight held in a float as Stata does, so the last digits agree. Rows \
             outside if and in, with a missing value or with a zero or missing weight, are \
             missing. A string variable makes a variable of missing values without an error, \
             as in Stata. r() is left as it was.",
        ),
        p(
            "Ties",
            "Without equal, Stata gives tied values different values in an order of its own \
             sort, which is arbitrary and cannot be reproduced. The R code gives them the \
             order of the rows. The values themselves, and every row with no tie, are \
             Stata's; which tied row gets which of the tied values may differ. Use equal \
             where the order matters.",
        ),
        p(
            "Errors",
            "pweights and iweights r(101), a negative weight r(402), a non-whole fweight \
             r(401), a missing generate() r(198), a name already there r(110), two \
             variables r(103). The messages were read off Stata's log by eye.",
        ),
    ],
}
