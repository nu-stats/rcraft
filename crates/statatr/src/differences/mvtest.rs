Guide {
    command: "mvtest",
    about: "Multivariate tests (mvtest means x1 x2 x3, by(g); mvtest normality x1 x2, stats(all))",
    points: &[
        p(
            "In R",
            "stata_mvtest works out Hotelling's T2 that the means are equal (or zero), the \
             four MANOVA statistics by() with Rao's, Pillai's, Lawley-Hotelling's and Roy's F, \
             the LR test allowing for heterogeneity (the common mean by maximum likelihood), \
             Box's M with its F and chi2, the adjusted LR tests that the covariance matrix is \
             diagonal, spherical or compound symmetric, Lawley's and Jennrich's tests of \
             correlations, and the Doornik-Hansen, Henze-Zirkler and Mardia tests of \
             normality with the univariate and bivariate tables. Rows with any of the \
             variables missing are left out; missing makes the missing by() values a group. \
             Stata 19's printout, its errors (r(198), and r(506) for a singular matrix) and \
             r(), matrices included.",
        ),
        p(
            "Digits",
            "The statistics agree with Stata's to about 1e-13; r(M) after lr, the common \
             mean, about 1e-10 (Stata stops its iterations sooner).",
        ),
        p(
            "Not yet",
            "heterogeneous (for more than two groups Stata takes the p-value of James' \
             approximation from its own search, a few 1e-6 off the root), protect(), \
             weights, missing without by(), fewer rows than contrasts in means, and \
             matrix list r(Btest) (its rows are named by pairs) are refused.",
        ),
    ],
}
