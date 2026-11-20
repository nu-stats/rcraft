* RCraft's Stata corpus (P2C.12): made-up data; Stata 19's results are in ucm.recorded
* commands: ucm predict
* (ucm: local level, random walk, local level with a regressor, local linear trend; e() scalars, _b and _se; predict xb and trend with smethod(onestep) and smethod(filter))
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
generate double v = .3*invnormal((mod(19*_n^3 + 37*_n^2 + 29*_n + 23, 953) + .5)/953)
generate double w = sum(e)
generate double z = w + nz
generate double y2 = z + .8*xr
generate double b = .2 + sum(v)
generate double y4 = sum(b) + w + nz
quietly ucm z, model(llevel)
display "@ ll " %24.17g e(ll) " " e(N) " " e(k) " " e(rank) " " e(df_m) " " e(stationary) " " e(converged) " " e(k_eq) " " e(k_cycles) " " e(tmin) " " e(tmax)
display "@ llb " %24.17g _b[/:var(level)] " " %24.17g _b[/:var(z)] " " %24.17g _se[/:var(level)] " " %24.17g _se[/:var(z)]
predict double l1, trend
predict double l1f, trend smethod(filter)
predict double l1x, xb
quietly summarize l1
display "@ l1 " r(N) " " %24.17g r(mean) " " l1[1] " " %24.17g l1[2] " " %24.17g l1[3] " " %24.17g l1[60] " " %24.17g l1[120]
quietly summarize l1f
display "@ l1f " r(N) " " %24.17g r(mean) " " l1f[1] " " %24.17g l1f[2] " " %24.17g l1f[60] " " %24.17g l1f[120]
quietly summarize l1x
display "@ l1x " r(N) " " %24.17g r(mean) " " l1x[1] " " %24.17g l1x[2] " " %24.17g l1x[60]
quietly ucm z, model(rwalk)
display "@ rw " %24.17g e(ll) " " e(k) " " e(rank) " " e(df_m) " " e(stationary) " " e(k_eq) " " %24.17g _b[/:var(level)] " " %24.17g _se[/:var(level)]
quietly ucm y2 xr, model(llevel)
display "@ reg " %24.17g e(ll) " " e(k) " " e(rank) " " e(df_m) " " %24.17g e(chi2) " " %24.17g e(p)
display "@ regb " %24.17g _b[xr] " " %24.17g _b[/:var(level)] " " %24.17g _b[/:var(y2)] " " %24.17g _se[xr] " " %24.17g _se[/:var(level)] " " %24.17g _se[/:var(y2)]
predict double l2, trend
predict double l2x, xb
quietly summarize l2
display "@ l2 " r(N) " " %24.17g r(mean) " " l2[1] " " %24.17g l2[2] " " %24.17g l2[60]
quietly summarize l2x
display "@ l2x " r(N) " " %24.17g r(mean) " " l2x[1] " " %24.17g l2x[2] " " %24.17g l2x[60]
quietly ucm y4, model(lltrend)
display "@ lt " %24.17g e(ll) " " e(k) " " e(rank) " " e(df_m) " " e(stationary) " " e(converged)
display "@ ltb " %24.17g _b[/:var(level)] " " %24.17g _b[/:var(slope)] " " %24.17g _b[/:var(y4)] " " %24.17g _se[/:var(level)] " " %24.17g _se[/:var(slope)] " " %24.17g _se[/:var(y4)]
predict double l4, trend
predict double l4f, trend smethod(filter)
quietly summarize l4
display "@ l4 " r(N) " " %24.17g r(mean) " " l4[1] " " %24.17g l4[2] " " %24.17g l4[3] " " %24.17g l4[60] " " %24.17g l4[120]
quietly summarize l4f
display "@ l4f " r(N) " " %24.17g r(mean) " " l4f[1] " " l4f[2] " " %24.17g l4f[3] " " %24.17g l4f[60] " " %24.17g l4f[120]
