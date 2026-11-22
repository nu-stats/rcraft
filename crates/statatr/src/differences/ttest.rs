Guide {
    command: "ttest",
    about: "t tests: one sample, paired, two samples (and ttesti from summary numbers)",
    points: &[
        p(
            "As Stata 19",
            "Stata's table and lines below it: one sample, paired, two samples by() (named \
             by their value labels, cut to 8) or unpaired; equal variances unless unequal \
             (Satterthwaite's degrees of freedom) or welch (Welch's); level(); the three \
             alternatives with their p-values; r() as Stata's. ttesti takes the counts, \
             means and standard deviations typed in. Stata's errors for unequal or welch \
             with one sample or pairs and for by() with other than two groups.",
        ),
        p(
            "Digits",
            "r() can differ from Stata's in the 16th digit (R's mean and standard \
             deviation are worked out a little differently).",
        ),
    ],
}
