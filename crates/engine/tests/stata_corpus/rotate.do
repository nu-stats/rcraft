* RCraft's Stata corpus (P2C.12): made-up data; Stata 19's results are in rotate.recorded
* commands: rotate
* (varimax, quartimax, equamax and promax after factor and pca, with and without Kaiser's normalization: e(r_L), e(r_T), e(r_Phi), e(r_Ev) and the texts; Stata's errors)
* tolerance: 1e-9
version 19
clear
set linesize 255
set obs 90
generate double f = mod(_n * 37, 101) / 101 - .5
generate double g = mod(_n * 53, 97) / 97 - .5
generate double h = mod(_n * 29, 89) / 89 - .5
generate double x1 = f + .5 * (mod(_n * 11, 83) / 83 - .5)
generate double x2 = f + .3 * g + .6 * (mod(_n * 17, 79) / 79 - .5)
generate double x3 = -f + .7 * (mod(_n * 19, 73) / 73 - .5)
generate double x4 = .5 * f + g + .5 * (mod(_n * 23, 71) / 71 - .5)
generate double x5 = g + .4 * (mod(_n * 31, 67) / 67 - .5)
generate double x6 = h + .3 * g + .5 * (mod(_n * 41, 61) / 61 - .5)
generate double x7 = h - .4 * f + .5 * (mod(_n * 43, 59) / 59 - .5)
quietly factor x1-x7, factors(3)
quietly rotate
display "@ vari `r(rtext)'|" e(r_ctitle) "|" e(r_class) "|" e(r_criterion) "|" e(r_normalization) "|" e(r_f)
display "@ vari T " %24.17g el(e(r_T), 1, 1) " " %24.17g el(e(r_T), 2, 3) " " %24.17g el(e(r_T), 3, 2)
display "@ vari L " %24.17g el(e(r_L), 1, 1) " " %24.17g el(e(r_L), 5, 2) " " %24.17g el(e(r_L), 7, 3)
display "@ vari Ev " %24.17g el(e(r_Ev), 1, 1) " " %24.17g el(e(r_Ev), 1, 3)
quietly rotate, normalize
display "@ norm `r(rtext)'|" e(r_normalization) " " %24.17g el(e(r_T), 1, 2) " " %24.17g el(e(r_L), 4, 2)
quietly rotate, quartimax
display "@ quarti " %24.17g el(e(r_T), 1, 1) " " %24.17g el(e(r_T), 3, 1) " " %24.17g el(e(r_Ev), 1, 2)
quietly rotate, equamax
display "@ equa " %24.17g el(e(r_T), 2, 2) " " %24.17g el(e(r_L), 6, 3) " " %24.17g el(e(r_Ev), 1, 1)
quietly rotate, promax
display "@ promax `r(rtext)'|" e(r_ctitle) "|" e(r_class) " " %24.17g el(e(r_T), 1, 1) " " %24.17g el(e(r_T), 2, 3)
display "@ promax L " %24.17g el(e(r_L), 1, 1) " " %24.17g el(e(r_L), 7, 3) " " %24.17g el(e(r_Phi), 2, 1) " " %24.17g el(e(r_Phi), 3, 1)
display "@ promax Ev " %24.17g el(e(r_Ev), 1, 1) " " %24.17g el(e(r_Ev), 1, 2) " " %24.17g el(e(r_Ev), 1, 3)
quietly rotate, promax(2) normalize
display "@ promax2n " e(r_ctitle) " " %24.17g el(e(r_T), 3, 3) " " %24.17g el(e(r_Phi), 3, 2) " " %24.17g el(e(r_Ev), 1, 2)
quietly factor x1-x7, factors(2)
quietly rotate
display "@ two " %24.17g el(e(r_T), 1, 1) " " %24.17g el(e(r_T), 2, 1) " " %24.17g el(e(r_L), 3, 1) " " %24.17g el(e(r_Ev), 1, 2)
quietly rotate, promax(4)
display "@ two promax " %24.17g el(e(r_T), 1, 2) " " %24.17g el(e(r_Phi), 2, 1) " " %24.17g el(e(r_Ev), 1, 1)
quietly factor x1-x7, ipf factors(2)
quietly rotate, equamax
display "@ ipf equa " %24.17g el(e(r_T), 1, 1) " " %24.17g el(e(r_L), 2, 2)
quietly pca x1-x7, components(3)
quietly rotate
display "@ pca `r(rtext)'|" e(r_ctitle) " " %24.17g el(e(r_T), 1, 1) " " %24.17g el(e(r_L), 2, 3) " " %24.17g el(e(r_Ev), 1, 1)
quietly rotate, promax
display "@ pca promax " %24.17g el(e(r_T), 2, 1) " " %24.17g el(e(r_L), 5, 2) " " %24.17g el(e(r_Ev), 1, 3)
quietly rotate, normalize
display "@ pca norm " %24.17g el(e(r_T), 3, 3) " " %24.17g el(e(r_Ev), 1, 2)
quietly rotate, clear
capture rotate, varimax promax
display "@ rc two " _rc
capture rotate, oblique
display "@ rc oblique " _rc
capture rotate, promax(.5)
display "@ rc power " _rc
capture rotate x1
display "@ rc varlist " _rc
quietly regress x1 x2
capture rotate
display "@ rc regress " _rc
