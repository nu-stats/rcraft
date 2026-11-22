Guide {
    command: "matrix",
    about: "Matrices: define, elements, names, list",
    points: &[
        p(
            "Columns",
            "e(b) and e(V) have Stata's columns (base levels as zeros, o. for omitted, the \
             equations), printed in Stata's layout at a linesize of 80.",
        ),
        p(
            "The do-file's matrices",
            "Kept in R as stata_mat, with Stata's row and column names: r1, r2, ... and c1, \
             c2, ... by default; a product takes the left one's rows and the right one's \
             columns, a sum the right one's names, a transpose swaps them, and joins with , \
             and \\ number default names again (read off Stata 19). A 1x1 matrix multiplies \
             as a number.",
        ),
        p(
            "Functions",
            "The Kronecker product # (names eq:name, numbered across the result when the \
             right one is typed in place), submatrices (A[1..2, 3...], A[\"a\"..\"b\", 1]), \
             cholesky(), corr(), hadamard(), vec(), vech(), sweep(), get(_b), get(VCE), and \
             invsym() of a singular matrix as Stata's generalized inverse (swept in order, \
             a pivot at or below 1e-13 of its first value left out as 0). trace(), det(), \
             diag0cnt(), issymmetric(), matmissing(), mreldif(), colnumb() and rownumb() \
             give numbers, missing where Stata's are. matuniform() and the Kronecker \
             product of matrices joined in place are declined.",
        ),
        p(
            "When a matrix is not known",
            "A matrix that a line not translated may have changed is declined from then on, \
             never guessed; nullmat() of it as well.",
        ),
    ],
}
