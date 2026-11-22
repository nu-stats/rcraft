Guide {
    command: "xi",
    about: "Expand i. terms into dummy variables",
    points: &[
        p(
            "As Stata 19",
            "xi: regress y i.g x (and any command with i. terms), xi i.g and xi, prefix(_D): ... make a byte \
             variable _Ig_# for each level of g but the omitted one (the lowest; the most frequent with char \
             _dta[omit] prevalent; # with char g[omit] #), labelled g==#, and run the command on them. \
             i.g*i.h adds both sets and _IgXh_#_# (g==2 & h==1), i.g*x the dummies and the float _IgXx_# \
             ((g==2)*x, of x's type), i.g|x the same with the dummies left out of the command. noomit makes every level's dummy and prints no line for the plain factors. A factor that is not whole numbers from 0 up (negative, fractional or a string) is named by rank, `_Im_1-3  (_Im_1 for m==-1 omitted)`, with the dummies labelled by value (`m==0`, `s==lo`). Each run drops the variables \
             an earlier run made with the same prefix, prints a line per term (`i.g  _Ig_1-3  (naturally coded; \
             _Ig_1 omitted)`) and leaves e(cmdline) with the stubs (`regress y _Ig_* x`). The levels are asked \
             of R when the do-file runs, so a dry run shows the line as known only when run.",
        ),
        p(
            "Not yet",
            "A factor in two terms, three or more parts in a term, | between two factors, the other options, \
             names over 32 characters and tied most-frequent levels.",
        ),
    ],
}
