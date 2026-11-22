Guide {
    command: "rangestat",
    about: "Statistics over a range of each row's key (SSC)",
    points: &[
        p(
            "As Stata 19",
            "For each row with a key, the rows whose key lies in [key + low, key + high] (. \
             for no end), in its by() group and among the rows of if/in, itself included. A \
             window with no rows gives missing; one whose values are all missing gives \
             (sum) and (count) 0 and (obs) its rows. The new variable is var_stat or the \
             name given, a double labelled \"sum of x\"; one in use stops (r(110)). Observed \
             in Stata 19 on made-up data; the add-on's program was not read.",
        ),
        p(
            "Not yet",
            "Statistics other than sum, count, obs, mean, min, max and sd; bounds that are \
             variables; excludeself and the other options.",
        ),
    ],
}
