* RCraft's Stata corpus (P2C.12): no data; Stata 19's results are in power6.recorded
* commands: power
* (power with one group's size: n1() or n2() alone, with nratio(), and n() with n1() or n2(); and logrank's events as Stata's doubles round them)
* tolerance: 1e-9
version 19
set linesize 255
quietly power twomeans 0 0.5, n1(40)
display "@ m1 " %24.17g r(power) " " r(N) " " r(N2)
quietly power twomeans 0 0.5, n2(60) nratio(1.5)
display "@ m2 " %24.17g r(power) " " r(N1) " " r(nratio)
quietly power twomeans 0 0.5, n(100) n2(70)
display "@ mn " %24.17g r(power) " " r(N1) " " %24.17g r(nratio)
quietly power twoproportions 0.3, n2(70) power(0.8) nratio(2)
display "@ pe " %24.17g r(p2) " " r(N) " " r(N1)
quietly power twocorrelations 0.2 0.5, n1(60)
display "@ cr " %24.17g r(power) " " r(N)
quietly power logrank 0.5 0.4, n1(100) n2(100)
display "@ lr " %24.17g r(power) " " r(E) " " %24.17g r(Pr_E)
