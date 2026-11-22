Guide {
    command: "pctile",
    about: "Stores the cut points of equal groups in a new variable",
    points: &[
        p(
            "As Stata 19",
            "pctile newvar = exp, nquantiles(#) writes the # - 1 cut points to the first rows of \
             newvar (a float unless a type comes first), genp() has their percentages, and \
             r(r1), r(r2), ... hold them as doubles. A percentile is the first value whose \
             running weight reaches W p / 100, or the average of it and the next value when \
             the running weight is exactly that; without weights each row weighs 1 (R's \
             quantile type 2). altdef is the interpolation at (n + 1) p / 100 (type 6) and \
             may not be used with weights. nquantiles() runs from 2 to the number of rows of \
             the data plus one, whatever the sample. Frequency, analytic and sampling weights \
             are taken; the percentiles are the same as summarize, detail gives. Nothing is \
             printed.",
        ),
        p(
            "Errors",
            "The return codes are Stata's: no observations 2000, a name already there 110, a \
             string 109, nquantiles() out of range or not whole 198, iweights 101, a \
             non-whole fweight 401. The messages were read off Stata's log by eye; the corpus \
             compares the return codes and values, not the messages.",
        ),
        p(
            "In R",
            "genp()'s variable is labelled \"percentages for __000002\", which is what Stata 19 \
             writes. by: and string types are refused.",
        ),
    ],
}
