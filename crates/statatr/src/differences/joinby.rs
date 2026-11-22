Guide {
    command: "joinby",
    about: "Pairs every master row with the using rows of the same key",
    points: &[
        p(
            "As Stata 19",
            "joinby [varlist] using file pairs each master row with every using row that has \
             the same key (many to many). unmatched(master|using|both) keeps the rows that found \
             no partner and makes _merge (1 only in master data, 2 only in using data, 3 both); \
             with no varlist it joins on the variables in common and says so. A variable in \
             both files keeps the master's value; with update a master value that is missing \
             or \"\" takes the using file's, and with update replace the using file's value \
             replaces the master's when it is not missing (replace alone is r(198)). An empty \
             result says (joinby on k is empty). \
             The errors have Stata's return codes: no using (r(100)), a name the data lack \
             (r(111)), no variable in common (r(198)), a key that is a string in one file and a \
             number in the other (r(106)), _merge already there (r(110)).",
        ),
        p(
            "In R",
            "Stata puts the rows paired within one key in an order of its own, which changes \
             from run to run; RCraft's are in the using file's order, so sort before looking. \
             by: stops with Stata's r(190).",
        ),
    ],
}
