Guide {
    command: "test",
    about: "Wald tests after a model",
    points: &[
        p(
            "Statistic",
            "F on the model's degrees of freedom after regress, chi2 after a likelihood \
             model, as Stata's.",
        ),
        p(
            "Lines",
            "Each constraint is written as Stata 19 writes it: the coefficients on the left in \
             the order typed, the constant on the right (test 2*x = 3*w + 1 is ( 1)  2*x - 3*w \
             = 1), each name with its equation after the likelihood models ([yb]x) and the \
             base outcome's as omitted after mlogit ([3]o.x). After the models where Stata's \
             lines were not seen the line is the constraint as typed.",
        ),
        p(
            "After xtregar",
            "xtregar's e(b) and e(V) as Stata 19 reads them: F on e(df_r) after fe, chi2 \
             after re. estat vce prints e(V) in Stata's layout; estat's other forms after \
             xtregar are refused.",
        ),
        p(
            "Options",
            "matvlc() keeps the constraints' variance R V R' as a matrix; mtest (noadjust, \
             bonferroni, holm, sidak) prints Stata 19's table of each constraint's own test \
             with its adjusted p and the joint test, and leaves r(mtest) and r(mtmethod); \
             accumulate tests the last test's constraints with these; notest prints the \
             constraints only and empties r(). mtest with accumulate is Stata's error.",
        ),
    ],
}
