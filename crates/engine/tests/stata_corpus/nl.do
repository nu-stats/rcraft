* RCraft's Stata corpus (P2C.9 continued): made-up data; Stata 19's results are in nl.recorded
* commands: nl
* (nl's interactive form: starting values as %9.0g writes them, Gauss-Newton with ml's step search and its stopping rule (e(ic)), the parameters in byte order, a parameter that is the constant (centered R2), e() and _b[/name])
* tolerance: 1e-6
version 19
clear
set obs 40
generate x = _n / 4
generate y1 = 3 * exp(0.3 * x) + sin(_n)
generate y2 = 10 / (1 + exp(-(x - 5))) + cos(_n * 3) / 2
generate y3 = 2 * x^1.5 + sin(_n * 7)
nl (y1 = {a = 1} * exp({b = 0.1} * x))
display "@ a " _b[/a] " " _b[/b] " " _se[/a] " " _se[/b] " " e(ic) " " e(rss) " " e(r2) " " e(dev)
nl (y2 = {k = 8} / (1 + exp(-({r = 1}) * (x - {m = 4})))), level(90)
display "@ b " _b[/k] " " _b[/m] " " _b[/r] " " _se[/m] " " e(ic) " " e(r2_a) " " e(rmse) " " e(df_r)
quietly nl (y3 = {c = 1} * x^{p = 1})
display "@ c " _b[/c] " " _b[/p] " " e(ic) " " e(mss) " " e(tss)
nl (y3 = {zeta = 1} + {alpha = 1} * x + {Mid = 0.1} * x^2) if x > 2
display "@ d " _b[/Mid] " " _b[/alpha] " " _b[/zeta] " " e(N) " " e(tss) " " e(r2) " " e(ic)
nl (y1 = {a = 123456.789} + {b = 0.123456789} * x), iterate(0)
display "@ e " _b[/a] " " _b[/b] " " e(ic) " " e(converged)
