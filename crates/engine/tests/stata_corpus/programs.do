* RCraft's Stata corpus (P2C.9 continued): made-up data; Stata 19's results are in programs.recorded
* commands: program syntax
* (programs the do-file defines: arguments and args, r-class returns, exit inside a program, a program calling one, syntax with a varlist, if, in, flags, noCONstant, a real default, a varlist option and *, marksample, anything, string options, and the temporary variables dropped when a program returns)
* tolerance: 1e-8
version 19
set linesize 255
clear
set obs 30
generate x = _n
generate y = 2 * x + mod(_n, 3)
generate x1 = mod(_n * 7, 11)
generate x2 = mod(_n * 5, 13)
replace x2 = . in 4
generate x3 = _n / 30
capture program drop mystats
program define mystats, rclass
  args v w
  quietly summarize `v'
  return scalar mean = r(mean)
  return scalar n = r(N)
  tempvar d
  quietly generate `d' = `v' * `w'
  quietly summarize `d'
  return scalar dmean = r(mean)
  return local name "`v'"
end
mystats x 3
display "@ args " r(mean) " " r(n) " " r(dmean) " " ("`r(name)'" == "x")
program twice
  args a
  display "@ twice " 2 * `a'
end
twice 21
program define withexit, rclass
  return scalar a = 1
  if `1' > 0 exit
  return scalar a = 2
end
withexit 5
display "@ exit " r(a)
program outer, rclass
  mystats y 1
  return scalar got = r(mean) + 1
end
outer
display "@ outer " r(got)
program define syn, rclass
  syntax varlist(numeric) [if] [in] [, Detail noCONstant KLOW(real 2) BY(varlist) *]
  marksample touse
  quietly count if `touse'
  return scalar n = r(N)
  foreach v of local varlist {
    quietly summarize `v' if `touse'
    return scalar mean_`v' = r(mean)
  }
  return scalar klow = `klow'
  return scalar flags = ("`detail'" == "detail") + 2 * ("`constant'" == "noconstant") + 4 * ("`by'" == "x3") + 8 * ("`options'" == "other(1)") + 16 * ("`varlist'" == "x1 x2")
end
syn x1 x2 if x3 > .5, det nocons klow(4) by(x3) other(1)
display "@ syntax " r(n) " " r(mean_x1) " " r(mean_x2) " " r(klow) " " r(flags)
syn x1 in 1/10
display "@ syntax_in " r(n) " " r(mean_x1) " " r(klow) " " r(flags)
program define takes, rclass
  syntax anything
  return scalar ok = ("`anything'" == "3.5 and more")
end
takes 3.5 and more
display "@ anything " r(ok)
program define opts, rclass
  syntax , file1(string) [file2(string)]
  return scalar ok = ("`file1'" == "a.dta") + 2 * ("`file2'" == "b c")
end
opts, file1(a.dta) file2(b c)
display "@ string_opts " r(ok)
quietly describe, short
display "@ temps_dropped " r(k)
