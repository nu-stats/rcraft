* RCraft's Stata corpus (P2C.12): no data; Stata 19's results are in power10.recorded
* commands: power
* (power trend: the Cochran-Armitage test of 2 to 20 equally spaced levels, sample sizes, power, a total that does not split, one- and two-sided)
* tolerance: 1e-9
version 19
set linesize 255
quietly power trend 0.2 0.4 0.6
display "@ tn " r(N) " " r(N_per_group) " " %24.17g r(N_a) " " r(N_g)
quietly power trend 0.2 0.4 0.6, onesided
display "@ to " r(N) " " r(N_per_group) " " %24.17g r(N_a)
quietly power trend 0.2 0.4 0.6, n(60)
display "@ tp " %24.17g r(power) " " r(N_avg) " " r(balanced)
quietly power trend 0.2 0.4 0.6, n(61)
display "@ ta " %24.17g r(power) " " r(N) " " r(N_a)
quietly power trend 0.6 0.4 0.2, onesided n(60)
display "@ td " %24.17g r(power)
quietly power trend 0.2 0.4 0.6, power(0.9) alpha(0.01)
display "@ tq " r(N) " " r(N_per_group)
quietly power trend 0.2 0.3 0.4 0.5, n(200)
display "@ tk " %24.17g r(power) " " r(N_g)
quietly power trend 0.2 0.4, n(50)
display "@ t2 " %24.17g r(power)
