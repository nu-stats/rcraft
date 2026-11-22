Guide {
    command: "inspect",
    about: "Describes the values of each variable: signs, integers, missing, a small histogram",
    points: &[
        p(
            "As Stata 19",
            "inspect prints, for each variable, the counts of negative, zero and positive values \
             (and how many of each are integers), the missing values, a six-row histogram (a \
             column for each value up to five, else five equal bins), the smallest and largest \
             value, the number of distinct values (counted as floats, \"More than 99\" above 99) \
             and, for a labelled variable, whether every value is in the label. The layout \
             is Stata's to the column. r() holds the last variable's counts. A string variable \
             counts as all missing.",
        ),
        p("Errors", "A name the data lack r(111), an unknown option r(101)."),
    ],
}
