* RCraft's Stata corpus (P2C.12): no data; Stata 19's results are in power8.recorded
* commands: power
* (power exponential: Lachin's test of two hazards, sample sizes, power and the events and probabilities in r(E0_vec), r(Ea_vec) and r(Pr_vec), conditional and unconditional, hazard difference and log hazard-ratio, studytime(), nratio(), n1() and n2(), a total that does not split)
* tolerance: 1e-9
version 19
set linesize 255
quietly power exponential 0.5 0.4
display "@ en " r(N) " " r(N1) " " r(N2) " " r(hdiff)
quietly power exponential 0.5 0.4, n(500)
display "@ ep " %24.17g r(power) " " el(r(E0_vec),1,1) " " el(r(Pr_vec),1,1)
quietly power exponential 0.5 0.4, loghazard studytime(2)
display "@ el " r(N) " " r(N1) " " el(r(E0_vec),1,1) " " el(r(Ea_vec),1,2) " " %12.8f el(r(Pr_vec),1,3)
quietly power exponential 0.5 0.4, studytime(2) unconditional n(600)
display "@ eu " %24.17g r(power) " " el(r(Ea_vec),1,3) " " r(unconditional) " " r(studytime)
quietly power exponential 0.5 0.4, nratio(2) studytime(3)
display "@ er " r(N) " " r(N1) " " r(N2) " " %24.17g r(nratio_a) " " %12.8f el(r(Pr_vec),1,1)
quietly power exponential 0.5, hratio(0.6) n(200) onesided
display "@ eh " %24.17g r(power) " " %24.17g r(delta) " " r(hratio)
quietly power exponential 0.5 0.4, n(502) nratio(1.3) studytime(2)
display "@ ea " %24.17g r(power) " " r(N) " " r(N_a) " " r(N1) " " r(N2) " " %12.8f el(r(Pr_vec),1,1)
quietly power exponential 0.3 0.6, n1(100) n2(150) studytime(1.5)
display "@ en2 " %24.17g r(power) " " el(r(E0_vec),1,2) " " el(r(Ea_vec),1,3)
quietly power exponential 0.6 0.4, n1(120) n2(80) loghazard onesided
display "@ ec " %24.17g r(power) " " %24.17g r(lnhratio)
