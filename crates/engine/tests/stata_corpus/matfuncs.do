* RCraft's Stata corpus (Step 3): made-up matrices; Stata 19's results are in matfuncs.recorded
* commands: matrix
* functions: trace det diag0cnt issymmetric matmissing mreldif colnumb rownumb
* (the Kronecker product and its eq:name names, submatrices by number, name, .. and ..., cholesky(), corr(), hadamard(), vec(), vech(), sweep(), get(_b) and get(VCE), invsym() of a singular matrix, and the functions that give a number from a matrix)
* tolerance: 1e-8
version 19
set linesize 255
clear
matrix A = (4, 2 \ 2, 3)
matrix rownames A = a b
matrix colnames A = x y
matrix B = (1, 2, 3 \ 4, 5, 6)
matrix colnames B = p q r
matrix K = A # B
display "@ kron " rowsof(K) " " colsof(K) " " el(K, 2, 3) " " el(K, 4, 5) " " rownumb(K, "a:r2") " " colnumb(K, "y:q")
matrix K2 = (1, 2) # (3 \ 4)
display "@ kron_default " el(K2, 2, 2) " " rownumb(K2, "r1:r2") " " colnumb(K2, "c2:c1")
matrix L1 = A # (1, 2 \ 3, 4)
display "@ kron_typed " rownumb(L1, "b:r4") " " colnumb(L1, "y:c4") " " el(L1, 4, 4)
matrix L3 = A # J(2, 2, 1)
display "@ kron_J " rownumb(L3, "b:r2") " " colnumb(L3, "y:c2") " " el(L3, 3, 1)
capture matrix K3 = 2 # A
display "@ kron_scalar " _rc
matrix S = B[1..2, 2..3]
display "@ sub " rowsof(S) " " colsof(S) " " el(S, 2, 1) " " colnumb(S, "q") " " rownumb(S, "r2")
matrix S2 = B[2, 1...]
display "@ sub_end " colsof(S2) " " el(S2, 1, 3) " " rownumb(S2, "r2")
matrix S3 = B[1..., "q".."r"]
display "@ sub_names " colsof(S3) " " el(S3, 2, 2) " " colnumb(S3, "r")
matrix S4 = A["b", "x"]
display "@ sub_one " el(S4, 1, 1) " " rownumb(S4, "b") " " colnumb(S4, "x")
capture matrix S5 = B[1..3, 1]
display "@ sub_outside " _rc
capture matrix S6 = B[2..1, 1]
display "@ sub_backwards " _rc
matrix E = A[1, 1..2] # B[1..2, 1]
display "@ sub_kron " el(E, 2, 2) " " colnumb(E, "y:p")
matrix Y = (1, 2 \ 3, 4)
matrix Y2 = Y[2..., 1...]'
display "@ sub_t " rowsof(Y2) " " el(Y2, 2, 1) " " rownumb(Y2, "c2") " " colnumb(Y2, "r2")
matrix C = cholesky(A)
display "@ cholesky " el(C, 1, 1) " " el(C, 1, 2) " " el(C, 2, 1) " " el(C, 2, 2) " " rownumb(C, "b") " " colnumb(C, "y")
capture matrix C2 = cholesky((1, 2 \ 2, 1))
display "@ cholesky_not_pd " _rc
matrix R = corr(A)
display "@ corr " el(R, 2, 1) " " el(R, 2, 2) " " rownumb(R, "b")
matrix H = hadamard(B, B)
matrix colnames H = u v w
matrix H2 = hadamard(H, B)
display "@ hadamard " el(H2, 2, 3) " " colnumb(H2, "w") " " colnumb(H2, "r")
matrix V1 = vec(B)
display "@ vec " rowsof(V1) " " el(V1, 2, 1) " " el(V1, 5, 1) " " rownumb(V1, "q:r2") " " colnumb(V1, "c1")
matrix V2 = vech(A)
display "@ vech " rowsof(V2) " " el(V2, 3, 1) " " rownumb(V2, "x:b") " " rownumb(V2, "y:b")
matrix W = sweep(A, 1)
display "@ sweep " el(W, 1, 1) " " el(W, 1, 2) " " el(W, 2, 1) " " el(W, 2, 2) " " rownumb(W, "x") " " colnumb(W, "a")
matrix G = invsym((4, 2, 2 \ 2, 1, 1 \ 2, 1, 3))
display "@ invsym_singular " el(G, 1, 1) " " el(G, 1, 3) " " el(G, 2, 2) " " el(G, 3, 3) " " diag0cnt(G)
matrix G2 = invsym((0, 0 \ 0, 2))
display "@ invsym_zero " el(G2, 1, 1) " " el(G2, 2, 2)
matrix Z = (1, 0 \ 0, 0)
matrix M = (1, .)
display "@ numbers " trace(A) " " det(A) " " diag0cnt(Z) " " issymmetric(A) " " issymmetric(B) " " matmissing(A) " " matmissing(M)
display "@ not_square " trace(B) " " det(B)
matrix A2 = A * 1.01
display "@ mreldif " mreldif(A, A2)
display "@ numb " colnumb(B, "q") " " rownumb(A, "b") " " colnumb(B, "zz") " " rownumb(A, "r2")
matrix D = (2, 1 \ 1, 2)
matrix rownames D = u v
display "@ symmetric_names " issymmetric(D)
capture display trace(nosuch)
display "@ not_there " _rc
set obs 20
generate x = mod(_n * 7, 11)
generate y = 2 + 0.5 * x + mod(_n, 4)
quietly regress y x
matrix gb = get(_b)
matrix gv = get(VCE)
display "@ get " el(gb, 1, 1) " " el(gv, 2, 2) " " colnumb(gb, "x")
display "@ of_e " trace(e(V)) " " colnumb(e(b), "_cons") " " issymmetric(e(V))
