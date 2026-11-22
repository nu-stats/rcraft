Guide {
    command: "xtdescribe",
    about: "Describes the participation pattern of a panel",
    points: &[
        p(
            "As Stata 19",
            "xtdescribe prints the panel and time lines (first, second, ..., last), Delta and \
             Span, the distribution of T_i and the patterns of presence, most frequent first \
             (patterns(#), 9 by default; width(#) groups periods into one column each, with \
             the note under the table). r() is summarize T_i, detail. A missing time counts \
             in T as a value of its own, and rows with a missing panel are left out. The \
             corpus file xtpanel checks r() and the tables were compared with Stata's \
             printout.",
        ),
        p(
            "Errors",
            "Without xtset (or with a panel and no time) r(459), a time with fewer than two \
             values r(459), weights and a varlist r(101), an unknown option r(198). \
             i() and t() other than the variables xtset holds are refused, as is width() \
             below 3 and a %tc time.",
        ),
    ],
}
