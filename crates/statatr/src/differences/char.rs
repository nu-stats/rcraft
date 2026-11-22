Guide {
    command: "char",
    about: "Sets, lists and reads characteristics of the data and of variables",
    points: &[
        p(
            "As Stata 19",
            "char var[name] text, char _dta[name] text and char list [var[]] behave as in Stata 19: the \
             text loses blanks at both ends and a pair of quotes round all of it, no text removes the \
             characteristic, names are case-sensitive, a variable name may be abbreviated and a bad name is \
             invalid syntax (r(198)). `: char var[name]` and `var[name]` read them in macros.",
        ),
        p(
            "In R",
            "A characteristic is the attribute stata_char on the column (on the data frame for _dta). \
             They go into a .dta file on save and come back on use. After merge, append, reshape, collapse and \
             the like they are not known, and the commands say so. char list after tsset, xtset, stset, svyset \
             or fvset is refused (Stata lists the characteristics those set). The order of a listing is the one \
             set last first, not always Stata's. char rename and quotes inside the text are refused.",
        ),
    ],
}
