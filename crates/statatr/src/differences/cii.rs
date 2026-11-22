Guide {
    command: "cii",
    about: "Confidence intervals from numbers typed in (cii means 50 10 3, cii proportions 60 18)",
    points: &[
        p(
            "In R",
            "stata_cii works ci's intervals out from the numbers typed (cii means #obs #mean \
             #sd, cii means #exposure #events, poisson, cii proportions #obs #succ with each \
             method, cii variances #obs #variance [#kurtosis] with sd and bonett) and prints \
             Stata 19's table with an empty Variable column; where an interval was cut at 0 or \
             1 it says so as Stata's cii does (Note: The Wald CI was truncated at 0 at the lower \
             endpoint.). r() is ci's.",
        ),
        p(
            "Not yet",
            "Values that are not numbers written out are refused. A proportion or rate typed in \
             place of the count (cii proportions 60 .31) is the count Stata 19 takes from it, \
             rounded half up (19).",
        ),
    ],
}
