* RCraft's Stata corpus (P2C.12d continued): made-up data; Stata 19's results are in noncentral.recorded
* functions: nF nFtail nchi2 nchi2tail nt nttail
* (the noncentral t, chi2 and F distributions and their tails: R's give Stata's to about 1e-9 in the cdf, so the file is compared to 1e-6; the missing values Stata gives past df 1e10 (t), 1e6 (chi2) and 1e8 (F), np past 1000 (t), 1e4 (chi2, F) and 1000 (nFtail), np below 0 for chi2 and F, df of 0 or less and missing arguments)
* tolerance: 1e-6
version 19
clear
set linesize 255
foreach d in .5 1 2 5 10.5 30 100 1000 1e6 1e10 {
  foreach np in -3 -.5 0 .01 1 3 10 37 {
    foreach k in -2 -1 0 1 2 {
      display "@ nt `d' `np' `k' " %24.17g nt(`d', `np', `np' + `k') " " %24.17g nttail(`d', `np', `np' + `k')
    }
  }
}
display "@ nt_na " nt(0, 1, 1) " " nt(-1, 1, 1) " " nt(., 1, 1) " " nt(5, ., 1) " " nt(5, 1, .) " " nt(1e10, 1, 1) " " nt(1.0000001e10, 1, 1) " " nt(5, 1000.1, 1) " " nt(5, -1000.1, 1) " " nttail(0, 1, 1) " " nttail(5, ., 1) " " nttail(5, 1, .) " " nttail(1e10, 1, 1) " " nttail(1.0000001e10, 1, 1) " " nttail(5, 1000.1, 1)
display "@ nt_t " %24.17g nt(5, 1, 1e300) " " %24.17g nt(5, 1, -1e300) " " %24.17g nt(5, 0, 0) " " %24.17g nt(5, 2, 0) " " %24.17g nt(5, -2, 0) " " %24.17g nttail(5, -2, 0)
foreach d in .5 1 2 5 10.5 30 100 1000 1e5 1e6 {
  foreach np in 0 .01 1 3 10 37 100 1000 {
    foreach k in -1 0 1 2 {
      display "@ nchi2 `d' `np' `k' " %24.17g nchi2(`d', `np', (`d' + `np') + `k' * sqrt(2 * (`d' + 2 * `np'))) " " %24.17g nchi2tail(`d', `np', (`d' + `np') + `k' * sqrt(2 * (`d' + 2 * `np')))
    }
  }
}
display "@ nchi2_na " nchi2(0, 1, 1) " " nchi2(-1, 1, 1) " " nchi2(., 1, 1) " " nchi2(5, ., 1) " " nchi2(5, 1, .) " " nchi2(5, -.5, 1) " " nchi2(1e6, 1, 1e6) " " nchi2(1000001, 1, 1e6) " " nchi2(5, 1e4, 1e4) " " nchi2(5, 10001, 1e4) " " nchi2tail(0, 1, 1) " " nchi2tail(5, -.5, 1) " " nchi2tail(1e6, 1, 1e6) " " nchi2tail(1000001, 1, 1e6) " " nchi2tail(5, 1e4, 1e4) " " nchi2tail(5, 10001, 1e4)
display "@ nchi2_x " %24.17g nchi2(5, 1, -1) " " %24.17g nchi2(5, 1, 0) " " %24.17g nchi2(5, 1, 1e300) " " %24.17g nchi2tail(5, 1, 1e300) " " %24.17g nchi2tail(5, 1, -1)
foreach a in .5 1 3 10 30 {
  foreach b in .5 1 5 10 100 1e6 {
    foreach np in 0 .01 1 5 30 {
      foreach k in .7 1 1.5 {
        display "@ nF `a' `b' `np' `k' " %24.17g nF(`a', `b', `np', (`a' + `np') / `a' * `k') " " %24.17g nFtail(`a', `b', `np', (`a' + `np') / `a' * `k')
      }
    }
  }
}
display "@ nF_na " nF(0, 5, 1, 1) " " nF(5, 0, 1, 1) " " nF(-1, 5, 1, 1) " " nF(., 5, 1, 1) " " nF(5, ., 1, 1) " " nF(5, 5, ., 1) " " nF(5, 5, 1, .) " " nF(5, 5, -.5, 1) " " nF(1e8, 5, 1, 1) " " nF(1.00000001e8, 5, 1, 1) " " nF(5, 1e8, 1, 1) " " nF(5, 1.00000001e8, 1, 1) " " nF(5, 5, 1e4, 1e4) " " nF(5, 5, 10001, 1e4) " " nFtail(0, 5, 1, 1) " " nFtail(5, 5, -.5, 1) " " nFtail(1e8, 5, 1, 1) " " nFtail(1.00000001e8, 5, 1, 1) " " nFtail(5, 5, 1000, 1000) " " nFtail(5, 5, 1000.5, 1000) " " nFtail(5, 5, 1e4, 1e4)
display "@ nF_x " %24.17g nF(3, 10, 1, -1) " " %24.17g nF(3, 10, 1, 0) " " %24.17g nFtail(3, 10, 1, 0)
