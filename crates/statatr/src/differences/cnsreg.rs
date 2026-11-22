Guide {
    command: "cnsreg",
    about: "Constrained linear regression",
    points: &[
        p(
            "As Stata 19",
            "cnsreg y x..., constraints(numlist) reads each defined constraint as a linear \
             equation in the coefficients (`x1 = x2`, `2*x1 - x2 = x3 + 1`, `_cons = 2`, \
             `_b[x1] = 1`), counts equal ones once, and fits restricted least squares by \
             projection: b = b0 + T a, b0 the smallest b meeting R b = r, T a basis of what \
             the constraints leave free. Degrees of freedom N - (k - m), s^2 = RSS/df. noconstant, \
             vce(robust) and vce(cluster v) (HC1, G/(G-1) with the clusters), level(), \
             fweights, aweights and pweights (which imply robust) are taken. The F test is that \
             of the slopes given the constraints, and goes with its Prob > F when it \
             contradicts them (a slope fixed at a value); e(p) keeps the residual degrees of \
             freedom even with clusters, as Stata does. A constraint that does not parse, or names \
             a variable the model lacks, is dropped with Stata's note; none left is r(111), \
             repeated or contradicting ones r(412), a missing constraints() r(198). The header, \
             the constraint lines, `(constrained)` and the table print as Stata's, and e() \
             (b, V, Cns, V_modelbased) is Stata's. predict (xb, residuals) and test work after it.",
        ),
        p(
            "Not yet",
            "Interactions and operators in the model, factor variables with noconstant or with \
             regressors omitted for collinearity, iweights, collinear, mse1, and \
             constraints with more than one =, products or powers of coefficients (Stata: r(131)) \
             beyond the error, or the [eq]x form beyond its r(303).",
        ),
        p(
            "Factor variables",
            "i.g and ib#.g are taken: the base level is a column of e(b) fixed at 0 by a constraint of its \
             own (the first rows of e(Cns), not lines of the header; 1.g in a constraint reads the base \
             1b.g), the degrees of freedom count it, and the table hides the base row. A coefficient the \
             constraints fix at 0 prints `0  (omitted)` (a plain variable too), one fixed elsewhere \
             `(constrained)`. The residual after predict is in double with factor variables.",
        ),
    ],
}
