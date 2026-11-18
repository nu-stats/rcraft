# dyntext, dyndoc and markdown (2026-11-18)

Ported from statcraft@c5c9c07 (its dyndoc.md and its Markdown rules, probes 546 and 547), then checked again on
Stata 19 SE with made-up data: corpus `dyndoc.do` (return codes in `dyndoc.recorded`) and the files it writes,
compared exactly with Stata's by their digests in `crates/engine/tests/stata_putdoc.rs`. Probe 547's three files
are byte for byte Stata's too. Translated in `crates/statatr/src/dofile/dyndoc.rs` (the tags) and
`crates/statatr/src/data_r/dyndoc.rs` (the R: the document, Markdown into HTML).

## Rule

- `dyntext src, saving(file) [replace]` prints the processed text and writes it (no message); `dyndoc src [,
  saving(file) replace]` does the same through Markdown into HTML, the file `src` with `.html` without `saving()`,
  and prints `successfully converted "C:/full/path.html"`; `markdown src, saving(file) [replace]` converts a
  Markdown file alone, with the same message. Before anything runs: a source that is not there is `file x.txt not
  found` r(601) (the name as typed); a target that is there without `replace` is `file C:/full/path already
  exists` r(602); `dyntext` or `markdown` without `saving()` is `option saving() required` r(198).
- A tag on a line of its own goes with its line, and the blank lines around it stay: `<<dd_version: 2>>`,
  `<<dd_do[: attributes]>>` ... `<</dd_do>>`, `<<dd_if: exp>>`, `<<dd_else>>`, `<<dd_endif>>`, `<<dd_skip_if: exp>>`,
  `<<dd_skip_else>>`, `<<dd_skip_end>>`, `<<dd_ignore>>` ... `<</dd_ignore>>`, `<<dd_include: file>>`.
- **`dd_if` does not nest**: a `dd_if` inside an open one starts over, and the `dd_endif` left after it is printed as
  text (corpus run, 2026-11-18). A `dd_if` inside a line is text. `dd_skip_if` hides its lines when true.
- A `dd_do` block shows each command as a do-file's log does: `. ` and the command, its output, a blank line.
  `quietly` shows nothing; `nocommands` drops the command line; `nooutput` keeps the command line and the blank
  line; both together show nothing; `noprompt` shows the command without `. `. In the file Stata writes on Windows
  these lines (echo, output, blank) end in a carriage return; the other lines do not.
- `<<dd_display: [%fmt] exp>>` is what `display` shows (`.14285714`, `1.235e+08`, `%9.2f` padded). `dd_ignore`
  keeps its lines exactly, tags and all; `dd_include` puts the file's lines in exactly, without running their
  tags, **a carriage return at their end kept** (the one `file write` puts there on Windows). `<<dd_remove>>text
  <</dd_remove>>` is printed but taken out of the file.
- `<<dd_graph[: saving(file.svg) replace alt() height() width()]>>` exports the graph as SVG and becomes `<img
  src="file" height="200" alt="plot" >`; with no `saving()` the file is `Graph.svg`, or `Graph1.svg` when that is
  there.
- After `regress`, quietly or not, r() holds only r(table) (observed 2026-11-18), so `r(N)` there is missing.

## Markdown into HTML

As StatCraft's note says, and checked here on four more files: only the body, one block a line, the file ending in
a newline; ATX and setext headings with an anchor (`<h2><a href="#hello-there-again" id="hello-there-again">`: lower
case, spaces as `-`, punctuation dropped, the same id twice for the same text); the typographer (`&ldquo;`,
`&rsquo;`, `&ndash;` for `--`, `&mdash;` for `---`, `&hellip;`, `&laquo;`/`&raquo;` for `<<`/`>>`); `&`, `<`, `>`
escaped, inline and block HTML kept; tight lists without `<p>` (**a paragraph after a blank line inside an item
keeps its `<p>`**), loose lists with it; indented and fenced code (`class="language-stata"` for an info string);
quotes, rules, images, autolinks, titled links; pipe tables keep each cell's spaces and alignment (`<th
align="left"> a </th>`); `\r\n` read as `\n`.

## Source

Stata 19.0 SE, 2026-11-18: StatCraft's probes 546 and 547 (private corpus), RCraft's `dyndoc.do`, and short runs
of our own (no saving(), nested dd_if, r() after regress).

## R cross-check

R's commonmark and markdown packages give CommonMark's HTML without the anchors, the entities or Stata's table
cells, so RCraft has its own converter (R helper code in `data_r/dyndoc.rs`), written to the rules above.

## Port hints

The tags become a run of their own: the source is read when the command is reached; each text line is added to the
document with its `dd_display` pieces worked out then; each `dd_do` command is run as a do-file line with what it
prints kept; a trailing blank line of a command's output is dropped (it is the blank line between commands).

## Open

Refused: a `dd_if` inside another, `dd_version` other than 2, `dd_do` blocks with blank lines, comments, braces or
continued lines, attributes other than the four, inline tags other than `dd_display`, `dd_graph` and `dd_remove`,
`dd_graph` to a file other than `.svg` or to one that is there without `replace`, `dd_include` of a missing file,
`dyndoc, docx`, `nostop` and the other options, and `dyntext`/`dyndoc` under `capture` or `quietly` unless Stata
stops before the tags run. A document whose tags have a line RCraft does not translate is not written (that line,
and the save, are refused): probe 546 reads `r(N)` after `regress`, which RCraft does not follow yet (Stata: missing).
Not compared: the SVG file itself (R's graph), and long message lines, which Stata wraps at the line size and RCraft
does not.
