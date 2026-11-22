Guide {
    command: "power",
    about: "Power and sample size (power onemean 0 0.5, power twoproportions 0.3 0.5, n(150))",
    points: &[
        p(
            "In R",
            "stata_power works out the sample size, the power, or (with one value, n() and \
             power()) the target value for onemean, twomeans, pairedmeans, oneproportion, \
             twoproportions, onevariance, twovariances, onecorrelation, twocorrelations, \
             pairedproportions, oneway, logrank, cox, exponential, trend, oneslope, rsquared and pcorr, from the [PSS] manual's power \
             functions (logrank: Freedman's or Schoenfeld's events; cox: Hsieh and Lavori's): the \
             noncentral t (or z with knownsd), Satterthwaite's t for unequal sds, the score or \
             Wald test of a proportion, the pooled test of two with Fleiss's continuity \
             correction, the chi-squared test of a variance, the F test of two, Fisher's z test \
             of correlations, the large-sample McNemar test and the F test of a group effect \
             (its noncentral F summed to full precision; R's pf() agrees to about 1e-9). Sample sizes are the smallest whole numbers \
             reaching the power (the second group nratio() times the first, rounded up); a \
             total that does not split is cut to Stata's actual sample sizes. Stata 19's \
             printout and r(), but for its search's own r(init), r(iter), r(deltax) and \
             r(function).",
        ),
        p(
            "As Stata",
            "Two variances are tested with F on n1 - 1 and n2 - 1 degrees of freedom, scaled by \
             v2/v1, as Stata 19 does. One-sided, pairedproportions' power is the upper tail even \
             when p21 is below p12 (its sample size uses the size of the difference), as in \
             Stata 19.",
        ),
        p(
            "Digits",
            "R's noncentral t, chi-squared and F and its root finding agree with Stata's to \
             about 1e-12; what is printed is the same.",
        ),
        p(
            "Not yet",
            "Graphs, parallel, table() with columns, return list after power oneway (Stata \
             lists its r() in an order that changes with the number of groups), oneway's \
             lists and contrasts, logrank's and cox's lists, wdprob() and simulation, the \
             other methods (twoway, repeated, mcc, cmh, \
             ...), the exact tests' sample size and effect size (Stata refuses them too), \
             one group's size that leaves the other not whole, \
             pairedproportions' corr(), diff() and ratio() \
             forms, and the other options are refused.",
        ),
        p(
            "Tables",
            "Lists of values print Stata's table, each row worked out as its own power line, in \
             Stata's order; r() is the last row's, with r(columns), r(labels), r(widths), \
             r(formats) and r(pss_table). return list after a table is refused: Stata lists \
             the table's r(separator) and r(divider) in an order RCraft does not follow.",
        ),
        p(
            "As Stata's limits",
            "Stata's noncentral t takes noncentralities up to 1000: past them two means stop \
             with r(498) (in a table, a missing power and Stata's warning) and one mean shows a \
             missing power, in RCraft as in Stata 19. After cox given b1 or worked out, \
             r(hratio) can be read but return list leaves it out, as Stata keeps it. \
             test(binomial) and test(fisher) give the power and the actual alpha only (their \
             power is saw-toothed in n, so Stata offers no sample size or effect size); Fisher's \
             two-sided p-value is twice the smaller tail.",
        ),
    ],
}
