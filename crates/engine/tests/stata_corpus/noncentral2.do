* RCraft's Stata corpus (P2C.12d continued): made-up data; Stata 19's results are in noncentral2.recorded
* functions: invnchi2 invnchi2tail invnt invnttail nt nttail
* (nt() and nttail() with a noncentrality parameter above 37.62 in size, where R's pt() takes an approximation Stata does not, integrated here; invnchi2() and invnchi2tail(); invnt() and invnttail() found by root-finding; the missing values Stata gives outside their domains: df, np and p)
* tolerance: 1e-7
version 19
clear
set linesize 255
foreach d in .5 1 2 5 10.5 30 100 1000 1e6 1e10 {
  foreach np in -1000 -500 -100 -50 -38 38 50 100 500 1000 {
    foreach k in -1 0 1 {
      display "@ ntbig `d' `np' `k' " %24.17g nt(`d', `np', `np' * (1 + `k' * .05)) " " %24.17g nttail(`d', `np', `np' * (1 + `k' * .05))
    }
  }
}
display "@ ntbig_x " %24.17g nt(5, 100, 1e300) " " %24.17g nt(5, 100, -1e300) " " %24.17g nttail(5, -100, 1e300) " " %24.17g nt(5, 1000, 0) " " %24.17g nt(5, -1000, 0) " " %24.17g nttail(5, 500, 0)
foreach d in .5 1 2 5 10.5 30 100 1e5 1e6 {
  foreach np in 0 .01 1 10 100 1000 1e4 {
    foreach p in .001 .1 .5 .9 .999 {
      display "@ invnchi2 `d' `np' `p' " %24.17g invnchi2(`d', `np', `p') " " %24.17g invnchi2tail(`d', `np', `p')
    }
  }
}
display "@ invnchi2_na " invnchi2(0, 1, .5) " " invnchi2(-1, 1, .5) " " invnchi2(., 1, .5) " " invnchi2(5, ., .5) " " invnchi2(5, 1, .) " " invnchi2(5, -.5, .5) " " invnchi2(1e6, 1, .5) " " invnchi2(1000001, 1, .5) " " invnchi2(5, 1e4, .5) " " invnchi2(5, 10001, .5) " " invnchi2(5, 1, -.1) " " invnchi2(5, 1, 1.1) " " invnchi2(5, 1, 0) " " invnchi2(5, 1, 1) " " invnchi2tail(5, 1, 0) " " invnchi2tail(5, 1, 1) " " invnchi2tail(1000001, 1, .5) " " invnchi2tail(5, 10001, .5) " " invnchi2tail(5, 1, 1.1)
foreach d in 1 2 5 10.5 30 100 1e6 {
  foreach np in -37 -5 -1 0 .01 1 3 10 37 {
    foreach p in .001 .025 .5 .9 .999 {
      display "@ invnt `d' `np' `p' " %24.17g invnt(`d', `np', `p') " " %24.17g invnttail(`d', `np', `p')
    }
  }
}
foreach d in 1 5 30 1e6 {
  foreach np in -1000 -500 -50 40 500 1000 {
    foreach p in .001 .5 .999 {
      display "@ invntbig `d' `np' `p' " %24.17g invnt(`d', `np', `p') " " %24.17g invnttail(`d', `np', `p')
    }
  }
}
display "@ invnt_na " invnt(.5, 1, .5) " " invnt(.99, 1, .5) " " invnt(0, 1, .5) " " invnt(-1, 1, .5) " " invnt(., 1, .5) " " invnt(5, ., .5) " " invnt(5, 1, .) " " invnt(1e6, 1, .5) " " invnt(1000001, 1, .5) " " invnt(5, 1000, .5) " " invnt(5, 1001, .5) " " invnt(5, -1001, .5) " " invnt(5, 1, 0) " " invnt(5, 1, 1) " " invnt(5, 1, -.1) " " invnt(5, 1, 1.1) " " invnt(5, 1, 1e-9) " " invnt(5, 1, .999999999) " " missing(invnt(5, 1, 1e-8)) " " missing(invnt(5, 1, .99999999)) " " missing(invnttail(5, 1, 1e-8)) " " missing(invnttail(5, 1, .99999999)) " " invnt(5, 0, 1e-10) " " invnt(5, 0, .9999999999)
display "@ invnttail_na " invnttail(.5, 1, .5) " " invnttail(1e6, 1, .5) " " invnttail(1000001, 1, .5) " " invnttail(5, 1001, .5) " " invnttail(5, 1, 0) " " invnttail(5, 1, 1) " " invnttail(5, 1, 1e-9) " " invnttail(5, 1, .999999999) " " invnttail(5, 0, 1e-9) " " invnttail(5, 0, .999999999) " " invnttail(5, 0, .5) " " invnttail(5, ., .5) " " invnttail(., 1, .5)
