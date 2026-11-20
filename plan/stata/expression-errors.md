# Expressions Stata does not read (2026-11-19)

Observed on Stata 19.0 SE for Windows with made-up do-files (StatCraft's error probes 704, 705 and 752 as the cases, run
again here), for fix batch 2 of the StatCraft port. The decoding of what the expression reader names is StatCraft's
(`statatr/src/lang/expr.rs`, the synced reader); RCraft's part is what a line does when the reader says no.

- **Rule:** a line whose expression Stata cannot read stops there with a return code, changes nothing (no variable, no
  scalar, no macro, the data as it was) and `_rc` holds the code under `capture`. Without `capture` it ends the do-file.
  - r(198): an operator at the end (`5-`, `x +`, `3 ^`, `!`), a token left after a whole expression (`x 56`, `x abc`,
    `x ; 2`), a text the reader cannot name (`4 \ 5`, `4 @ 5`, `3 + * 4`, `1e`), a function called with no argument
    (`abs()`, `sqrt()`, `max()`, `round()`, `string()`), `generate y = x , 2` (an option generate does not have), and
    a `+` sign before anything but a number written out (`+x`, `1 + +x`, `+sc`, `+.a`, `!+x`).
  - r(132): parentheses or brackets that do not pair, either way (`(3 + 4`, `3 + 4)`, `x[1`, `max(1, 2`).
  - r(133): `+(2)` and `+abs(2)` (a + sign before a parenthesis: Stata calls it an unknown function).
  - r(101): `generate y = x [2]` (a `[` after a blank is a weight, which generate does not take).
  - Same codes in `display`, `scalar`, `generate`, `replace`, `assert`, `count if`, `list if` and `local`/`global`.
- **display** shows the items before the bad one, then stops (`display "a" \ 3` shows a). `display 3 = 4` is not an error:
  the `=` before an expression is dropped (it shows 34), as is `display = 4`.
- **What stays:** a failed `generate` leaves the name free (a later `generate` of it works, r(111) for `confirm`);
  a failed `replace`, `scalar` or `local` leaves the old value.
- **RCraft:** `Untranslated.stata_rc` carries the code from the reader; `Session::command` runs the line on a copy of its
  state, and when it says a code the line is translated again as `_rcraft_fail <code>` (a `stop()` with RCraft's own
  words and `(Stata: r(code))`, which `capture` turns into `_rc`). The words are RCraft's, not Stata's. Without
  `capture` the line is refused as before.
- **Corpus:** `expression_errors.do`, 187 return codes, tolerance 0.
- **Open (refused, not observed or not matched):** an unknown function (the reader's table may lack a Stata function, so no
  code is claimed); a `[` after a blank in `replace`, `scalar`, `assert` or `count` (r(101) not observed there); `x[1] + +2`
  (r(303)); `"a" * +2`; the same expression errors in commands other than those above; lines not under `capture`
  (Stata ends the do-file there).
