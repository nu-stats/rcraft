* RCraft's Stata corpus (P2C.12y, batch A1): made-up data; Stata 19's results are in joinby.recorded
* commands: joinby
* (joinby: every pairing of the rows with the same key, unmatched() and _merge(), the keys by default, a string key and two keys, a variable in both files, an empty result, and the errors with their return codes; the order of the paired rows is Stata's own, so each result is sorted before it is looked at)
version 19
set linesize 255
clear
input byte k double uv byte tag
1 7 1
1 8 2
3 9 3
end
save joinby_u, replace
clear
input byte k double mv byte tag
1 10 5
1 20 6
2 30 7
4 40 8
end
save joinby_m, replace
joinby k using joinby_u
display "@ n1 " _N
capture confirm variable _merge
display "@ n1_merge " _rc
sort k mv uv
forvalues i = 1/`=_N' {
    display "@ r1_`i' " k[`i'] " " mv[`i'] " " tag[`i'] " " uv[`i']
}
use joinby_m, clear
joinby k using joinby_u, unmatched(both) _merge(how)
display "@ n2 " _N
sort how k mv uv
forvalues i = 1/`=_N' {
    display "@ r2_`i' " how[`i'] " " k[`i'] " " mv[`i'] " " tag[`i'] " " uv[`i']
}
use joinby_m, clear
joinby k using joinby_u, unmatched(master)
display "@ n3 " _N
sort _merge k mv uv
forvalues i = 1/`=_N' {
    display "@ r3_`i' " _merge[`i'] " " k[`i'] " " mv[`i'] " " uv[`i']
}
use joinby_m, clear
joinby k using joinby_u, unmatched(using)
display "@ n4 " _N
sort _merge k mv uv
forvalues i = 1/`=_N' {
    display "@ r4_`i' " _merge[`i'] " " k[`i'] " " mv[`i'] " " uv[`i']
}
use joinby_m, clear
joinby k using joinby_u, unmatched(none)
display "@ n5 " _N
use joinby_m, clear
joinby k tag using joinby_u
display "@ n6 " _N
use joinby_m, clear
generate double uv = 99
joinby k using joinby_u
display "@ n7 " _N " " uv[1] " " uv[2] " " uv[3] " " uv[4]
* string key and two keys
clear
input str2 g byte t double a
"x" 1 1
"x" 2 2
"y" 1 3
end
save joinby_u2, replace
clear
input str2 g byte t double b
"x" 1 10
"y" 1 20
"y" 2 30
"z" 1 40
end
joinby g t using joinby_u2, unmatched(both)
display "@ n8 " _N
sort _merge g t
forvalues i = 1/`=_N' {
    display "@ r8_`i' " _merge[`i'] " " (g[`i'] == "x") " " (g[`i'] == "y") " " (g[`i'] == "z") " " t[`i'] " " b[`i'] " " a[`i']
}
* every key variable, by default
clear
input str2 g byte t double b
"x" 1 10
"x" 2 11
end
joinby using joinby_u2
display "@ n9 " _N
* an empty result: no rows and the variables stay
clear
input byte k double mv byte tag
7 1 1
end
joinby k using joinby_u
display "@ n10 " _N
capture confirm variable uv
display "@ n10_var " _rc
* errors
use joinby_m, clear
capture joinby k using nosuchfile
display "@ rc_nofile " _rc
capture joinby k
display "@ rc_nousing " _rc
capture joinby nosuch using joinby_u
display "@ rc_nosuch " _rc
capture joinby k using joinby_u, unmatched(foo)
display "@ rc_unm " _rc
capture joinby k using joinby_u, foo
display "@ rc_opt " _rc
capture joinby k using joinby_u if k == 1
display "@ rc_if " _rc
generate byte _merge = 1
capture joinby k using joinby_u, unmatched(both)
display "@ rc_merge " _rc
capture joinby k using joinby_u
display "@ rc_merge_ok " _rc " " _N
clear
input str2 k double mv
"a" 1
end
capture joinby k using joinby_u
display "@ rc_type " _rc
clear
input str2 j double w
"a" 1
end
capture joinby using joinby_u
display "@ rc_common " _rc
* update and update replace: the master's value against the using file's, every kind of missing
clear
set obs 16
generate byte k = _n
generate byte ui = mod(_n-1, 4) + 1
generate double u = .
replace u = 7 if ui == 1
replace u = .a if ui == 3
replace u = .c if ui == 4
generate str1 s = cond(mod(ceil(_n/2), 2) == 1, "x", "")
keep k u s
save joinby_u3, replace
clear
set obs 17
generate byte k = _n
generate byte mi = ceil(_n/4)
generate double u = .
replace u = 5 if mi == 1
replace u = .a if mi == 3
replace u = .b if mi == 4
generate str1 s = cond(mod(_n, 2) == 1, "m", "")
generate byte id = _n
keep k u s id
save joinby_m3, replace
joinby k using joinby_u3
display "@ n11 " _N
forvalues i = 1/`=_N' {
    display "@ r11_`i' " (u[`i'] == 5) (u[`i'] == .) (u[`i'] == .a) (u[`i'] == .b) " " (s[`i'] == "m")
}
foreach how in update "update replace" {
    use joinby_m3, clear
    joinby k using joinby_u3, `how'
    display "@ n12 " _N
    sort id
    forvalues i = 1/`=_N' {
        display "@ r12_`i' " (u[`i'] == 5) (u[`i'] == 7) (u[`i'] == .) (u[`i'] == .a) (u[`i'] == .b) (u[`i'] == .c) " " (s[`i'] == "m") (s[`i'] == "x") (s[`i'] == "")
    }
    capture confirm variable _merge
    display "@ n12_merge " _rc
}
* update with unmatched(): _merge, its labels and format, the using-only rows take the using file's values
use joinby_m3, clear
drop if k == 17 | k == 1
joinby k using joinby_u3, update unmatched(both) _merge(how)
display "@ n13 " _N
sort how k
forvalues i = 1/`=_N' {
    display "@ r13_`i' " how[`i'] " " k[`i'] " " (u[`i'] == 5) (u[`i'] == 7) (u[`i'] == .) (u[`i'] == .a) (u[`i'] == .b) (u[`i'] == .c) " " (s[`i'] == "m") (s[`i'] == "x") (s[`i'] == "")
}
* a variable that is a string in one file and a number in the other is not touched
clear
input byte k double s
1 5
3 6
end
joinby k using joinby_u3, update unmatched(both)
display "@ n14 " _N
sort _merge k
forvalues i = 1/`=_N' {
    display "@ r14_`i' " _merge[`i'] " " k[`i'] " " s[`i'] " " (u[`i'] == 7) (u[`i'] == .)
}
* the errors of update and replace
use joinby_m3, clear
capture joinby k using joinby_u3, replace
display "@ rc_replace " _rc
capture joinby k using joinby_u3, upd
display "@ rc_abbrev " _rc
capture joinby nosuch using joinby_u3, replace
display "@ rc_replace_var " _rc
capture joinby k using joinby_u3, update unmatched(foo)
display "@ rc_update_unm " _rc
generate byte _merge = 1
capture joinby k using joinby_u3, update
display "@ rc_update_merge " _rc
capture joinby k using joinby_u3, update unmatched(master)
display "@ rc_update_merge2 " _rc
erase joinby_u.dta
erase joinby_m.dta
erase joinby_u2.dta
erase joinby_u3.dta
erase joinby_m3.dta
