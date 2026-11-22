Guide {
    command: "winsor",
    about: "Winsorizes a variable (winsor, SSC)",
    points: &[
        p(
            "As Stata 19",
            "generate() with p() or h(): over the nonmissing values of if and in, the h lowest \
             become the (h+1)th smallest and the h highest the (h+1)th largest (high or low \
             for one end); h is h() or floor(p() N). The new variable keeps the old one's \
             type and format, is missing outside if and in, and is labelled as the add-on \
             labels it; its errors. Observed on made-up data; the add-on's program was not \
             read.",
        ),
        p("Not yet", "String variables (which the add-on takes)."),
    ],
}
