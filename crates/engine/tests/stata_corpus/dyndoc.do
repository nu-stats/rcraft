* RCraft's Stata corpus (P2C.12x, batch D): made-up data; Stata 19's results are in dyndoc.recorded
* commands: dyntext dyndoc markdown
* (dyntext, dyndoc and markdown: dd_do blocks with each attribute, dd_display with and without a format, dd_if, dd_else, dd_skip_if, and dd_skip_else, dd_ignore, dd_remove, dd_include, Markdown into HTML, and the errors with their return codes; the files themselves are compared with Stata's by crates/engine/tests/stata_putdoc.rs)
version 19
set linesize 80
clear
set obs 12
generate x = _n
generate y = mod(_n * 5, 7)
save ddx_data.dta, replace
file open f using ddx_inc.txt, write replace text
file write f "Included <<dd_display: 1>> as it is." _n
file write f "second included line" _n
file close f
file open f using ddx_src.txt, write replace text
file write f "<<dd_version: 2>>" _n
file write f "# A made-up report" _n _n
file write f "Some *emphasis*, **strong** and " _char(96) "code" _char(96) `" text -- with 'single' and "double" quotes..."' _n _n
file write f "<<dd_do>>" _n
file write f "use ddx_data, clear" _n
file write f "summarize x y" _n
file write f "<</dd_do>>" _n _n
file write f "The mean of y is <<dd_display: %5.2f r(mean)>> for <<dd_display: r(N)>> rows; its sd is <<dd_display: %6.3f r(sd)>>." _n _n
file write f "<<dd_do: quietly>>" _n
file write f "summarize x" _n
file write f "<</dd_do>>" _n _n
file write f `"Quietly: <<dd_display: r(max)>> and <<dd_display: "a string">> and <<dd_display: 1/3>>."' _n _n
file write f "<<dd_if: r(N) > 5>>" _n
file write f "Shown when true." _n
file write f "<<dd_else>>" _n
file write f "Not shown." _n
file write f "<<dd_endif>>" _n
file write f "<<dd_if: r(N) > 50>>" _n
file write f "Not shown either." _n
file write f "<<dd_endif>>" _n _n
file write f "<<dd_skip_if: 1 == 2>>" _n
file write f "Not skipped." _n
file write f "<<dd_skip_else>>" _n
file write f "Skipped else." _n
file write f "<<dd_skip_end>>" _n _n
file write f "<<dd_do: nocommands>>" _n
file write f "display 6 * 7" _n
file write f "<</dd_do>>" _n _n
file write f "<<dd_do: nooutput>>" _n
file write f "display 8" _n
file write f "<</dd_do>>" _n _n
file write f "<<dd_do: noprompt nooutput>>" _n
file write f "display 9" _n
file write f "<</dd_do>>" _n _n
file write f "<<dd_do: nocommands nooutput>>" _n
file write f "display 10" _n
file write f "<</dd_do>>" _n _n
file write f "<<dd_do: noprompt>>" _n
file write f "display 11" _n
file write f "list x y in 1/3" _n
file write f "<</dd_do>>" _n _n
file write f "<<dd_ignore>>" _n
file write f "Kept <<dd_display: 5>> as it is." _n
file write f "<</dd_ignore>>" _n _n
file write f "Removed: <<dd_remove>>this part<</dd_remove>> and kept." _n _n
file write f "<<dd_include: ddx_inc.txt>>" _n _n
file write f "1. one" _n
file write f "2. two" _n _n
file write f "- a" _n
file write f "- b" _n _n
file write f "    code block" _n _n
file write f "> a quote" _n _n
file write f "A [link](https://example.com) here." _n
file close f
dyntext ddx_src.txt, saving(ddx_out.txt) replace
capture confirm file ddx_out.txt
display "@ dt_written " _rc
dyndoc ddx_src.txt, saving(ddx_out.html) replace
capture confirm file ddx_out.html
display "@ dd_written " _rc
capture noisily dyntext ddx_src.txt, saving(ddx_out.txt)
display "@ dt_exists " _rc
capture noisily dyntext ddx_none.txt, saving(ddx_none_out.txt) replace
display "@ dt_missing " _rc
capture noisily dyndoc ddx_src.txt, saving(ddx_out.html)
display "@ dd_exists " _rc
capture noisily dyndoc ddx_none.txt, saving(ddx_none_out.html) replace
display "@ dd_missing " _rc
capture noisily dyntext ddx_src.txt
display "@ dt_nosaving " _rc
dyndoc ddx_src.txt, replace
capture confirm file ddx_src.html
display "@ dd_default " _rc
file open f using ddx_md.md, write replace text
file write f "Setext title" _n
file write f "============" _n _n
file write f "## Hello, there! (again)" _n _n
file write f "## Hello, there! (again)" _n _n
file write f `"Typography: "quoted" and 'single', it's -- en, --- em, dots... << and >>."' _n _n
file write f "Less 3 < 4 & 5 > 2, an <span>inline tag</span> and " _char(96) "x < y" _char(96) " code." _n _n
file write f "_under_ and __strong__ and ***both*** and a\_b and a line  " _n
file write f "break." _n _n
file write f "Sub title" _n
file write f "---------" _n _n
file write f "1. first" _n
file write f "2. second" _n
file write f "    - nested" _n _n
file write f "- loose one" _n _n
file write f "- loose two" _n _n
file write f "***" _n _n
file write f "~~~ stata" _n
file write f "display <1>" _n
file write f "~~~" _n _n
file write f "| a | b |" _n
file write f "|:--|--:|" _n
file write f "| 1 | 2 |" _n
file write f "| 3 | 4 |" _n _n
file write f "<div>" _n
file write f "kept html" _n
file write f "</div>" _n _n
file write f "<https://example.org> and ![a picture](pic.png) and [titled](u.html " _char(34) "T" _char(34) ")." _n _n
file write f "> quoted line" _n
file write f "> second quoted" _n _n
file write f "#### Four ####" _n
file write f "last line" _n
file close f
markdown ddx_md.md, saving(ddx_md.html) replace
capture confirm file ddx_md.html
display "@ md_written " _rc
capture noisily markdown ddx_md.md, saving(ddx_md.html)
display "@ md_exists " _rc
capture noisily markdown ddx_none.md, saving(ddx_none.html) replace
display "@ md_missing " _rc
capture noisily markdown ddx_md.md
display "@ md_nosaving " _rc
erase ddx_data.dta
erase ddx_inc.txt
erase ddx_src.txt
erase ddx_out.txt
erase ddx_out.html
erase ddx_src.html
erase ddx_md.md
erase ddx_md.html
