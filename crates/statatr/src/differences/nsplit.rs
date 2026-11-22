Guide {
    command: "nsplit",
    about: "Splits a number's digits into new variables (SSC)",
    points: &[
        p(
            "As Stata 19",
            "Each value's digits are cut from the left into parts of the lengths digits() \
             gives (20060131 with 4 2 2 is 2006, 1 and 31), into var1, var2, ... or \
             generate()'s names, labelled \"Part 1 of var\"; missing stays missing. Observed \
             in Stata 19 on made-up data; the add-on's program was not read.",
        ),
        p(
            "Errors",
            "Every value needs as many digits as the pattern adds up to, else Stata's \
             message (r(459)), after its note when the lengths vary; fractional values and \
             new names in use stop with Stata's words.",
        ),
        p(
            "Not yet",
            "nsplit without digits(), with if or in, or of negative values.",
        ),
    ],
}
