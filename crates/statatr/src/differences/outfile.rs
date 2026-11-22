Guide {
    command: "outfile",
    about: "Writes the data as a text file in columns",
    points: &[
        p(
            "As Stata 19",
            "The free layout: each number right-aligned in its display format's width, each \
             string quoted and left-aligned in at least 11 (a str20 in 22), a labelled value \
             quoted and left-aligned in the format's width or its longest label, two blanks \
             after each field; without wide a field that would bring the line to column 80 \
             starts a new line and lines lose their trailing blanks. comma writes the fields \
             trimmed with commas (a missing value empty), noquote and nolabel, runtogether \
             for string variables only (r(109) otherwise), if, in, a variable list, .raw added \
             to a name with no extension, and the errors for a file that exists, a missing \
             variable and no using.",
        ),
        p(
            "String widths",
            "Stata lays a string out by its declared width. R does not keep it, so RCraft \
             keeps the width a variable was given by input str#, generate str# or a file, and \
             for a string made any other way takes the longest value; the layout then differs \
             only when the declared width is above 9 and larger than the longest value.",
        ),
        p(
            "Not yet",
            "The dictionary option, and a variable list with ranges or wildcards.",
        ),
    ],
}
