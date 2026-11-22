Guide {
    command: "infix",
    about: "Reads raw text by columns",
    points: &[p(
        "As Stata",
        "Each field is its columns, blanks trimmed; str alone is as wide as its columns, \
         str# cut to #; # lines with #: for records of several lines; in counts records; \
         if keeps observations; Stata's messages for what is not a number and its count. \
         Not yet: dictionaries (infix using a .dct).",
    )],
}
