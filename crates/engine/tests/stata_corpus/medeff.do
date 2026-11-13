* RCraft's Stata corpus (P2C.9 continued): made-up data; Stata 19's results are in medeff.recorded
* commands: medeff
* (medeff of the mediation package: the draws from Stata's generator after seed(), the ACME, direct and total effects and the share mediated with _pctile's intervals, the generator left after the draws, e() the outcome model)
* tolerance: 1e-8
version 19
clear
set obs 60
generate double t = mod(_n, 2)
generate double x = sin(_n)
generate double m = 0.5 * t + 0.3 * x + cos(_n * 3)
generate double y = 0.2 * t - 0.6 * m + 0.1 * x + sin(_n * 7)
medeff (regress m t x) (regress y m t x), mediate(m) treat(t) sims(100) seed(12345)
display "@ a " r(delta0) " " r(delta0lo) " " r(delta0hi) " " r(delta1)
display "@ b " r(zeta1) " " r(zeta0lo) " " r(tau) " " r(taulo) " " r(tauhi)
display "@ c " r(navg) " " r(navglo) " " r(navghi)
display "@ u " runiform()
display "@ e " e(N) " " _b[t] " " _b[m] " " colsof(e(b))
quietly medeff (regress m t x) (regress y m t x), mediate(m) treat(t) sims(40) seed(12345)
display "@ f " r(delta0lo) " " r(delta0hi) " " r(navg) " " r(taulo)
quietly medeff (regress m t) (regress y m t), mediate(m) treat(t) sims(3) seed(99)
display "@ g " r(delta0) " " r(zeta0) " " r(navg)
