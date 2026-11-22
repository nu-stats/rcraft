* RCraft's Stata corpus (P2C.12ao): made-up data; Stata 19's results are in frames_more.recorded
* (frame create with variables, frame name: command and frame name { }, frame copy (replace), rename and put, cwf, frames reset, c(frame); frame dir runs and prints)
* tolerance: 1e-9
version 19
clear
set obs 4
generate id = _n
generate x = 10 * _n
frame create cities
frame cities: set obs 3
frame cities: generate id = _n + 1
frame cities: generate pop = 1000 * id
frame cities {
    summarize pop
    display "@ pop_sum " r(sum)
    display "@ in_cities " c(frame)
}
display "@ back " c(frame)
display "@ n_default " _N
frame cities: display "@ n_cities " _N
frame copy default backup
frame backup: display "@ n_backup " _N
capture frame copy default backup
display "@ copy_again " _rc
capture frame copy nosuch other
display "@ copy_missing " _rc
frame copy cities backup, replace
frame backup: display "@ n_backup2 " _N
frame copy cities default, replace
display "@ n_default2 " _N
frame rename backup late
capture frame change backup
display "@ renamed_old " _rc
frame late: display "@ n_late " _N
capture frame rename late cities
display "@ rename_exists " _rc
capture frame rename nosuch zzz
display "@ rename_missing " _rc
frame put id pop if id > 2, into(subset)
frame subset: display "@ n_subset " _N
frame subset: summarize pop
frame subset: display "@ pop_sum2 " r(sum)
capture frame put id, into(subset)
display "@ put_exists " _rc
capture frame nosuch: summarize pop
display "@ prefix_missing " _rc
frame create other id str3 lab
frame other: display "@ n_other " _N
frame other: describe
frame other: set obs 2
frame other: display "@ n_other2 " _N
cwf cities
display "@ cwf " c(frame)
cwf default
display "@ cwf_back " c(frame)
frame dir
frames reset
display "@ frame_after_reset " c(frame)
capture frame change cities
display "@ cities_gone " _rc
frame dir
