Guide {
    command: "outreg2",
    about: "Writes a table of models to a file (the SSC package)",
    points: &[
        p(
            "In R",
            "stata_outreg2 writes the same .txt, .xls and .tex files as the package, byte for \
             byte, matched to the files it writes (not its code), for tstat or se with bdec(#) \
             tdec(#), parenthesis(), adjr2, r2, keep() (a##b included) and e(); tex(fragment); \
             label (a LABELS column, the labels in the .tex); ctitle(); addtext() and \
             addstat(): a column per model added to the table already in f.txt unless \
             replace, a variable a later model adds placed before Constant, the statistics \
             rows in Stata's order (a column alone in its model's order, xtreg's Number of \
             panels included; Observations and R-squared first once columns are added; e() \
             rows sorted by name), standard errors with 3 decimals, Robust in the footer \
             after a robust or clustered first column. Without tstat or se it writes standard \
             errors in outreg2's own number format (by the size of the number) and with bdec() \
             alone 3 decimals for them; sdec(); nocons, noobs, nor2, noaster, alpha() with \
             symbol(); stats(coef), (coef se), (coef tstat|pval|ci) and (coef se tstat pval); \
             tex alone (a whole document) and word (Rich Text, .rtf or .doc); a .txt name; and \
             logit, probit and poisson.",
        ),
        p(
            "Not yet",
            "stats(... beta), tex(pretty), label with word, nocons, noobs, nor2, noaster, \
             alpha(), symbol() or stats() with append, noobs, nor2, noaster, alpha(), \
             symbol() or stats() with an .xls or .tex file, label with an .xls file, stored \
             models by name, or after factor variables that keep() does not leave out.",
        ),
    ],
}
