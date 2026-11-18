* RCraft's Stata corpus (P2C.12 continued): made-up data; Stata 19's results are in spmatrix.recorded
* commands: spset spmatrix
* (spset with planar coordinates; inverse-distance weighting matrices with each normalize() and
* vtruncate(), on a subset and of two places; summarize's r(), export and import, copy, fromdata,
* save and use, and Stata's errors with their return codes)
* tolerance: 1e-13
version 19
set linesize 255
clear
set obs 40
generate int id = 100 + _n
generate double cx = mod(_n - 1, 8) + mod(_n * 5, 7) / 8
generate double cy = floor((_n - 1) / 8) + mod(_n * 3, 5) / 16
generate double a = mod(_n, 3)
generate double b = mod(_n, 2)
capture spset
display "@ err_notset " _rc
capture spmatrix create idistance W
display "@ err_create_notset " _rc
capture spset, coord(cx cy)
display "@ err_modify " _rc
capture spset dup, coord(cx cy)
display "@ err_novar " _rc
spset id, coord(cx cy)
capture spset id, coord(cx cy)
display "@ err_already " _rc
capture spset, modify coord(cx)
display "@ err_coord " _rc
summarize _ID _CX _CY
display "@ ids " r(N) " " r(min) " " r(max)
foreach n in none row minmax spectral {
    spmatrix create idistance W_`n', normalize(`n')
    spmatrix summarize W_`n'
    display "@ `n' " %21.0g r(max) " " %21.0g r(min0) " " %21.0g r(mean) " " r(min) " " r(n)
}
spmatrix create idistance Wt, normalize(row) vtruncate(.4)
spmatrix summarize Wt
display "@ trunc " %21.0g r(max) " " %21.0g r(min0) " " %21.0g r(mean)
spmatrix create idistance Ws if mod(_n, 3) == 0
spmatrix summarize Ws
display "@ subset " %21.0g r(max) " " %21.0g r(min0) " " %21.0g r(mean) " " r(n)
spmatrix create idistance W2 in 1/2
spmatrix summarize W2
display "@ two " r(max) " " r(min0) " " r(mean) " " r(n)
capture spmatrix create idistance W2 in 1/2
display "@ err_exists " _rc
spmatrix create idistance W2 in 3/5, replace normalize(none)
spmatrix summarize W2
display "@ replaced " %21.0g r(max) " " r(n)
capture spmatrix create idistance W9, normalize(bogus)
display "@ err_norm " _rc
capture spmatrix create idistance W9, dtruncate(2)
display "@ err_option " _rc
capture spmatrix create idistance
display "@ err_name " _rc
capture spmatrix create contiguity C
display "@ err_contig " _rc
capture spmatrix create bogus Z
display "@ err_sub " _rc
capture spmatrix
display "@ err_nosub " _rc
capture spmatrix summarize Nope
display "@ err_notfound " _rc " " r(n)
capture spmatrix summarize
display "@ err_summ " _rc
spmatrix export W_row using spmatrix_w, replace
capture spmatrix export W_row using spmatrix_w
display "@ err_export " _rc
spmatrix import G using spmatrix_w
spmatrix summarize G
display "@ import " %21.0g r(max) " " %21.0g r(min0) " " %21.0g r(mean) " " r(n)
capture spmatrix import G using spmatrix_w
display "@ err_import " _rc
capture spmatrix summarize G
display "@ import_dropped " _rc
capture spmatrix import G using spmatrix_nothere
display "@ err_nofile " _rc
spmatrix copy W_none C1
spmatrix summarize C1
display "@ copy " %21.0g r(mean) " " r(n)
capture spmatrix copy W_none C1
display "@ err_copy " _rc
capture spmatrix summarize C1
display "@ copy_dropped " _rc
capture spmatrix copy W_none
display "@ err_copy1 " _rc
capture spmatrix drop W_row W_none
display "@ err_drop2 " _rc
spmatrix drop Wt
capture spmatrix drop Wt
display "@ err_drop " _rc
spmatrix note W_none: made-up note
capture spmatrix note W_none, drop
display "@ err_note " _rc
spmatrix save W_none using spmatrix_s, replace
capture spmatrix save W_none using spmatrix_s
display "@ err_save " _rc
spmatrix drop W_none
spmatrix use W_none using spmatrix_s
spmatrix summarize W_none
display "@ used " %21.0g r(max) " " %21.0g r(mean) " " r(n)
capture spmatrix fromdata F = a b in 1/2
display "@ err_diag " _rc
replace a = 0 in 1
replace b = 0 in 2
spmatrix fromdata F = a b in 1/2, normalize(none)
spmatrix summarize F
display "@ fromdata " r(max) " " r(min0) " " r(mean) " " r(n)
capture spmatrix fromdata F2 = a b
display "@ err_square " _rc
clear
capture spset
display "@ err_nodata " _rc
spmatrix summarize W_spectral
display "@ kept " %21.0g r(max) " " r(n)
erase spmatrix_w.txt
erase spmatrix_s.stswm
