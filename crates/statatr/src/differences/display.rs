Guide {
    command: "display",
    about: "Prints text and values",
    points: &[
        p(
            "Formats",
            "Stata's formats (%9.0g, %6.2f, %e, %s, with c, - and 0) are worked out as \
             Stata does at every width: R's format() and sprintf() give other digits.",
        ),
        p(
            "Variables",
            "A variable in an expression means its first row, as in Stata.",
        ),
        p(
            "%21x",
            "The double's own bits in hexadecimal, as Stata writes them (+1.921fb54442d18X+001), worked out from the double in R; a missing value is Stata's . (+1.0000000000000X+3ff).",
        ),
        p("Colours", "as text, as result and as error are left out: R prints plain text."),
    ],
}
