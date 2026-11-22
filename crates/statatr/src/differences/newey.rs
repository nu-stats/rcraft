Guide {
    command: "newey",
    about: "Regression with Newey-West standard errors",
    points: &[p(
        "As Stata 19",
        "Least squares, with V = n/(n - k) (X'X)^-1 S (X'X)^-1: the White sum plus the \
         Bartlett-weighted cross products of observations l time units apart (StatCraft's \
         decoding). Plain variable names only for now.",
    )],
}
