Guide {
    command: "lroc",
    about: "Area under the ROC curve after logit or probit (lroc [depvar], nograph all beta())",
    points: &[
        p(
            "In R",
            "stata_lroc sorts the predicted probabilities (a float, as predict makes them) and \
             sums the trapezoids under sensitivity against 1 - specificity, each point a float \
             as in Stata 19, so r(area) carries Stata's residue in its last digits. The rows \
             are e(sample), or those if and in choose (they replace it), or every row with \
             all. beta() scores the first equation of a row vector: variables, factor levels \
             (o. columns add nothing) and _cons, ignoring the fit's weights. The fit's \
             frequency weights or iweights (which weigh the curve while N counts rows), or \
             lroc's own; r(N) and r(area), and Stata's errors (after other models, weights, \
             by, nograph with other options, beta()'s matrix).",
        ),
        p(
            "Graph",
            "Stata draws the ROC curve unless nograph is given; RCraft prints the area and does \
             not draw the curve. Options that only change its look (titles, labels, scales, \
             legend, scheme, regions, sizes, nodraw) are taken and change nothing; name(), \
             saving(), the curve's line and marker options and addplot() are not, and graph \
             export after lroc is refused.",
        ),
        p(
            "Not yet",
            "lroc after logistic (not translated); bysort and by (...); beta() columns with \
             interactions or c., or a string variable; negative iweights. matrix colnames with \
             factor or equation names is refused by the matrix translation, so such columns \
             reach beta() only from e(b).",
        ),
    ],
}
