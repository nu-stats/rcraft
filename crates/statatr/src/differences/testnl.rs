Guide {
    command: "testnl",
    about: "Wald test of nonlinear restrictions (testnl _b[x]/_b[z] = 1, testnl (exp = exp) (exp = exp))",
    points: &[
        p(
            "In R",
            "stata_testnl lists the equations, then Wald's chi2 of g(b) = 0 by the delta \
             method, its derivatives taken as nlcom's; after regress too it is chi2, as \
             Stata's. mtest prints each equation's test as well, unadjusted or Bonferroni's \
             (r(mtest) then gains adjustp). r() as Stata 19's: p, chi2, df, mtmethod, \
             r(mtest), r(G) (the derivatives, in Stata's order with _cons last) and r(R). \
             The derivatives agree with Stata's own to about 1e-11.",
        ),
        p(
            "Not yet",
            "mtest's Holm and Sidak adjustments, testnl after the models nlcom refuses, and \
             expressions of the data's variables are refused.",
        ),
    ],
}
