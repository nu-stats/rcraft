Guide {
    command: "esttab",
    about: "Tables of stored models (estout package, SSC)",
    points: &[
        p(
            "In R",
            "stata_esttab lays the table out as esttab prints and writes it, matched to its \
             output: Stata's names in e(b) in order of first appearance with _cons last, base \
             levels as 0 over (.), t statistics (se, p or none) in parentheses, esttab's own \
             number format (three significant digits) or the format given, stars at 0.05, 0.01 \
             and 0.001 or star(), labels with label, N and r2, ar2, aic, bic and scalars(); \
             the screen table and .txt in fixed columns, .csv with =\"...\" cells, .tex with \
             sym stars, booktabs or plain rules, lines ending CR LF.",
        ),
        p(
            "More options",
            "stats() with fmt() and labels() (only the rows it names, in b()'s format without \
             fmt()), varlabels() (pairs in turn; it also takes away the blank lines between the \
             coefficients), mgroups() with pattern(), and prefix() suffix() span in .tex, \
             starlevels() (the stars as typed), compress, nogaps, and tex on the screen, each \
             as Stata 19 printed and wrote them. After estpost summarize, cells() of its \
             statistics with fmt() (a statistic without one takes the first one's).",
        ),
        p(
            "Head and foot",
            "prehead() posthead() prefoot() postfoot() put their lines in place of a part of the \
             table (@title in prehead() is the title), fragment leaves out the .tex head and \
             foot, mgroups(..., erepeat()) follows the group row, mlabels() with span prefix() \
             suffix() or without them, alignment() sets .tex's columns, and addnotes() goes \
             between the first note and the stars', as Stata 19 printed and wrote them.",
        ),
        p(
            "matrix()",
            "esttab matrix(M, fmt()) lays a matrix out as Stata 19 does: its row and column \
             names, a row of the matrix's name (mlabels(\" \") blank, mlabels(none) none), \
             each cell %9.0g or fmt()'s decimals (one for all or one a column), . for a \
             missing one; on the screen and in .txt between rules, in .tex a tabular unless \
             fragment with posthead() prefoot() postfoot() in their places, in .csv =\"\" \
             cells. fmt() with %formats, other options and names over 12 characters are \
             refused.",
        ),
        p(
            "Not yet",
            "wide, cells() of other models, mlabels() with mtitles(), other @ \
             substitutions, fragment with prehead() or postfoot(), .rtf (it carries the time \
             it was written) and not with a file are refused.",
        ),
    ],
}
