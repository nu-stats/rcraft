Guide {
    command: "mvdecode",
    about: "Turns codes such as -99 into missing values",
    points: &[
        p(
            "As Stata 19",
            "mvdecode varlist, mv(numlist) makes the listed values . ; mv(numlist = .a \\ numlist = .b) \
             makes others .a to .z. A value is matched in the variable's own type, so 1.1 matches a float 1.1. \
             It says (name, right-aligned in 12) N missing values generated for each variable it changed and \
             string variable ignored for a string. The errors have Stata's messages and return codes: no mv() \
             (r(198)), . or text or a list such as 5 to 8 or 1(0)3 (r(121)), a code that is not a missing value (r(198)).",
        ),
        p(
            "In R",
            "Labels, display formats and storage stay. When the variable's type is not known (data read \
             from a file) the value is matched as typed and as rounded to float. The numlist may be 1 2, \
             5/7, 1(2)9, 1 3 to 9 or 1 3 : 9, as Stata reads them. by: stops with Stata's r(190).",
        ),
    ],
}
