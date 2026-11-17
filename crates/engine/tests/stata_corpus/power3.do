* RCraft's Stata corpus (P2C.12): no data; Stata 19's results are in power3.recorded
* commands: power
* (power with lists of values, Stata's table: every combination in Stata's order, the last row's r() and r(pss_table); and a total that does not split at nratio())
* tolerance: 1e-9
version 19
set linesize 255
quietly power onemean 0 (0.4 0.5), alpha(0.01 0.05)
display "@ om " r(N) " " el(r(pss_table), 1, 3) " " el(r(pss_table), 2, 3) " " el(r(pss_table), 3, 3) " " el(r(pss_table), 2, 1)
quietly power twomeans 0 0.5, n(50 100) nratio(1 2)
display "@ tm " %24.17g r(power) " " r(N_a) " " r(N2) " " %24.17g el(r(pss_table), 2, 1 + 1) " " el(r(pss_table), 2, 5)
quietly power twoproportions 0.3 (0.4(0.1)0.6), power(0.8 0.9)
display "@ tp " r(N) " " rowsof(r(pss_table)) " " colsof(r(pss_table)) " " el(r(pss_table), 4, 3)
quietly power pairedmeans 0 0.5, sd(1 1.2) corr(0.3 0.5) n(30)
display "@ pm " %24.17g el(r(pss_table), 3, 2) " " %24.17g el(r(pss_table), 3, 9)
quietly power twovariances 1 2, sd n(50 100)
display "@ tv " %24.17g el(r(pss_table), 1, 2) " " %24.17g r(power)
quietly power pairedproportions, n(100 200) prdiscordant(0.3) power(0.8)
display "@ pp " %24.17g el(r(pss_table), 2, 5) " " %24.17g r(p21)
quietly power twomeans 0, n(100 200) power(0.8) onesided
display "@ te " %24.17g r(m2) " " %24.17g el(r(pss_table), 1, 7)
quietly power twoproportions 0.3 0.5, n(53) nratio(1.5)
display "@ ac " r(N1) " " r(N2) " " %24.17g r(nratio_a) " " %24.17g r(power)
quietly power onecorrelation 0 0.3, table
display "@ ot " r(N) " " rowsof(r(pss_table))
