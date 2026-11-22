Guide {
    command: "merge",
    about: "Joins another file",
    points: &[
        p(
            "Pairing",
            "stata_merge pairs the rows as Stata does for 1:1, m:1, 1:m, m:m and 1:1 _n: \
             within a key the k-th row of the master goes with the k-th row of the using \
             data, and the shorter side's last row stands in for the rows it lacks (m:m is \
             not every combination, as R's joins would give). A variable in both keeps the \
             master's value.",
        ),
        p(
            "_merge",
            "_merge is made as Stata makes it (1 to 5, with Stata's labels), and Stata's \
             report is printed. update fills missing values (4) and marks conflicts (5); \
             replace takes the using value there. assert() is checked before keep(), and \
             the merged data stay when it fails (r(9)).",
        ),
        p(
            "Order",
            "The master's rows come first, sorted by the key, then the rest, sorted by the \
             key. Rows with the same key keep their order here; Stata's sort may put them \
             in another order.",
        ),
        p(
            "Not yet",
            "The old syntax (merge varlist using) and force.",
        ),
    ],
}
