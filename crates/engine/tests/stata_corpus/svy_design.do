* RCraft's Stata corpus (P2C.12, batch 5): made-up data; Stata 19's results are in svy_design.recorded
* commands: svydescribe svyset
* (svyset with strata, fpc, singleunit, later stages (a stage with no fpc drops those after it), pweight or iweight, and the r() it leaves; svyset alone and svyset, clear; svydescribe: the first stage's table with and without a varlist, a stratum with a unit whose rows are all missing, finalstage, if and in, a design with a second stage, and its r())
* tolerance: 1e-8
version 19
set linesize 255
clear
set obs 120
generate int strata = ceil(_n / 40)
generate int psu = ceil(_n / 8)
generate int ssu = ceil(_n / 4)
generate double w = 10 + mod(_n * 7, 13)
generate double fpc1 = 30
generate double fpc2 = 6
generate double x = mod(_n * 37, 101) / 20
generate double y = 2 + x + mod(_n * 11, 7) / 5
replace y = . in 7
replace x = . in 50
replace y = . if psu == 3
generate str3 sname = "s" + string(strata)
svyset
svyset psu [pweight=w], strata(strata)
display "@ r1 " r(stages) " `r(strata1)' `r(su1)' `r(vce)' `r(singleunit)' `r(wvar)' `r(wtype)'"
display "@ set1 `r(settings)'"
svyset
display "@ r1b " r(stages) " `r(strata1)' `r(su1)' `r(wexp)'"
svyset psu [pw=w], strata(strata) fpc(fpc1)
display "@ r2 " r(stages) " `r(strata1)' `r(su1)' `r(fpc1)'"
display "@ set2 `r(settings)'"
svyset psu [pweight=w], strata(strata) fpc(fpc1) || ssu, fpc(fpc2)
display "@ r3 " r(stages) " `r(strata1)' `r(su1)' `r(fpc1)' `r(su2)' `r(fpc2)'"
display "@ set3 `r(settings)'"
svyset psu [pweight=w], strata(strata) fpc(fpc1) || ssu, fpc(fpc2) || _n
display "@ r4 " r(stages) " `r(su2)' `r(fpc2)' `r(su3)'"
display "@ set4 `r(settings)'"
svyset psu [pweight=w], strata(strata) fpc(fpc1) || ssu || _n
display "@ r5 " r(stages) " `r(su2)' `r(fpc2)'"
display "@ set5 `r(settings)'"
svyset psu [pweight=w], strata(strata) || ssu, fpc(fpc2)
display "@ r6 " r(stages) " `r(su2)'"
display "@ set6 `r(settings)'"
svyset psu [pweight=w], strata(strata) fpc(fpc1) || ssu, fpc(fpc2) || _n, strata(strata)
display "@ r7 " r(stages) " `r(strata3)' `r(su3)'"
display "@ set7 `r(settings)'"
svyset psu, strata(strata) singleunit(certainty)
display "@ r8 " r(stages) " `r(singleunit)' `r(wvar)' `r(wtype)'"
display "@ set8 `r(settings)'"
svyset psu [pweight=w], fpc(fpc1) singleunit(scaled) vce(linearized)
display "@ r9 " r(stages) " `r(singleunit)' `r(strata1)' `r(fpc1)'"
display "@ set9 `r(settings)'"
svyset [iweight=w], strata(strata) singleunit(centered)
display "@ r10 " r(stages) " `r(singleunit)' `r(su1)' `r(wtype)'"
display "@ set10 `r(settings)'"
svyset _n [pweight=w]
display "@ r11 " r(stages) " `r(su1)' `r(strata1)'"
display "@ set11 `r(settings)'"
svyset psu [pweight=w], strata(sname)
display "@ r12 `r(strata1)'"
svyset psu [pweight=w], strata(strata)
svydescribe
display "@ d1 " r(N) " " r(N_units) " " r(N_strata) " " r(N_single) " " r(N_mstrata) " " r(N_munits) " " r(N_miss)
svydescribe x
display "@ d2 " r(N) " " r(N_units) " " r(N_strata) " " r(N_single) " " r(N_mstrata) " " r(N_munits) " " r(N_miss)
svydescribe y
display "@ d3 " r(N) " " r(N_units) " " r(N_strata) " " r(N_single) " " r(N_mstrata) " " r(N_munits) " " r(N_miss)
svydescribe y x
display "@ d4 " r(N) " " r(N_units) " " r(N_strata) " " r(N_single) " " r(N_mstrata) " " r(N_munits) " " r(N_miss)
svydescribe, finalstage
display "@ d5 " r(N) " " r(N_units) " " r(N_strata) " " r(N_miss)
svydescribe y x, finalstage
display "@ d6 " r(N) " " r(N_units) " " r(N_strata) " " r(N_miss)
svydescribe if x > 3
display "@ d7 " r(N) " " r(N_units) " " r(N_strata) " " r(N_miss)
svydescribe y x if x > 3
display "@ d8 " r(N) " " r(N_units) " " r(N_strata) " " r(N_miss)
svydescribe in 1/90
display "@ d9 " r(N) " " r(N_units) " " r(N_strata) " " r(N_miss)
svydescribe y in 20/120
display "@ d10 " r(N) " " r(N_units) " " r(N_strata) " " r(N_miss)
svydescribe y x if x > 3, finalstage
display "@ d11 " r(N) " " r(N_units) " " r(N_strata) " " r(N_miss)
svydescribe, stage(1)
display "@ d12 " r(N) " " r(N_units) " " r(N_strata)
svyset psu [pweight=w], strata(strata) fpc(fpc1) || ssu, fpc(fpc2)
svydescribe
display "@ d13 " r(N) " " r(N_units) " " r(N_strata)
svydescribe y, stage(1)
display "@ d14 " r(N) " " r(N_units) " " r(N_strata) " " r(N_munits)
svyset _n [pweight=w], strata(strata)
svydescribe
display "@ d15 " r(N) " " r(N_units) " " r(N_strata)
svyset psu [pweight=w]
svydescribe y
display "@ d16 " r(N) " " r(N_units) " " r(N_strata) " " r(N_munits) " " r(N_miss)
svyset psu, strata(strata)
svydescribe
display "@ d17 " r(N) " " r(N_units) " " r(N_strata)
svyset psu [pweight=w], strata(sname)
svydescribe y
display "@ d18 " r(N) " " r(N_units) " " r(N_strata) " " r(N_munits)
svyset, clear
display "@ c1 `r(settings)' " r(stages)
svyset
