Guide {
    command: "mds",
    about: "Classical metric multidimensional scaling",
    points: &[
        p(
            "As Stata 19",
            "mds x1 x2 x3, id(id) [dimension(#) std]: L2 distances (standardized variables as Stata holds them, in \
             float), the eigenvalue table, Mardia fit measures and e() including Y, D, linearf and coding, checked \
             against Stata 19 to 1e-9. No variables are created and no plot is drawn.",
        ),
        p(
            "Not yet",
            "Modern and nonmetric scaling, other dissimilarity measures, loss() and transform(), generate(), \
             mdsmat, mdslong, estat and the plots. The eigenvalues that are zero in exact arithmetic in e(Ev) \
             are noise (about 1e-14) in Stata and in R, and differ.",
        ),
    ],
}
