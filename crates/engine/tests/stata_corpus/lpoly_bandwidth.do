* RCraft's Stata corpus (P2C.12ay): made-up data; Stata 19's results are in lpoly_bandwidth.recorded
* commands: lpoly
* (lpoly's rule-of-thumb and pilot bandwidths on data without random numbers: degrees 0 to 4, three
* kernels, two sample sizes and curvatures, then the other kernels, uneven x and weights)
* tolerance: 1e-6
version 19
set linesize 255
foreach n in 40 100 {
    foreach c in 0.5 2 {
        clear
        quietly set obs `n'
        generate double x = _n / 10
        generate double e = mod(_n * 37, 17) / 17 - 0.5
        generate double y = `c' * x^4 / 100 + x + e
        foreach p in 0 1 2 3 4 {
            foreach k in epanechnikov gaussian rectangle {
                quietly lpoly y x, nograph degree(`p') kernel(`k')
                display "@ bw_`n'_`c'_`p'_`k' " %21x r(bwidth)
            }
        }
        quietly lpoly y x, nograph degree(1) generate(gx s) se(se) n(3)
        display "@ pilot_`n'_`c' " %21x r(pwidth) " " %21x r(bwidth) " " %21x s[2] " " %21x se[1] " " %21x se[2] " " %21x se[3]
    }
}
clear
quietly set obs 60
generate double x = (_n / 10)^2
generate double y = sin(x) + mod(_n * 13, 11) / 11
generate w = 1 + mod(_n, 4)
quietly lpoly y x, nograph degree(1)
display "@ uneven " %21x r(bwidth)
quietly lpoly y x [aweight=w], nograph degree(1)
display "@ aweight " %21x r(bwidth)
quietly lpoly y x [fweight=w], nograph degree(1)
display "@ fweight " %21x r(bwidth) " " r(N)
clear
quietly set obs 40
generate double x = _n / 10
generate double y = 0.5 * x^4 / 100 + x + mod(_n * 37, 17) / 17 - 0.5
foreach k in biweight cosine triangle parzen epan2 {
    foreach p in 1 3 5 {
        quietly lpoly y x, nograph degree(`p') kernel(`k')
        display "@ k40_`p'_`k' " %21x r(bwidth)
    }
}
