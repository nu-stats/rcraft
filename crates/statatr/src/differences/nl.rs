Guide {
    command: "nl",
    about: "Nonlinear least squares (nl (y = {b0 = 1} * exp({b1} * x)))",
    points: &[
        p(
            "In R",
            "stata_nl fits it as Stata 19 does, worked out from its output on probes: the \
             starting values as %9.0g writes them, Gauss-Newton with exact derivatives and \
             ml's step search, ml's stopping rule on the residual SS, so its iteration log \
             and e(ic) are Stata's; the parameters in byte order (Mid before alpha); a \
             parameter whose derivative is the same on every row taken as the constant \
             (centered R2, Stata's note); e(V) s2 (J'J)^-1, t tests on N - k.",
        ),
        p(
            "Not yet",
            "The function-program form, linear combinations ({xb: x1 x2}), weights, vce(), \
             hasconstant(), initial() and the other options, and predict after nl.",
        ),
    ],
}
