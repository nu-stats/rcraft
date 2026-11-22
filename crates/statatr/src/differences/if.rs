Guide {
    command: "if",
    about: "if and else: blocks, and one command on the same line",
    points: &[
        p("How they run", "RCraft works the condition out (R does when it needs the data or a stored result, after the lines before it) and gives R only the lines that run, as Stata does: if exp { ... } else { ... }, and if exp command with else command on the next line."),
        p("On one line", "In if exp command, the condition runs as far as an expression can go, as in Stata: if _rc == 0 display \"ok\" runs display."),
    ],
}
