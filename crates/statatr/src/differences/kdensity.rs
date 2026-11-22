Guide {
    command: "kdensity",
    about: "Kernel density",
    points: &[
        p(
            "Kernel and bandwidth",
            "Stata's: the Epanechnikov kernel unless kernel() names another, the bandwidth \
             0.9 min(sd, IQR / 1.349) N^(-1/5) unless bwidth(), at min(N, 50) points. R's \
             density() uses a Gaussian kernel and another bandwidth.",
        ),
        GRAPH,
    ],
}
