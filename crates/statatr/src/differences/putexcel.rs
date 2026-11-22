Guide {
    command: "putexcel",
    about: "Writes values into the cells of an Excel workbook",
    points: &[
        p(
            "As Stata 19",
            "putexcel set file.xlsx with sheet() and modify or replace (or neither, for a \
             new file); cell = \"text\", a number or ( expression ), several to a line; \
             putexcel clear. Text cells, numbers to 17 digits and . an empty cell, the other \
             sheets and cells kept, file … saved after each write, and Stata's errors (a file \
             that exists, no set, a column past XFD). Written with the R package openxlsx2.",
        ),
        p(
            "matrix()",
            "cell = matrix(M) writes the values from the cell down and right; names puts the \
             column names on the first row and the row names in the first column (the corner \
             left as it was), rownames and colnames one of them. The names are Stata's full \
             ones (1bn._at, 1b.g). putexcel leaves r() as it was, as in Stata 19.",
        ),
        p(
            "Not yet",
            "Ranges, matrices with equation names, etable, formula(), pictures, the formatting options (bold, \
             nformat() and others), sheet(, replace), .xls files, and putexcel save, close \
             and describe.",
        ),
    ],
}
