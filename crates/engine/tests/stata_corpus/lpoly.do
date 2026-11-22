* RCraft's Stata corpus (P2C.12ay): made-up data; Stata 19's results are in lpoly.recorded
* commands: lpoly
* (lpoly: the rule-of-thumb bandwidth for every degree and kernel, the pilot bandwidth, the
* smooth on the grid and at(), generate(), se() with var() and pwidth(), aweights and fweights,
* if and in, n() above N, the errors)
* tolerance: 1e-6
version 19
set linesize 255
clear
set obs 70
generate double x = (_n + mod(_n * 11, 5) / 5) / 7
generate double y = 3 + sin(x * 1.3) * 2 + mod(_n * 23, 13) / 13 - 0.5
generate double w = 1 + mod(_n * 7, 4)
quietly lpoly y x, nograph
display "@ default " %20.0g r(bwidth) " " r(degree) " " r(ngrid) " " r(N)
foreach p in 0 1 2 3 4 5 {
    quietly lpoly y x, nograph degree(`p')
    display "@ degree`p' " %20.0g r(bwidth)
}
local i = 0
foreach k in epanechnikov epan2 biweight cosine gaussian parzen rectangle triangle {
    local ++i
    quietly lpoly y x, nograph kernel(`k') degree(1)
    display "@ kern`i' " %20.0g r(bwidth)
    quietly lpoly y x, nograph kernel(`k') degree(3)
    display "@ kern3_`i' " %20.0g r(bwidth)
}
quietly lpoly y x, nograph generate(gx gs)
display "@ grid " %20.0g gx[1] " " %20.0g gx[17] " " %20.0g gx[50] " " (gx[51] == .)
display "@ smooth " %20.0g gs[1] " " %20.0g gs[17] " " %20.0g gs[50]
quietly lpoly y x, nograph degree(1) generate(g1x g1s) se(g1se) n(12)
display "@ se " %20.0g g1s[1] " " %20.0g g1s[6] " " %20.0g g1se[1] " " %20.0g g1se[6] " " %20.0g g1se[12]
display "@ pilot " %20.0g r(pwidth) " " %20.0g r(bwidth) " " r(ngrid)
quietly lpoly y x, nograph degree(2) kernel(gaussian) generate(g2s) at(x) se(g2se)
display "@ at " %20.0g g2s[1] " " %20.0g g2s[33] " " %20.0g g2s[70] " " %20.0g g2se[33] " " r(ngrid)
quietly lpoly y x, nograph degree(1) bwidth(0.5) n(5) generate(bx bs) se(bse)
display "@ given " %20.0g bs[1] " " %20.0g bs[3] " " %20.0g bse[3] " " %20.0g r(bwidth) " " %20.0g r(pwidth)
quietly lpoly y x, nograph n(5) generate(vx vs) se(vse) var(0.09) pwidth(0.7)
display "@ var " %20.0g vs[2] " " %20.0g vse[2] " " %20.0g vse[5] " " %20.0g r(pwidth)
quietly lpoly y x [aweight=w], nograph degree(1) n(6) generate(ax aws)
display "@ aweight " %20.0g r(bwidth) " " r(N) " " %20.0g aws[1] " " %20.0g aws[4]
quietly lpoly y x [fweight=w], nograph degree(1) n(6) generate(fx fs)
display "@ fweight " %20.0g r(bwidth) " " r(N) " " %20.0g fs[1] " " %20.0g fs[4]
quietly lpoly y x if x < 6, nograph degree(3) n(4) generate(dx ds)
display "@ if " %20.0g r(bwidth) " " r(N) " " %20.0g ds[1] " " %20.0g ds[4]
quietly lpoly y x in 5/50, nograph n(7)
display "@ in " %20.0g r(bwidth) " " r(N) " " r(ngrid)
quietly lpoly y x, nograph n(500)
display "@ bign " r(ngrid) " " r(N)
quietly lpoly y x, nograph n(1) generate(ox os)
display "@ one " %20.0g ox[1] " " %20.0g os[1] " " r(ngrid)
capture lpoly y x, nograph generate(gx gs)
display "@ err110 " _rc
capture lpoly y
display "@ err102 " _rc
capture lpoly y x, nograph kernel(foo)
display "@ err198 " _rc
capture lpoly y x, nograph kernel(triweight)
display "@ errtri " _rc
capture lpoly y x, nograph generate(onlyone)
display "@ errone " _rc
capture lpoly y x [pweight=w], nograph
display "@ errp " _rc
capture lpoly y x [iweight=w], nograph
display "@ erri " _rc

capture lpoly y x, nograph n(0)
display "@ errn " _rc
capture lpoly y x, nograph bwidth(-1)
display "@ errbw " _rc
capture lpoly y x, nograph pwidth(0) generate(a1 a2) se(a3)
display "@ errpw " _rc
capture lpoly y x, nograph degree(-1)
display "@ errdeg " _rc
capture lpoly y x, nograph generate(a1 a2 a3)
display "@ errgen " _rc
capture lpoly y x, nograph se(a3)
display "@ errse " _rc
capture lpoly y x, nograph at(nosuch)
display "@ errat " _rc
capture lpoly y nosuch, nograph
display "@ errvar " _rc
capture lpoly y x if x < 0, nograph
display "@ errnone " _rc
generate double fw = _n / 2
capture lpoly y x [fweight=fw], nograph
display "@ errfw " _rc
generate double nw = _n - 30
capture lpoly y x [aweight=nw], nograph
display "@ errnw " _rc
quietly lpoly y x, nograph degree(1.5)
display "@ degfrac " r(degree) " " %20.0g r(bwidth)
quietly lpoly y x, nograph n(2.5)
display "@ nfrac " r(ngrid)
quietly lpoly y x, nograph n(3) generate(tx ts)
display "@ n3 " r(ngrid) " " %20.0g tx[2] " " %20.0g ts[3]
quietly lpoly y x, nograph generate(ux us) at(x) degree(1)
display "@ at2 " %20.0g ux[5] " " %20.0g us[5] " " r(ngrid)
foreach k in ep epa epan gau gaus cos cosi bi biw par rec tri {
    quietly lpoly y x, nograph kernel(`k') n(2)
    display "@ abbr_`k' " %20.0g r(bwidth)
}
foreach k in e ga co Gaussian epan3 {
    capture lpoly y x, nograph kernel(`k')
    display "@ abbrerr_`k' " _rc
}
quietly lpoly y x, nograph bwidth(0.05) n(3) generate(sx ss) se(sse)
display "@ smallbw " (ss[1] == .) " " (sse[1] == .) " " %20.0g ss[2] " " %20.0g sse[3]
generate double cz = 1
quietly lpoly y cz, nograph
display "@ constx " (r(bwidth) == .) " " r(ngrid) " " r(N)
quietly lpoly y cz, nograph bwidth(1) n(3) generate(zx zs)
display "@ constx2 " zx[2] " " %20.0g zs[2]
quietly lpoly y x if x < 0.9, nograph degree(1)
display "@ few " r(N) " " %20.0g r(bwidth)
* windows with one, two and a few points (rectangle kernel on whole-number x): a fit with one point
* is missing, with fewer points than coefficients the later powers are left out
clear
set obs 20
generate double x = _n
generate double y = sin(_n) + mod(_n * 7, 5)
generate double z = 5.5 in 1
replace z = 5 in 2
replace z = 5.2 in 3
local j = 0
foreach p in 0 1 2 3 {
    foreach h in 0.4 0.6 1.001 1.4 2.001 2.6 {
        local ++j
        capture drop s
        quietly lpoly y x, nograph kernel(rectangle) degree(`p') bwidth(`h') at(z) generate(s)
        display "@ win`j' " %20.0g s[1] " " %20.0g s[2] " " %20.0g s[3]
    }
}
