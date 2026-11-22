* RCraft's Stata corpus (batch MV): made-up data; Stata 19's results are in canon_discrim_mds.recorded
* commands: canon discrim mds predict estat
* (canonical correlations and their tests, linear discriminant analysis with its classification table, predict and canonical functions, classical metric scaling with std and dimension())
* tolerance: 1e-9
version 19
set linesize 255
clear
set obs 60
generate double x1 = mod(_n * 37, 101) / 7
generate double x2 = mod(_n * 13, 17) / 3 + x1 / 4
generate double x3 = mod(_n * 11, 23) / 4 - x2 / 5
generate double y1 = x1 / 2 + mod(_n * 7, 19) / 3
generate double y2 = x2 - x3 + mod(_n * 5, 13) / 2
generate int g = 1 + mod(_n, 3)
replace x1 = x1 + 3 * (g == 2)
replace x2 = x2 - 2 * (g == 3)

* ---------------------------------------------------------------- canon
canon (y1 y2) (x1 x2 x3)
display "@ c_sc " e(df_r) " " e(N) " " e(df) " " e(n_cc) " " e(df1) " " e(df2) " " e(rank)
foreach m in stat_1 stat_2 stat_m canload11 canload22 canload12 canload21 rawcoef_var1 rawcoef_var2 stdcoef_var1 stdcoef_var2 ccorr corr_var1 corr_var2 corr_mixed {
    matrix M = e(`m')
    forvalues i = 1/`=rowsof(M)' {
        local s ""
        forvalues j = 1/`=colsof(M)' {
            local s "`s' `=string(M[`i',`j'], "%17.0g")'"
        }
        display "@ c_`m' `i'`s'"
    }
}
matrix b = e(b)
local s ""
forvalues j = 1/10 {
    local s "`s' `=string(b[1,`j'], "%17.0g")'"
}
display "@ c_b`s'"
canon (y1 y2) (x1 x2 x3), lc(1)
display "@ c_lc " e(N) " " e(n_cc)
canon (y1 y2) (x1 x2 x3), stdcoef
display "@ c_std " e(N) " " %17.0g e(df2)
canon (y1 y2) (x1 x2 x3), first(1)
display "@ c_first " e(N) " " e(rank)

* ---------------------------------------------------------------- discrim lda
discrim lda x1 x2 x3, group(g)
display "@ d_sc " e(f) " " e(N) " " e(N_groups) " " e(k)
display "@ d_txt " ("`e(subcmd)'" == "lda") ("`e(groupvar)'" == "g") ("`e(varlist)'" == "x1 x2 x3") ("`e(ties)'" == "missing") ("`e(properties)'" == "nob noV eigen")
foreach m in grouppriors groupcounts groupvalues C W_eigvecs W_eigvals candisc_stat cmeans canstruct L_totalstd L_std L_unstd L_raw Ev sqrtSinv Sinv S means SSCP_B SSCP_W SSCP_W1 SSCP_W2 SSCP_W3 SSCP_T {
    matrix M = e(`m')
    forvalues i = 1/`=rowsof(M)' {
        local s ""
        forvalues j = 1/`=colsof(M)' {
            local s "`s' `=string(M[`i',`j'], "%17.0g")'"
        }
        display "@ d_`m' `i'`s'"
    }
}
discrim lda x1 x2 x3, group(g) priors(equal) notable
estat classtable
predict cls
tabulate g cls
forvalues i = 1/3 {
    count if g == `i' & cls == 1
    local a = r(N)
    count if g == `i' & cls == 2
    local b = r(N)
    count if g == `i' & cls == 3
    display "@ d_tab `i' `a' `b' " r(N)
}
discrim lda x1 x2 x3 if g < 3, group(g) priors(proportional)
display "@ d_prop " e(N) " " e(N_groups) " " e(f)
matrix P = e(grouppriors)
display "@ d_pri " %17.0g P[1,1] " " %17.0g P[1,2]
predict cls2
count if cls2 == 1
display "@ d_c2 " r(N)

* ---------------------------------------------------------------- mds
mds x1 x2 x3, id(g)
display "@ m_dup " e(N) " " e(np) " " e(p) " " e(addcons) " " e(rc) " " %17.0g e(mardia1) " " %17.0g e(mardia2)
display "@ m_txt " ("`e(duplicates)'" == "1") ("`e(unique)'" == "0") ("`e(idtype)'" == "int")
matrix L = e(Ev)
display "@ m_ev " %17.0g L[1,1] " " %17.0g L[1,2] " " %17.0g L[1,3]
generate int id = _n
mds x1 x2 x3 in 1/10, id(id)
display "@ m_sc " e(N) " " e(np) " " e(p) " " e(addcons) " " e(rc) " " %17.0g e(mardia1) " " %17.0g e(mardia2)
display "@ m_txt2 " ("`e(duplicates)'" == "0") ("`e(unique)'" == "1") ("`e(idtype)'" == "int") ("`e(norm)'" == "principal") ("`e(dname)'" == "L2")
foreach m in Y linearf coding idcoding {
    matrix M = e(`m')
    forvalues i = 1/`=rowsof(M)' {
        local s ""
        forvalues j = 1/`=colsof(M)' {
            local s "`s' `=string(M[`i',`j'], "%17.0g")'"
        }
        display "@ m_`m' `i'`s'"
    }
}
matrix L = e(Ev)
display "@ m_ev2 " %17.0g L[1,1] " " %17.0g L[1,2] " " %17.0g L[1,3]
matrix D = e(D)
display "@ m_D " %17.0g D[1,2] " " %17.0g D[3,7] " " %17.0g D[10,4] " " D[5,5]
mds x1 x2 x3 in 1/10, id(id) dimension(3)
display "@ m_d3 " e(p) " " %17.0g e(mardia1) " " %17.0g e(mardia2)
matrix M = e(Y)
display "@ m_d3Y " %17.0g M[1,3] " " %17.0g M[4,3] " " %17.0g M[9,3]
mds x1 x2 x3 in 1/10, id(id) std
display "@ m_std " e(p) " " %17.0g e(mardia1) " " %17.0g e(mardia2)
matrix L = e(Ev)
display "@ m_stdev " %17.0g L[1,1] " " %17.0g L[1,2] " " %17.0g L[1,3]
matrix M = e(coding)
display "@ m_cod " %17.0g M[1,1] " " %17.0g M[1,2] " " %17.0g M[3,2]
capture noisily mds x1 x2 x3 in 1/10
display "@ rc " _rc
