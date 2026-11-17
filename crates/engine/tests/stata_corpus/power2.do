* RCraft's Stata corpus (P2C.12): no data; Stata 19's results are in power2.recorded
* commands: power
* (power onevariance, twovariances, onecorrelation, twocorrelations and pairedproportions: sample sizes, power and target values, one- and two-sided, on the variance and sd scales, with nratio(), n1() and n2(), a total that does not split, and prdiscordant())
* tolerance: 1e-9
version 19
set linesize 255
quietly power onevariance 1 1.5
display "@ ov " r(N)
quietly power onevariance 1 0.5, n(40) onesided
display "@ ovp " %24.17g r(power)
quietly power onevariance 1, n(40) power(0.8)
display "@ ove " %24.17g r(va)
quietly power onevariance 1, n(40) power(0.8) onesided direction(lower)
display "@ ovc " %24.17g r(va)
quietly power onevariance 2, sd n(30) power(0.9)
display "@ ovs " %24.17g r(sa) " " %24.17g r(delta)
quietly power twovariances 1 2, nratio(1.3)
display "@ tv " r(N) " " r(N1) " " r(N2) " " %24.17g r(nratio_a)
quietly power twovariances 1 0.5, n1(30) n2(50) onesided
display "@ tvp " %24.17g r(power)
quietly power twovariances 1, n(101) power(0.9) alpha(0.01)
display "@ tve " %24.17g r(v2) " " r(N_a)
quietly power twovariances 1.5, sd n(100) power(0.8) onesided
display "@ tvs " %24.17g r(s2)
quietly power onecorrelation 0 0.3
display "@ oc " r(N)
quietly power onecorrelation 0.5 0.2, onesided
display "@ occ " r(N)
quietly power onecorrelation 0.2, n(50) power(0.8) direction(lower)
display "@ oce " %24.17g r(ra)
quietly power twocorrelations 0.2 0.5, n1(60) n2(90)
display "@ tcp " %24.17g r(power)
quietly power twocorrelations 0.2 0.5, nratio(1.3) onesided
display "@ tcc " r(N) " " r(N1) " " r(N2) " " %24.17g r(nratio_a)
quietly power twocorrelations 0.2, n(200) power(0.8)
display "@ tce " %24.17g r(r2)
quietly power pairedproportions 0.2 0.3
display "@ pp " r(N)
quietly power pairedproportions 0.3 0.2, n(200) onesided
display "@ ppl " %24.17g r(power)
quietly power pairedproportions 0.25 0.1, alpha(0.1) power(0.9) onesided
display "@ ppc " r(N)
quietly power pairedproportions, n(100) prdiscordant(0.3) power(0.8) effect(ratio)
display "@ ppe " %24.17g r(p12) " " %24.17g r(p21) " " %24.17g r(delta)
