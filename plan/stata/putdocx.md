# putdocx and putpdf (2026-11-18)

Ported from statcraft@c5c9c07 (its putdocx.md, probes 544 and 545), then checked again on Stata 19 SE with
made-up data: corpus `putdocx.do` (messages and return codes, in `putdocx.recorded`) and the files read back in
`crates/engine/tests/stata_putdoc.rs`. Translated in `crates/statatr/src/data_r/putdoc.rs`. The printed
messages were set against Stata's log by eye; only the return codes and r() counts are compared by the test.

## Rule

- `begin` opens a document (one open already: `document already open in memory` r(110)); `clear` drops it.
  Every other subcommand with none open, or after `save` (which closes it): `document not opened` r(198).
- `paragraph [, style() halign()]`, `text (exp) [, bold italic underline linebreak nformat()]`. `text` needs
  an active paragraph: right after `begin`, after a table and after `pagebreak` there is none
  (`no active paragraph` r(198)). A table ends the active paragraph, so does `pagebreak`.
- `style()` takes Title, Subtitle and Heading1 to Heading9 (Heading10, Normal, `title`, Emphasis, Caption:
  `option style() specified incorrectly` r(198)); `halign(bogus)` the same with `halign()`; `nformat(bogus)`
  likewise. `putpdf paragraph, style(Title)`: `option style(Title) not allowed` r(198). A word that is no
  option: `option X not allowed` r(198) (not translated: Stata also has many options RCraft does not).
- Numbers: a plain number is shown in `%11.0g` trimmed (`.333333333`, `1234567.89`, `1.2346e+11`, `.` for
  missing). With `nformat(%w.df)` it is shown in that format padded to its width (`%9.3f` of 1/3 is
  `    0.333`, `%-9.3f` pads on the right). A matrix without `nformat()` is shown in `%12.0g` trimmed
  (`.3333333333`, `1.00000e-07`); with `nformat()` trimmed (`1.5` and `2.0` for `%5.1f`). A table cell
  `name(i,j) = (exp)` follows `text`: `%11.0g` trimmed, `nformat()` padded.
- `table name = (r, c)`; `name(i,j) = (exp)` (a row or column outside: `invalid cell specification; row
  out of range`, `... column out of range` r(198); a table that is not there: `table zz does not exist`
  r(198)); `r` or `c` below 1: `failed to add table` r(198) for putdocx. **putpdf accepts it** and its
  document is unusable after (the next command stops with a Mata error r(3301)); RCraft stops there
  with its own message. `putdocx table` with nothing after it: `table or cell specification required` r(198).
- `data(varlist) [if] [in] [, varnames obsno]`: one row per observation, each variable in its display format
  trimmed, value labels used for the values that have one (a value without one shows as a number), a
  heading row of names with `varnames`, the **original** observation numbers in a first column with
  `obsno` (its heading cell empty). A variable that is not there: `variable nosuch not found` r(111).
- `matrix(M) [, rownames colnames nformat()]`: row names down the first column, column names across the
  first row, the corner cell empty when both. An unknown matrix: `NOSUCH not found` r(111).
