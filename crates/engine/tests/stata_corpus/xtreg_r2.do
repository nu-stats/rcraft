* RCraft's Stata corpus (packages' gaps): made-up data; Stata 19's results are in xtreg_r2.recorded
* commands: xtset xtreg
* (TZEN38 reads e(r2) after xtreg, fe: the within R-squared, with e(r2_a), e(rss), e(mss), e(tss) and e(rmse), plain, robust and clustered on the panel or on groups of panels; re leaves e(r2) missing)
* tolerance: 1e-10
version 19
set linesize 255
import delimited corpus.csv, clear
xtset panel t
generate big = ceil(panel / 2)
xtreg y x w, fe
display "@ fe " %21.0g e(r2) " " %21.0g e(r2_a) " " %21.0g e(rss) " " %21.0g e(mss) " " %21.0g e(tss) " " %21.0g e(rmse)
display "@ fe_w " (abs(e(r2) - e(r2_w)) < 1e-12)
xtreg y x w, fe vce(robust)
display "@ fe_rob " %21.0g e(r2) " " %21.0g e(r2_a) " " %21.0g e(rss) " " %21.0g e(mss) " " %21.0g e(tss) " " %21.0g e(rmse)
xtreg y x w, fe vce(cluster big)
display "@ fe_cl " %21.0g e(r2) " " %21.0g e(r2_a) " " %21.0g e(rmse)
xtreg y x w if t > 1, fe
display "@ fe_if " %21.0g e(r2) " " %21.0g e(r2_a) " " %21.0g e(rss) " " %21.0g e(mss) " " %21.0g e(tss) " " %21.0g e(rmse)
xtreg y x w i.cat3, fe
display "@ fe_i " %21.0g e(r2) " " %21.0g e(r2_a) " " %21.0g e(rmse)
local coeff = round(e(r2), 0.001)
display "@ fe_local `coeff'"
xtreg y x w, re
display "@ re " missing(e(r2))
