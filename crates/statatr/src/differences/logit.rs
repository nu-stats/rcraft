Guide {
    command: "logit",
    about: "Logistic regression (logit and probit alike)",
    points: &[
        p(
            "Outcome",
            "The outcome is 1 where it is not 0 (and not missing), as Stata takes it; R's \
             glm() needs 0 and 1.",
        ),
        p(
            "Perfect prediction",
            "A variable that predicts the outcome perfectly is dealt with as Stata does: \
             omitted with the rows it decides (a column of 0 in e(b), where it was written), \
             or the fit stops with Stata's message; predict after it is missing on those \
             rows, in the sample or not, as Stata's default. R's glm() would go on with huge \
             estimates.",
        ),
        p(
            "Collinear regressors",
            "The later of a collinear set, in the order written, is omitted with Stata's note \
             and tolerance, as for poisson, nbreg, ologit, oprobit and mlogit (in every \
             equation). R's glm() checks far more tightly, so a variable stored as float that \
             is nearly another plus a constant stays in and the fit fails; polr() and \
             multinom() do not leave a column out at all.",
        ),
        p(
            "Where probit stops",
            "Stata reports where its own Newton-Raphson stops, which can be short of the \
             maximum by a few parts in 1e8. probit follows that path (from the constant-only \
             fit, with Stata's steps and stopping rule) when the model has a constant and no \
             offset, so its estimates are Stata's to the last digits there.",
        ),
        ML_CNS,
        ML_SE,
        ML_TABLE,
        ML_ROBUST,
        ML_DIGITS,
        MISSING_IN_MODELS,
    ],
}
