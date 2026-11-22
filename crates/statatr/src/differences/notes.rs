Guide {
    command: "notes",
    about: "Adds, lists, drops, replaces and searches notes of the data and of variables",
    points: &[
        p(
            "As Stata 19",
            "note: text and note var: text add; notes, notes var, notes list and notes in # list; notes drop \
             (_dta, _all or variables, in #, #/# or a list) says (N notes dropped); notes replace var in # : text, \
             notes renumber and notes search word work as in Stata 19. The count is kept in note0 and each note in \
             noteN, so the numbers stay as Stata leaves them after a drop. A variable that is not there is r(111).",
        ),
        p(
            "In R",
            "Notes are characteristics (see char). A note with the word TS (the date and time Stata puts there), \
             quotes in the text and notes search with several words are refused.",
        ),
    ],
}
