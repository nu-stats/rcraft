* RCraft's Stata corpus (packages' gaps): made-up data; Stata 19's results are in survwgt.recorded
* commands: isid survwgt
* (survwgt rake from SSC, as YFBPT6 rakes to age-by-gender quotas: one dimension with totals in percent, two and three dimensions with weights that vary, maxrep() reached, the add-on's errors, a missing weight and a missing category; isid's checks)
version 19
set linesize 255
clear
set seed 4321
set obs 300
generate long id = _n
generate byte g = 1 + floor(runiform()*5)
generate byte h = 1 + (runiform() > .55)
generate byte k = 1 + floor(runiform()*3)
generate double pre = .4 + runiform()*2
generate double tg = cond(g==1, 12, cond(g==2, 18, cond(g==3, 25, cond(g==4, 30, 15))))
generate double th = cond(h==1, 48, 52)
generate double tk = cond(k==1, 20, cond(k==2, 45, 35))
generate byte one = 1
isid id
capture isid g h
display "@ isid_dup " _rc
isid g id
survwgt rake one, by(g) totvars(tg) generate(w1) maxrep(1000)
display "@ sorted " g[1] " " g[_N]
sort id
summarize w1
display "@ one " w1[1] " " w1[2] " " w1[150] " " r(sum) " " r(min) " " r(max)
survwgt rake pre, by(g h) totvars(tg th) generate(w2) maxrep(50)
sort id
summarize w2
display "@ two " %18.0g w2[1] " " %18.0g w2[2] " " %18.0g w2[150] " " %18.0g w2[300] " " r(sum)
survwgt rake pre, by(g h k) totvars(tg th tk) generate(w3) maxrep(50)
sort id
display "@ three " %18.0g w3[1] " " %18.0g w3[2] " " %18.0g w3[150] " " %18.0g w3[300]
survwgt rake pre, by(g h k) totvars(tg th tk) generate(w4) maxrep(2)
sort id
display "@ maxrep " %18.0g w4[1] " " %18.0g w4[2] " " %18.0g w4[300]
survwgt rake pre, by(g h k) totvars(tg th tk) generate(w5)
sort id
display "@ default " %18.0g w5[1] " " %18.0g w5[300]
capture survwgt rake pre, by(g h) totvars(tg) generate(w6)
display "@ counts " _rc
capture survwgt rake pre, by(g) totvars(tg) generate(w1)
display "@ defined " _rc
generate double bad = tg + (id == 7)
capture survwgt rake pre, by(g) totvars(bad) generate(w6)
display "@ constant " _rc
generate double th2 = th * 1.01
capture survwgt rake pre, by(g h) totvars(tg th2) generate(w6)
display "@ unequal " _rc
sort id
replace pre = . in 3
replace g = . in 4
survwgt rake pre, by(g) totvars(tg) generate(w7) maxrep(1000)
sort id
display "@ missing " missing(w7[3]) " " %18.0g w7[4] " " %18.0g w7[5]
capture isid g id
display "@ isid_missing " _rc
isid g id, missok
