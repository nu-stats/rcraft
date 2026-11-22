Guide {
    command: "fillin",
    about: "Adds a row for every combination of the values the data lack",
    points: &[
        p(
            "As Stata 19",
            "fillin varlist adds the combinations of the variables' values that the data lack, \
             each variable's levels being the values it has (`.`, each of `.a` to `.z` and \"\" \
             are levels of their own). The other variables are missing in the added rows, \
             _fillin is 0 on the old rows and 1 on the new, and the data end sorted by the \
             varlist with repeated combinations kept. Nothing is printed. The errors have \
             Stata's return codes: one variable (r(102)), a name the data lack (r(111)), the \
             same variable twice (r(5)) and _fillin already there (r(110)).",
        ),
        p(
            "In R",
            "Labels, display formats and value labels stay on the columns. Patterns in the \
             varlist and data with no rows are refused; by: stops with Stata's r(190).",
        ),
    ],
}
