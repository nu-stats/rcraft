Guide {
    command: "egen",
    about: "Generate with functions over rows or groups",
    points: &[p(
        "Missing values",
        "The functions leave missing values out of their sums and means, as Stata's do; \
         R's mean() would give NA without na.rm.",
    )],
}
