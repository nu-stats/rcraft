* RCraft's Stata corpus (packages' gaps): made-up data; Stata 19's results are in isvar.recorded
* commands: isvar
* (isvar from SSC, as a replication package's own program runs it to find pairs of variables one of which a regression explains fully: names, abbreviations, patterns, ranges, unknown and ambiguous names, operators, repeats, what it leaves in r() and that it clears r())
version 19
set linesize 255
clear
set seed 97
set obs 40
generate alpha = runiform()
generate beta = rnormal()
generate gamma1 = 2 * alpha + 1
generate gamma2 = rnormal()
generate str3 name = "a"
isvar alpha beta
display "@ a " wordcount("`r(varlist)'") " " ("`r(varlist)'" == "alpha beta") " " ("`r(badlist)'" == "")
isvar beta zz alpha qq
display "@ b " wordcount("`r(varlist)'") " " ("`r(varlist)'" == "beta alpha") " " ("`r(badlist)'" == "zz qq")
isvar alp gamma* gamma
display "@ c " wordcount("`r(varlist)'") " " ("`r(varlist)'" == "alpha gamma1 gamma2") " " ("`r(badlist)'" == "gamma")
isvar alpha alpha name _n 1x Alpha
display "@ d " wordcount("`r(varlist)'") " " ("`r(varlist)'" == "alpha alpha name") " " ("`r(badlist)'" == "_n 1x Alpha")
isvar alpha-gamma1 i.alpha l.beta
display "@ e " wordcount("`r(varlist)'") " " ("`r(varlist)'" == "alpha beta gamma1") " " ("`r(badlist)'" == "i.alpha l.beta")
codebook alpha name
quietly summarize alpha
quietly isvar zz
display "@ f " wordcount("`r(varlist)'") " " ("`r(badlist)'" == "zz") " " missing(r(mean))
generate _p1 = alpha
generate _p2 = beta
generate _p3 = 3 * alpha - 2
generate _p4 = gamma2
local drop " "
unab all: _p*
local K: word count `all'
forvalues i = 1/`=`K'-1' {
    forvalues j = `=`i'+1'/`K' {
        local y: word `i' of `all'
        local x: word `j' of `all'
        quietly isvar `y' `x'
        if wordcount("`r(varlist)'") == 2 {
            quietly regress `y' `x'
            if e(r2) == 1 {
                local drop "`drop' `x'"
                quietly drop `x'
            }
        }
    }
}
display "@ drop " wordcount("`drop'") " " ("`drop'" == "  _p3")
