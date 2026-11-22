Guide {
    command: "xtsum",
    about: "Summarizes variation within and between panels",
    points: &[
        p(
            "As Stata 19",
            "xtsum [varlist] prints the overall, between and within mean, standard \
             deviation, minimum and maximum with N, n and T-bar (T when every panel has as \
             many rows). Without a varlist every variable is shown (a string one as no \
             rows). A missing panel value is a panel of its own. r() holds the last \
             variable's thirteen values.",
        ),
        p(
            "Errors",
            "Without xtset r(459), weights r(101), an unknown option r(198), a name the \
             data lack r(111). i() other than the panel variable is refused.",
        ),
    ],
}
