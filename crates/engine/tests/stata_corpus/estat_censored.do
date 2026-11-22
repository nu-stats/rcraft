* RCraft's Stata corpus (P2C.12be, estat after the censored models): made-up data; Stata 19's results are in estat_censored.recorded
* commands: cloglog heckman hetprobit intreg tobit truncreg
* (estat ic, vce and summarize after the six: r(S), r(V), r(stats), n() and df(), weights, robust, clusters, twostep, the refused subcommands)
* tolerance: 1e-5
version 19
set linesize 255
clear
set obs 300
generate double x1 = mod(_n * 37, 101) / 25 - 2
generate double x2 = mod(_n * 29, 53) / 53
generate double z1 = invnormal((mod(_n * 41, 307) + 0.5) / 307)
generate byte z2 = mod(_n * 7, 5) < 2
generate byte id = 1 + mod(_n, 15)
generate double e1 = invnormal((mod(_n * 53, 997) + 0.5) / 997)
generate double u = invnormal((mod(_n * 71, 991) + 0.5) / 991)
generate double e2 = 0.5 * e1 + sqrt(0.75) * u
generate byte s = 0.3 + 0.8 * z1 + 0.5 * z2 + 0.4 * x1 + e2 > 0
generate double ystar = 1 + 0.8 * x1 - 0.5 * x2 + e1
generate double y = max(ystar, 0)
generate double yh = 1 + 0.7 * x1 - 0.4 * x2 + e1 if s
generate double lo = floor(ystar)
generate double hi = lo + 1
replace lo = . in 1/6
generate double ys = ystar if ystar > 0
generate byte yb = (0.3 + 0.7 * x1 - 0.6 * x2 + exp(0.8 * z1) * e1) > 0
generate byte yc = (e2 + 0.5 * x1) > 0.3
generate double fw = 1 + mod(_n, 3)
generate double pw = 0.5 + mod(_n, 4) / 2
label variable x1 "the first x"
program define eic
    estat ic
    matrix SS = r(S)
    display "@ ic " %20.12g SS[1,1] " " %20.12g SS[1,2] " " %20.12g SS[1,3] " " %20.12g SS[1,4] " " %20.12g SS[1,5] " " %20.12g SS[1,6]
end
program define evc
    args cor
    if "`cor'" == "" {
        estat vce
    }
    else {
        estat vce, correlation
    }
    matrix VV = r(V)
    local k = rowsof(VV)
    display "@ vk `k' " colsof(VV)
    forvalues i = 1/`k' {
        forvalues j = 1/`i' {
            display "@ v`i'_`j' " %20.12g VV[`i',`j']
        }
    }
end
program define esu
    matrix TT = r(stats)
    local k = rowsof(TT)
    display "@ sk `k' " colsof(TT)
    forvalues i = 1/`k' {
        display "@ s`i' " %20.12g TT[`i',1] " " %20.12g TT[`i',2] " " %20.12g TT[`i',3] " " %20.12g TT[`i',4]
    }
end
program define allest
    eic
    evc
    evc cor
    estat summarize
    esu
    estat summarize, equation
    esu
    estat summarize, labels
    esu
    estat summarize, noheader
    estat summarize x2 s
    esu
end
* tobit
quietly tobit y x1 x2, ll(0)
allest
estat ic, n(100)
eic
estat ic, df(7)
eic
estat ic, n(1000) df(5)
eic
quietly tobit y x1 x2 [fweight = fw], ll(0)
allest
quietly tobit y x1 x2 [pweight = pw], ll(0)
allest
quietly tobit y x1 x2 [iweight = pw], ll(0)
allest
quietly tobit y x1 x2, ll(0) vce(robust)
allest
quietly tobit y x1 x2, ll(0) vce(cluster id)
allest
quietly tobit y x1 x2 [pweight = pw], ll(0) vce(cluster id)
allest
quietly tobit y x1 x2 if x2 > 0.2, ll(0) ul(3)
allest
* intreg
quietly intreg lo hi x1 x2
allest
quietly intreg lo hi x1 x2 [fweight = fw]
allest
quietly intreg lo hi x1 x2 [pweight = pw]
allest
quietly intreg lo hi x1 x2, vce(cluster id)
allest
* truncreg
quietly truncreg ys x1 x2, ll(0)
allest
quietly truncreg ys x1 x2 [fweight = fw], ll(0)
allest
quietly truncreg ys x1 x2 [pweight = pw], ll(0) vce(cluster id)
allest
* cloglog
quietly cloglog yc x1 x2
allest
quietly cloglog yc x1 x2 [fweight = fw]
allest
quietly cloglog yc x1 x2, vce(robust)
allest
quietly cloglog yc x1 x2 [pweight = pw], vce(cluster id)
allest
* hetprobit
quietly hetprobit yb x1 x2, het(z1)
allest
quietly hetprobit yb x1 x2, het(z1 z2)
allest
quietly hetprobit yb x1 x2 [fweight = fw], het(z1)
allest
quietly hetprobit yb x1 x2, het(z1) vce(robust)
allest
quietly hetprobit yb x1 x2, het(z1) vce(cluster id)
allest
* heckman
quietly heckman yh x1 x2, select(s = z1 z2 x1)
allest
quietly heckman yh x1 x2 [fweight = fw], select(s = z1 z2 x1)
allest
quietly heckman yh x1 x2 [pweight = pw], select(s = z1 z2 x1)
allest
quietly heckman yh x1 x2, select(s = z1 z2 x1) vce(cluster id)
allest
* heckman twostep keeps no likelihood: estat ic is Stata's r(321), the rest works
quietly heckman yh x1 x2, select(s = z1 z2 x1) twostep
capture estat ic
display "@ rc_ic " _rc
capture estat ic, n(100)
display "@ rc_icn " _rc
evc
evc cor
estat summarize
esu
estat summarize, equation
esu
* the subcommands Stata refuses after these models and its errors for the rest
quietly tobit y x1 x2, ll(0)
capture estat
display "@ rc_none " _rc
capture estat gof
display "@ rc_gof " _rc
capture estat classification
display "@ rc_class " _rc
capture estat lroc
display "@ rc_lroc " _rc
capture estat hettest
display "@ rc_het " _rc
capture estat bootstrap
display "@ rc_boot " _rc
capture estat ic x1
display "@ rc_icv " _rc
capture estat vce x1
display "@ rc_vcev " _rc
capture estat ic, foo
display "@ rc_icfoo " _rc
capture estat ic, n(0)
display "@ rc_icn0 " _rc
quietly heckman yh x1 x2, select(s = z1 z2 x1)
capture estat gof
display "@ rc_hgof " _rc
quietly cloglog yc x1 x2
capture estat classification
display "@ rc_cclass " _rc
