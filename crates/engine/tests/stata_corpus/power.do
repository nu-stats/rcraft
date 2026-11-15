* RCraft's Stata corpus (P2C.12): no data; Stata 19's results are in power.recorded
* commands: power
* (power onemean, twomeans, pairedmeans, oneproportion and twoproportions: sample sizes, power and target values, one- and two-sided, with knownsd, unequal sds, nratio(), a total that does not split, corr(), the Wald test, continuity and effect())
* tolerance: 1e-9
version 19
set linesize 255
quietly power onemean 0 0.5
display "@ om " r(N) " " %24.17g r(power)
quietly power onemean 1 1.5, sd(2.5) n(33) alpha(0.1)
display "@ omp " %24.17g r(power) " " %24.17g r(delta)
quietly power onemean 0, n(40) power(0.8) direction(lower)
display "@ ome " %24.17g r(ma) " " %24.17g r(delta)
quietly power onemean 0 0.5, n(40) knownsd onesided
display "@ omz " %24.17g r(power)
quietly power twomeans 0 0.5, sd1(1) sd2(1.5) nratio(2)
display "@ tm " r(N) " " r(N1) " " r(N2)
quietly power twomeans 0 0.5, n(101)
display "@ tmp " %24.17g r(power) " " r(N) " " r(N_a) " " r(N1)
quietly power twomeans 0, n(100) power(0.8)
display "@ tme " %24.17g r(m2)
quietly power twomeans 0 0.5, n1(40) n2(60) knownsds
display "@ tmz " %24.17g r(power) " " %24.17g r(nratio)
quietly power pairedmeans 0 0.5, sd(1) corr(0.5) n(30)
display "@ pm " %24.17g r(power) " " %24.17g r(sd_d)
quietly power pairedmeans 0, sddiff(1) n(30) power(0.9) onesided
display "@ pme " %24.17g r(da)
quietly power oneproportion 0.5 0.65
display "@ op " r(N)
quietly power oneproportion 0.5 0.65, test(wald) n(80)
display "@ opw " %24.17g r(power)
quietly power oneproportion 0.5, n(80) power(0.8)
display "@ ope " %24.17g r(pa)
quietly power twoproportions 0.3 0.5, nratio(1.5) continuity
display "@ tp " r(N) " " r(N1) " " r(N2) " " %24.17g r(nratio_a)
quietly power twoproportions 0.3 0.5, continuity n(150)
display "@ tpc " %24.17g r(power)
quietly power twoproportions 0.3 0.5, effect(oratio) n1(60) n2(90)
display "@ tpo " %24.17g r(power) " " %24.17g r(delta)
quietly power twoproportions 0.3, n(150) power(0.8) onesided
display "@ tpe " %24.17g r(p2)
