Guide {
    command: "infile",
    about: "Reads raw text, free format",
    points: &[p(
        "As Stata",
        "Values separated by blanks or commas, a quoted string one value, flowing across \
         lines; variables typed byte, int, long, float (the default), double or str#; \
         ranges (x1-x3) and _skip(#); a string cut to its width; what is not a number is \
         missing with Stata's message, numbered among the observations if keeps; an \
         observation the file ends inside is kept, with \"(eof not at end of obs)\". A \
         file with no extension is .raw. Not yet: dictionaries, in, byvariable(), \
         automatic, value labels (v:lbl).",
    )],
}
