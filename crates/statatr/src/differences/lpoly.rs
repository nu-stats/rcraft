Guide {
    command: "lpoly",
    about: "Local polynomial smoothing",
    points: &[
        p(
            "As Stata 19",
            "The smooth by weighted local polynomials of degree 0 or more on min(N, 50) \
             points or at the rows of at(), every kernel (the default Epanechnikov), the \
             rule-of-thumb bandwidth and pilot bandwidth, se() and var(), generate() with its \
             labels, aweights and fweights, if and in, r(), and Stata's errors \
             (StatCraft's decoding of Stata 19).",
        ),
        p(
            "Differences",
            "The rule-of-thumb bandwidth agrees with Stata to about 1e-6, the smooth and its \
             errors to 8 digits. A window with exactly as many points as the degree plus 3 \
             gives a standard error Stata fills with rounding noise; here it is missing. The \
             graph is drawn by ggplot2: the data, the smooth and, with ci, a band; Stata's \
             scatter and line options are not carried over. Degrees above 5 are not translated.",
        ),
    ],
}
