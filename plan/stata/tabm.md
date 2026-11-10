# tabm (tab_chi, SSC, Nick Cox), 2026-11-10

Installed from SSC that day with the user's yes; observed on Stata 19 SE with made-up data (probes
`tabm/t1.do`, `t2.do` in the session scratchpad); its code and help were not read. Corpus `tabm.do`.

- `tabm a b c d`: one two-way table, `variable` (a row for each variable, in the varlist's order, its
  name) by `values` (a column for each value any of them takes), as tabulate prints it: columns 10
  wide, five to a block at linesize 80, the rest in a block below; Total row and column.
- The columns are labelled with the first variable's value label (a value it does not label shows as
  the number); the other variables' labels are not used.
- `if` keeps rows; missing values are left out unless `missing` (a `.` column last).
- r(N) is the number of values counted (21 for 4 variables of 6 rows with 3 missing), r(r) the
  variables, r(c) the values; other r() names are cleared.

## tabulate's column labels (found here)

- A two-way table's column label longer than 9 characters is cut to its first 9: "abcdefghij" is
  "abcdefghi", "Secondary - complete" "Secondary", "16-30 years old" "16-30 yea". RCraft had laid such
  tables out in R's form (it took labels of up to 10 as they were); it now cuts them as Stata does.
