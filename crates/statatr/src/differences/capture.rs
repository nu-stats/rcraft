Guide {
    command: "capture",
    about: "Runs a line and goes on if it fails; _rc says whether it did",
    points: &[
        p("In R", "The line's R runs in tryCatch(): an error is kept from stopping the do-file, as in Stata."),
        p("_rc", "_rc is 0 when the line ran. When it failed because a name is not there, _rc is 111 as in Stata, and 110 for a variable already defined; for any other failure it is missing, since R's errors are not Stata's, so if _rc, if _rc == 0 and if _rc != 0 still pick what Stata would pick. When the line under capture was not translated, _rc is not known at all and the lines that read it say so."),
        p("capture noisily", "Shows Stata's message for the errors above (a not found, variable x already defined) and R's own message for any other."),
    ],
}
