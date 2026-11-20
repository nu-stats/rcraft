* RCraft's Stata corpus (P2C.12aq): made-up data; Stata 19's results are in frlink.recorded
* (frlink 1:1 and m:1 with one or two keys and other names for the keys, generate(); frget with prefix(), suffix(), newvar = var; frval() with and without a default; describe with no data)
* tolerance: 1e-9
version 19
clear
set obs 6
generate k = _n
generate y = 10 * _n
frame create t
frame t {
    set obs 4
    generate k = 5 - _n
    generate v = 100 * k
    generate str6 s = "s" + string(k)
    generate float f = k / 7
    generate double dd = k / 3
    label variable v "the v"
    format dd %9.4f
}
frlink m:1 k, frame(t)
* the linked frame is sorted by its keys; the link holds the row numbers
forvalues i = 1/4 {
    frame t: display "@ tk`i' " k[`i']
}
forvalues i = 1/6 {
    display "@ link`i' " %24.17g cond(missing(t[`i']), -1, t[`i'])
}
count if missing(t)
display "@ unmatched " r(N)
frame dir
* frget copies values, text and missing where the link is missing
frget v s f dd, from(t)
forvalues i = 1/6 {
    display "@ v`i' " %24.17g cond(missing(v[`i']), -1, v[`i'])
    display "@ s`i' " (s[`i'] == "s" + string(`i'))
    display "@ slen`i' " strlen(s[`i'])
    display "@ f`i' " %24.17g cond(missing(f[`i']), -1, f[`i'])
    display "@ dd`i' " %24.17g cond(missing(dd[`i']), -1, dd[`i'])
}
local lab : variable label v
display "@ vlab " ("`lab'" == "the v")
* prefix, suffix, a new name, a variable named twice, a match variable
frget v, from(t) prefix(p_)
frget v, from(t) suffix(_q)
frget nv = f, from(t)
frget f v, from(t) prefix(z_)
frget k, from(t) prefix(kk_)
describe, short
display "@ nvars " r(k)
display "@ pv3 " p_v[3]
display "@ vq3 " v_q[3]
display "@ nv3 " %24.17g nv[3]
display "@ zv2 " z_v[2]
* frval, with and without a default
generate double e1 = frval(t, v)
generate e2 = frval(t, f) + frval(t, v)
generate e3 = frval(t, v, -1)
generate str6 e4 = frval(t, s, "none")
generate e5 = frval(t, s) == ""
count if frval(t, v) > 250
display "@ big " r(N)
count if frval(t, v, 0) > 250
display "@ big0 " r(N)
forvalues i = 1/6 {
    display "@ e1_`i' " %24.17g cond(missing(e1[`i']), -1, e1[`i'])
    display "@ e2_`i' " %24.17g cond(missing(e2[`i']), -1, e2[`i'])
    display "@ e3_`i' " %24.17g e3[`i']
    display "@ e4_`i' " (e4[`i'] == "none")
    display "@ e5_`i' " e5[`i']
}
* rows of this frame move; the link follows its rows
sort y
drop if y == 20
forvalues i = 1/5 {
    display "@ keep`i' " %24.17g cond(missing(t[`i']), -1, t[`i'])
}
generate double late = frval(t, dd)
display "@ late1 " %24.17g cond(missing(late[1]), -1, late[1])
* two keys with other names, generate(), 1:1
clear
set obs 6
generate state = 1 + mod(_n, 2)
generate year = 2000 + mod(_n, 3)
generate y = _n
frame create rates
frame rates {
    set obs 6
    generate st = 1 + mod(_n - 1, 2)
    generate yr = 2000 + floor((_n - 1) / 2)
    generate rate = 10 * _n
}
frlink m:1 state year, frame(rates st yr) generate(lk)
forvalues i = 1/6 {
    display "@ lk`i' " %24.17g cond(missing(lk[`i']), -1, lk[`i'])
}
frame rates: display "@ rates1_st " st[1]
frame rates: display "@ rates1_yr " yr[1]
frame rates: display "@ rates6_st " st[6]
frame rates: display "@ rates6_yr " yr[6]
frget rate, from(lk) prefix(r_)
forvalues i = 1/6 {
    display "@ r_rate`i' " %24.17g r_rate[`i']
}
quietly frlink m:1 state year, frame(rates st yr) generate(lk2)
display "@ lk2_1 " %24.17g lk2[1]
clear
set obs 3
generate id = _n
generate str3 w = "a" + string(_n)
frame create ids
frame ids {
    set obs 5
    generate id = 6 - _n
    generate str3 w = "a" + string(id)
    generate double z = id / 8
}
frlink 1:1 id, frame(ids)
frlink 1:1 w, frame(ids) generate(lw)
display "@ ids1 " ids[1]
display "@ lw1 " lw[1]
display "@ ids3 " ids[3]
display "@ lw3 " lw[3]
frame ids: display "@ id_first " id[1]
frget z, from(lw)
display "@ z3 " %24.17g z[3]
* a frame with missing keys: missing matches missing
clear
set obs 4
generate k = cond(_n <= 2, ., _n)
frame create m
frame m {
    set obs 3
    generate k = cond(_n == 1, ., _n)
    generate v = 10 * _n
}
frlink m:1 k, frame(m) generate(lm)
forvalues i = 1/4 {
    display "@ lm`i' " %24.17g cond(missing(lm[`i']), -1, lm[`i'])
}
* a link made from inside another frame
clear
set obs 3
generate id = _n
frame create back
frame back {
    set obs 5
    generate id = _n
    generate val = 7 * id
}
frame back: frlink m:1 id, frame(default)
frame back: display "@ nback " _N
frame dir
* no data in memory
clear
describe, short
display "@ empty_k " r(k)
display "@ empty_n " r(N)
frames reset
describe, short
display "@ reset_k " r(k)
