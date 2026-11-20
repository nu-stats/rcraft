* RCraft's Stata corpus (P2C.12, batch E): made-up data; Stata 19's results are in cnsreg.recorded
* commands: constraint cnsreg
* (constraint define, list, get, free and drop; cnsreg with one and several constraints, constraints on the constant, weights, robust and clustered errors, if, level(), noconstant, a bare cnsreg to show it again, predict and test after it, e(b), e(V), e(Cns), the F test that goes when the constraints contradict the slopes, factor variables (the base level a fixed column, 1.g as 1b.g, fixed-at-0 shown as omitted), and the errors with their return codes)
* tolerance: 1e-8
version 19
set linesize 255
clear
set obs 40
generate double x1 = mod(_n * 37, 101) / 7
generate double x2 = mod(_n * 13, 17) / 3
generate double x3 = mod(_n * 11, 23) / 4
generate double y = 2 + .5 * x1 + .3 * x2 - .2 * x3 + mod(_n * 7, 11) / 5
generate int g = 1 + mod(_n, 4)
generate int w = 1 + mod(_n, 3)
generate double w2 = .5 + mod(_n * 7, 13) / 10
program define shwcns
    display "@ s " %24.17g e(N) " " %24.17g e(df_r) " " %24.17g e(rmse) " " %24.17g e(ll) " " e(rank)
    display "@ f " %24.17g e(df_m) " " %24.17g e(F) " " %24.17g e(p)
    forvalues j = 1/`=colsof(e(b))' {
        display "@ b`j' " %24.17g el(e(b), 1, `j') " " %24.17g el(e(V), `j', `j') " " %24.17g el(e(V), 1, `j')
    }
    forvalues i = 1/`=rowsof(e(Cns))' {
        forvalues j = 1/`=colsof(e(Cns))' {
            display "@ c`i'_`j' " %24.17g el(e(Cns), `i', `j')
        }
    }
