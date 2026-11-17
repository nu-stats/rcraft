* RCraft's Stata corpus (P2C.12): no data; Stata 19's results are in power7.recorded
* commands: power
* (power oneslope, rsquared and pcorr: sample sizes, power and the target slope or R2, with sdx(), sderror(), sdy(), ncontrol() and ntested(), one- and two-sided)
* tolerance: 1e-9
version 19
set linesize 255
quietly power oneslope 0 0.5
display "@ on " r(N) " " %24.17g r(corr) " " %24.17g r(sdy)
quietly power oneslope 0.2 0.5, n(30) sdx(1.5) onesided
display "@ op " %24.17g r(power)
quietly power oneslope 0 0.5, sdy(1.2)
display "@ oy " r(N) " " %24.17g r(sderror)
quietly power oneslope 0, n(40) power(0.8) direction(lower) sdy(2)
display "@ oe " %24.17g r(ba) " " %24.17g r(delta)
quietly power oneslope 0 -0.3, onesided
display "@ om " r(N)
quietly power rsquared 0.1, ntested(3)
display "@ rn " r(N)
quietly power rsquared 0.1, n(80)
display "@ rp " %24.17g r(power)
quietly power rsquared, n(80) power(0.8) ntested(2)
display "@ re " %24.17g r(R2_T)
quietly power rsquared 0.2 0.3, n(80) ncontrol(2) ntested(2)
display "@ sp " %24.17g r(power)
quietly power rsquared 0.2, n(80) power(0.8) ncontrol(2)
display "@ se " %24.17g r(R2_F)
quietly power pcorr 0.3, ncontrol(2) ntested(2) alpha(0.01)
display "@ pn " r(N)
quietly power pcorr 0.3, n(50) ncontrol(2)
display "@ pp " %24.17g r(power)
quietly power pcorr, n(50) power(0.8) ncontrol(1)
display "@ pe " %24.17g r(rho2_p)
