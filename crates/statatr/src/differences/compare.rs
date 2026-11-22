Guide {
    command: "compare",
    about: "Compares two variables value by value",
    points: &[
        p(
            "As Stata 19",
            "compare a b [if] [in] gives the counts of a<b, a=b and a>b, those defined in both with the minimum, \
             average and maximum of the difference, those missing in one only and in both, and the total; r(N) is the \
             number of rows. Strings compare in byte order. One variable is r(102), three r(103), a weight r(101).",
        ),
        p(
            "In R",
            "A number against a string is refused (R stops). Options are refused.",
        ),
    ],
}
