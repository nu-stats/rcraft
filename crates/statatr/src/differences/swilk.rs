Guide {
    command: "swilk",
    about: "Shapiro-Wilk test of normality",
    points: &[
        p(
            "What agrees",
            "W is R's shapiro.test (Royston's AS R94), which differs from Stata's by about \
             5e-9 to 9e-9, so W agrees to 7 digits. V, z and the p value are worked out from W \
             by Royston's normalising transformation (for 12 to 2000 values), which gives \
             Stata's own V, z and p from Stata's own W to 16 digits; they carry W's \
             difference, up to 4e-7 relative in the p value. The printed table agrees to its \
             digits except where a value is within that rounding of a boundary.",
        ),
        p(
            "What does not",
            "With tied values Stata's W differs from Royston's by 0.003 or more, by a rule \
             not decoded; for 3 to 11 values V comes from a rule not decoded either. Both \
             stop with a message instead of printing numbers that would be off. So do more \
             than 2000 values. A string variable or fewer than 3 values print a row of dots, \
             as in Stata.",
        ),
    ],
}
