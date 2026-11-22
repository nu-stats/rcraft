Guide {
    command: "import",
    about: "import delimited and import excel",
    points: &[p(
        "Variable names",
        "Columns without names are v1, v2, ... as Stata names them; names that are not \
         valid Stata names are made valid by R's rules, which can differ from Stata's.",
    )],
}
