Guide {
    command: "gsort",
    about: "Sorts, descending where asked",
    points: &[p(
        "Missing values",
        "Missing values sort last ascending and first descending, as in Stata; mfirst is \
         translated for numbers, and for text the empty string counts as missing.",
    )],
}
