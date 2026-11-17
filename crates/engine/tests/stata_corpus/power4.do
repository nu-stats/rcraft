* RCraft's Stata corpus (P2C.12): no data; Stata 19's results are in power4.recorded
* commands: power
* (power oneway: sample sizes from the means or varmeans(), power with n(), npergroup() or n1() to n3(), group weights, a total that does not split, and the between-group variance)
* tolerance: 1e-9
version 19
set linesize 255
quietly power oneway 1 2 3
display "@ on " r(N) " " %24.17g r(N_a) " " r(N_per_group)
quietly power oneway 1 2 3, n(61)
display "@ op " %24.17g r(power) " " r(N) " " r(N_a) " " %24.17g r(Var_m)
quietly power oneway, ngroups(4) varmeans(0.5) varerror(2) power(0.9) alpha(0.01)
display "@ ov " r(N) " " %24.17g r(N_a) " " %24.17g r(delta)
quietly power oneway 1 2 3, grweights(1 2 1)
display "@ ow " r(N) " " r(N1) " " r(N2) " " r(N3) " " %24.17g r(N_a) " " r(balanced)
quietly power oneway 1 2 3, n1(10) n2(20) n3(30)
display "@ os " %24.17g r(power) " " %24.17g r(Var_m) " " r(N_avg)
quietly power oneway, npergroup(12) ngroups(5) power(0.9) varerror(3)
display "@ oe " %24.17g r(Var_m) " " %24.17g r(delta) " " r(N)
quietly power oneway 1 1.5 2, alpha(0.1) power(0.95)
display "@ oa " r(N) " " %24.17g r(N_a)
quietly power oneway 1 2 3 4, npergroup(10)
display "@ og " %24.17g r(power) " " %24.17g r(beta)
