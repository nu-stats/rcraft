* RCraft's Stata corpus (P2C.12d, third round): made-up numbers; Stata 19's results are in noncentral3.recorded
* functions: invnF invnFtail npnF npnchi2 npnt
* (the noncentral F's inverses and the noncentrality solvers, with the missing values Stata gives at the edges of their domains, of p and of the root)
* tolerance: 1e-6
version 19
clear
set linesize 255
foreach a in .5 1 3 10.5 100 1e4 {
  foreach b in .5 2 7 30 1000 1e6 {
    foreach np in 0 .01 1 10 100 1000 {
      display "@ invnF `a' `b' `np' " %24.17g invnF(`a', `b', `np', 1e-5) " " %24.17g invnF(`a', `b', `np', .025) " " %24.17g invnF(`a', `b', `np', .5) " " %24.17g invnF(`a', `b', `np', .9) " " %24.17g invnF(`a', `b', `np', .995)
    }
  }
}
foreach a in 1 7 {
  foreach b in 3 50 {
    foreach np in 1500 5000 1e4 {
      display "@ invnF_big `a' `b' `np' " %24.17g invnF(`a', `b', `np', .001) " " %24.17g invnF(`a', `b', `np', .5) " " %24.17g invnF(`a', `b', `np', .99)
    }
  }
}
foreach a in .5 1 3 10.5 100 1000 {
  foreach b in 1.5 2 7 30 1000 {
    foreach np in 0 .01 1 10 100 1000 {
      display "@ invnFtail `a' `b' `np' " %24.17g invnFtail(`a', `b', `np', 1e-5) " " %24.17g invnFtail(`a', `b', `np', .025) " " %24.17g invnFtail(`a', `b', `np', .5) " " %24.17g invnFtail(`a', `b', `np', .9) " " %24.17g invnFtail(`a', `b', `np', .99999)
    }
  }
}
display "@ invnF_na1 " missing(invnF(1e-6, 5, 0, .5)) " " invnF(9.9e-7, 5, 0, .5) " " missing(invnF(3, 1e-6, 0, .001)) " " invnF(3, 9.9e-7, 0, .001) " " invnF(1e6, 5, 1, .5) " " invnF(1000001, 5, 1, .5) " " invnF(3, 1e6, 1, .5) " " invnF(3, 1000001, 1, .5) " " invnF(3, 5, -1e-9, .5) " " invnF(3, 5, 1e4, .5) " " invnF(3, 5, 10001, .5) " " invnF(3, 5, ., .5) " " invnF(., 5, 1, .5) " " invnF(3, ., 1, .5) " " invnF(3, 5, 1, .)
display "@ invnF_na2 " invnF(3, 5, 1, 0) " " invnF(3, 5, 0, 0) " " invnF(3, 5, 1, -1e-9) " " %24.17g invnF(3, 5, 1, .999755859375) " " invnF(3, 5, 1, .99976) " " invnF(3, 5, 0, .99976) " " invnF(3, 5, 1, 1) " " %24.17g invnF(3, 5, 1, 1e-8) " " invnF(3, 5, 1, 9.9e-9) " " %24.17g invnF(3, 5, 0, 1e-10) " " %24.17g invnF(.5, .5, 0, 1e-10) " " missing(invnF(1, 1e6, 0, 1e-300))
display "@ invnF_cap " invnF(1, 1, 1, .99) " " invnF(1, 1, 100, .999) " " invnF(10, 1, 1000, .999) " " invnF(100, 1, 1000, .999) " " invnF(.5, .5, .01, .999) " " %24.17g invnF(3, 1, 30, .99) " " %24.17g invnF(1, 1, 1, .9)
display "@ invnFtail_na " invnFtail(3, 5, 1, 1) " " invnFtail(0, 5, 1, 1) " " invnFtail(3, ., 1, 1) " " invnFtail(3, 5, -1, 1) " " invnFtail(3, 5, 1001, 1) " " invnFtail(3, 5, ., 1) " " invnFtail(3, 5, 1, 0) " " invnFtail(3, 5, 1, 9.9e-15) " " invnFtail(3, 5, 1, 1.1) " " invnFtail(0, 5, 1, .5) " " invnFtail(3, 0, 1, .5) " " invnFtail(3, 5, -1e-9, .5) " " %24.17g invnFtail(3, 5, 1000, .5) " " invnFtail(3, 5, 1000.5, .5) " " missing(invnFtail(1e-12, 5, 1, .5)) " " missing(invnFtail(3, 1e12, 1, .5))
display "@ invnFtail_cap " %24.17g invnFtail(1, 1, 1, 1e-12) " " %24.17g invnFtail(.5, 1, 1, 1e-10) " " %24.17g invnFtail(1, 1, 1000, 1e-12) " " %24.17g invnFtail(1, 1, 0, 1e-12) " " %24.17g invnFtail(.5, .5, 0, 1e-10)
foreach d in .5 1 5 30.5 1000 1e6 {
  foreach x in .1 1 5 30 300 1e4 1.2e4 {
    display "@ npnchi2 `d' `x' " %24.17g npnchi2(`d', `x', 1e-9) " " %24.17g npnchi2(`d', `x', 1e-6) " " %24.17g npnchi2(`d', `x', .01) " " %24.17g npnchi2(`d', `x', .3) " " %24.17g npnchi2(`d', `x', .5) " " %24.17g npnchi2(`d', `x', .9) " " %24.17g npnchi2(`d', `x', .999)
  }
}
display "@ npnchi2_na " %24.17g npnchi2(2e-10, 10, .5) " " npnchi2(1.9e-10, 10, .5) " " npnchi2(0, 10, .5) " " %24.17g npnchi2(1e6, 1e6 + 10, .5) " " npnchi2(1000001, 1000011, .5) " " %24.17g npnchi2(5, 30, 2^-30) " " npnchi2(5, 30, 9e-10) " " npnchi2(5, 30, 0) " " npnchi2(5, 30, 1) " " npnchi2(5, 1e5, 1) " " npnchi2(5, 0, .5) " " npnchi2(5, -1, .5) " " npnchi2(5, 10, .999) " " npnchi2(5, 3, chi2(5, 3)) " " %24.17g npnchi2(1, 1e4, .5) " " npnchi2(1, 10001, .5) " " npnchi2(., 3, .5) " " npnchi2(5, ., .5) " " npnchi2(5, 3, .)
foreach a in .5 1 3 30 1e4 {
  foreach b in .5 5 100 1e6 {
    foreach x in .1 1 3 30 1e3 {
      display "@ npnF `a' `b' `x' " %24.17g npnF(`a', `b', `x', 1e-8) " " %24.17g npnF(`a', `b', `x', 1e-5) " " %24.17g npnF(`a', `b', `x', .1) " " %24.17g npnF(`a', `b', `x', .5) " " %24.17g npnF(`a', `b', `x', .9) " " %24.17g npnF(`a', `b', `x', .999)
    }
  }
}
display "@ npnF_na " %24.17g npnF(2e-10, 5, 10, .5) " " npnF(1.9e-10, 5, 10, .5) " " %24.17g npnF(1e6, 5, 100, .001) " " npnF(1000001, 5, 100, .001) " " %24.17g npnF(3, 1e6, 10, .5) " " npnF(3, 1000001, 10, .5) " " npnF(0, 5, 10, .5) " " npnF(3, 0, 10, .5) " " npnF(3, 5, 10, 9.9e-9) " " npnF(3, 5, 10, 0) " " npnF(3, 5, 10, 1) " " npnF(3, 5, 1, .9) " " npnF(3, 5, 1, F(3, 5, 1)) " " npnF(3, 5, 0, .5) " " npnF(3, 5, -1, .5) " " npnF(3, 1e6, 3400, .5) " " npnF(., 5, 10, .5) " " npnF(3, ., 10, .5) " " npnF(3, 5, ., .5) " " npnF(3, 5, 10, .)
foreach d in .5 1 3 30 1000 1e8 {
  foreach t in -50 -3 -.5 0 .5 2 10 100 900 {
    display "@ npnt `d' `t' " %24.17g npnt(`d', `t', 1e-4) " " %24.17g npnt(`d', `t', 1e-5) " " %24.17g npnt(`d', `t', .1) " " %24.17g npnt(`d', `t', .5) " " %24.17g npnt(`d', `t', .9) " " %24.17g npnt(`d', `t', .99999)
  }
}
display "@ npnt_na " %24.17g npnt(1e8, 1, .5) " " npnt(100000001, 1, .5) " " npnt(0, 1, .5) " " npnt(-1, 1, .5) " " npnt(5, 1, 9.9e-9) " " %24.17g npnt(5, 1, 1 - 1e-8) " " npnt(5, 1, .999999991) " " npnt(5, 1, 0) " " npnt(5, 1, 1) " " %24.17g npnt(1e6, 1000, .5) " " npnt(1e6, 1001, .5) " " npnt(1e6, -1001, .5) " " npnt(5, 1e300, .5) " " npnt(., 1, .5) " " npnt(5, ., .5) " " npnt(5, 1, .)
