* RCraft's Stata corpus (P2C.12): made-up data; Stata 19's results are in distributions.recorded
* functions: F Fden Ftail betaden binomial binomialp binomialtail chi2 chi2den chi2tail comb digamma gammaden gammap gammaptail ibeta ibetatail invF invFtail invchi2 invchi2tail invgammap invgammaptail invibeta invibetatail invt invttail lnfactorial lngamma lnnormal lnnormalden normalden poisson poissonp poissontail t tden tprob trigamma ttail
* (Stata's distribution functions in display and generate: t, F, chi2, normal, beta, gamma, binomial and Poisson, their tails, densities and inverses; the missing values Stata gives where R gives a number: a degree of freedom, shape or sd of 0, counts that are not whole, k outside 0..n, and the limits past which Stata gives missing; invF() at small p, where R's qf() gives 0)
* tolerance: 1e-9
version 19
clear
set linesize 255
* p-values and critical values as do-files work them out by hand
display "@ t " %24.17g t(12, 1.7) " " %24.17g ttail(12, 1.7) " " %24.17g 2*ttail(37.5, abs(-2.31)) " " %24.17g tprob(12, -1.7)
display "@ tden " %24.17g tden(4, .3) " " %24.17g tden(1e8, 1.96) " " %24.17g ttail(1, -40)
display "@ invt " %24.17g invt(12, .975) " " %24.17g invttail(12, .025) " " %24.17g invttail(2.5, 1e-10) " " %24.17g invt(30, .5)
display "@ F " %24.17g F(3, 20, 2.1) " " %24.17g Ftail(3, 20, 2.1) " " %24.17g Fden(3, 20, 2.1) " " %24.17g Ftail(2.5, 1e6, 3.2)
display "@ invF " %24.17g invF(3, 20, .95) " " %24.17g invFtail(3, 20, .05) " " %24.17g invF(1, 10, 1e-10) " " %24.17g invF(2, 2, 1e-12)
display "@ invF2 " %24.17g invFtail(1, 1e6, .001) " " %24.17g invFtail(1, 1, 1e-8) " " %24.17g invF(50, 3, .999)
display "@ chi2 " %24.17g chi2(2, 3.84) " " %24.17g chi2tail(1, 3.84) " " %24.17g chi2den(2.5, 1) " " %24.17g chi2tail(1e5, 1e5)
display "@ invchi2 " %24.17g invchi2(4, .95) " " %24.17g invchi2tail(4, .05) " " %24.17g invchi2tail(1, 1e-12)
display "@ normal " %24.17g normalden(1.5) " " %24.17g normalden(1.5, 2) " " %24.17g normalden(1.5, 1, 2) " " %24.17g lnnormal(-38.5)
display "@ lnnormal " %24.17g lnnormalden(10) " " %24.17g lnnormalden(10, 2) " " %24.17g lnnormalden(10, 1, 2)
display "@ beta " %24.17g ibeta(2.5, 3, .3) " " %24.17g ibetatail(.5, .5, .3) " " %24.17g betaden(2.5, 3, .3) " " %24.17g invibeta(2.5, 3, .5) " " %24.17g invibetatail(10, .5, .3)
display "@ gamma " %24.17g gammap(2.5, 3) " " %24.17g gammaptail(.5, 30) " " %24.17g gammaden(2, 1, .5, .6) " " %24.17g gammaden(2, 1, -3, -2)
display "@ invgamma " %24.17g invgammap(2.5, .3) " " %24.17g invgammaptail(10, 1e-10)
display "@ lngamma " %24.17g lngamma(3.7) " " %24.17g lngamma(-2.5) " " %24.17g lngamma(1e306) " " %24.17g digamma(3.7) " " %24.17g trigamma(3.7)
display "@ lnfact " %24.17g lnfactorial(0) " " %24.17g lnfactorial(10) " " %24.17g lnfactorial(1e5) " " %24.17g comb(10, 3) " " %24.17g comb(60, 30) " " %24.17g comb(1e10, 2)
display "@ binomial " %24.17g binomial(10, 3, .3) " " %24.17g binomialtail(10, 3, .3) " " %24.17g binomialp(10, 3, .3) " " %24.17g binomial(10.5, 3.5, .3)
display "@ binomial2 " %24.17g binomial(10, -1, .3) " " %24.17g binomial(10, 11, .3) " " %24.17g binomialtail(10, 0, .3) " " %24.17g binomialtail(10, 11, .3) " " %24.17g binomialp(10.5, 10, 1)
display "@ poisson " %24.17g poisson(3, 2) " " %24.17g poissontail(3, 2.5) " " %24.17g poissonp(3, 2.5) " " %24.17g poissontail(.5, 0) " " %24.17g poisson(100, 10)
* where Stata gives missing and R a number
display "@ miss_df " ttail(0, 1) " " t(-1, 1) " " chi2(0, 1) " " chi2tail(0, .5) " " invchi2(0, .5) " " F(0, 10, 1) " " Ftail(3, 0, 1) " " invFtail(3, 0, .05)
display "@ miss_shape " gammap(0, 1) " " gammaptail(0, .5) " " invgammap(0, .5) " " ibeta(0, 3, .3) " " ibeta(.5, 0, .3) " " invibeta(0, 3, .5) " " gammaden(0, 1, 0, 1) " " gammaden(2, 0, 0, 1)
display "@ miss_sd " normalden(1, 0) " " normalden(1, 1, 0) " " lnnormalden(1, 0) " " lnnormalden(1, 1, -1) " " %24.17g normalden(., 2) " " normalden(., 1, 2)
display "@ miss_big " ttail(3e17, 2) " " chi2tail(3e17, 5) " " Ftail(3, 3e17, 2) " " ibeta(1.1e17, 2, .5) " " gammap(1.1e17, 1e17) " " binomial(1.1e17, 2, .5)
display "@ miss_whole " lnfactorial(3.7) " " lnfactorial(-1) " " comb(5, 2.5) " " comb(5.5, 2) " " comb(5, 10) " " comb(5, -1) " " %24.17g comb(0, 0)
display "@ miss_count " binomialp(0, 0, .3) " " binomialp(10, 11, .3) " " binomialp(10, -1, .3) " " binomial(10, 3, 1.2) " " binomial(-1, 0, .3) " " poisson(0, 2) " " poisson(3, -1) " " poissonp(-1, 2)
display "@ miss_args " ttail(., 1) " " ttail(5, .) " " invttail(5, 0) " " invttail(5, 1) " " invttail(5, 1.1) " " lngamma(0) " " lngamma(-2)
* on the data, a row at a time
set obs 7
generate double x = _n - 3.5
replace x = . in 6
generate double df = cond(_n == 4, 0, _n)
generate double p1 = 2*ttail(df, abs(x))
generate double p2 = Ftail(df, 30, x^2)
generate double p3 = normalden(x, 2)
generate double p4 = invchi2tail(df, .05)
generate double p5 = binomial(7, _n, .4)
generate double p6 = comb(7, _n)
forvalues i = 1/7 {
    display "@ row`i' " %24.17g p1[`i'] " " %24.17g p2[`i'] " " %24.17g p3[`i'] " " %24.17g p4[`i'] " " %24.17g p5[`i'] " " %24.17g p6[`i']
}
scalar s = invttail(df[2], .025)
display "@ scalar " %24.17g s " " %24.17g chi2tail(2, s^2)
* the limits past which Stata 19 gives missing
display "@ lim_pois " %24.17g poisson(9.007e15, 2) " " poisson(9.008e15, 2) " " %24.17g poissontail(9e15, 9e15) " " poissontail(9.1e15, 2)
display "@ lim_poisp " %24.17g poissonp(1e8, 5) " " poissonp(1.0000001e8, 5) " " %24.17g poissonp(5, 1e9) " " poissonp(5, 1.0000001e9)
display "@ lim_binp " %24.17g binomialp(1e6, 5e5, .5) " " binomialp(1000001, 5, .5) " " %24.17g binomialp(1000000.5, 5e5, .5)
display "@ lim_invt " %24.17g invt(1, 1e-154) " " invt(1, 1e-155) " " %24.17g invttail(1, 5e-155) " " invttail(1, 3e-155) " " %24.17g invttail(0.9, 1e-130) " " invt(0.9, 1e-140)
