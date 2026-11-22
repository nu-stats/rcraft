Guide {
    command: "drawnorm",
    about: "Draws from a multivariate normal",
    points: &[p(
        "As Stata 19",
        "Stata's draws, bit for bit: its rnormal() stream taken a variable at a time, the \
         deviations L z (L the lower Cholesky factor of cov()) kept in float, the means added; \
         a float variable is rounded again. n() with clear, means() and cov() as matrices \
         (e(b), e(V) too), double; corr(), sds() and seed() are not translated yet.",
    )],
}
