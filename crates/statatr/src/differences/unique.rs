Guide {
    command: "unique",
    about: "Counts the distinct values of variables (unique, SSC)",
    points: &[
        p(
            "As Stata 19",
            "The number of distinct combinations of the varlist's values (a missing value is a \
             value) among the rows of if and in, and the number of records, printed and left in \
             r(N), r(sum) and r(unique); detail adds the add-on's table of records per \
             combination; by() puts the count of each group in a new variable (_Unique, or \
             generate()) on one row of the group and lists the groups, and generate() with no \
             by() makes nothing. Its error messages and return codes are the add-on's. \
             Observed on made-up data; the add-on's program was not read.",
        ),
        p(
            "In R",
            "With by(), Stata puts the count on the first row of the group in its own sort of \
             the by variables and the varlist, and where several rows tie there it picks one \
             of them (its sort does not keep ties in order); RCraft takes the first in the \
             data's order and says so when rows tie.",
        ),
        p(
            "Not yet",
            "detail with by() of several variables, variable patterns, weights.",
        ),
    ],
}
