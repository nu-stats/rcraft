* RCraft's Stata corpus (P2C.15): made-up data; Stata 19's results are in interactions.recorded
* Stata stops iterating at its tolerance, so the likelihood model agrees to about 7 digits
* commands: logit regress
* (two factors interacted: i.g#i.h alone, with one main effect, i.g##i.h, a bare g#h, an ib#. base, an empty cell; their cells in e(b))
* tolerance: 1e-6
version 19
import delimited corpus.csv, clear
regress y i.cat3#i.g
display "@ alone_b " %24.17g _b[1.cat3#1.g] " " %24.17g _b[2.cat3#0.g]
display "@ alone_b2 " %24.17g _b[3.cat3#1.g] " " %24.17g _b[_cons]
display "@ alone_fit " %24.17g e(r2) " " e(df_m) " " e(rank)
regress y cat3#g
display "@ bare_b " %24.17g _b[1.cat3#1.g] " " %24.17g _b[3.cat3#0.g]
regress y i.cat3##i.g
display "@ full_b " %24.17g _b[2.cat3] " " %24.17g _b[1.g]
display "@ full_b2 " %24.17g _b[2.cat3#1.g] " " %24.17g _b[3.cat3#1.g]
display "@ full_fit " %24.17g _se[3.cat3#1.g] " " e(df_m) " " e(rank)
regress y i.cat3 i.cat3#i.g
display "@ one_b " %24.17g _b[1.cat3#1.g] " " %24.17g _b[2.cat3#1.g]
display "@ one_b2 " %24.17g _b[3.cat3] " " e(df_m)
regress y ib2.cat3#i.g x
display "@ ib_b " %24.17g _b[1.cat3#0.g] " " %24.17g _b[2.cat3#1.g]
display "@ ib_b2 " %24.17g _b[x] " " %24.17g _b[_cons]
regress y i.cat3#i.g if !(cat3 == 2 & g == 1)
display "@ empty_b " %24.17g _b[2.cat3#1.g] " " %24.17g _b[3.cat3#1.g]
display "@ empty_fit " %24.17g e(r2) " " e(df_m) " " e(rank)
logit yb i.cat3##i.g, nolog
display "@ lg_b " %24.17g _b[2.cat3#1.g] " " %24.17g _b[1.g]
display "@ lg_fit " %24.17g e(ll) " " e(df_m)
logit yb i.cat3#i.g, nolog
display "@ lga_b " %24.17g _b[1.cat3#1.g] " " %24.17g _b[3.cat3#0.g]
