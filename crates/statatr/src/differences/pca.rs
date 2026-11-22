Guide {
    command: "pca",
    about: "Principal components (pca varlist, components(#), covariance)",
    points: &[
        p(
            "In R",
            "stata_pca takes the eigenvalues and eigenvectors of the correlation (or with \
             covariance the covariance) matrix of the rows with every variable present, each \
             eigenvector signed so that its elements sum to more than 0, as Stata's manual \
             gives it, and prints Stata 19's two tables (the eigenvalues, their differences \
             and proportions; the eigenvectors kept with each variable's unexplained \
             variance). e() is pca's: e(Ev), e(L), e(Psi), e(C), e(components), e(rho), \
             e(trace), e(cond), e(lndet), e(N).",
        ),
        p(
            "Not yet",
            "Weights, the options other than components() and covariance, and what follows \
             pca (predict, estat loadings, screeplot) are not translated yet; rotate is.",
        ),
    ],
}
