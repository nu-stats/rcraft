Guide {
    command: "svy",
    about: "Survey design and estimation (svyset, svydescribe, svy:)",
    points: &[
        p(
            "As Stata 19",
            "svyset with strata(), fpc(), singleunit(), later stages (a stage with no fpc \
             drops those after it, with Stata's note), pweight or iweight, and the r() it \
             leaves; svydescribe: the first stage's table with and without a varlist, \
             finalstage for one stage, if and in, and r(). svy: clogit as before (the weighted \
             estimates on Stata's Newton-Raphson path, the variance linearized over the PSUs, \
             the adjusted Wald F, predict, margins with vce(unconditional)). svy: mean, total, \
             ratio and proportion under strata, fpc as a count or a rate, the four singleunit() \
             rules, subpop() as a variable or an if, level(), over() of one variable for mean \
             and total: the estimates, the stratified linearized variance (PSU totals about \
             their stratum's mean, n/(n - 1), 1 - f), design df = PSUs - strata, the header \
             with the design and the counts, the notes under the table, and e() as Stata's.",
        ),
        p(
            "Not yet",
            "svy: regress, logit, probit and poisson; designs with later stages under svy:; \
             over() with several variables, or with ratio, proportion or subpop(); svyset's \
             replicate-weight and other variance methods and poststrata; svydescribe for later \
             stages, single and generate(); a stratum sampled completely; e(V_srs) and the \
             other SRS matrices.",
        ),
    ],
}
