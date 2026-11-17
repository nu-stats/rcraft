* RCraft's Stata corpus (P2C.12 continued): made-up data; Stata 19's results are in npregress.recorded
* commands: npregress
* (npregress kernel with one continuous covariate: the cross-validated mean and derivative
* bandwidths, the kernels, the local-constant estimator, given bandwidths, iterate(), the
* derivative's cap, identification, the variables it leaves; Stata's errors)
* tolerance: 1e-9
version 19
set linesize 255
clear
set obs 70
generate double x = (_n + mod(_n * 11, 5) / 5) / 7
generate double y = 3 + sin(x * 1.3) * 2 + mod(_n * 23, 13) / 13 - 0.5
quietly npregress kernel y x
display "@ default " %20.0g el(e(bwidth), 1, 1) " " %20.0g el(e(bwidth), 1, 2) " " %20.0g el(e(b), 1, 1) " " %20.0g el(e(b), 1, 2) " " %20.0g e(r2) " " e(nh) " " e(converged)
display "@ ilog " %20.0g el(e(ilog_mean), 1, 1) " " %20.0g el(e(ilog_effect), 1, 2)
display "@ vars " %20.0g _Mean_y[10] " " %20.0g _d_Mean_y_dx[10]
quietly npregress kernel y x, kernel(gaussian)
display "@ gaussian " %20.0g el(e(bwidth), 1, 1) " " %20.0g el(e(bwidth), 1, 2) " " %20.0g el(e(b), 1, 2)
quietly npregress kernel y x, kernel(triweight)
display "@ triweight " %20.0g el(e(bwidth), 1, 1) " " %20.0g el(e(bwidth), 1, 2)
quietly npregress kernel y x, estimator(constant) noderivatives kernel(epan2)
display "@ constant " %20.0g el(e(bwidth), 1, 1) " " %20.0g e(mean) " " %20.0g e(r2)
quietly npregress kernel y x, bwidth(0.6 0.9, copy) predict(m d)
display "@ given " %20.0g el(e(b), 1, 1) " " %20.0g el(e(b), 1, 2) " " %20.0g m[5] " " %20.0g d[5] " " e(converged) " " e(converged_mean)
quietly npregress kernel y x, meanbwidth(0.5, copy) noderivatives
display "@ meanbw " %20.0g e(mean) " " e(converged) " " e(nh)
quietly npregress kernel y x, iterate(2)
display "@ iterate " e(converged) " " e(converged_mean) " " e(converged_effect) " " %20.0g el(e(bwidth), 1, 1)
quietly npregress kernel y x in 1/45
display "@ capped " %20.0g el(e(bwidth), 1, 1) " " %20.0g el(e(bwidth), 1, 2)
generate double xg = cond(_n < 70, x, 30)
quietly npregress kernel y xg, bwidth(0.6 0.9, copy)
display "@ unident " %20.0g e(mean) " " e(N) " " _unident_sample[70] " " _unident_sample[1]
capture npregress kernel y
display "@ err_covariate " _rc
capture npregress kernel y x, bwidth(0.5)
display "@ err_copy " _rc
capture npregress kernel y x, bwidth(0.5 0.8 0.9, copy)
display "@ err_dim " _rc
capture npregress kernel y x, kernel(nonsense)
display "@ err_kernel " _rc
capture npregress kernel y x, predict(onlyone)
display "@ err_predict " _rc
