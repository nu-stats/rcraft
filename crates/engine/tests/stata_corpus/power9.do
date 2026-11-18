* RCraft's Stata corpus (P2C.12): no data; Stata 19's results are in power9.recorded
* commands: power
* (power oneproportion test(binomial) and twoproportions test(fisher): exact power and the actual alpha, critical values, unequal groups, one- and two-sided; and test(lrchi2): power, sample sizes and the target proportion)
* tolerance: 1e-9
version 19
set linesize 255
quietly power oneproportion 0.5 0.65, test(binomial) n(50)
display "@ bn " %24.17g r(power) " " %24.17g r(alpha_a) " " r(C_l) " " r(C_u)
quietly power oneproportion 0.5 0.35, test(binomial) n(50)
display "@ bl " %24.17g r(power) " " %24.17g r(alpha_a) " " r(C_l) " " r(C_u)
quietly power oneproportion 0.5 0.65, test(binomial) n(50) onesided
display "@ bo " %24.17g r(power) " " %24.17g r(alpha_a) " " r(C_l) " " r(C_u)
quietly power oneproportion 0.3 0.2, test(binomial) n(40) onesided
display "@ bd " %24.17g r(power) " " %24.17g r(alpha_a) " " r(C_l) " " r(C_u)
quietly power twoproportions 0.3 0.5, test(fisher) n(100)
display "@ fn " %24.17g r(power) " " %24.17g r(alpha_a) " " r(N1) " " r(N2)
quietly power twoproportions 0.3 0.5, test(fisher) n1(40) n2(70)
display "@ fu " %24.17g r(power) " " %24.17g r(alpha_a)
quietly power twoproportions 0.5 0.3, test(fisher) n(100) onesided
display "@ fo " %24.17g r(power) " " %24.17g r(alpha_a)
quietly power twoproportions 0.3 0.5, test(fisher) n(60) nratio(2) alpha(0.1)
display "@ fr " %24.17g r(power) " " %24.17g r(alpha_a) " " r(N1) " " r(N2)
quietly power twoproportions 0.3 0.5, test(lrchi2) n(100)
display "@ lp " %24.17g r(power)
quietly power twoproportions 0.3 0.5, test(lrchi2)
display "@ ln " r(N) " " r(N1)
quietly power twoproportions 0.3 0.5, test(lrchi2) nratio(2)
display "@ lr " r(N) " " r(N1) " " r(N2)
quietly power twoproportions 0.3 0.5, test(lrchi2) onesided
display "@ lo " r(N)
quietly power twoproportions 0.3, test(lrchi2) n(100) power(0.8)
display "@ le " %24.17g r(p2)
quietly power twoproportions 0.3 0.5, test(lrchi2) effect(oratio) n1(40) n2(60)
display "@ lq " %24.17g r(power)
