Guide {
    command: "ivprobit",
    about: "Probit with an endogenous regressor",
    points: &[
        p(
            "As Stata 19",
            "One endogenous regressor: the maximum-likelihood fit from Stata's exogenous \
             probit and ml's Newton steps (both iteration logs), oim, robust and clustered \
             variances, first, level(), Newey's two-step estimator, the exogeneity test, e() \
             and predict's xb, pr and stdp (StatCraft's decoding of Stata 19).",
        ),
        p(
            "Differences",
            "The variance comes from differences of the analytic gradient, as StatCraft's \
             does: it agrees with Stata's to about 1e-9. Weights, several endogenous \
             variables, factor variables, twostep with first or vce(), iterate() below the \
             default, and test, margins or estat afterwards are not translated. An instrument \
             collinear with the other instruments is left out with Stata's note, as Stata \
             does; any other collinearity stops the line when it runs, saying so.",
        ),
    ],
}
