* RCraft's Stata corpus (P2C.12y, batch B): made-up data; Stata 19's results are in snapshot.recorded
* commands: snapshot
* (snapshot: save with and without a label and r(snapshot), the next number is the highest one there plus one, restore brings back the data (variables, values, strings, labels), erase of one, several and _all, label, list, and the errors with their return codes)
version 19
set linesize 80
clear
set obs 3
generate x = _n
generate str5 t = "a" + string(_n)
label variable x "the x"
label define lb 1 "one" 2 "two"
label values x lb
capture noisily snapshot restore 1
display "@ none_restore " _rc
capture noisily snapshot label 1 "a"
display "@ none_label " _rc
capture noisily snapshot erase 1
display "@ none_erase " _rc
snapshot save
display "@ id1 " r(snapshot)
snapshot save, label("second one")
display "@ id2 " r(snapshot)
replace x = x * 10
generate y = 5
drop t
snapshot save
display "@ id3 " r(snapshot)
snapshot restore 1
display "@ r1 " _N " " x[1] " " x[3] " " (t[2] == "a2")
capture confirm variable y
display "@ r1_y " _rc
decode x, generate(xt)
display "@ r1_lab " (xt[1] == "one") " " (xt[3] == "")
drop xt
snapshot restore 3
display "@ r3 " _N " " x[1] " " x[3] " " y[2]
capture confirm variable t
display "@ r3_t " _rc
snapshot label 2 "renamed"
snapshot erase 2
capture noisily snapshot restore 2
display "@ gone " _rc
capture noisily snapshot erase 2
display "@ gone_erase " _rc
capture noisily snapshot restore 0
display "@ low " _rc
capture noisily snapshot restore 9
display "@ high " _rc
capture noisily snapshot label 9 "x"
display "@ label_high " _rc
capture noisily snapshot erase 1/3
display "@ range_missing " _rc
snapshot list
snapshot save
display "@ id_after " r(snapshot)
snapshot erase 3
snapshot save, label("")
display "@ id_again " r(snapshot)
snapshot list 1
snapshot list _all
capture noisily snapshot list 9
display "@ list_missing " _rc
capture noisily snapshot bogus
display "@ bogus " _rc
capture noisily snapshot
display "@ bare " _rc
capture noisily snapshot save, bogus
display "@ opt " _rc
capture noisily snapshot save x
display "@ varlist " _rc
capture noisily snapshot restore
display "@ rest_none " _rc
capture noisily snapshot restore 1 3
display "@ rest_two " _rc
capture noisily snapshot restore a
display "@ rest_a " _rc
capture noisily snapshot erase
display "@ erase_none " _rc
capture noisily snapshot erase a
display "@ erase_a " _rc
snapshot save, label("a label that is quite long, longer than eighty characters for sure, yes indeed it is long")
display "@ id_long " r(snapshot)
snapshot list
snapshot erase _all
snapshot list
snapshot save
display "@ id_reset " r(snapshot)
clear
snapshot restore 1
display "@ r_again " _N " " x[2]
snapshot erase _all
