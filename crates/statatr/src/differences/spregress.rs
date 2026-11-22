Guide {
    command: "spregress",
    about: "Spatial autoregressive models",
    points: &[p(
        "Not translated yet",
        "Stata 19 stops its likelihood iterations where its numerical derivatives lead, \
         before the maximum (by up to 1e-3 in rho on made-up data), and on some data it \
         reaches another optimum. Until that is decoded, spregress lines are marked as not \
         translated rather than given results that differ from Stata's.",
    )],
}