- `etable`: the coefficient table of the last `regress` that was printed: 3 rows by 7 columns for one
  variable (the dependent variable's name, `Coefficient`, `Std. err.`, `t`, `P>|t|`, and `[95% conf.
  interval]` across two columns, then one row per coefficient as printed). **Stata takes it from r()**: after
  `quietly regress`, after any command that replaces r() (`summarize`, `putdocx describe name`, `regress`
  itself under quietly) and with no estimation at all it says `information for the estimation table not
  found;` and `    Please replay estimation results and try again`, r(111). `display`, `list`, `return list`,
  `putdocx describe` (no name) and `estimates store` leave it alone; another printed fit (logit) replaces it.
- A table name used again: `(note: table t1 has been redefined)`; the new table goes where the document has
  got to and the old one stays.
- `describe name` prints a rule (two blanks, 20 dashes, `+`, 23 dashes), `Table name`, `No. of rows` and
  `No. of cols` (labels padded to 20, then `|  value`) and leaves r(nrows) and r(ncols) (a spanning cell
  counts for its columns: 7 for the coefficient table); an unknown name: `table t9 does not exist` r(198).
  `describe` alone prints `No. of tables` and `No. of paragraphs` (each `paragraph` command counts) and
  leaves r() **as it was**.
- `pagebreak` ends the active paragraph. Every table is followed by an empty paragraph.
- `save file [, replace append]`: `.docx` (`.pdf`) is added to a name without an extension; another one
  (`a.txt`) is `a.txt is not a docx file` (`pdf`) r(198); nothing: `invalid file specification` r(198). It
  prints `successfully created "C:/full/path.docx"` (`successfully appended to` with `append`), then the
  document is closed. A file there and neither option: `file p.docx already exists` and `you must specify
  one of the replace , append, and append() options` (putpdf: `p.pdf already exists` and `you must specify
  the replace option`), r(198); `append` to a file that is not there: `file q.docx not found` r(601);
  `putpdf save, append`: `option append not allowed`. `append` puts an empty paragraph between the old body
  and the new one (the old section properties stay in the middle of the body in Stata's file).

## Source

Stata 19.0 SE, 2026-11-18: StatCraft's probes 544 and 545 (private corpus) and RCraft's `putdocx.do`.

## Cases

Same shape in the files Stata writes: a Word body of paragraphs (Title is `<w:pStyle w:val="Title"/>`,
bold and italic runs `<w:b/>`, `<w:i/>`, `<w:u>`, a break `<w:br/>`), tables with grid style `TableGrid`, the
coefficient table with rules only above and below the heading row and right of the names column and every cell
right aligned, a page break as an empty paragraph with `<w:br w:type="page"/>`. Letter pages with one inch
margins and styles Normal, Title, Subtitle, Heading1 to 4 (and `TableGrid`) in the file Stata wrote. The PDF
(made with Haru, A4 here, 36 points at the sides and about 86 at the top, Helvetica 11 with bold and oblique
faces, tables as a grid of cells `page width / columns` wide and 18.7 high) has the text of each run and
cell as separate strings in reading order.

## R cross-check

R's officer package writes a Word file too, but its default template has French style ids (`Titre1`), no
`Title` or `TableGrid` style and A4 pages, so the style names in the file would not be Stata's. RCraft
writes the three parts of the Word package itself (document, styles, relationships) and zips them with the
zip package. The PDF is drawn with grDevices::pdf (Helvetica, text kept as text; R kerns it into TJ
arrays, so a reader joins the pieces).

## Port hints

Keep the document as a list of blocks (paragraph with runs, table with rows of cells, page break) and an
index of table names; the active paragraph is the last paragraph block and is cleared by a table or a page
break. Number text: `%11.0g` trimmed for text and cells, `%12.0g` trimmed for matrices, a given
`nformat()` padded for text and cells and trimmed for matrices. `etable` is only available while r() still
holds the last printed estimation table.

## Open

Refused (`Untranslated`): `image`, `sectionbreak`, `pagenumber`, `textblock`, `textfile`, `append` of
documents (`append(docname)`), `collect`, begin's options (`pagesize()`, `landscape`, fonts, headers),
`text` and cell options other than the ones above (fonts, sizes, colours, shading), table options
(`border()`, `width()`, `layout()`), `halign(justified)`, other styles, `etable` after a model other than
`regress` (and with weights), a table of `data()` with no rows or variables, a variable whose display format
is not a number format, and `putpdf table` with no rows or columns. Not compared: fonts, sizes and widths in
the Word file (Stata's table cell widths are the page width over the columns), the PDF's line and row
heights beyond the first page, and what Stata does when a paragraph is longer than a line.
