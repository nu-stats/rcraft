Guide {
    command: "asclogit",
    about: "Alternative-specific conditional logit",
    points: &[
        p(
            "As Stata 19",
            "With alternative-specific variables only and noconstant it is clogit on the \
             cases: Stata 19's estimates and variance are clogit's, to the last digit. Its \
             notes (cases with no within-case variability), header, e() (e(N_case), \
             e(k_alt), e(alteqs), alt: as the equation of e(b)), e(sample) and estat \
             summarize are Stata's. Like clogit, the estimates are where Stata's \
             Newton-Raphson stops, from an ordinary logit's slopes.",
        ),
        p(
            "Not yet",
            "The alternatives' constants (without noconstant), casevars(), \
             basealternative(), weights, predict and the other estat subcommands.",
        ),
    ],
}
