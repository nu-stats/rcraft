* RCraft's Stata corpus (P2C.14): made-up data; Stata 19's results are in dates.recorded
* functions: clock date dhms dofc dofm dofq dofw dow doy halfyearly hh hms mdy mdyhms mm mofd monthly qofd quarter quarterly ss week weekly wofd year yearly yq yw
* (Stata's date and time functions on columns: strings read against masks, two-digit years, times, periods, and the parts of dates)
* tolerance: 0
version 19
clear
set obs 12
generate str40 s = ""
replace s = "01dec2006" in 1
replace s = "December 1, 2006" in 2
replace s = "2006-12-01" in 3
replace s = "20061201" in 4
replace s = "29feb2023" in 5
replace s = "29feb2024" in 6
replace s = "01dec98" in 7
replace s = "Fri Dec 01 14:22:43 CST 2006" in 8
replace s = "01dec2006 2:22 p.m." in 9
replace s = "01dec2006 14:22:43.213" in 10
replace s = "nonsense" in 11
generate double d = date(s, "DMY")
generate double d2 = date(s, "MDY")
generate double d3 = date(s, "YMD")
generate double d98 = date(s, "DMY", 2020)
generate double c = clock(s, "DMYhms")
generate double c2 = clock(s, "DMYhm")
generate double c3 = clock(s, "#MDhms#Y")
display "@ d_1 " %24.17g d[1] " " %24.17g d[5]
display "@ d_6 " %24.17g d[6] " " %24.17g d[7]
display "@ d_11 " %24.17g d[11] " " %24.17g d[12]
display "@ d2_2 " %24.17g d2[2] " " %24.17g d3[3]
display "@ d3_4 " %24.17g d3[4] " " %24.17g d98[7]
display "@ c_10 " %24.17g c[10] " " %24.17g c2[9]
display "@ c3_8 " %24.17g c3[8] " " %24.17g c[1]
generate str10 m = ""
replace m = "2006m1" in 1
replace m = "1999m12" in 2
replace m = "2024m13" in 3
replace m = "06m4" in 4
generate double mo = monthly(m, "YM")
generate double mo2 = monthly(m, "YM", 2050)
display "@ mo_1 " %24.17g mo[1] " " %24.17g mo[2]
display "@ mo_3 " %24.17g mo[3] " " %24.17g mo2[4]
generate str10 q = ""
replace q = "2006q3" in 1
replace q = "1q2006" in 2
replace q = "2006w52" in 3
replace q = "2006" in 4
generate double qu = quarterly(q, "YQ")
generate double qu2 = quarterly(q, "QY")
generate double wk = weekly(q, "YW")
generate double hy = halfyearly(q, "YH")
generate double yr = yearly(q, "Y")
display "@ qu_1 " %24.17g qu[1] " " %24.17g qu2[2]
display "@ wk_3 " %24.17g wk[3] " " %24.17g hy[1]
display "@ yr_4 " %24.17g yr[4] " " %24.17g yr[1]
* the parts of dates, on a column of days
generate double day = mdy(mod(_n * 5, 12) + 1, mod(_n * 7, 28) + 1, 1950 + _n * 9)
generate double yy = year(day)
generate double dw = dow(day)
generate double dy = doy(day)
generate double wn = week(day)
generate double qt = quarter(day)
summarize day
display "@ day_sum " %24.17g r(sum) " " %24.17g day[12]
summarize yy
display "@ yy_sum " %24.17g r(sum)
summarize dw
display "@ dw_sum " %24.17g r(sum)
summarize dy
display "@ dy_sum " %24.17g r(sum)
summarize wn
display "@ wn_sum " %24.17g r(sum)
summarize qt
display "@ qt_sum " %24.17g r(sum)
generate double per = mofd(day) + qofd(day) + wofd(day)
generate double back = dofm(mofd(day)) + dofq(qofd(day)) + dofw(wofd(day))
summarize per
display "@ per_sum " %24.17g r(sum)
summarize back
display "@ back_sum " %24.17g r(sum)
generate double ywq = yw(1950 + _n, _n * 4) + yq(2000, mod(_n, 5))
display "@ ywq " %24.17g ywq[3] " " %24.17g ywq[5]
* times
generate double t = mdyhms(12, 1, 2006, _n, _n * 4, _n * 4.5) + dhms(day, 1, 2, 3.25) - hms(_n, 0, 1)
generate double hm = hh(t) * 10000 + mm(t) * 100 + ss(t)
generate double dc = dofc(t)
summarize hm
display "@ hm_sum " %24.17g r(sum) " " %24.17g hm[7]
summarize dc
display "@ dc_sum " %24.17g r(sum) " " %24.17g t[3]
