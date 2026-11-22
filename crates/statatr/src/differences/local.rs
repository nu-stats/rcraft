Guide {
    command: "local",
    about: "Macros: local, global, macro drop, tempvar, tempname, tempfile",
    points: &[
        p("How they work", "RCraft keeps the do-file's macros itself and replaces them in each line before the line is translated, as Stata does before it runs a line: nested `x`i'', `=exp', $name and ${name}, and the escapes \\` and \\$."),
        p("Numbers", "local x = exp keeps a number as Stata does, in %18.0g: 1/3 is .3333333333333333, 16 digits, not the full double."),
        p("Values only R knows", "A value from the data or a stored result (local m = r(mean), local n = _N) is worked out by R when the do-file runs, after the lines before it, and the lines after it use that value. The R script saved after a run holds the value it had then. A dry run says such lines are known only when the do-file runs."),
        p("Macro functions", "word count, word # of, the list functions (uniq, dups, sort, sizeof, retokenize, -, |, &, in, ==, ===, posof), length, strlen, subinstr and display are done as Stata 19 does them; variable label and label (var) # come from the data's labels in R, and label set # from the do-file's own label define. type, format and value label are not translated: R does not keep Stata's storage types, display formats or the names of value-label sets."),
        p("unab", "unab gets the data's variable names from R and expands the varlist as Stata does."),
        p("tokenize", "tokenize sets `1', `2', ... as Stata 19 does: every numbered local is cleared first, the outer quotes are taken off, quotes inside hold words together, and each parse() character is a word of its own."),
        p("gettoken", "gettoken takes the first token of a macro and what is left as Stata 19 does: leading blanks skipped, the rest keeping its delimiter (\" b c\"), quotes taken off, a parse() character a token of its own (<= and == together), and match() for a token in parentheses. The quotes, qed() and bind options are not translated yet."),
        p("Temporary names", "tempvar and tempname give __000000, __000001, ... as Stata does; tempfile gives a file in a folder of RCraft's own."),
    ],
}
