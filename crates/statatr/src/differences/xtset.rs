Guide {
    command: "xtset",
    about: "Declares panel data",
    points: &[p(
        "Time",
        "With a time variable it is tsset: the times are checked, the data sorted by panel \
         and time, and L., F., D. and S. work as under tsset. A time unit (daily, yearly) or \
         format() makes Stata show the time as dates and gives it that format; here it shows \
         as a number. The lags are the same. A .dta file Stata saved after xtset or tsset \
         keeps the settings, and Stata sets them again when it reads the file: when the \
         do-file runs, RCraft reads them from the file too (the delta included).",
    )],
}
