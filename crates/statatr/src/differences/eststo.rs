Guide {
    command: "eststo",
    about: "Stores results and adds to them (eststo and estadd, the SSC estout package)",
    points: &[
        p(
            "In R",
            "eststo keeps the active results as estimates store does (stata_stored), under its \
             name or est1, est2, ... by position, with e(_estimates_name); eststo name: \
             command fits and then stores. eststo clear and drop work as estimates clear and \
             drop.",
        ),
        p(
            "estadd",
            "estadd scalar name = exp and estadd local name text add to e() (stata_e), and to \
             the stored copy when the active results are stored, by eststo or estimates store, \
             as the package does.",
        ),
        p(
            "Not yet",
            "esttab and estout, which write the tables; the other estadd subcommands \
             (ysumm, matrix, ...) and eststo's other options.",
        ),
    ],
}
