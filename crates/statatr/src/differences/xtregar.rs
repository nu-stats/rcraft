Guide {
    command: "xtregar",
    about: "Fixed and random effects with AR(1) disturbances",
    points: &[
        p(
            "As Stata 19",
            "rho from Prais-Winsten on the within-demeaned data (each rhotype(), rhof(), \
             twostep); fe on Baltagi and Wu's transform, kept in single precision as Stata \
             keeps it, the constant divided by 1 - rho; re by GLS with each panel's theta; \
             lbi's statistics (StatCraft's decoding of Stata 19).",
        ),
        p(
            "Differences",
            "re's sigma_u and what follows from it agree with Stata to about 1 part in 1e8, \
             not to the last digit. Weights, factor variables, time-series operators, \
             predict and margins after it are not translated yet.",
        ),
    ],
}
