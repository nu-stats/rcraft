Guide {
    command: "assert",
    about: "Stops when an expression is false",
    points: &[p(
        "Messages",
        "Stata's count of the rows where it is false (23 contradictions in 60 observations, \
         none for a single row), then assertion is false and r(9); rc0 prints them and \
         goes on, and no rows is (null assertion). A missing value counts as large, as in \
         Stata. fast and null are not translated yet.",
    )],
}
