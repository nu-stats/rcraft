* RCraft's Stata corpus (P2C.12, batch 5): made-up data; Stata 19's results are in svy_mean.recorded
* commands: svy
* (svy: mean, total, ratio and proportion under a design with strata, an fpc as a population or a rate or one a stratum, a PSU variable alone, observations as PSUs, no weights, and strata with one PSU under each singleunit() rule; over() of one variable, subpop() as a variable and as if, if, level(), and the rows a missing weight, PSU, stratum or subpop() variable leaves out; the design in e(), e(b) and e(V))
* tolerance: 1e-8
version 19
set linesize 255
clear
set obs 120
generate int strata = ceil(_n / 40)
generate int psu = ceil(_n / 8)
generate double w = 10 + mod(_n * 7, 13)
generate double fpc1 = 30
generate double fpcr = .25
generate double fpcv = cond(strata == 1, 20, cond(strata == 2, 30, 8))
generate double x = mod(_n * 37, 101) / 20
generate double y = 2 + x + mod(_n * 11, 7) / 5
generate double z = 5 + mod(_n * 13, 17) / 4
generate byte grp = 1 + mod(_n, 3)
generate byte sub = mod(_n, 4) != 0
generate int s2 = strata
replace s2 = 4 if psu == 15
replace y = . in 7
program define shw
    display "@ n " e(N) " " e(N_pop) " " e(N_psu) " " e(N_strata) " " e(df_r) " " e(N_over) " " e(rank) " " e(singleton) " " e(census) " " e(N_strata_omit) " " e(stages) " " e(k_eq)
    forvalues j = 1/`=colsof(e(b))' {
        display "@ b`j' " %24.17g el(e(b),1,`j') " " %24.17g el(e(V),`j',`j') " " %24.17g el(e(V),1,`j')
    }
end
program define shwd
    display "@ t `e(cmd)' `e(prefix)' `e(cmdname)' `e(vce)' `e(vcetype)' `e(wtype)' `e(wvar)' `e(wexp)' `e(su1)' `e(strata1)' `e(fpc1)' `e(subpop)' `e(singleunit)' `e(estat_cmd)' `e(marginsnotok)' `e(properties)' `e(namelist)' `e(over)'"
    display "@ c [`e(cmdline)'] [`e(command)'] [`e(varlist)'] [`e(title)']"
    display "@ ns " el(e(_N_strata_certain),1,1) " " el(e(_N_strata_single),1,1) " " el(e(_N_strata),1,1)
    forvalues j = 1/`=colsof(e(b))' {
        display "@ nj`j' " el(e(_N_subp),1,`j') " " el(e(_N),1,`j') " " el(e(error),1,`j')
    }
end
svyset psu [pweight=w], strata(strata)
svy: mean y
shw
shwd
ereturn list
svy: mean y x
shw
svy: total y x
shw
shwd
svy: ratio y/z
shw
shwd
svy: ratio (r1: y/z) (r2: x/z)
shw
shwd
svy: mean y, over(grp)
shw
shwd
display "@ over " el(e(_N_overpsu),1,1) " " el(e(_N_overpsu),1,3) " " el(e(_N_overstrata),1,2)
svy: total y, over(grp)
shw
svy: proportion grp
shw
shwd
display "@ freq " el(e(freq),1,1) " " el(e(freq),1,2) " " el(e(freq),1,3)
svy: mean y x, level(90)
shw
svy, level(90): mean y
shw
svy: mean y if grp != 1
shw
svy, subpop(sub): mean y
shw
shwd
display "@ sub " e(N_sub) " " e(N_subpop) " " e(N_subpsu)
svy, subpop(if grp == 2): mean y x
shw
shwd
display "@ sub " e(N_sub) " " e(N_subpop) " " e(N_subpsu)
svy, subpop(sub): total y
shw
svy, subpop(sub): ratio y/z
shw
svy, subpop(sub): proportion grp
shw
shwd
display "@ sub " e(N_sub) " " e(N_subpop) " " e(N_subpsu)
svy, subpop(sub) level(90): mean y
shw
svy, subpop(sub) vce(linearized): mean y if x > 2
shw
display "@ sub " e(N_sub) " " e(N_subpop) " " e(N_subpsu)
svy, vce(linearized): mean x
shw
svyset psu [pweight=w], strata(strata) fpc(fpc1)
svy: mean y x
shw
shwd
svy: total y
shw
svyset psu [pweight=w], strata(strata) fpc(fpcr)
svy: mean y
shw
svyset psu [pweight=w], strata(strata) fpc(fpcv)
svy: mean y
shw
svy: total y
shw
svy: ratio y/z
shw
svyset psu [pweight=w], strata(s2)
svy: mean y
shw
shwd
svy: total y
shw
svyset psu [pweight=w], strata(s2) singleunit(certainty)
svy: mean y
shw
shwd
svy: total y
shw
svyset psu [pweight=w], strata(s2) singleunit(scaled)
svy: mean y
shw
shwd
svy: total y
shw
svyset psu [pweight=w], strata(s2) singleunit(centered)
svy: mean y
shw
svy: total y x
shw
svy: ratio y/z
shw
svyset psu [pweight=w]
svy: mean y
shw
shwd
svy: mean y if grp != 1
shw
svyset psu, strata(strata)
svy: mean y
shw
shwd
svyset psu
svy: mean y
shw
shwd
svyset _n [pweight=w]
svy: mean y
shw
svy: total y
shw
svyset psu [pweight=w], strata(strata)
replace sub = . in 3
svy, subpop(sub): mean y
display "@ miss1 " e(N) " " e(N_sub) " " e(N_subpop) " " e(N_pop)
replace w = . in 5
svy: mean y
display "@ miss2 " e(N) " " e(N_pop)
replace psu = . in 9
svy: mean y
display "@ miss3 " e(N) " " e(N_pop) " " e(N_psu)
replace strata = . in 11
svy: mean y
display "@ miss4 " e(N) " " e(N_pop) " " e(N_psu) " " e(N_strata)
svy, subpop(if sub): mean y
display "@ miss5 " e(N) " " e(N_sub) " " e(N_subpop) " " e(N_pop)
replace w = 0 in 20
svy: mean y
display "@ zero " e(N) " " e(N_pop)
quietly svy: mean y
display "@ quiet " e(N)
