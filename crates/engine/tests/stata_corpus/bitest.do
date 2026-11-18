* RCraft's Stata corpus (P2C.12, batch C): made-up data; Stata 19's results are in bitest.recorded
* commands: bitest bitesti
* (bitest and bitesti: N, k, expected k, the one-sided and two-sided probabilities, the opposite k, r(), frequency weights, if, the one-sided-only cases, and the errors with their return codes)
* tolerance: 1e-9
version 19
set linesize 255
clear
input b k w2 str2 s
1 3 .5 "a"
0 1 1.5 "b"
1 2 2.2 "c"
. 4 1 "d"
1 1 .3 "e"
0 5 2 "f"
1 2 1 "g"
0 3 .7 "h"
0 1 1.1 "i"
1 2 3 "j"
end
foreach p in 0.3 0.5 0.8 0.1 0.95 0.55556 {
    bitest b == `p'
    display "@ b_`p' " r(N) " " r(k) " " %24.17g r(P_p) " " r(k_opp) " " %24.17g r(p) " " %24.17g r(p_l) " " %24.17g r(p_u)
}
bitest b == .5 [fweight=k]
display "@ bw " r(N) " " r(k) " " r(k_opp) " " %24.17g r(p) " " %24.17g r(p_l) " " %24.17g r(p_u)
bitest b == .4 [fweight=k]
display "@ bw4 " r(N) " " r(k) " " r(k_opp) " " %24.17g r(p) " " %24.17g r(p_l) " " %24.17g r(p_u)
bitest b == .5 if k > 1
display "@ bif " r(N) " " r(k) " " r(k_opp) " " %24.17g r(p) " " %24.17g r(p_l) " " %24.17g r(p_u)
bitest b == .5 in 1/5
display "@ bin " r(N) " " r(k) " " r(k_opp) " " %24.17g r(p) " " %24.17g r(p_l) " " %24.17g r(p_u)
generate byte nz = b * 3
replace nz = . if b == .
replace nz = 1 if nz == 3
bitest nz == .5
display "@ bnz " r(N) " " r(k) " " r(k_opp) " " %24.17g r(p)
bitesti 20 7 .5
display "@ i1 " r(N) " " r(k) " " r(k_opp) " " %24.17g r(p) " " %24.17g r(p_l) " " %24.17g r(p_u)
bitesti 20 7 .3
display "@ i2 " r(N) " " r(k) " " r(k_opp) " " %24.17g r(p) " " %24.17g r(p_l) " " %24.17g r(p_u)
bitesti 100 50 .5
display "@ i3 " r(N) " " r(k) " " r(k_opp) " " %24.17g r(p) " " %24.17g r(p_l) " " %24.17g r(p_u)
bitesti 10 0 .5
display "@ i4 " r(N) " " r(k) " " r(k_opp) " " %24.17g r(p) " " %24.17g r(p_l) " " %24.17g r(p_u)
bitesti 10 10 .5
display "@ i5 " r(N) " " r(k) " " r(k_opp) " " %24.17g r(p) " " %24.17g r(p_l) " " %24.17g r(p_u)
bitesti 10 5 .5
display "@ i6 " r(N) " " r(k) " " r(k_opp) " " %24.17g r(p) " " %24.17g r(p_l) " " %24.17g r(p_u)
bitesti 1000 520 .5
display "@ i7 " r(N) " " r(k) " " r(k_opp) " " %24.17g r(p) " " %24.17g r(p_l) " " %24.17g r(p_u)
bitesti 30 12 .3333333
display "@ i8 " r(N) " " r(k) " " r(k_opp) " " %24.17g r(p) " " %24.17g r(p_l) " " %24.17g r(p_u)
bitesti 10 3 .0000001
display "@ i9 " r(N) " " r(k) " " r(k_opp) " " %24.17g r(p) " " %24.17g r(p_l) " " %24.17g r(p_u)
bitesti 5000 2600 .5
display "@ i10 " r(N) " " r(k) " " r(k_opp) " " %24.17g r(p) " " %24.17g r(p_l) " " %24.17g r(p_u)
bitesti 50 5 .2
display "@ i11 " r(N) " " r(k) " " r(k_opp) " " %24.17g r(p) " " %24.17g r(p_l) " " %24.17g r(p_u)
bitesti 2 1 .5
display "@ i12 " r(N) " " r(k) " " r(k_opp) " " %24.17g r(p) " " %24.17g r(p_l) " " %24.17g r(p_u)
capture bitest b == 1.5
display "@ rc1 " _rc
capture bitest k == .5
display "@ rc2 " _rc
capture bitest s == .5
display "@ rc3 " _rc
capture bitest b
display "@ rc4 " _rc
capture bitest b == .5 if k > 100
display "@ rc5 " _rc
capture bitest b k == .5
display "@ rc6 " _rc
capture bitest b == .5 [aweight=w2]
display "@ rc7 " _rc
capture bitest b == .5 [pweight=w2]
display "@ rc8 " _rc
capture bitest b == .5 [iweight=w2]
display "@ rc9 " _rc
capture bitest b == .5 [fweight=w2]
display "@ rc10 " _rc
capture bitesti 7 3 1
display "@ rc11 " _rc
capture bitesti 7 3 0
display "@ rc12 " _rc
capture bitesti 10 11 .5
display "@ rc13 " _rc
capture bitesti 10.5 3 .5
display "@ rc14 " _rc
capture bitesti 10 3
display "@ rc15 " _rc
capture bitesti 10 -1 .5
display "@ rc16 " _rc
capture bitesti 1 1 .5
display "@ rc17 " _rc
