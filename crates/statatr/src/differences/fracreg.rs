Guide {
    command: "fracreg",
    about: "Fractional response regression",
    points: &[
        p(
            "As Stata 19",
            "fracreg probit|logit y x... as Stata fits it: the Bernoulli quasi-likelihood by Newton-Raphson from the \
             least squares start (the iteration log), the robust Wald test and pseudo R2 in the header, het() for the \
             heteroskedastic probit (lnsigma equation), or for odds ratios, level(), if and in, fweights, pweights, \
             iweights, vce(cluster), factor variables, e() with V_modelbased and mns. predict gives cm and xb; \
             aweights stop as Stata stops (r(101)).",
        ),
        p(
            "Not yet",
            "interactions and ib. bases, vce(oim), vce(bootstrap) and vce(jackknife), from(), estat after it.",
        ),
    ],
}
