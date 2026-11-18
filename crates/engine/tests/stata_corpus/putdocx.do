* RCraft's Stata corpus (P2C.12y, batch D): made-up data; Stata 19's results are in putdocx.recorded
* commands: putdocx putpdf
* (putdocx and putpdf: begin, clear, paragraph with style() and halign(), text with its options and number formats, tables from a size, data(), matrix() and etable, cells, describe and its r(), pagebreak, save with and without replace and append, and the errors with their return codes; the files themselves are read back by crates/engine/tests/stata_putdoc.rs)
version 19
set linesize 80
clear
set obs 6
generate float x = _n / 4
generate str5 s = "a" + string(_n)
generate byte k = mod(_n, 3)
label define kl 1 "one" 2 "two"
label values k kl
format x %6.2f
putdocx clear
capture noisily putdocx paragraph
display "@ pd_nodoc " _rc
capture noisily putdocx text ("x")
display "@ pd_nodoc_text " _rc
putdocx begin
capture noisily putdocx begin
display "@ pd_twice " _rc
capture noisily putdocx text ("x")
display "@ pd_noparagraph " _rc
putdocx describe
capture noisily putdocx table z = (0, 2)
display "@ pd_zero_rows " _rc
capture noisily putdocx table z = (2, 0)
display "@ pd_zero_cols " _rc
capture noisily putdocx paragraph, halign(bogus)
display "@ pd_halign " _rc
foreach st in Normal Title Subtitle Heading1 Heading2 Heading9 Heading10 title Emphasis {
    capture noisily putdocx paragraph, style(`st')
    display "@ pd_style_`st' " _rc
}
putdocx paragraph
putdocx text (1/3)
putdocx text (1234567.891)
putdocx text (123456789012)
putdocx text (.)
putdocx text (1/3), nformat(%9.3f)
putdocx text ("u"), bold italic underline linebreak
capture noisily putdocx text ("x"), nformat(bogus)
display "@ pd_nformat " _rc
putdocx table a = (2, 2)
capture noisily putdocx text ("after table")
display "@ pd_after_table " _rc
capture noisily putdocx table a(1,3) = ("c")
display "@ pd_cell_col " _rc
capture noisily putdocx table a(0,1) = ("c")
display "@ pd_cell_row " _rc
capture noisily putdocx table nosuch(1,1) = ("c")
display "@ pd_cell_none " _rc
putdocx table a(1,1) = (1/3), nformat(%5.2f)
putdocx table a(1,2) = (1234.5678)
putdocx table a(2,1) = ("text")
putdocx table a(2,2) = (.)
putdocx describe a
display "@ pd_a " r(nrows) " " r(ncols)
putdocx paragraph
putdocx table b = data(x s k) if x > .4, varnames obsno
putdocx describe b
display "@ pd_b " r(nrows) " " r(ncols)
putdocx table c = data(x) in 2/3
putdocx describe c
display "@ pd_c " r(nrows) " " r(ncols)
capture noisily putdocx table d = data(nosuch)
display "@ pd_data_nosuch " _rc
capture noisily putdocx table d = matrix(NOSUCH)
display "@ pd_matrix_nosuch " _rc
matrix M = (1.5, 2 \ 3, 4 \ 5, 6)
matrix rownames M = r1 r2 r3
matrix colnames M = c1 c2
putdocx table m1 = matrix(M)
putdocx describe m1
display "@ pd_m1 " r(nrows) " " r(ncols)
putdocx table m2 = matrix(M), rownames
putdocx describe m2
display "@ pd_m2 " r(nrows) " " r(ncols)
putdocx table m3 = matrix(M), colnames
putdocx describe m3
display "@ pd_m3 " r(nrows) " " r(ncols)
putdocx table m4 = matrix(M), rownames colnames nformat(%5.1f)
putdocx describe m4
display "@ pd_m4 " r(nrows) " " r(ncols)
quietly regress x k
capture noisily putdocx table e = etable
display "@ pd_etable_quiet " _rc
regress x k
putdocx table e = etable
putdocx describe e
display "@ pd_e " r(nrows) " " r(ncols)
capture noisily putdocx table e2 = etable
display "@ pd_etable_after_describe " _rc
regress x k
putdocx table e = etable
putdocx describe
capture noisily putdocx describe nosuch
display "@ pd_describe_none " _rc
putdocx pagebreak
capture noisily putdocx text ("after pagebreak")
display "@ pd_after_break " _rc
putdocx paragraph, halign(center)
putdocx text ("Centered")
capture noisily putdocx save
display "@ pd_save_nofile " _rc
capture noisily putdocx save pdx_a.txt
display "@ pd_save_ext " _rc
putdocx save pdx_a, replace
capture noisily putdocx save pdx_a
display "@ pd_save_closed " _rc
putdocx begin
putdocx paragraph
putdocx text ("second")
capture noisily putdocx save pdx_a
display "@ pd_save_exists " _rc
capture noisily putdocx save pdx_none, append
display "@ pd_append_none " _rc
putdocx save pdx_a, append
putdocx begin
putdocx paragraph
putdocx text ("t")
putdocx save pdx_b.docx, replace
putdocx clear
capture noisily putdocx describe
display "@ pd_cleared " _rc

putpdf clear
capture noisily putpdf paragraph
display "@ pf_nodoc " _rc
putpdf begin
capture noisily putpdf begin
display "@ pf_twice " _rc
capture noisily putpdf text ("x")
display "@ pf_noparagraph " _rc
capture noisily putpdf paragraph, halign(bogus)
display "@ pf_halign " _rc
capture noisily putpdf paragraph, style(Title)
display "@ pf_style " _rc
putpdf paragraph
putpdf text ("Plain, ")
putpdf text ("bold"), bold
putpdf text (1/3), nformat(%5.3f)
putpdf table a = (2, 3)
putpdf table a(2,3) = ("b")
capture noisily putpdf table a(3,1) = ("c")
display "@ pf_cell_row " _rc
putpdf describe a
display "@ pf_a " r(nrows) " " r(ncols)
putpdf paragraph
putpdf table b = data(x s) if x > .4, varnames obsno
putpdf describe b
display "@ pf_b " r(nrows) " " r(ncols)
putpdf table m1 = matrix(M), rownames colnames
putpdf describe m1
display "@ pf_m1 " r(nrows) " " r(ncols)
regress x k
putpdf table e = etable
putpdf describe e
display "@ pf_e " r(nrows) " " r(ncols)
putpdf describe
putpdf pagebreak
putpdf paragraph, halign(right)
putpdf text ("Right")
capture noisily putpdf save pdf_a.txt
display "@ pf_save_ext " _rc
putpdf save pdf_a, replace
capture noisily putpdf save pdf_a
display "@ pf_save_closed " _rc
putpdf begin
putpdf paragraph
putpdf text ("second")
capture noisily putpdf save pdf_a
display "@ pf_save_exists " _rc
capture noisily putpdf save pdf_a, append
display "@ pf_append " _rc
putpdf clear
display "@ done 1"
erase pdx_a.docx
erase pdx_b.docx
erase pdf_a.pdf
