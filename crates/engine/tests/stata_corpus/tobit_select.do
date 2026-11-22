* RCraft's Stata corpus (P2C.12ax, batch J round 2): made-up data; Stata 19's results are in tobit_select.recorded
* commands: nestreg stepwise tobit
* (nestreg and stepwise with tobit: the F and likelihood-ratio block tables, the search by Wald and LR tests, the final model's e())
* tolerance: 1e-6
version 19
set linesize 255
clear
set obs 120
generate double x1 = mod(_n * 37, 101) / 25 - 2
generate byte x2 = mod(_n * 7, 5) < 2
generate double x3 = mod(_n * 11, 37) / 10
generate double x4 = mod(_n * 17, 29) / 9
generate double e = invnormal((mod(_n * 53, 997) + 0.5) / 997)
generate double ystar = 1 + 0.8 * x1 - 0.5 * x2 + e
generate double y = max(ystar, 0)
generate double fw = 1 + mod(_n, 3)
program define shwt
    display "@ s " %24.17g e(N) " " %24.17g e(ll) " " %24.17g e(ll_0) " " %24.17g e(chi2) " " %24.17g e(F) " " %24.17g e(p) " " e(df_m) " " e(df_r) " " e(rank) " " e(k)
    display "@ c " e(N_unc) " " e(N_lc) " " e(N_rc) " " e(converged) " " e(ic)
    forvalues j = 1/`=colsof(e(b))' {
        display "@ b`j' " %24.17g el(e(b), 1, `j') " " %24.17g el(e(V), `j', `j')
    }
end
program define shwm
    args name
    matrix m = r(`name')
    forvalues i = 1/`=rowsof(m)' {
        forvalues j = 1/`=colsof(m)' {
            display "@ `name'`i'_`j' " %24.17g el(m, `i', `j')
        }
    }
end
nestreg: tobit y (x1) (x2 x3), ll(0)
shwm wald
shwt
nestreg, lr: tobit y (x1) (x2) (x3), ll(0)
shwm lr
nestreg, quietly lrtable waldtable: tobit y (x1) (x2) (x3 x4), ll(0)
shwm wald
shwm lr
nestreg: tobit y (x1) (x2), ll(0) ul(3) vce(robust)
shwm wald
stepwise, pr(.2): tobit y x1 x2 x3 x4, ll(0)
shwt
display "@ sw " (e(stepwise) == "stepwise")
stepwise, pe(.2): tobit y x1 x2 x3 x4, ll(0)
shwt
stepwise, pr(.2) lr: tobit y x1 x2 x3 x4, ll(0)
shwt
stepwise, pe(.2) lr: tobit y x1 x2 x3 x4, ll(0)
shwt
stepwise, pr(.3) pe(.2): tobit y x4 x3 x2 x1, ll(0)
shwt
stepwise, pr(.2) hierarchical: tobit y x4 x3 x2 x1, ll(0) ul(3)
shwt
stepwise, pr(.2): tobit y x1 x2 x3 x4 [fw = fw], ll(0)
shwt
stepwise, pr(.2) lockterm1: tobit y (x4 x3) x2 x1, ll(0) level(90)
shwt
