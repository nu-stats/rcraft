* RCraft's Stata corpus (P2C.12): made-up data; Stata 19's results are in predictnl.recorded
* commands: predictnl
* (predictnl after logit and regress: an expression of _b[] and the data, predict(pr), xb(), if, se(), variance(), wald(), p(), ci(), level(), double; an expression constant over rows; a variable already defined)
* tolerance: 1e-6
version 19
clear
set linesize 255
set seed 315
set obs 300
generate double x1 = round(rnormal(), .01)
generate double x2 = round(runiform()*2, .01)
generate double y = round(1 + .5*x1 - .3*x2 + rnormal(), .01)
generate byte yb = (x1 + rnormal() > 0)
generate byte yc = floor(exp(.2 + .3*x1 + .5*runiform()))
logit yb x1 x2
predictnl p = invlogit(_b[_cons] + _b[x1]*x1 + _b[x2]*x2), se(p_se) ci(p_lo p_hi)
summarize p
display "@ p " %24.17g r(mean) " " %24.17g r(sd) " " %24.17g r(min) " " %24.17g r(max) " " r(N)
summarize p_se
display "@ pse " %24.17g r(mean) " " %24.17g r(sd) " " %24.17g r(min) " " %24.17g r(max)
summarize p_lo
display "@ plo " %24.17g r(mean) " " %24.17g r(min)
summarize p_hi
display "@ phi " %24.17g r(mean) " " %24.17g r(max)
predictnl q = predict(pr), se(q_se) variance(q_v) wald(q_w) p(q_p)
summarize q
display "@ q " %24.17g r(mean) " " %24.17g r(sd)
summarize q_v
display "@ qv " %24.17g r(mean) " " %24.17g r(max)
summarize q_w
display "@ qw " %24.17g r(mean) " " %24.17g r(min)
summarize q_p
display "@ qp " %24.17g r(mean) " " %24.17g r(max)
predictnl double xq = xb(), se(xq_se)
summarize xq_se
display "@ xqse " %24.17g r(mean) " " %24.17g r(sd)
display "@ xq1 " %24.17g xq[1] " " %24.17g xq_se[1]
regress y x1 x2
predictnl r = exp(xb()) if x2 > 1, se(r_se) level(90) ci(r_lo r_hi)
summarize r
display "@ r " %24.17g r(mean) " " r(N)
summarize r_se
display "@ rse " %24.17g r(mean) " " %24.17g r(max)
summarize r_lo
display "@ rlo " %24.17g r(mean)
summarize r_hi
display "@ rhi " %24.17g r(mean)
predictnl s = _b[x1]/_b[x2], se(s_se)
summarize s
display "@ s " %24.17g r(mean) " " %24.17g r(sd)
summarize s_se
display "@ sse " %24.17g r(mean) " " %24.17g r(sd)
poisson yc x1 x2
predictnl m = predict(n), se(m_se)
summarize m_se
display "@ mse " %24.17g r(mean) " " %24.17g r(max)
capture noisily predictnl p = xb()
display "@ rc " _rc
