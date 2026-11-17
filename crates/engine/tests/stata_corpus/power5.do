* RCraft's Stata corpus (P2C.12): no data; Stata 19's results are in power5.recorded
* commands: power
* (power logrank, Freedman's and Schoenfeld's, and power cox: events and sample sizes, power and the hazard ratio or coefficient, with and without censoring, nratio(), n1() and n2(), and a total that does not split)
* tolerance: 1e-9
version 19
set linesize 255
quietly power logrank 0.5 0.4
display "@ ln " r(E) " " r(N) " " r(N1) " " %24.17g r(hratio) " " %24.17g r(Pr_E)
quietly power logrank 0.5, hratio(0.6) nratio(1.5)
display "@ lr " r(E) " " r(N) " " r(N1) " " r(N2) " " %24.17g r(s2)
quietly power logrank, hratio(0.6) nratio(2)
display "@ le " r(E) " " r(N) " " r(N1) " " r(N2)
quietly power logrank 0.5, hratio(0.6) n(200)
display "@ lp " %24.17g r(power) " " r(E)
quietly power logrank 0.5 0.4, n1(100) n2(200)
display "@ lq " %24.17g r(power) " " %24.17g r(Pr_E)
quietly power logrank 0.5, n(300) power(0.8)
display "@ lh " %24.17g r(hratio) " " r(E)
quietly power logrank 0.3, n(401) nratio(1.5) power(0.8)
display "@ la " %24.17g r(hratio) " " r(E) " " r(N_a)
quietly power logrank, n(300) power(0.8) onesided direction(upper)
display "@ lc " %24.17g r(hratio)
quietly power logrank 0.5 0.4, schoenfeld
display "@ ls " r(E) " " r(N) " " %24.17g r(delta)
quietly power logrank 0.5, schoenfeld n(300) power(0.8)
display "@ lt " %24.17g r(delta) " " %24.17g r(hratio)
quietly power cox, hratio(0.6) eventprob(0.4)
display "@ cn " r(E) " " r(N) " " %24.17g r(b1)
quietly power cox 0.5, sd(0.5) r2(0.2) n(100)
display "@ cp " %24.17g r(power)
quietly power cox, n(150) power(0.9) sd(2) r2(0.3) eventprob(0.6) direction(upper)
display "@ ce " %24.17g r(b1) " " r(E)
quietly power cox, hratio(0.6) n(100) onesided
display "@ co " %24.17g r(power)
