Guide {
    command: "preserve",
    about: "Keeps a copy of the data, which restore brings back",
    points: &[
        p("In R", "preserve keeps a copy of the data frame (stata_preserved) and what RCraft knows of it: its variables' types, the value labels defined and xtset or stset; restore puts them back and drops the copy. restore, preserve keeps the copy and restore, not drops it unused."),
        p("At the end", "If the do-file ends before restore, Stata puts the preserved data back by itself; the R session keeps the data as it was at the end."),
    ],
}
