Guide {
    command: "bitesti",
    about: "Exact binomial test from the numbers of trials and successes",
    points: &[p(
        "As Stata 19",
        "bitesti #N #succ #p is bitest on the counts, with the same tails, k_opp and r(). \
         A non-whole #N is r(7), #succ above #N or #p outside (0,1) is r(450), a wrong \
         number of arguments r(198). The detail option is refused.",
    )],
}
