Guide {
    command: "foreach",
    about: "Loops: foreach, forvalues and while (and continue)",
    points: &[
        p("How they run", "RCraft runs a loop itself and gives R each round's lines with the loop's macro replaced, so the R script has the loop written out round by round."),
        p("forvalues", "Each value is the one before plus the step, as Stata does, so 0(.1)1 ends at .9999999999999999, and each is kept as Stata keeps a number in a macro (16 digits)."),
        p("numlist", "foreach ... of numlist works its numbers out from the first and the step and keeps 15 digits, as Stata's numlists do."),
        p("varlist", "foreach ... of varlist with a wildcard, a range or _all asks R for the data's variable names when it runs and makes them Stata's way (x* every name that matches, x1-x3 the names between in the data's order, an ambiguous abbreviation Stata's error). Plain names are taken as written. A dry run cannot know the names, so it says the loop is known only when the do-file runs."),
        p("After the loop", "The loop's macro is gone after the loop, as in Stata. A while loop whose condition is still true after 100,000 rounds is stopped and said so."),
        p("Values only R knows", "A condition such as while r(N) > 0 is worked out by R when the do-file runs, after the lines before it have run."),
    ],
}
