Guide {
    command: "rotate",
    about: "Rotating factor's or pca's loadings (rotate, varimax, quartimax, equamax, promax(#), normalize, blanks(#))",
    points: &[
        p(
            "In R",
            "stata_rotate does what Stata 19 was seen to do: varimax by Kaiser's angle for two \
             factors and his SVD iteration for more (stopped when the singular values' sum \
             grows by less than 1e-8 of itself), quartimax and equamax (weight p/2) by the \
             gradient projection algorithm (stopped when T changes by less than 1e-6 and the \
             gradient is below 1e-6), promax(k) from the varimax loadings, Kaiser's \
             normalization with normalize or horst. Each factor is signed so its loadings sum \
             to more than 0 and the factors are ordered by variance. Stata 19's tables, \
             blanks(), e(r_L), e(r_T), e(r_Phi), e(r_Ev) and the texts, r(rtext) and Stata's \
             errors; rotate, clear.",
        ),
        p(
            "Not yet",
            "The other criteria (oblimin, quartimin, parsimax, ...), factors(), random starts, \
             the maximize options, blanks() below 0, rotate after pca with every component \
             kept, e(r_fmin), matrix list e(r_Phi) after an orthogonal rotation, and estat \
             rotatecompare are not translated yet.",
        ),
    ],
}
