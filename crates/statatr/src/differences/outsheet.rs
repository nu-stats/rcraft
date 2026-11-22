Guide {
    command: "outsheet",
    about: "Writes the data as a text file (tab or comma separated)",
    points: &[
        p(
            "As Stata 19",
            "Tab-separated, or comma or delimiter(char); a first line of names unless nonames; \
             numbers written in their display formats (a %8.3f variable shows 1.500, a double \
             %10.0g, a float %9.0g), value labels and strings in double quotes (a quote inside \
             is not doubled) unless noquote, labels dropped by nolabel; a missing number is \
             empty and .a to .z are written as they are; a file with no extension gets .out; \
             if, in and a variable list. Nothing is printed when the file is written, \
             (file x not found) only with replace and no file there, and the errors for a file \
             that exists (r(602)), a missing variable, no using, and bad delimiter() or comma \
             together with delimiter().",
        ),
        p(
            "Not yet",
            "A variable list with ranges or wildcards.",
        ),
    ],
}
