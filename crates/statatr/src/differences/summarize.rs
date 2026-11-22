Guide {
    command: "summarize",
    about: "Summary statistics",
    points: &[
        p(
            "Standard deviation",
            "The sample standard deviation (n - 1), as R's sd(); with weights Stata's \
             rules for each kind of weight.",
        ),
        p(
            "Percentiles",
            "With detail, Stata's rule: x[ceil(np)], or the mean of the two neighbours \
             when np is whole, which is R's quantile type 2, not R's default type 7.",
        ),
        p(
            "Skewness and kurtosis",
            "Stata's, over the whole sample, so a normal sample's kurtosis is near 3, not 0.",
        ),
        p(
            "Printing",
            "Stata's own table, checked character for character: fweights count in Obs, \
             aweights and iweights add the Weight column; detail's block (the label wrapped \
             at 50 and centred, the percentiles beside the four smallest and largest values, \
             the moments added as Stata adds them, so a skewness that is 0 but for rounding \
             is Stata's too). iweights with detail is Stata's error. meanonly may be written \
             mean, as in Stata.",
        ),
    ],
}
