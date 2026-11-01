# Dates and times on R (P2C.14)

The rules come from StatCraft's decoding, which used the public datetime manual entries and probes 033 to
035 of Stata 19. Nothing of StataCorp's program files was read. The rules were already ported to Rust
here as `statatr/src/values/datetime`, which reads dates written out in a do-file when the line is
translated. P2C.14 writes the same rules again in R (`data_r/dates.rs`, the helper `stata_dates`), so
they run on columns.

## What is translated

- **Strings read against a mask:** `date()` (and `daily()`), `clock()`, `weekly()`, `monthly()`,
  `quarterly()`, `halfyearly()` and `yearly()`, with Stata's optional top year.
- **Dates from numbers:** `mdy()`, `yw()`, `ym()`, `yq()`, `yh()`, `dofw()`, `dofm()`, `dofq()`, `dofh()`,
  `dofy()`.
- **The parts of a date:** `year()`, `month()`, `day()`, `dow()`, `doy()`, `week()`, `quarter()`,
  `halfyear()`, `wofd()`, `mofd()`, `qofd()`, `hofd()`, `yofd()`, `isleapyear()`.
- **Times in datetime/c:** `cofd()`, `dofc()`, `hh()`, `mm()`, `ss()`, `hms()`, `dhms()`, `mdyhms()`,
  `hours()`, `minutes()`, `seconds()`, `msofhours()`, `msofminutes()`, `msofseconds()`.

Each distinct string, mask and top year is read once. A missing or invalid argument, or a date outside
0100 to 9999, gives missing.

## The check

Stata's 13,177 recorded answers in probes 034 and 035 were put through the R helper (`%21x`, missing as
`+1.0000000000000X+3ff`). 13,172 are the same bit for bit. Four are `wofd()` outside 0100 to 9999, where
Stata gives arbitrary numbers (−306,465,739 for 01jan10000) and RCraft gives missing, as StatCraft does.
The fifth was the check script misreading an empty string.

The corpus file `dates.do` runs the functions on columns: strings, masks, two-digit years, times,
periods and the parts of dates. Its 23 results match Stata 19 exactly.

## Not done

- The leap-second family (datetime/C: `Clock()`, `Cofd()`, `Cofc()`, `cofC()`, `dofC()`, `hhC()`,
  `mmC()`, `ssC()`, `Cmdyhms()`, `Cdhms()`, `Chms()`), refused with the reason. `values/datetime/leap.rs`
  has the rules for it.
- Display formats on R's side beyond what `display` already does.
