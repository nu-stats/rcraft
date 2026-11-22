Guide {
    command: "tpoisson",
    about: "Truncated Poisson regression (truncated at zero)",
    points: &[
        p(
            "As Stata 19",
            "tpoisson y x... with the limits header, the LR test against the constant-only fit, irr, level(), if and \
             in, fweights, pweights, iweights, vce(robust), vce(cluster), e(). predict gives n and xb.",
        ),
        p(
            "Not yet",
            "ll() and ul() other than zero, offset() and exposure(), predict's other statistics (cm, pr()).",
        ),
    ],
}
