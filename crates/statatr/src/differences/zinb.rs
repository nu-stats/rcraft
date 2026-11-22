Guide {
    command: "zinb",
    about: "Zero-inflated negative binomial regression",
    points: &[
        p(
            "As Stata 19",
            "zinb y x..., inflate(z...) as zip, with /lnalpha and alpha; the not-concave steps are taken as Stata's \
             ml does.",
        ),
        p(
            "Not yet",
            "predict after zinb, probit inflation, offset() and exposure(), factor variables in inflate().",
        ),
    ],
}
