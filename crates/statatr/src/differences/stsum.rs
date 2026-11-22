Guide {
    command: "stsum",
    about: "Summarizes survival-time data: time at risk, rate, survival times",
    points: &[
        p(
            "Method",
            "Time at risk, the incidence rate, the subjects and the 25%, 50% and 75% survival \
             times (the first time the Kaplan-Meier estimate falls to the level), with by(); \
             r() holds the total's.",
        ),
        LAYOUT_R,
    ],
}
