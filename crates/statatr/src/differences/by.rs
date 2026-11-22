Guide {
    command: "by",
    about: "by varlist: and bysort varlist: (the prefix)",
    points: &[
        p(
            "Groups",
            "Each group in turn, in sorted order with a missing value last, as in Stata; \
             after a model the results kept are the last group's.",
        ),
        p(
            "Order",
            "Without bysort the rows must already be in order of the by variables: R \
             checks the order itself and stops with Stata's not sorted.",
        ),
    ],
}
