* RCraft's Stata corpus (batch MV): made-up data; Stata 19's results are in manova.recorded
* commands: manova
* (one-way, two-way, interaction and a continuous covariate: the four statistics, e() scalars and matrices, if, and the error without =)
* tolerance: 1e-9
version 19
set linesize 255
clear
set obs 60
generate double x1 = mod(_n * 37, 101) / 7
generate double x2 = mod(_n * 13, 17) / 3 + x1 / 4
generate double x3 = mod(_n * 11, 23) / 4 - x2 / 5
generate int g = 1 + mod(_n, 3)
generate int h = 1 + mod(_n * 7, 2)
generate double c = mod(_n * 5, 13) / 2

manova x1 x2 x3 = g
display "@ a_sc " e(N) " " e(k) " " e(rank) " " e(df_m) " " e(df_r) " " e(k_eq) " " e(df_1)
display "@ a_p " ("`e(p_F)'" == "0.7765 0.8599 0.9733") ("`e(F)'" == ".2540456 .1513038 .0270932") ("`e(r2)'" == "0.0088 0.0053 0.0009") ("`e(rmse)'" == "4.246411 2.026936 1.709539") ("`e(depvar)'" == "x1 x2 x3") ("`e(term_1)'" == "g")
foreach m in stat_1 H_1 eigvals_1 aux_1 stat_m H_m eigvals_m aux_m E B xpxinv {
    matrix M = e(`m')
    forvalues i = 1/`=rowsof(M)' {
        local s ""
        forvalues j = 1/`=colsof(M)' {
            local s "`s' `=string(M[`i',`j'], "%17.0g")'"
        }
        display "@ a_`m' `i'`s'"
    }
}
matrix V = e(V)
matrix b = e(b)
forvalues k = 1/3 {
    local s ""
    forvalues j = 1/4 {
        local s "`s' `=string(b[1,(`k'-1)*4+`j'], "%17.0g")' `=string(V[(`k'-1)*4+`j',(`k'-1)*4+`j'], "%17.0g")'"
    }
    display "@ a_bV`k'`s'"
}
display "@ a_cross " %17.0g V[2,6] " " %17.0g V[8,12]

manova x1 x2 = g
display "@ b_sc " e(N) " " e(rank) " " e(df_m) " " e(df_r)
matrix S = e(stat_1)
display "@ b_s1 " %17.0g S[1,1] " " %17.0g S[2,2] " " %17.0g S[3,3] " " %17.0g S[4,4]

manova x1 x2 x3 = g h
display "@ c_sc " e(N) " " e(rank) " " e(df_m) " " e(df_r) " " e(df_1) " " e(df_2)
display "@ c_t " ("`e(term_1)'" == "g") ("`e(term_2)'" == "h") ("`e(indepvars)'" == "g h")
foreach m in stat_1 stat_2 stat_m H_2 aux_2 eigvals_2 {
    matrix M = e(`m')
    forvalues i = 1/`=rowsof(M)' {
        local s ""
        forvalues j = 1/`=colsof(M)' {
            local s "`s' `=string(M[`i',`j'], "%17.0g")'"
        }
        display "@ c_`m' `i'`s'"
    }
}

manova x1 x2 x3 = g##h
display "@ d_sc " e(N) " " e(rank) " " e(df_m) " " e(df_r) " " e(df_1) " " e(df_2) " " e(df_3)
display "@ d_t " ("`e(term_1)'" == "g") ("`e(term_2)'" == "h") ("`e(term_3)'" == "g#h")
foreach m in stat_1 stat_2 stat_3 stat_m B {
    matrix M = e(`m')
    forvalues i = 1/`=rowsof(M)' {
        local s ""
        forvalues j = 1/`=colsof(M)' {
            local s "`s' `=string(M[`i',`j'], "%17.0g")'"
        }
        display "@ d_`m' `i'`s'"
    }
}
matrix X = e(xpxinv)
display "@ d_x " %17.0g X[2,2] " " %17.0g X[9,9] " " %17.0g X[11,9] " " %17.0g X[12,12] " " %17.0g X[13,13]

manova x1 x2 x3 = g c.c
display "@ e_sc " e(N) " " e(rank) " " e(df_m) " " e(df_r) " " e(df_1) " " e(df_2)
display "@ e_t " ("`e(term_1)'" == "g") ("`e(term_2)'" == "c")
foreach m in stat_1 stat_2 stat_m B {
    matrix M = e(`m')
    forvalues i = 1/`=rowsof(M)' {
        local s ""
        forvalues j = 1/`=colsof(M)' {
            local s "`s' `=string(M[`i',`j'], "%17.0g")'"
        }
        display "@ e_`m' `i'`s'"
    }
}

manova x1 x2 x3 = g if h == 1
display "@ f_sc " e(N) " " e(rank) " " e(df_m) " " e(df_r)
matrix S = e(stat_1)
display "@ f_s " %17.0g S[1,1] " " %17.0g S[2,2] " " %17.0g S[3,5] " " %17.0g S[4,4]

capture noisily manova x1 x2 x3
display "@ rc " _rc
