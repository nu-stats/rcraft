Guide {
    command: "ereturn",
    about: "Sets e() in a program that estimates, and lists or shows it",
    points: &[
        p(
            "What is listed",
            "Stata's scalars, macros and matrices in Stata's layout; what Stata also lists and \
             RCraft does not keep (the optimizer's settings, e(ilog), ...) is named in a note.",
        ),
        p(
            "In R",
            "e() is a named list in R, stata_e. ereturn post b V makes it new from the \
             do-file's matrices (which it moves, as Stata does), with depname(), obs(), \
             dof(), properties() and esample(), whose variable leaves the data and marks \
             e(sample). ereturn scalar, local and matrix add to it in the order ereturn \
             list shows, an empty local removes one, and ereturn display prints Stata's \
             coefficient table, with t when dof() was given and z otherwise.",
        ),
        p(
            "Stata's rules",
            "post and clear work anywhere; scalar, local and matrix only in an e-class \
             program (r(152) elsewhere). A V whose size is not b's is r(503), and one whose \
             names are not b's is r(507).",
        ),
        p(
            "Not yet",
            "ereturn repost, a variance matrix that is not symmetric, and predict, test, \
             lincom and margins after posted results are refused.",
        ),
    ],
}
