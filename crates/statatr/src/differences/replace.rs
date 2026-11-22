Guide {
    command: "replace",
    about: "Changes the values of a variable",
    points: &[
        p(
            "Storage type",
            "replace keeps a float a float in Stata; R keeps full precision unless the \
         do-file made the variable (also when it saved the data and read it back with use \
         or merge), with a note.",
        ),
        p(
            "Message",
            "Stata 19's count of what changed: (7 real changes made), (1 real change made, 1 \
             to missing) for numbers (a . made .a is a change), counted group by group under \
             by; nothing under quietly or capture.",
        ),
    ],
}
