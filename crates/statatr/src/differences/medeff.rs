Guide {
    command: "medeff",
    about: "Causal mediation by simulation (Hicks and Tingley's mediation package, SSC)",
    points: &[
        p(
            "As Stata 19",
            "Worked out from medeff's output, not its code: the mediator model and the \
             outcome model (refitted with the treatment first, so e() is Stata's) on the rows \
             complete in both; seed() then sims rows of each model's coefficients from \
             Stata's own generator (b + L z, z = invnormal(runiform()) a row at a time, the \
             seed set again before the outcome model), the ACME, direct and total effects \
             stored as floats, their means and _pctile's 2.5 and 97.5 percentiles, the share \
             mediated as the median of mean ACME over each total effect; Stata's table and \
             r(), and the generator left where medeff leaves it.",
        ),
        p(
            "Not yet",
            "Models other than regress (probit, logit), interactions, treatment values other \
             than 0 and 1, medeff without sims() or seed(), its other options, and medsens.",
        ),
    ],
}
