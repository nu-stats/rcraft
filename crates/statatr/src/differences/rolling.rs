Guide {
    command: "rolling",
    about: "Collects a command's results over windows of a time series",
    points: &[
        p(
            "Windows",
            "window(#) consecutive observations of the data tsset by a time variable, \
             stepsize(#), and recursive for windows that keep the first observation. start and \
             end are the time variable's values; the data become the results.",
        ),
        p(
            "Not yet",
            "Gaps in time, xtset data, rrecursive, saving() and a missing clear are refused \
             with the reason.",
        ),
    ],
}
