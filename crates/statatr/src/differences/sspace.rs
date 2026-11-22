Guide {
    command: "sspace",
    about: "Linear state-space models by maximum likelihood, and predict after them",
    points: &[
        p(
            "As Stata 19",
            "sspace (u L.u, state noconstant) (y u, noconstant), constraints(1): first-order state equations and \
             observation equations with states, a constant and exogenous variables; constraints of the form \
             [eq]term = #. The exact likelihood by the Kalman filter (stationary start, or the exact diffuse start \
             when constraints fix a unit root, each diffuse step adding -ln(2 pi)/2), the observed-information \
             standard errors, the Wald test, e() and _b[/state:var(u)] as Stata names them. predict has xb and \
             states with equation() and smethod(onestep|filter) (StatCraft's decoding of Stata 19).",
        ),
        p(
            "Differences",
            "RCraft finds the optimum by a search of its own (BFGS, then Newton), so Stata's iteration log and \
             e(ic), e(tech_steps), e(technique), e(ilog), e(gradient) and the data signature are not reproduced; \
             the estimates, the log likelihood, the standard errors and the Wald test agree with Stata to about \
             1e-6 (the p-value of a large chi2 to 2e-6). The table is printed in R's layout. Constants or exogenous variables in state equations, lags \
             beyond the first, covstate() and the other covariance options, from(), technique(), estat, \
             smethod(smooth), and gaps in the sample are not translated yet.",
        ),
    ],
}
