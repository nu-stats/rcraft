Guide {
    command: "lincom",
    about: "A linear combination of coefficients",
    points: &[
        p(
            "Tests",
            "t on the model's degrees of freedom after regress, z after a likelihood model, as \
             Stata's; with or, irr or rrr the estimate and its interval are exponentiated and \
             the standard error is the delta method's.",
        ),
        p(
            "Layout",
            "Stata 19's line and table: ( 1) with the combination as test writes it (lincom x \
             + w - 1 is x + w = 1), then the (1) row under the dependent variable (none after \
             margins, post). A constant is part of the estimate; with or, irr, rrr or eform it \
             is Stata's own error (r(198)). After robreg, reghdfe, cluster2, asclogit and suest \
             the table is R's and a constant is refused.",
        ),
        p(
            "After xtregar",
            "t on e(df_r) after fe, z after re, under the dependent variable, with the \
             constant in the estimate, as Stata 19 prints them; nlcom's table too.",
        ),
    ],
}
