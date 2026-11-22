Guide {
    command: "tsreport",
    about: "Reports the periods and gaps of time-series data",
    points: &[
        p(
            "As Stata 19",
            "tsreport prints the starting and ending period, the observations and the gaps \
             (with a panel the end of each panel counts too, the last row only when its \
             period is before the sample's end, unless panel is given). detail lists the \
             gaps; a varlist gives a summary table (one report each with detail, or one \
             for the rows complete in all with casewise). r() holds N_gaps, N_obs, start, \
             end (numbered with a varlist), the text panelvar, timevar and tsfmt, var# and \
             the table(s) of the gap report.",
        ),
        p(
            "Errors",
            "Without tsset r(111), a name the data lack r(111), a string variable r(109), \
             casewise with one variable r(198), an unknown option r(198), weights r(101). \
             Data not in panel and time order and a %tc time are refused.",
        ),
    ],
}
