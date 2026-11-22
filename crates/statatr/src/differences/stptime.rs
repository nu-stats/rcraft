Guide {
    command: "stptime",
    about: "Person-time and the incidence rate of survival-time data",
    points: &[
        p(
            "Method",
            "Needs an id() set by stset. Person-time, failures and the rate with the interval \
             rate * exp(+-z / sqrt(failures)), by(), at() (pieces (a, b] and a last open one \
             unless trim), per(), dd() and level(); r(ptime), r(failures), r(rate), r(lb) and \
             r(ub) hold the total.",
        ),
        p("Not yet", "smr(), using(), jackknife, cox, weibull and the other models."),
        LAYOUT_R,
    ],
}
