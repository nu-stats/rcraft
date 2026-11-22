Guide {
    command: "ssc",
    about: "ssc install: Stata's add-ons",
    points: &[p(
        "Add-ons",
        "Stata's add-ons (ado programs) are out of scope: their commands are not \
         translated. ssc install name offers the R package of the same name instead, \
         through rc install, when CRAN has one, and says so when it has none; the R \
         package is R's, not a translation of the add-on. net install is refused.",
    )],
}
