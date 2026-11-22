Guide {
    command: "list",
    about: "Lists rows",
    points: &[
        p(
            "As Stata 19",
            "Stata's table: each column as wide as its widest value or its name, cells \
             right-aligned (left with a left-justified format), a line every five rows \
             (separator(#)) or where a sepby() variable changes, the row numbers before the \
             box; noobs, clean, nolabel, table and display as Stata's. A table wider than \
             the line puts each row in a box of its own, as Stata does, and a line longer \
             than the linesize breaks after > as Stata's. Numbers, dates and times in their \
             display format (a set one, the one their storage type was made with, or else \
             %9.0g for values a float holds, %10.0g for others and %12.0g for whole \
             numbers), labelled values by their label.",
        ),
        p(
            "Names in full",
            "RCraft's choice: the names are written out in full, where Stata cuts them to 8 \
             characters (a_very~e); abbreviate(#) cuts them as Stata does.",
        ),
        p(
            "In R's layout",
            "A variable R holds in a form of its own (a date class, a factor), a display \
             format other than numbers, dates and times (%21x), or a variable wider than \
             the line prints the rows in R's layout.",
        ),
    ],
}
