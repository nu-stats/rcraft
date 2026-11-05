* RCraft's Stata corpus (Step 3): made-up data; Stata 19's results are in eclass.recorded
* commands: ereturn
* (programs that estimate: ereturn post with depname, obs, dof and esample, the matrices it moves, e() and _b[] and _se[] after it, ereturn scalar, local and matrix in their order, an empty local removed, ereturn clear, ereturn scalar after regress, r(152) outside an e-class program, r(503) and r(507))
* tolerance: 1e-8
version 19
set linesize 255
clear
set obs 8
generate y = _n
generate x = mod(_n, 3)
capture program drop pfull
program pfull, eclass
  matrix b = (1.5, -2.25)
  matrix V = (0.25, 0.01 \ 0.01, 0.5)
  matrix colnames b = x _cons
  matrix colnames V = x _cons
  matrix rownames V = x _cons
  generate byte touse = _n <= 5
  matrix V2 = V
  ereturn post b V, depname(y) obs(5) dof(3) esample(touse)
  ereturn local cmd "pfull"
  ereturn matrix extra = V2, copy
  matrix C = (9)
  ereturn matrix C = C
  ereturn scalar k = 2
  ereturn local t = 2 + 3
  ereturn local w "keep"
  ereturn local w
end
pfull
display "@ post " e(N) " " e(df_r) " " ("`e(depvar)'" == "y") " " ("`e(cmd)'" == "pfull") " " ("`e(properties)'" == "b V")
display "@ coef " _b[x] " " _b[_cons] " " _se[x] " " _se[_cons]
display "@ more " e(k) " " ("`e(t)'" == "5") " " ("`e(w)'" == "") " " el(e(C), 1, 1) " " el(e(extra), 1, 2)
quietly count if e(sample)
display "@ sample " r(N)
quietly describe, short
display "@ touse_gone " r(k)
capture matrix list V
display "@ V_moved " _rc
capture matrix list C
display "@ C_moved " _rc
display "@ V2_kept " el(V2, 2, 2)
display "@ eb " el(e(b), 1, 2) " " colsof(e(b)) " " rowsof(e(V))
matrix e2 = e(V)
display "@ eV " e2[1, 2]
capture program drop pz
program pz, eclass
  matrix b = (4, 5, 6)
  ereturn post b
end
pz
display "@ default_names " _b[c1] " " _b[c3]
quietly count if e(sample)
display "@ no_sample " r(N)
capture display _se[c1]
display "@ no_V " _rc
capture display _b[nosuch]
display "@ no_name " _rc
capture program drop pclear
program pclear, eclass
  ereturn clear
  ereturn scalar only = 7
end
pclear
display "@ clear " e(only) " " e(N)
capture program drop preg
program preg, eclass
  quietly regress y x
  ereturn scalar extra = 7
  ereturn local note "hi"
end
preg
display "@ after_regress " e(extra) " " ("`e(note)'" == "hi") " " e(N) " " _b[x]
capture program drop pnot
program pnot
  ereturn scalar s = 5
end
capture pnot
display "@ not_eclass " _rc
capture ereturn local a "x"
display "@ top_level " _rc
matrix b = (1, 2)
matrix V = (1, 0, 0 \ 0, 1, 0 \ 0, 0, 1)
capture ereturn post b V
display "@ conform " _rc
matrix V = I(2)
capture ereturn post b V
display "@ names " _rc
capture ereturn post nosuch
display "@ not_found " _rc
matrix colnames V = c1 c2
matrix rownames V = c1 c2
ereturn post b V
display "@ top_post " _b[c2] " " _se[c2]
