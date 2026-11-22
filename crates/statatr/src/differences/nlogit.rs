Guide {
    command: "nlogit",
    about: "Nested logit with nlogitgen and nlogittree",
    points: &[
        p(
            "As Stata 19",
            "nlogitgen type = alt(ground: car | bus, other: train | plane) and nlogittree alt type, choice(choice), \
             then nlogit depvar vars || type: , base(group) || alt: , case(id) [noconstant technique(nr|bfgs) nolog]: \
             a two-level RUM-consistent model, e(b) in Stata's order, e(V), e(ll), the Wald test and the LR test of \
             tau = 1 (e(ll_c), e(chi2_c), e(p_c)); predict q* (the probabilities of the group and of the \
             alternative).",
        ),
        p(
            "Differences",
            "Stata's default bfgs stops short of the optimum, so its estimates differ from R's in about the 5th \
             digit (technique(nr) agrees to about 1e-6); iteration logs, e(ic) and e(gradient) are not reproduced.",
        ),
        p(
            "Not yet",
            "More than two levels, type-level variables together with alternative constants, nonnormalized and the \
             other options, estat.",
        ),
    ],
}
