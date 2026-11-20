* RCraft's Stata corpus (P2C.12): made-up data; Stata 19's results are in sspace.recorded
* commands: sspace constraint predict
* (sspace: an AR(1) state seen with noise, a random walk seen with noise (diffuse start, no stationarity), a constant and a regressor in the observation equation, a sub-sample; constraints(); e() scalars, _b and _se; predict xb and states with smethod(onestep) and smethod(filter))
* tolerance: 1e-5
version 19
clear
set linesize 255
set obs 120
generate t = _n
tsset t
generate double e = invnormal((mod(7*_n^3 + 13*_n^2 + 101*_n + 5, 997) + .5)/997)
generate double nz = 1.3*invnormal((mod(11*_n^3 + 17*_n^2 + 53*_n + 3, 991) + .5)/991)
generate double xr = 2*(mod(5*_n^3 + 19*_n^2 + 71*_n + 9, 983) + .5)/983
generate double s = e in 1
replace s = 0.7*s[_n-1] + e if _n > 1
generate double y = s + nz
generate double w = sum(e)
generate double z = w + nz
generate double y3 = y + .5*xr
constraint 1 [y]u = 1
quietly sspace (u L.u, state noconstant) (y u, noconstant), constraints(1)
display "@ ar1 " %24.17g e(ll) " " %24.17g e(chi2) " " e(N) " " e(k) " " e(rank) " " e(df_m) " " e(stationary) " " e(converged) " " e(k_eq) " " e(tmin) " " e(tmax)
display "@ ar1b " %24.17g _b[u:L.u] " " %24.17g _b[/state:var(u)] " " %24.17g _b[/observable:var(y)] " " %24.17g _se[u:L.u] " " %24.17g _se[/state:var(u)] " " %24.17g _se[/observable:var(y)] " " %24.17g _b[y:u]
predict double p1, xb
predict double p1f, states smethod(filter) equation(u)
predict double p1o, states smethod(onestep) equation(u)
quietly summarize p1
display "@ p1 " r(N) " " %24.17g r(mean) " " %24.17g r(sd) " " %24.17g p1[1] " " %24.17g p1[2] " " %24.17g p1[60] " " %24.17g p1[120]
quietly summarize p1f
display "@ p1f " r(N) " " %24.17g r(mean) " " %24.17g p1f[1] " " %24.17g p1f[2] " " %24.17g p1f[60] " " %24.17g p1f[120]
quietly summarize p1o
display "@ p1o " r(N) " " %24.17g r(mean) " " %24.17g p1o[1] " " %24.17g p1o[2] " " %24.17g p1o[60] " " %24.17g p1o[120]
constraint 2 [z]lev = 1
constraint 3 [lev]L.lev = 1
quietly sspace (lev L.lev, state noconstant) (z lev, noconstant), constraints(2 3)
display "@ rw " %24.17g e(ll) " " e(N) " " e(k) " " e(rank) " " e(df_m) " " e(stationary) " " e(converged) " " e(k_eq) " " e(chi2) " " e(p)
display "@ rwb " %24.17g _b[/state:var(lev)] " " %24.17g _b[/observable:var(z)] " " %24.17g _se[/state:var(lev)] " " %24.17g _se[/observable:var(z)] " " %24.17g _b[lev:L.lev] " " %24.17g _b[z:lev]
predict double p2, xb
predict double p2f, states smethod(filter) equation(lev)
predict double p2o, states equation(lev)
quietly summarize p2
display "@ p2 " r(N) " " %24.17g r(mean) " " p2[1] " " %24.17g p2[2] " " %24.17g p2[3] " " %24.17g p2[60] " " %24.17g p2[120]
quietly summarize p2f
display "@ p2f " r(N) " " %24.17g r(mean) " " p2f[1] " " %24.17g p2f[2] " " %24.17g p2f[60] " " %24.17g p2f[120]
display "@ p2o " %24.17g p2o[1] " " %24.17g p2o[2] " " %24.17g p2o[60]
quietly sspace (u L.u, state noconstant) (y u), constraints(1)
display "@ cons " %24.17g e(ll) " " e(k) " " e(rank) " " e(df_m) " " %24.17g e(chi2) " " %24.17g e(p)
display "@ consb " %24.17g _b[u:L.u] " " %24.17g _b[y:_cons] " " %24.17g _b[/state:var(u)] " " %24.17g _b[/observable:var(y)] " " %24.17g _se[u:L.u] " " %24.17g _se[y:_cons] " " %24.17g _se[/state:var(u)] " " %24.17g _se[/observable:var(y)]
constraint 4 [y3]u = 1
quietly sspace (u L.u, state noconstant) (y3 u xr, noconstant), constraints(4)
display "@ reg " %24.17g e(ll) " " e(k) " " e(rank) " " e(df_m) " " %24.17g e(chi2)
display "@ regb " %24.17g _b[u:L.u] " " %24.17g _b[y3:xr] " " %24.17g _b[/state:var(u)] " " %24.17g _b[/observable:var(y3)] " " %24.17g _se[u:L.u] " " %24.17g _se[y3:xr] " " %24.17g _se[/state:var(u)] " " %24.17g _se[/observable:var(y3)]
predict double p4, xb
quietly summarize p4
display "@ p4 " r(N) " " %24.17g r(mean) " " %24.17g p4[1] " " %24.17g p4[2] " " %24.17g p4[60]
quietly sspace (u L.u, state noconstant) (y u, noconstant) if t > 10 & t <= 110, constraints(1)
display "@ sub " %24.17g e(ll) " " e(N) " " e(tmin) " " e(tmax) " " %24.17g _b[u:L.u] " " %24.17g _se[u:L.u]
