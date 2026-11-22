* RCraft's Stata corpus (batch MV): made-up data; Stata 19's results are in cluster.recorded
* commands: cluster
* (the four linkage methods with their id, order and height variables, generate groups(), kmeans and kmedians from firstk and from random starts, the objects kept as characteristics, list and dir and drop)
* tolerance: 1e-9
version 19
set linesize 255
clear
set seed 248
set obs 30
generate g0 = 1 + mod(_n, 3)
generate x = round(2 * g0 + invnormal(runiform()) * 0.7, .01)
generate y = round(-g0 + invnormal(runiform()) * 0.7, .01)
cluster singlelinkage x y, name(sl)
cluster generate s3 = groups(3), name(sl)
cluster completelinkage x y, name(cl)
cluster generate c3 = groups(3)
cluster averagelinkage x y, name(al)
cluster generate a2 = groups(2/4)
cluster wardslinkage x y, name(wl)
cluster generate w3 = groups(3)
cluster list wl
cluster dir
display "@ chars " ("`_dta[_cluster_objects]'" == "wl al cl sl") ("`_dta[wl_cmd]'" == "cluster wardslinkage x y, name(wl)") ("`_dta[sl_vars]'" == "x y")
forvalues i = 1/30 {
    display "@ grp `i' " s3[`i'] " " c3[`i'] " " a22[`i'] " " a23[`i'] " " a24[`i'] " " w3[`i']
    display "@ sl `i' " sl_id[`i'] " " sl_ord[`i'] " " %17.0g sl_hgt[`i']
    display "@ cl `i' " cl_ord[`i'] " " %17.0g cl_hgt[`i']
    display "@ al `i' " al_ord[`i'] " " %17.0g al_hgt[`i']
    display "@ wl `i' " wl_ord[`i'] " " %17.0g wl_hgt[`i']
}
cluster kmeans x y, k(3) start(firstk) name(km)
cluster kmedians x y, k(3) start(firstk) name(kmd)
cluster kmeans x y, k(3) name(kr)
forvalues i = 1/30 {
    display "@ km `i' " km[`i'] " " kmd[`i'] " " kr[`i']
}
local t : char _dta[km_info]
display "@ kchars " (strpos(`"`t'"', "m kmeans") > 0) (strpos(`"`t'"', "o k 3") > 0) (strpos(`"`t'"', "o start firstk") > 0) (strpos(`"`t'"', "d L2") > 0)
cluster drop sl
cluster dir
display "@ dropped " ("`_dta[_cluster_objects]'" == "kr kmd km wl al cl") (0 == 0)
cluster singlelinkage x y, measure(L1)
cluster dir
display "@ l1 " ("`_dta[_cluster_objects]'" == "_clus_1 kr kmd km wl al cl")
forvalues i = 1/5 {
    display "@ l1v `i' " _clus_1_id[`i'] " " _clus_1_ord[`i'] " " %17.0g _clus_1_hgt[`i']
}
cluster kmedians x y, k(2) start(lastk) name(k2)
display "@ k2 " k2[1] " " k2[30]
cluster kmeans x y, k(3) start(krandom) name(k3)
display "@ k3 " k3[1] " " k3[2] " " k3[30]
capture noisily cluster generate z = groups(3), name(km)
display "@ rc " _rc
capture noisily cluster generate z = groups(3), name(nosuch)
display "@ rc2 " _rc