end
constraint define 1 x1 = x2
constraint 2 x3 = -0.2
constraint 5 _cons = 2
constraint get 2
display "@ g1 " r(defined) " " (r(contents) == "x3 = -0.2")
constraint get 7
display "@ g2 " r(defined)
constraint free
display "@ g3 " real(r(free))
constraint list 1 2
cnsreg y x1 x2 x3, constraints(1)
shwcns
display "@ r1 " (e(cmd) == "cnsreg") " " (e(title) == "Constrained linear regression") " " (e(depvar) == "y") " " (e(vce) == "ols")
cnsreg y x1 x2 x3, constraints(1 2)
shwcns
cnsreg y x1 x2 x3, constraints(1/2)
shwcns
cnsreg y x1 x2 x3 [aweight = w], constraints(1)
shwcns
cnsreg y x1 x2 x3 [aweight = w2], constraints(1)
shwcns
cnsreg y x1 x2 x3 [fweight = w], constraints(1)
shwcns
cnsreg y x1 x2 x3 [pweight = w], constraints(1)
shwcns
cnsreg y x1 x2 x3, constraints(1) vce(robust)
shwcns
display "@ r2 " (e(vcetype) == "Robust") " " (e(vce) == "robust")
cnsreg y x1 x2 x3, constraints(1) vce(cluster g)
shwcns
display "@ r3 " e(N_clust) " " (e(clustvar) == "g") " " (e(vce) == "cluster")
cnsreg y x1 x2 x3 if g < 4, constraints(1) level(90)
shwcns
cnsreg y x1 x2 x3, constraints(1 5)
shwcns
cnsreg y x1 x2 x3, constraints(1) noconstant
shwcns
cnsreg
display "@ r4 " e(N) " " e(df_r)
predict yhat
predict r, residuals
summarize yhat r
display "@ p1 " %24.17g r(mean) " " %24.17g r(sd) " " %24.17g r(min) " " %24.17g r(max)
test x1
display "@ t1 " %24.17g r(F) " " %24.17g r(p) " " r(df) " " r(df_r)
constraint 3 x1 + x2 = 1
constraint 4 x1 + x2 = 2
capture noisily cnsreg y x1 x2 x3
display "@ e1 " _rc
capture noisily cnsreg y x1 x2 x3, constraints(9)
display "@ e2 " _rc
capture noisily cnsreg y x1 x2 x3, constraints(3 4)
display "@ e3 " _rc
constraint 6 2*x1 - x2 = x3 + 1
cnsreg y x1 x2 x3, constraints(6)
shwcns
constraint 11 x2 = x1
constraint 12 x1 + 2*x2 - 3 = x3
constraint 13 .5*x1 = x2
constraint 14 x1 = (x2 + x3)/2
constraint 15 3 = x1
cnsreg y x1 x2 x3, constraints(11)
shwcns
cnsreg y x1 x2 x3, constraints(12)
shwcns
cnsreg y x1 x2 x3, constraints(13)
shwcns
cnsreg y x1 x2 x3, constraints(14)
shwcns
cnsreg y x1 x2 x3, constraints(15)
shwcns
constraint 16 _b[x1] = 1
cnsreg y x1 x2 x3, constraints(16)
shwcns
* equal constraints count once; a constraint on a variable the model lacks is dropped with a note
constraint 17 x1 - x2 = 0
cnsreg y x1 x2 x3, constraints(1 17)
shwcns
capture noisily cnsreg y x1 x2 x3, constraints(1 17 11)
display "@ e10 " _rc
constraint 20 x4 = 1
capture noisily cnsreg y x1 x2 x3, constraints(20)
display "@ e4 " _rc
capture noisily cnsreg y x1 x2 x3, constraints(1 20)
display "@ e5 " _rc
constraint 21 x1 = x1
capture noisily cnsreg y x1 x2 x3, constraints(21)
display "@ e6 " _rc
constraint 22 x1 * x2 = 0
capture noisily cnsreg y x1 x2 x3, constraints(22)
display "@ e7 " _rc
constraint 23 [y]x1 = 1
capture noisily cnsreg y x1 x2 x3, constraints(23)
display "@ e8 " _rc
capture noisily cnsreg y x1 x2 x3 if g == 9, constraints(1)
display "@ e9 " _rc
constraint drop 3/4
constraint drop 20
constraint list
constraint drop _all
constraint get 1
display "@ g4 " r(defined)
* factor variables: the base level is a column fixed at 0 (a row of e(Cns), not a line of the header), 1.g reads 1b.g,
* a coefficient fixed at 0 shows as omitted, one fixed elsewhere as constrained
generate byte hh = mod(floor(_n / 3), 2)
generate byte cg = 1 + mod(floor((_n - 1) / 4), 10)
constraint define 11 2.g = 3.g
constraint define 12 1.g = 0
constraint define 13 _b[2.g] = .5
constraint define 14 x1 = 0
constraint define 15 1.g = 2.g
constraint define 16 2.g + 3.g = 1
constraint define 17 x2 = 2*x3
cnsreg y x1 i.g, constraints(11)
shwcns
display "@ r4 " (e(cmdline) == "cnsreg y x1 i.g, constraints(11)")
cnsreg y x1 i.g, constraints(12)
shwcns
cnsreg y x1 i.g, constraints(13)
shwcns
cnsreg y x1 x2 i.g, constraints(14)
shwcns
cnsreg y x1 x2, constraints(14)
shwcns
cnsreg y x1 ib2.g, constraints(11)
shwcns
cnsreg y x1 i.g, constraints(15)
shwcns
cnsreg y x1 x3 i.g, constraints(17 16)
shwcns
cnsreg y x1 i.g i.hh, constraints(11)
shwcns
cnsreg y x1 i.g [aweight = w], constraints(11)
shwcns
cnsreg y x1 i.g, constraints(11) vce(robust)
shwcns
cnsreg y x1 i.g, constraints(11) vce(cluster cg)
shwcns
predict double pf1
predict double rf1, residuals
summarize pf1
display "@ pf1 " %24.17g r(mean) " " %24.17g r(sd) " " %24.17g r(min) " " %24.17g r(max)
summarize rf1
display "@ rf1 " %24.17g r(sd) " " %24.17g r(min) " " %24.17g r(max)
test x1
display "@ tf1 " %24.17g r(F) " " r(df) " " r(df_r) " " %24.17g r(p)
cnsreg y x1 i.g, constraints(11)
test 2.g
display "@ tf2 " %24.17g r(F) " " r(df) " " r(df_r) " " %24.17g r(p)
