* RCraft's Stata corpus (P2C.9 continued): made-up data; Stata 19's results are in matrices.recorded
* commands: insheet matrix mkmat svmat
* (Stata's matrices the do-file makes: J(), I(), joins with , and \, products, sums, a scalar times a matrix, transposes, invsym() and inv(), vecdiag() and diag(), e(b) and e(V), elements by number and by name, el(), rowsof() and colsof(), an element or a matrix put in place, nullmat() growing a matrix in a loop, rownames and colnames (quoted ones too), drop, mkmat with if, in and nomissing, and insheet, its floats and double, into a matrix, and svmat: its names, floats and double, and the rows it adds)
* tolerance: 1e-8
version 19
set linesize 255
clear
set obs 12
generate x = _n
generate z = mod(_n * 7, 5)
generate y = 1 + 2 * x - z + mod(_n, 3)
replace z = . in 5
matrix A = (1, 2 \ 3, 4)
matrix B = J(2, 3, 0.5)
matrix C = A * B
display "@ product " C[1, 1] " " C[2, 3] " " rowsof(C) " " colsof(C)
matrix D = A'
display "@ transpose " D[1, 2] " " D[2, 1]
matrix E = A + A - 2 * A / 4
display "@ sum " E[1, 1] " " E[2, 2]
matrix G = (A \ (5, 6))
matrix H = (A, (7 \ 8))
display "@ joins " G[3, 2] " " rowsof(G) " " H[2, 3] " " colsof(H)
matrix K = invsym(A' * A)
display "@ invsym " K[1, 1] " " K[2, 1] " " K[2, 2]
matrix L = inv(A)
display "@ inv " L[1, 1] " " L[1, 2] " " L[2, 1] " " L[2, 2]
matrix A[1, 2] = 9
display "@ element " A[1, 2] " " el(A, 2, 1)
matrix S1 = (2)
matrix Y = S1 * A
display "@ one by one " Y[1, 2] " " Y[2, 2]
matrix w = (1, 2 \ 3, 4)
matrix w[1, 1] = (7, 8)
matrix u = -w
matrix q = w' * w / 2 - I(2)
display "@ placed " w[1, 1] " " w[1, 2] " " u[2, 1] " " q[1, 1] " " q[2, 1] " " q[2, 2]
matrix rownames A = a b
matrix colnames A = x y
display "@ names " A["a", "y"] " " A["b", "x"]
matrix Q = (1, 2 \ 3, 4 \ 5, 6)
matrix rownames Q = "Female" "18--24" "65+"
matrix colnames Q = "Proportion" "Quota diff"
display "@ quoted " Q["18--24", "Proportion"] " " Q["65+", "Quota diff"] " " rowsof(Q)
matrix k = J(1, 3, 0)
forvalues i = 1/3 {
  matrix k[1, `i'] = `i' * 10
}
display "@ loop " k[1, 1] " " k[1, 3]
forvalues i = 1/4 {
  matrix grow = nullmat(grow) \ (`i', `i'^2, -`i')
}
display "@ nullmat " rowsof(grow) " " grow[4, 2] " " grow[3, 3]
capture matrix bad = inv((1, 2 \ 2, 4))
display "@ singular " _rc
capture matrix A[3, 1] = 1
display "@ outside " _rc
quietly regress y x z
matrix b = e(b)
matrix V = e(V)
matrix v = vecdiag(V)
matrix d = diag(v)
display "@ eb " b[1, 1] " " b[1, "z"] " " b[1, 3] " " el(b, 1, 2)
display "@ vecdiag " v[1, 1] " " v[1, "_cons"] " " d[2, 2] " " d[1, 2] " " d["z", "z"]
matrix se = (sqrt(v[1, 1]), sqrt(v[1, 2]), sqrt(v[1, 3]))
display "@ se " se[1, 1] " " se[1, 2] " " se[1, 3]
matrix bv = b * invsym(V) * b'
display "@ wald " bv[1, 1]
mkmat x y z, matrix(M)
display "@ mkmat " rowsof(M) " " colsof(M) " " M[5, 2] " " M[12, 3] " " M["r4", "y"] " " missing(M[5, 3])
mkmat x z if x > 6, matrix(N2) nomissing
display "@ mkmat if " rowsof(N2) " " N2[1, 1] " " N2[6, 2]
mkmat x y in 1/3
display "@ mkmat vectors " rowsof(x) " " colsof(x) " " y[3, 1]
matrix MM = M' * M
display "@ cross " MM[1, 1] " " MM[1, 2]
matrix drop MM
capture matrix list MM
display "@ dropped " _rc
insheet using corpus.csv, clear
mkmat x y time, matrix(F)
display "@ insheet " rowsof(F) " " colsof(F) " " F[1, 1] " " F[2, 2] " " F[60, 3]
summarize x
display "@ insheet floats " r(mean) " " r(sd) " " r(max)
insheet using corpus.csv, clear double names
summarize x
display "@ insheet double " r(mean) " " r(sd) " " r(max)
insheet using corpus.csv, clear nonames
display "@ insheet nonames " _N
clear
set obs 2
generate k = _n
matrix S = (0.1, 2 \ 3, 4 \ 5, 6)
matrix colnames S = pa pb
svmat S
svmat double S, names(col)
svmat S, names(e)
display "@ svmat " _N " " S1[1] " " S2[3] " " pa[1] " " pb[2] " " e1[3] " " missing(k[3]) " " (S1[1] == float(0.1)) " " (pa[1] == 0.1)
capture svmat S
display "@ svmat taken " _rc
