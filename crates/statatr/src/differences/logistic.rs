Guide {
    command: "logistic",
    about: "Logistic regression, reporting odds ratios",
    points: &[
        p(
            "As Stata 19",
            "logistic y x... is logit that shows odds ratios (coef shows the coefficients) and prints no \
             iteration log, with `Note: _cons estimates baseline odds.`; the estimates, e() and the header are \
             logit's, with e(cmd) logistic and e(predict) logistic_p. level(), vce(), robust, if, in and \
             weights are logit's; noconstant is too (the Wald header, no e(ll_0)), and constraints(); predict and test work after it.",
        ),
        p(
            "Not yet",
            "The options logit has not been translated with.",
        ),
    ],
}
