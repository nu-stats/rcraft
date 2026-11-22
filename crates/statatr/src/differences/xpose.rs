Guide {
    command: "xpose",
    about: "Turns the data so observations become variables",
    points: &[p(
        "In R",
        "stata_xpose makes variables v1, v2, ... from the observations and observations \
         from the variables; a text variable is a row of missing values, as in Stata. The \
         new variables are rounded to float, as Stata stores them, unless promote keeps \
         them double; varname adds _varname with the old names. Labels and formats go.",
    )],
}
