Guide {
    command: "general",
    about: "What differs between Stata and the R that Stata mode writes, for every command",
    points: &[
        p(
            "Missing values",
            "Stata's missing value (.) is larger than every number, so x > 5 is true for a \
             missing x; R's NA compared with anything is NA. The translation writes \
             Stata's rule out (is.na(x) | x > 5), so conditions keep the rows Stata keeps. \
             The extended missing values .a to .z are kept as haven's tagged NAs and \
             compare, sort, group and tabulate in Stata's order (. < .a < ... < .z).",
        ),
        p(
            "Float and double",
            "generate without a type makes a float in Stata (about 7 digits); R keeps every \
             number as a double. The translation rounds a new variable to float as Stata \
             stores it, unless the do-file says double, so x == 0.1 behaves as in Stata. \
             replace keeps a float a float in Stata, and so does the translation for a \
             variable the do-file made, also after it saved the data and read it back with \
             use or merge; R keeps full precision for one it did not make, with a note.",
        ),
        p(
            "String widths",
            "str5 holds 5 characters in Stata; the translation cuts the text to that width. \
             Text has no width in R otherwise.",
        ),
        p(
            "Sorting",
            "Stata's sort leaves rows that tie in no set order unless sort, stable; R's \
             order() keeps them as they were, which is one of the orders Stata may give.",
        ),
        p(
            "Dates and times",
            "Stata's dates are numbers (days, milliseconds, months, ... since 1960), and so \
             they stay in R: date(), clock(), monthly() and the others read text against \
             Stata's masks (DMY, YMDhms, 20YMD, #) with Stata's rules, and mdy(), year(), \
             dofm() and the rest give Stata's numbers, checked against about 13,000 of \
             Stata 19's answers. They are not R Date or POSIXct values. Outside 0100 to \
             9999 they are missing (wofd() there gives Stata arbitrary numbers). The \
             leap-second functions (Clock(), Cofd(), ...) are not translated yet.",
        ),
        p(
            "Random numbers",
            "set seed, the random functions (runiform(), rnormal(), rgamma(), rbinomial(), \
             runiformint(), rexponential(), ...), bsample and bootstrap give Stata's own \
             draws: RCraft runs Stata's generator (the 64-bit Mersenne Twister) in R, row by \
             row as Stata draws. rt() is Stata's draw to within its last bit or so, the \
             draws after it exactly Stata's; rbeta() is Stata's draw to its last bits or so, \
             its stream exactly Stata's. rpoisson(), rbeta() with equal shapes above 1, \
             rbinomial(), rnbinomial() and \
             rhypergeometric() past the ranges whose method is known (they stop with the \
             reason), simulate and permute, two draws in one expression other than plain \
             runiform()s (several of those are Stata's, a row at a time) and version 13 or \
             earlier are refused.",
        ),
        p(
            "Distribution functions",
            "ttail(), invttail(), Ftail(), invFtail(), chi2tail(), normalden(), ibeta(), \
             gammap(), binomial(), poisson(), comb(), lngamma() and the rest of Stata's \
             t, F, chi2, normal, beta, gamma, binomial and Poisson functions run on R's \
             pt(), pf(), pchisq(), pbeta(), pgamma(), pbinom() and ppois(), which give \
             Stata 19's doubles to 1e-9 or closer. Where R gives a number and Stata a \
             missing value, the translation gives Stata's: degrees of freedom, shapes or \
             a standard deviation of 0 or below, t, F and chi2 degrees of freedom above \
             2e17 and beta and gamma shapes above 1e17, k outside 0 to n in binomialp(), \
             and comb() or lnfactorial() of a number that is not whole. The counts of \
             binomial() and poisson() are cut to whole numbers, as Stata does, and \
             normalden(., s) is 0, as in Stata. invF() and invFtail() come from qbeta(), \
             since R's qf() gives 0 at small p and an approximation for df2 above 4e5. \
             Stata's limits are kept too: missing for a Poisson mean above 2^53, \
             poissonp() with a mean above 1e8 or a count above 1e9, binomialp() with n \
             above 1e6, and invt() and invttail() past 9.3675e153. Only where Stata's own \
             number loses digits do the two differ: invt() below p = 1e-200 with a few \
             degrees of freedom, the densities with degrees of freedom of 1e10 or more, and \
             the Poisson probabilities near a mean of 1e15 (by 1e-9); RCraft gives R's \
             exact number there.",
        ),
        p(
            "Returned values",
            "e() and r() are kept in R as stata_e and stata_r, with Stata's names and \
             values; one Stata keeps that RCraft does not is refused with the reason, not \
             left missing.",
        ),
        p(
            "What is not translated",
            "A line RCraft cannot translate exactly is kept as a comment with the reason, \
             never guessed.",
        ),
    ],
}
