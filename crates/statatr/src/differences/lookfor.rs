Guide {
    command: "lookfor",
    about: "Finds variables by words in their name or label",
    points: &[
        p(
            "As Stata 19",
            "lookfor word ... finds the variables whose name or variable label holds any of the words, capitals \
             ignored, a quoted phrase as one word, and leaves them in r(varlist). No words is r(198).",
        ),
        p(
            "In R",
            "Stata prints describe's rows; R has no storage types or display formats, so RCraft prints the name, \
             R's class and the label.",
        ),
    ],
}
