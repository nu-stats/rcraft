# split (2026-11-04)

Read off Stata 19 with made-up strings (never its ado or help files).

- Without `parse()`: words. The string's ends are trimmed and runs of spaces count as one
  (`MR  JOHN SMITH` is MR, JOHN, SMITH).
- With `parse()`, `" "` included: the ends of the whole string are trimmed (spaces only; `notrim` keeps
  them), then it is cut at each delimiter. Pieces keep their own spaces (`a . b` on "." is "a " and " b"),
  empty pieces are kept (`a.b..c.` is a, b, "", c; `MR  JOHN` on " " is MR, ""), a leading delimiter gives
  an empty first piece, and nothing follows a last delimiter.
- Several parse strings: the earliest match; at the same place the one listed later wins (`abab` on "ab"
  "a" is "", b, b; on "a" "ab" it is "", ""). Strings that share characters are declined, since overlaps
  have not been read off.
- `limit(k)` keeps the first k pieces and drops the rest.
- New variables stub1 ... stubK, K the most pieces in any row, as text; "variables created as string:"
  and the names on the next line, two spaces apart. r(k_new) and r(varlist) (each name followed by a
  space). Rows left out by if or in get "".
- Errors: a stub variable already there, "cannot generate new variables using stub x", r(110); a numeric
  variable, r(109); `notrim` without `parse()`, "parsing on spaces incompatible with notrim", r(198).
- Not translated: `destring` (and its `float`, `ignore()`, `force`, `percent`).
