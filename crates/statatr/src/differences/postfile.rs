Guide {
    command: "postfile",
    about: "Posts results row by row to a .dta file (postfile, post, postclose)",
    points: &[p(
        "As Stata 19",
        "Each post keeps a row, each value in its variable's type as Stata stores it (float \
         rounded to single precision, a string cut to its width; float is the default); \
         postclose writes the file with haven. replace says \"(file name.dta not found)\" \
         as Stata does; an existing file without replace is r(602), a wrong count of \
         expressions r(198) and a string for a number r(109), each closing the postfile as \
         Stata does. every(#), which only sets how often Stata writes, changes nothing.",
    )],
}
