Guide {
    command: "xtdpdsys",
    about: "Dynamic panel-data GMM: xtabond, xtdpdsys and xtdpd",
    points: &[
        p(
            "As Stata 19",
            "The differenced equations with the lags of the outcome (and of pre() and \
             endogenous() variables) as GMM-type instruments, one column per period, and the \
             level equations with LD.y for xtdpdsys; instruments zero in every panel dropped; \
             the one-step weight from the differencing, two steps from its residuals, \
             vce(robust) and Windmeijer's two-step correction; estat sargan and estat abond, \
             predict's xb, e and difference (StatCraft's decoding of Stata 19).",
        ),
        p(
            "Differences",
            "Stata's e(datasignature) checksum is not kept. Weights, factor variables, the \
             display options, abbreviated variable names, diffvars() with a constant and \
             estat abond after a one-step fit without a level equation are not translated, \
             nor test, lincom or margins after these commands.",
        ),
    ],
}
