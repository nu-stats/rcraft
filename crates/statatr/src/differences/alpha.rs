Guide {
    command: "alpha",
    about: "Cronbach's alpha and the scale of the items",
    points: &[
        p(
            "As Stata 19",
            "generate(): in each row the mean of the items present (min(), casewise), an item \
             with a negative loading on the first principal factor reversed unless asis (as \
             Stata chose in 200 of 200 data sets), std standardizing each item first; summed \
             in float as Stata does, so the scale is Stata's to the bit. With casewise, \
             r(alpha), r(cov) and r(rho) as Stata's.",
        ),
        p(
            "Not yet",
            "r(alpha) and r(cov) without casewise (Stata pairs the data in a way not pinned \
             down yet); item, label, detail, reverse(), weights. Printed in R's layout.",
        ),
    ],
}
