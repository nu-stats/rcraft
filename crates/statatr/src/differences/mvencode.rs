Guide {
    command: "mvencode",
    about: "Turns missing values into codes such as -99",
    points: &[
        p(
            "As Stata 19",
            "mvencode varlist, mv(#) turns every kind of missing value into #; mv(.a = #) only .a. It says N missing \
             values recoded, and (recast to int) when the value does not fit the type. If a variable already holds # \
             it says already # in N observations, then no action taken (r(9)) and changes no variable, unless \
             override is given. When several clauses name the same missing value the first takes it, and only a \
             clause that changed something can recast.",
        ),
        p(
            "In R",
            "A value that may not fit the type needs the storage type: known for variables the do-file made, and \
             for a .dta file use read (its header is read when the R runs) while nothing since could have \
             changed a type (sort, keep, drop, generate and the like are fine; replace, recode, merge and so \
             on are not). Otherwise the line is refused.",
        ),
    ],
}
