Guide {
    command: "contract",
    about: "A dataset of frequencies of combinations of variables",
    points: &[
        p(
            "As Stata 19",
            "One row per combination of the variables' values, sorted by them (missing last, \
             an empty string first), with its frequency; freq(), cfreq(), percent(), \
             cpercent() (percents to the bit, labelled as Stata labels them, doubles or with \
             float floats, format() or %8.2f), zero (every combination of the values seen), \
             nomiss, if, in and fweights. The variables keep their labels and formats. A new \
             name already in the data is r(110), no rows r(2000); the data stay as they were.",
        ),
        p(
            "Not yet",
            "Weights of zero; the storage type Stata gives the frequency (byte, int or long), \
             which R does not have.",
        ),
    ],
}
