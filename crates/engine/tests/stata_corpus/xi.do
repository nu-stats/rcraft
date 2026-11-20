* RCraft's Stata corpus (P2C.12, batch E): made-up data; Stata 19's results are in xi.recorded
* commands: xi
* (xi: i.g dummies and the omitted level (lowest, prevalent, char g[omit]), i.g*i.h, i.g*x and i.g|x, xi alone, prefix(), the variables an earlier run made dropped, the models run on the dummies; levels that are negative, fractional or strings named by rank, labels as g==label, noomit, the product's type and format)
* tolerance: 1e-8
version 19
set linesize 255
clear
set obs 120
generate int g = 1 + mod(_n * 7, 3)
generate byte h = mod(_n * 5, 4) < 2
generate double x = mod(_n * 37, 101) / 20
generate double y = 1 + .5 * x + .3 * (g == 2) + .2 * h + mod(_n * 11, 23) / 9
replace g = 3 if mod(_n, 3) == 1 & mod(_n, 2) == 0
replace g = . in 4
program define shwxi
    display "@ s " %24.17g e(N) " " %24.17g e(df_m) " " %24.17g e(df_r) " " %24.17g e(r2) " " %24.17g e(rmse) " " e(rank)
    forvalues j = 1/`=colsof(e(b))' {
        display "@ b`j' " %24.17g el(e(b), 1, `j') " " %24.17g el(e(V), `j', `j')
    }
end
xi: regress y i.g x
shwxi
display "@ n1 " (e(cmdline) == "regress y _Ig_* x") " " (e(depvar) == "y")
summarize _Ig_2 _Ig_3
display "@ v1 " r(N) " " r(mean) " " r(max)
count if missing(_Ig_2)
display "@ v2 " r(N)
xi: regress y i.g*i.h
shwxi
display "@ n2 " (e(cmdline) == "regress y _Ig_* _Ih_* _IgXh_*")
display "[" e(cmdline) "]"
summarize _IgXh_2_1 _IgXh_3_1 _Ih_1
display "@ v3 " r(N) " " r(sum)
xi: regress y i.g*x
shwxi
display "@ n3 " (e(cmdline) == "regress y _Ig_* x _IgXx_*")
display "[" e(cmdline) "]"
summarize _IgXx_2
display "@ v4 " %24.17g r(mean) " " %24.17g r(sum) " " r(N)
xi: regress y i.g|x
shwxi
display "@ n4 " (e(cmdline) == "regress y x _IgXx_*")
display "[" e(cmdline) "]"
xi i.g
summarize _Ig_2
display "@ v5 " r(N) " " r(sum)
capture confirm variable _IgXx_2
display "@ v6 " _rc
xi, prefix(_D): regress y i.h x
shwxi
capture confirm variable _Ig_2
display "@ v7 " _rc
capture confirm variable _Dh_1
display "@ v8 " _rc
char _dta[omit] prevalent
xi: regress y i.g
shwxi
char _dta[omit]
char h[omit] 1
xi: regress y i.h x
shwxi
xi: logit h i.g x
display "@ l1 " %24.17g e(ll) " " e(N)
xi: regress y i.g x if x > 2, robust
shwxi
* levels that are not whole numbers from 0 up are named by rank; strings; noomit; the product's type
char h[omit]
generate int m = -1 + mod(_n * 11, 3)
generate double q = 1.5 * (1 + mod(_n * 13, 3))
generate str5 s = word("lo mid hi", 1 + mod(_n * 7, 3))
generate long big = cond(mod(_n, 3) == 0, 0, cond(mod(_n, 3) == 1, 40000, 1000000))
generate float xf = mod(_n * 37, 101) / 20
replace s = "" in 5
program define shlab
    args v lab
    display "@ lab " (("`: variable label `v''") == "`lab'")
end
xi: regress y i.m x
shwxi
display "@ r1 " (e(cmdline) == "regress y _Im_* x")
shlab _Im_2 m==0
shlab _Im_3 m==1
xi: regress y i.q x
shwxi
shlab _Iq_2 q==3
shlab _Iq_3 q==4.5
xi: regress y i.s x
shwxi
shlab _Is_2 s==lo
shlab _Is_3 s==mid
summarize _Is_2
display "@ v9 " r(N) " " r(sum)
xi: regress y i.m*i.h
shwxi
shlab _ImXh_2_1 "m==0 & h==1"
shlab _ImXh_3_1 "m==1 & h==1"
xi: regress y i.s*x
shwxi
shlab _IsXx_2 "(s==lo)*x"
xi: regress y i.g*xf
shwxi
xi: regress y i.g*x
xi: regress y i.big
shwxi
display "@ r2 " (e(cmdline) == "regress y _Ibig_*")
shlab _Ibig_40000 big==40000
char s[omit] lo
xi: regress y i.s x
shwxi
shlab _Is_1 s==hi
shlab _Is_3 s==mid
char s[omit]
xi, noomit: regress y i.g, noconstant
shwxi
display "@ r3 " (e(cmdline) == "regress y _Ig_*, noconstant")
xi, noomit: regress y i.h*x, noconstant
shwxi
xi, noomit i.g
summarize _Ig_1
display "@ v10 " r(N) " " r(sum)
