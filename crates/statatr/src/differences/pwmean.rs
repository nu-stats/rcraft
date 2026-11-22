Guide {
    command: "pwmean",
    about: "Pairwise comparisons of means with equal variances",
    points: &[
        p(
            "As Stata 19",
            "pwmean y, over(vars) gives every group's mean and the pairs j vs i (i < j) with the \
             pooled within-group variance on N - G degrees of freedom, unadjusted or adjusted \
             by mcompare(bonferroni), sidak, scheffe, tukey, snk, duncan or dunnett; effects, \
             pveffects, cieffects, cimeans, groups, sort and level() choose the tables. Several \
             over() variables make the groups g#w, labelled as Stata labels them. A bare pwmean \
             shows the last results again with other options, and groups with no or one \
             residual degree of freedom print missing values, as Stata does. The printed tables \
             are Stata's to the column. e() holds the means and contrasts with b, V, b_vs, V_vs \
             and the error matrices; r() has table, table_vs, k_groups, level and the method.",
        ),
        p(
            "Precision",
            "Tukey's p-values and critical values come from R's ptukey, which differs from \
             Stata's tukeyprob by about 1e-10; r(table_vs) and the Tukey columns agree to \
             about 1e-7 (the SNK and Duncan critical values to about 3e-8 for long spans, which \
             shows as 1e-6 in a limit near zero). Dunnett's critical value and p-values come \
             from the exact integral, while Stata's dunnettprob is approximate: the critical \
             value differs by up to about 6e-6 and a p-value by up to about 3e-4 in the \
             relative, so a printed Dunnett interval can differ from Stata's in its fifth \
             digit. Stata leaves rounding noise of about 1e-16 where e(V) is zero in a table of \
             several over() variables, and two means that are equal to the last bit can be \
             spanned in another order in SNK and Duncan.",
        ),
        p(
            "Not yet",
            "Weights (an error, as in Stata) and the order of the letters of groups when the \
             means form overlapping sets: the sets of means that share a letter agree, but the \
             letters can be dealt in another order, so r(groups1) and the rest are not kept.",
        ),
    ],
}
