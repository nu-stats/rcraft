Guide {
    command: "outreg",
    about: "Tables of estimates and statistics (outreg, SSC)",
    points: &[
        p(
            "As Stata 19",
            "The balance-table use: after a model, keep() rtitle() stats(b) noautosumm \
             starlevels() starloc(1) store(), the coefficient with three decimals, thousands \
             commas and stars; replay() with append() or merge(), ctitles() (rows split by \
             \\), note(\"\"), store(); and using a file with tex fragment plain nocenter, \
             multicol() and replace, the .tex byte for byte as outreg writes it (CR LF). \
             Observed on made-up data; the package's programs were not read.",
        ),
        p(
            "Not yet",
            "Every other layout: standard errors or t statistics under the coefficients, \
             summary statistics, outreg's own notes, Word and RTF files, a table without \
             fragment. On screen the table is printed in R's layout.",
        ),
    ],
}
