Guide {
    command: "xttab",
    about: "Tabulates a variable overall, between and within panels",
    points: &[
        p(
            "As Stata 19",
            "xttab x prints the overall, between and within frequencies and percents (the \
             between percent of all panels, so the column can pass 100; the within percent \
             the mean share of the panels' rows). A value shows as its value label cut to 8 \
             characters, or as %9.0g writes it. r(results) is the table (its values as \
             floats) and r(n_panels) the number of panels.",
        ),
        p(
            "Errors",
            "No varlist r(100), two variables r(103), a string variable r(109), no rows \
             r(2000), without xtset r(459), weights r(101), any option other than i() \
             r(198).",
        ),
    ],
}
