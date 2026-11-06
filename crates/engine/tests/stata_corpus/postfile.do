* RCraft's Stata corpus (packages' gaps): made-up data; Stata 19's results are in postfile.recorded
* commands: post postclose postfile
* (results posted row by row to a .dta file, with float, double, int and string variables, a type for several names in parentheses, a string cut to its width and a missing value; ibn. factor variables with noconstant, where every level has its coefficient)
version 19
set linesize 255
clear
set seed 1979
set obs 40
generate double x = round(rnormal()*3, .001)
generate int g = mod(_n, 3) + 1
generate double y = round(2 + .7*x + g + rnormal(), .001)
tempname h
postfile `h' str6 name int n mean double sd using corpus_post, replace
foreach v in x y g {
    quietly summarize `v'
    post `h' ("`v'_var") (r(N)) (r(mean)) (r(sd))
}
post `h' ("none") (.) (1/3) (1/3)
postclose `h'
preserve
use corpus_post, clear
display "@ posted " _N " " n[1] " " %21x mean[1] " " %21x sd[1] " " %21x mean[2] " " %21x sd[2]
display "@ posted last " (n[4] == .) " " %21x mean[4] " " %21x sd[4] " " length(name[1]) " " (name[1] == "x_var")
restore
erase corpus_post.dta
quietly regress y x ibn.g, noconstant
display "@ ibn " _b[x] " " _b[1bn.g] " " _b[2bn.g] " " _b[3bn.g] " " _se[3bn.g] " " e(N) " " e(df_m) " " e(r2) " " e(ll) " " e(rank)
quietly regress y x i.g
display "@ ib " _b[x] " " _b[2.g] " " _b[3.g] " " _b[_cons] " " e(ll) " " e(ll_0) " " e(rank)
postfile `h' str3 v double(m s) lo using corpus_post2, replace
quietly summarize x
post `h' ("abcd") (r(mean)) (r(sd)) (r(min))
postclose `h'
preserve
use corpus_post2, clear
display "@ grouped " _N " " %21x m[1] " " %21x s[1] " " %21x lo[1] " " length(v[1])
restore
erase corpus_post2.dta
