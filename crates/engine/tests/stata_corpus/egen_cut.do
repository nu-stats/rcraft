* RCraft's Stata corpus (P2C.12ao): made-up data; Stata 19's results are in egen_cut.recorded
* (egen cut(): at() points and ranges, icodes, label, a value read as single precision, group() on counts, ties and missing values, if and in, double, errors under capture)
* tolerance: 1e-9
version 19
set linesize 255
clear
set obs 40
generate age = 18 + mod(_n * 13, 50)
generate double x = mod(_n * 7, 11) + mod(_n, 3) / 4
replace x = . in 5
generate tie = mod(_n, 4)
egen a1 = cut(age), at(18 30 50 70)
egen a2 = cut(age), at(20(10)60)
egen a3 = cut(age), at(18 30 50 70) icodes
egen a4 = cut(age), at(18 30 50 70) label
egen a5 = cut(x), at(0, 2.5, 5, 7.5, 12)
egen double a6 = cut(x), at(0 2.5 5 7.5 12)
egen g3 = cut(age), group(3)
egen g4 = cut(age), group(4)
egen g4i = cut(age), group(4) icodes
egen g4l = cut(age), group(4) label
egen gx = cut(x), group(5)
egen gt = cut(tie), group(3)
egen gt2 = cut(tie), group(6)
egen ga = cut(age) if age > 30, group(2)
egen gb = cut(age) in 1/20, at(18 40 70)
forvalues i = 1/40 {
    display "@ a1_`i' " a1[`i']
    display "@ a2_`i' " a2[`i']
    display "@ a3_`i' " a3[`i']
    display "@ a4_`i' " a4[`i']
    display "@ a5_`i' " a5[`i']
    display "@ a6_`i' " a6[`i']
    display "@ g3_`i' " g3[`i']
    display "@ g4_`i' " g4[`i']
    display "@ g4i_`i' " g4i[`i']
    display "@ g4l_`i' " g4l[`i']
    display "@ gx_`i' " gx[`i']
    display "@ gt_`i' " gt[`i']
    display "@ gt2_`i' " gt2[`i']
    display "@ ga_`i' " ga[`i']
    display "@ gb_`i' " gb[`i']
}
forvalues k = 0/4 {
    display "@ a4_label`k' " real(subinstr("`: label (a4) `k''", "-", "", .))
    display "@ g4l_label`k' " real(subinstr("`: label (g4l) `k''", "-", "", .))
}
* the value is read as a single-precision number
clear
set obs 8
generate double xd = .
replace xd = .5999999999 in 1
replace xd = 2.9999999999 in 2
replace xd = .7 in 3
replace xd = .70000001 in 4
replace xd = .6 in 5
replace xd = .69999999999 in 6
replace xd = 3 in 7
replace xd = .59999 in 8
generate float xf = xd
egen cd = cut(xd), at(0 .6 .7 3 4)
egen cf = cut(xf), at(0 .6 .7 3 4)
egen double cdd = cut(xd), at(0 .6 .7 3 4)
egen ce = cut(xd), at(0 .6 .7 3 4) icodes
egen cr = cut(xd), at(0(.1)1)
forvalues i = 1/8 {
    display "@ cd`i' " %24.17g cd[`i']
    display "@ cf`i' " %24.17g cf[`i']
    display "@ cdd`i' " %24.17g cdd[`i']
    display "@ ce`i' " ce[`i']
    display "@ cr`i' " %24.17g cr[`i']
}
* group(): counts that tie, many sizes and numbers of groups, in double and float
foreach n of numlist 9 10 29 30 31 60 {
    foreach k of numlist 2 3 4 5 7 10 {
        quietly {
            clear
            set obs `n'
            generate double v = mod(_n * 37, 101) / 7
            generate float vf = mod(_n * 7, 10) / 10
            egen g = cut(v), group(`k')
            egen gf = cut(vf), group(`k')
            generate double w = g * _n
            generate double wf = gf * _n
            summarize w, meanonly
            local s1 = r(sum)
            summarize wf, meanonly
            local s2 = r(sum)
        }
        display "@ gk_`n'_`k' `s1' `s2'"
    }
}
* errors under capture, a single point, and a name in use
clear
set obs 12
generate v = (_n - 3) * 5
capture egen e1 = cut(v)
display "@ e1 " _rc
capture egen e2 = cut(v), at(0 10) group(2)
display "@ e2 " _rc
capture egen e3 = cut(v), at(30 10)
display "@ e3 " _rc
capture egen e4 = cut(v), group(0)
display "@ e4 " _rc
capture egen e5 = cut(v), at(18)
display "@ e5 " _rc
count if missing(e5)
display "@ e5_missing " r(N)
egen e6 = cut(v), at(0 10 25 40) label
capture egen e6 = cut(v), at(0 10)
display "@ e6_again " _rc
capture egen e7 = cut(v), group(1)
display "@ e7 " _rc
capture egen e8 = cut(v), group(2.5)
display "@ e8 " _rc
capture egen e9 = cut(v), group(20)
display "@ e9 " _rc
capture egen e10 = cut(v), group(13)
display "@ e10 " _rc
capture egen e11 = cut(v) if v > 1000, group(2)
display "@ e11 " _rc
capture egen e12 = cut(v) if v > 1000, at(1 5)
display "@ e12 " _rc
