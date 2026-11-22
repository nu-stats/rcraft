Guide {
    command: "tsappend",
    about: "Adds periods to time-series data",
    points: &[
        p(
            "As Stata 19",
            "tsappend, add(#) adds # periods after each panel's last; last(date) \
             tsfmt(td, tw, tm, tq or th) adds periods up to a date. The periods missing \
             inside each panel are filled too, as tsfill does; the other variables are \
             missing in new rows, and the data end in panel and time order. r(add) is # (or, \
             with last(), the periods added after the panels' ends). When last() is not \
             after the data's end Stata says so and adds nothing.",
        ),
        p(
            "Errors",
            "Neither add() nor last() r(198), add() not a positive whole number, add() with \
             last() or tsfmt(), last() without tsfmt(), a missing time, an unknown option \
             r(198); a varlist, if, in and weights r(101); without tsset r(111). Data not in \
             panel and time order, a missing panel value and tsfmt() other than td, tw, tm, \
             tq and th are refused.",
        ),
    ],
}
