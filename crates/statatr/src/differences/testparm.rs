Guide {
    command: "testparm",
    about: "Joint test of the coefficients a varlist names (testparm varlist, equal)",
    points: &[
        p(
            "In R",
            "stata_testparm finds the model's coefficients whose terms the varlist matches \
             (i.g, c.x, x, i.g#c.x in either order, i.g##i.h as its main effects and \
             interaction, x* against the data's names), base levels left out, in e(b)'s order, \
             and runs test's Wald test on them: F after regress, xtreg fe and ivregress, \
             small; chi2 after xtreg re, ivregress and stcox, and after logit, probit, poisson, \
             nbreg, ologit, oprobit and clogit, whose lines name the equation ([y]x) as \
             Stata's do. equal tests each against the first (- x1 + x2 = 0). r() is test's.",
        ),
        p(
            "Not yet",
            "testparm after the other models, interactions of two continuous variables after \
             mlogit (after mlogit each coefficient is tested in every equation, the base \
             outcome's dropped, as Stata does, factor-by-continuous interactions too), a level \
             of its own (2.g), lists in parentheses (i.(g h)) and the options other than equal \
             are refused. No \
             coefficient matched is Stata's r(111), equal with one coefficient r(102).",
        ),
    ],
}
