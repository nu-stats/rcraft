Guide {
    command: "putdocx",
    about: "Builds a Word document in memory and saves it",
    points: &[
        p(
            "As Stata 19",
            "begin opens a document (a second one: document already open in memory, r(110)); \
             every other subcommand without one, or after save, stops with document not \
             opened (r(198)). paragraph [, style() halign()] starts a paragraph (styles \
             Title, Subtitle and Heading1 to Heading9; halign left, center or right) and \
             text (exp) [, bold italic underline linebreak nformat()] adds a run to it; a \
             number without nformat() is shown in %11.0g, with nformat() in that format \
             padded to its width; text with no paragraph, or after a table or pagebreak, is \
             no active paragraph (r(198)). table name = (r, c) makes an empty table, \
             name(i,j) = (exp) fills a cell, data(varlist) [if] [in] [, varnames obsno] \
             writes each variable in its display format (value labels used, the original \
             observation numbers with obsno), matrix(M) [, rownames colnames nformat()] \
             the matrix (default format %12.0g), and etable the coefficient table the last \
             regress printed, which Stata takes from r(): none after quietly regress or \
             once a later command has replaced r() (information for the estimation table \
             not found, r(111)). Every table is followed by an empty paragraph, a table \
             name used again is redefined with a note and the old table stays. describe \
             [name] prints the table's rows and columns (r(nrows), r(ncols)) or the number \
             of tables and paragraphs (r() unchanged). save [, replace append] adds .docx \
             to a name without an extension, another extension is r(198), an existing file \
             without replace or append is r(198), append to one that is not there r(601); \
             the document is closed after it. append puts an empty paragraph between the \
             old body and the new one.",
        ),
        p(
            "In R",
            "The document is built in R and the Word file written from it (the package is \
             zipped by R's zip package; officer's template has no Title or TableGrid style \
             and is A4 with French style names). The file holds the same paragraphs, styles, \
             alignment, runs, tables, column spans and page break as Stata's \
             (crates/engine/tests/stata_putdoc.rs reads both back), but not its fonts, \
             sizes or cell widths. Refused: image, sectionbreak, pagenumber, textblock, \
             textfile, append of documents, collect, the options of begin, fonts, cell \
             formats and table options other than nformat(), etable after a model other \
             than regress, and a table with no observations.",
        ),
    ],
}
