* RCraft's Stata corpus (P2C.12): made-up data; Stata 19's results are in dfactor.recorded
* commands: dfactor predict
* (dfactor: one AR(1) factor behind three series, and a model with an autoregressive observation equation; e() scalars, _b and _se; predict xb and factors with smethod(onestep) and smethod(filter))
* tolerance: 1e-5
version 19
clear
set linesize 255
set obs 120
generate t = _n
tsset t
generate double e = invnormal((mod(7*_n^3 + 13*_n^2 + 101*_n + 5, 997) + .5)/997)
generate double n1 = .5*invnormal((mod(3*_n^3 + 23*_n^2 + 61*_n + 17, 977) + .5)/977)
generate double n2 = .5*invnormal((mod(13*_n^3 + 29*_n^2 + 37*_n + 11, 971) + .5)/971)
generate double n3 = .5*invnormal((mod(17*_n^3 + 31*_n^2 + 43*_n + 19, 967) + .5)/967)
generate double f = e in 1
replace f = 0.6*f[_n-1] + e if _n > 1
generate double y1 = f + n1
generate double y2 = .8*f + n2
generate double y3 = -.5*f + n3
quietly dfactor (y1 y2 y3 = , noconstant) (f = , ar(1))
display "@ df " %24.17g e(ll) " " %24.17g e(chi2) " " e(N) " " e(k) " " e(rank) " " e(df_m) " " e(stationary) " " e(converged) " " e(k_eq) " " e(k_factor) " " e(k_obser) " " e(f_ar_max) " " e(o_ar_max)
display "@ dfb " %24.17g _b[f:L.f] " " %24.17g _b[y1:f] " " %24.17g _b[y2:f] " " %24.17g _b[y3:f] " " %24.17g _b[/observable:var(e.y1)] " " %24.17g _b[/observable:var(e.y2)] " " %24.17g _b[/observable:var(e.y3)]
display "@ dfs " %24.17g _se[f:L.f] " " %24.17g _se[y1:f] " " %24.17g _se[y2:f] " " %24.17g _se[y3:f] " " %24.17g _se[/observable:var(e.y1)] " " %24.17g _se[/observable:var(e.y2)] " " %24.17g _se[/observable:var(e.y3)]
predict double ff, factors smethod(filter)
predict double fo, factors
predict double xb1, xb
quietly summarize ff
display "@ ff " r(N) " " %24.17g r(mean) " " %24.17g ff[1] " " %24.17g ff[2] " " %24.17g ff[60] " " %24.17g ff[120]
quietly summarize fo
display "@ fo " r(N) " " %24.17g r(mean) " " %24.17g fo[1] " " %24.17g fo[2] " " %24.17g fo[60] " " %24.17g fo[120]
quietly summarize xb1
display "@ xb " r(N) " " %24.17g r(mean) " " %24.17g xb1[1] " " %24.17g xb1[2] " " %24.17g xb1[60]
quietly dfactor (y1 y2 = , noconstant ar(1))
display "@ ar " %24.17g e(ll) " " %24.17g e(chi2) " " e(N) " " e(k) " " e(rank) " " e(df_m) " " e(stationary) " " e(k_eq) " " e(k_factor) " " e(o_ar_max) " " e(f_ar_max)
display "@ arb " %24.17g _b[e.y1:L.e.y1] " " %24.17g _b[e.y2:L.e.y2] " " %24.17g _b[/observable:var(e.y1)] " " %24.17g _b[/observable:var(e.y2)] " " %24.17g _se[e.y1:L.e.y1] " " %24.17g _se[e.y2:L.e.y2] " " %24.17g _se[/observable:var(e.y1)] " " %24.17g _se[/observable:var(e.y2)]
predict double xb2, xb
quietly summarize xb2
display "@ xb2 " r(N) " " %24.17g r(mean) " " %24.17g xb2[1] " " %24.17g xb2[2] " " %24.17g xb2[60]
