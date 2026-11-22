Guide {
    command: "fdasave",
    about: "Writes the data as a SAS transport file (fdause reads one)",
    points: &[
        p(
            "As Stata 19",
            "fdasave file [if] [in] [, rename replace vallabfile(xpf|sascode|both|none)] writes \
             a SAS XPORT version 5 file, byte for byte what Stata writes bar the time stamps: \
             names in capitals cut to 8 characters (a taken one ends in 2, 3, ...; without \
             rename the list of changes is shown and it stops, r(110)), labels cut to 40, \
             numbers as IBM floats, strings at their width, BEST 12 or MMDDYY 10 for dates, \
             and the value labels in formats.xpf (or as SAS code, or both) beside the file. \
             With replace the old file goes first, before the checks that can stop it (not \
             no variables, r(111)): no observations (r(2000)), a number of 9.046e+74 or more \
             or a string over 200 bytes (r(459)), a value label that is not defined (r(111)), \
             and the file that exists without replace (r(602)). fdause file [, clear \
             novallabels] reads one back: names in lower case, numbers as doubles, dates, the \
             labels of formats.xpf (a warning when it is not there), and r(601) for a missing \
             file.",
        ),
        p(
            "In R",
            "A string is written at its declared width where the variable was made by input \
             str#, generate str# or fdause, else at its longest value. The name of a value \
             label is kept on the column by label values and fdause; a labelled variable \
             without one (read from a .dta) is refused. export sasxport5, import sasxport5 \
             and the member() option of fdause are not translated.",
        ),
    ],
}
