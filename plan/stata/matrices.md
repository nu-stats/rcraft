# Stata's matrices the do-file makes (2026-11-04)

What Stata 19 does, read off made-up probes (never its ado or help files), and what RCraft does.

## Kept in R

`stata_mat`, a list of R matrices with Stata's row and column names (`data_r/mexpr.rs`, `MATRIX_HELPER`).
The translator knows which names it holds (`user_mats`) and which a line it did not translate may have
changed (`lost_mats`: a `matrix` line's target, `mkmat`'s matrix, matrix options, and any name a refused
line of another command mentions, since an add-on such as `mat_rapp AB : A B` makes one); those are
declined from then on, `nullmat()` of them too. A matrix never made otherwise stops R where Stata stops
(r(111)), so `capture` gives `_rc` 111 as in Stata.

## Names (Stata 19)

- Default names are `r1, r2, ...` and `c1, c2, ...`.
- A product takes the left one's rows and the right one's columns; a transpose swaps them.
- A sum or difference takes the right one's names (`P + Q` has Q's, `Q + P` has P's).
- A number or a 1x1 matrix times a matrix is a scalar product, with the matrix's names.
- Joins with `,` and `\` keep each part's names; when every name is a default one they are numbered
  again (`(A \ (5, 6))` has r1..r3). The other side's names come from the first part.
- `invsym()` and `inv()` swap the names. `invsym()` of a singular matrix gives a particular generalized
  inverse: declined. `inv()` of one stops with "matrix has missing values", r(504).
- `vecdiag(V)`: one row named r1, V's column names. `diag(v)`: the vector's names on both sides, or r1..
  and c1.. when it has Stata's defaults.
- `matrix list` says `symmetric` when the values are symmetric, whatever the names (I(3) is).
- An element put outside the matrix: "conformability error", r(503). `A[i,j] = M` puts M with its corner
  at (i, j).
- `nullmat(name)` of no matrix outside a join: r(111).
- `mkmat` rows are r1, r2, ...; without `matrix()` each variable is a column vector named after it.
- `svmat`: A1, A2, ... (or the column names, or a stub), floats unless double; more rows than the data
  add observations ("number of observations will be reset to n"); a name taken stops, r(110).

## More functions (2026-11-05)

- `A # B`: rows and columns `eq:name`, A's names the equations and B's the names. When B is typed in
  place (`A # (1, 2 \ 3, 4)`) its names are numbered across the result (`b:r3 b:r4`, `y:c3 y:c4`); a
  named B, `J()`, `I()`, `B'` and `B * 2` keep theirs (`b:r1 b:r2`). A typed-in-place A keeps its
  default names as equations. A number on either side is "type mismatch", r(109).
- Submatrices: `B[1..2, 2..3]`, `B[2, 1...]`, `B[1..., "q".."r"]`, `A["b", "x"]` keep their rows' and
  columns' names. A range past the matrix or backwards is r(503); `B[1.., 2]` is "invalid syntax". An
  expression with a submatrix inside `el()` is refused by Stata ("matrix operators that return matrices
  not allowed in this context").
- `cholesky(A)`: the lower triangle, A's names; not positive definite is r(506). `corr(A)`: A's names.
  `hadamard(A, B)`: the left one's names. `vec(B)`: one column `c1`, rows `col:row` column by column;
  `vech(A)`: the lower triangle the same way. `sweep(A, k)`: row k's name becomes column k's and the
  other way round.
- `invsym()` of a singular matrix: the generalized inverse StatCraft decoded (pivots in order, one not
  positive or at or below 1e-13 of its first value left out, its row and column 0):
  `invsym((4, 2, 2 \ 2, 1, 1 \ 2, 1, 3))` is `(.375, 0, -.25 \ 0, 0, 0 \ -.25, 0, .5)`.
- Numbers: `trace()` and `det()` of a matrix that is not square are missing, with no error;
  `issymmetric()` looks at the values, not the names; `matmissing()`; `mreldif(X, Y)` is the largest
  |x - y| / (|y| + 1); `colnumb()` and `rownumb()` are missing for a name not there. They take a
  matrix's name or `e(V)`; `trace(A')` is "A' not found", r(111).
- A `.` in a matrix typed in place is a missing value.
- A matrix typed in place has no names of its own: `invsym((4, 2 \ 2, 3))` has rows r1.. and columns
  c1.., where a stored matrix with default names has them swapped (rows c1.., columns r1..), singular or
  not.

## Declined

`matuniform()`, the Kronecker product of matrices joined in place or with equation names, `sweep()` on a
zero pivot, row or column names with equations or factor variables, `matrix accum` and the other
subcommands not listed here, `svmat` with names(eqcol) or names(matcol).

## insheet

Read off Stata 19: a tab in the first line makes it tab, else comma; names from the first row when no
cell is a number, lowercase with what is not a letter, digit or _ left out, the header kept as the label
where it differs (`case` keeps the case); v1, v2, ... when every cell is a number; decimals as float
unless `double`; `(k vars, n obs)`; without `clear` and data in memory, r(18). A first row of both, names
Stata would make up and an empty column stop rather than guess.

## The dry run

A loop whose range is known only when the do-file runs no longer leaves the macros it sets with their old
values; they are known only when it runs. A line refused for what such a line leaves (a variable, a
scalar, a matrix) counts as known when run, not as not translated.
