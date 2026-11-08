* RCraft's Stata corpus (packages' gaps): made-up data; Stata 19's results are in winsor.recorded
* commands: winsor
* (winsor from SSC, as a replication package caps income at its top 5 percent: p() and h(), high and low, ties at the cut, missing values, if and in, a float and a double, the label and format kept, and the add-on's errors)
* tolerance: 1e-13
version 19
set linesize 255
clear
set seed 7
set obs 60
generate id = _n
generate double x = rnormal()^3
replace x = . in 4
replace x = 2.5 if inlist(id, 20, 21)
generate y = runiform() * 100
format y %6.2f
label variable x "the x"
preserve
winsor x, gen(w) p(.05)
summarize w
display "@ p05 " %20.0g r(min) " " %20.0g r(max) " " r(N) " " missing(w[4]) " " ("`: variable label w'" == "x, Winsorized fraction .05")
restore
preserve
winsor x, gen(w) h(3)
summarize w
display "@ h3 " %20.0g r(min) " " %20.0g r(max) " " %20.0g r(mean) " " ("`: variable label w'" == "x, Winsorized extreme 3")
restore
preserve
winsor x, gen(w) high p(.1)
summarize w
display "@ high " %20.0g r(min) " " %20.0g r(max) " " %20.0g r(sum) " " ("`: variable label w'" == "x, Winsorized fraction .1, high only")
restore
preserve
winsor x, gen(w) h(2) low
summarize w
display "@ low " %20.0g r(min) " " %20.0g r(max) " " ("`: variable label w'" == "x, Winsorized extreme 2, low only")
restore
preserve
winsor x if id > 10, gen(w) h(4)
summarize w
display "@ if " %20.0g r(min) " " %20.0g r(max) " " r(N) " " missing(w[5])
restore
preserve
winsor y, gen(w) p(.2)
summarize w
display "@ float " %20.0g r(min) " " %20.0g r(max) " " %20.0g r(sum)
restore
capture winsor x, p(.1)
display "@ nogen " _rc
capture winsor x, gen(y) p(.1)
display "@ exists " _rc
capture winsor x, gen(w) p(.1) h(2)
display "@ both " _rc
capture winsor x, gen(w) p(.5)
display "@ toohigh " _rc
capture winsor x, gen(w) h(30)
display "@ toomany " _rc
capture winsor x, gen(w) p(.01)
display "@ zero " _rc
