Guide {
    command: "nestreg",
    about: "Nested regression: blocks of variables added in turn, for regress, logit, probit and poisson",
    points: &[
        p(
            "As Stata 19",
            "Each block's model with its own output, then the Wald table (F, R2 and the change \
             in R2 for regress; Wald chi2 for the others), the likelihood-ratio table with lr or \
             lrtable, or both with waldtable; r(wald) and r(lr), store(), if and in, level(), \
             vce(robust) and vce(cluster) for regress, e() of the last model with its \
             e(cmdline), and Stata's errors with their return codes.",
        ),
        p(
            "Differences",
            "Weights, factor variables, other models and options, and a variable listed twice \
             (where Stata notes it and goes on) are not translated; a collinear model, or a \
             number too wide for its column of the table, stops the line when it runs, saying \
             so. The likelihood models agree with Stata to about 7 digits. The change in R2 \
             of block 1 in r(wald) is a plain missing value, which `matrix list` shows as `.` \
             where Stata shows `.z`.",
        ),
    ],
}
