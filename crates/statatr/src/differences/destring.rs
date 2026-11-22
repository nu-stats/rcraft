Guide {
    command: "destring",
    about: "Text to numbers",
    points: &[p(
        "Non-numbers",
        "A variable with text that is not a number is left as it is, with Stata's message \
         (contains nonnumeric characters), unless force or ignore() says otherwise.",
    )],
}
