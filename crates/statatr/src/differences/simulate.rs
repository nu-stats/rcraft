Guide {
    command: "simulate",
    about: "Repeats a program or command and keeps its results as the data",
    points: &[
        p(
            "Method",
            "seed() is set, then the program or command runs reps() times on the data as the \
             last run left them; the statistics (r() and e() values, or _b and _se) become \
             the data, floats labelled with their expressions. r() is what the last run left.",
        ),
        p(
            "Not yet",
            "A replication that stops with an error (Stata records a missing row; R stops), \
             saving(, double), and _b after a program that does not end with regress, logit or \
             probit are refused with the reason.",
        ),
    ],
}
