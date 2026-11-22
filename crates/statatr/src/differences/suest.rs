Guide {
    command: "suest",
    about: "Combines stored models into one set of results",
    points: &[
        p(
            "As Stata 19",
            "suest of conditional logits (clogit) stored with estimates store: e(b) with each \
             model's coefficients and base levels, its equations named name_depvar, and the \
             sandwich e(V) from the models' own variances and the rows' scores, clustered by \
             vce(cluster v) or by clogit's groups, times G / (G - 1). After it, margins, \
             dydx() of factor levels with an expression() that sums invlogit(predict(\
             equation(eq))) or predict(equation(eq)) terms, posted or not, and test after the \
             posted margins.",
        ),
        p(
            "Not yet",
            "Other models; models fitted with robust errors or weights; predict and other \
             margins after suest. The table is printed in R's layout.",
        ),
    ],
}
