Guide {
    command: "ir",
    about: "Incidence-rate table (ir case exposed time, iri 20 15 2000 3500)",
    points: &[p(
        "In R",
        "stata_epi prints Stata 19's table: cases and person-time of the exposed and the \
         unexposed (rows named by the variables' labels), the rates, the rate difference \
         with its Wald interval, the rate ratio with the exact interval (R's poisson.test, \
         the same as Stata's), the attributable or prevented fractions, and the mid-p and \
         exact tests; r() as Stata's. by(): each stratum's rate ratio with its exact \
         interval and Mantel-Haenszel weight, the crude ratio, the Mantel-Haenszel combined \
         one (Greenland and Robins's interval), the pooled (inverse-variance) one in r() and \
         the test of homogeneity. ird, tb and the other options are refused.",
    )],
}
