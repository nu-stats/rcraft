# file open, write and close (2026-11-04)

Read off Stata 19 by writing made-up files and reading their bytes (never its ado or help files).

- `file open h using f, write`: text by default. In a text file `_n` writes the system's new line (`\r\n`
  on Windows); every other byte is written as it is (`_char(10)` and a `char(10)` in an expression are
  one `\n`). A binary file takes only formatted pieces: plain strings, `_n`, `_char()`, `_tab`, `_skip()`
  and unformatted `(exp)` stop with r(198); `%#s` pads with NUL bytes (not translated).
- `replace` on a file that is not there prints "(file f not found)"; `append` adds at the end.
- Errors: the file there without replace or append, "file f already exists", r(602); a handle already
  open, "file handle h already exists", r(110); writing to or closing a handle not open, "file handle h
  not found", r(111).
- `file write`: strings and compound strings as written; `_n`/`_newline(n)`, `_tab(n)`, `_skip(n)` (n
  spaces), `_char(n)`; `(exp)` as `display` writes a number (`%10.0g` trimmed: `.33333333`, `12345678`,
  `1.235e+08`, `1.000e-10`, `-.5`), a string as it is; `%fmt (exp)` padded to the format (`%5.2f` " 0.33",
  `%4s` "  ab"); `%1bu (exp)` and `%1bs (exp)` one byte each in a binary file.
- `file close h` or `_all`.
- RCraft: the handles a translated `file open` opened are known; one a refused `file open` names is not,
  and lines on it are declined. A handle never opened stops R with r(111) as Stata. Extended missing
  values (`.a`) are written as "." (they are all NA in R; the expression says so in a caveat).
- Not translated: `file read`, `seek`, `query`, `set`, and `%2b`, `%4b`, `%4z`, `%8z`.
