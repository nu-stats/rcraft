Guide {
    command: "tnbreg",
    about: "Truncated negative binomial regression (truncated at zero)",
    points: &[
        p(
            "As Stata 19",
            "tnbreg y x... with /lnalpha, alpha and the LR test of alpha = 0, the three fits and their logs, tolerance \
             options, weights, vce(), e(). predict gives n and xb.",
        ),
        p(
            "Not yet",
            "Stata's own start of the full model: its iteration 0 is higher than any start RCraft found, so the full \
             model's log differs from Stata's (the estimates and e() agree); ll() and ul() other than zero, offset().",
        ),
    ],
}
