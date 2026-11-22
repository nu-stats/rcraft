Guide {
    command: "snapshot",
    about: "Keeps copies of the data in memory and puts them back",
    points: &[
        p(
            "As Stata 19",
            "snapshot save [, label()] prints snapshot N (label) created at the time and \
             leaves r(snapshot); the next number is the highest one there plus one (1 when none), \
             so a number can come back after the highest is erased. list shows them all, \
             _all or some, label # text changes a label (none clears it), restore # puts the \
             data back (variables, values, labels) and erase takes _all or a numlist off. \
             Long lines wrap at the line size, continued with two blanks. The errors have \
             Stata's messages and return codes: no snapshots, a number outside 1 to the \
             highest (r(198)), one that was erased (r(111), before anything is erased), a \
             bad numlist (r(121), r(122)), and a bad number (r(7)).",
        ),
        p(
            "In R",
            "The snapshots are kept in the R session, as Stata keeps them outside the data, \
             and survive clear. A snapshot of empty data restores empty data, but RCraft \
             does not follow data that is empty, so the lines after it that need rows are \
             refused. The time is the computer's, to the minute; a day before the 10th is \
             written without a leading zero (not observed in Stata).",
        ),
    ],
}
