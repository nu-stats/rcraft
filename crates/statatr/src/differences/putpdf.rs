Guide {
    command: "putpdf",
    about: "Builds a PDF document in memory and saves it",
    points: &[
        p(
            "As Stata 19",
            "The same commands as putdocx (begin, clear, paragraph [, halign()], text, table \
             name = (r, c), name(i,j), data(), matrix() and etable, describe, pagebreak, save) \
             with the same messages and return codes; putpdf has no style() (option \
             style(Title) not allowed), save has no append (option append not allowed), \
             adds .pdf, says X.pdf already exists and you must specify the replace option, \
             and a table with no rows or columns is accepted but leaves Stata's own document \
             unusable.",
        ),
        p(
            "In R",
            "The PDF is drawn by R's pdf device: A4 pages (as the file Stata wrote here), 36 \
             points of margin at the sides and 86 at the top, Helvetica 11, a grid per \
             table and each run and cell in the order Stata puts it (the first line starts \
             at Stata's left margin, the runs after it at the same points; the rows' and \
             gaps' heights are measured from one file). Long paragraphs wrap at the \
             margin. Refused: the same forms as putdocx, and a table with no rows or \
             columns.",
        ),
    ],
}
