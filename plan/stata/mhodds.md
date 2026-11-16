# mhodds (P2C.12)

Observed in Stata 19 on made-up data (probes 301–304); translated in `crates/statatr/src/data_r/mhodds.rs`.

- Binary exposure: exactly two values (the larger compared with the smaller), or `compare(v1, v2)` (the data cut
  to those two). Per stratum of the controls: R = ad/n, S = bc/n, U = a − (a + b)(a + c)/n, V = the null
  hypergeometric variance (a + b)(c + d)(a + c)(b + d)/(n²(n − 1)). OR = ΣR/ΣS, chi2 = (ΣU)²/ΣV (no continuity
  correction), the interval exp(ln OR ± z·√(ΣV/(ΣR·ΣS))). Checked to about 1e-15 on ten designed 2×2 tables and
  the stratified cases; Woolf's and the test-based intervals do not fit (that is why the interval looked "wider
  than Woolf's by a factor").
- Otherwise (three or more values, or one): the score test for trend, U = Σx·y − DΣx/N, V = D(N − D)/(N²(N − 1))
  (NΣx² − (Σx)²) summed over strata, OR exp(U/V), interval exp(U/V ± z/√V), with the Note on a one-unit increase.
- Title: `Maximum likelihood estimate ...` with no controls, `Mantel–Haenszel estimate ...` with them (en dash),
  `Score test for trend of odds with x`; `  controlling for a`, `a and b`, `a, b, and c`; `  by a`. Each part is
  wrapped at the table's width (68, or 79 with by()), continuation indented 2. Values shown as %10.0g (`dhalf==.5`).
- Table: `%15.6f %11.2f %14.4f %17.6f %11.6f`; a missing p prints in 13 and the lower bound in 18. R = 0 prints
  0.000000 with `.` bounds; S = 0 prints `.`.
- Notes (wrapped, `Note: ` indent 6, two of them `Notes: ` indent 7): `Only i of the n strata formed in this
  analysis contribute information about the effect of the explanatory variable.` when a stratum has V = 0.
- by(): rows `%9s |` `%11.6f %15.2f %15.4f %16.5f %11.5f` (two variables `%9s%10s |` and `%11 %12 %12 %12 %11`),
  names abbreviated to 8; then `Mantel–Haenszel estimate controlling for <controls and by()>` over all strata and
  `Approximate test of homogeneity of odds ratios: chi2(df)   =` with `Pr>chi2   =`. Binary homogeneity:
  Σ(R_k − ψS_k)²/(ψV_k), ψ the overall OR; trend: ΣU_k²/V_k − (ΣU)²/ΣV; df = groups − 1. Fewer than two
  informative groups: `Note: Too few informative strata to test for unequal odds ratios.` and no df_hom/chi2_hom.
- r(): or, lb_or, ub_or, chi2, p; with by() df_hom, chi2_hom and strata_table (or chi2 pvalue ll ul, rows named by
  the group values). r() and the matrix can differ from Stata's in the last digit or two.
- Missing values in any of the variables drop the row; fweights count; aweights/iweights/pweights `... not
  allowed` r(101); one variable `too few variables specified` r(102); `response variable y not coded 0/1` r(198);
  `option compare(v_1, v_2) must specify two categories of variable x` r(198); `no observations` r(2000);
  `binary` is not an option (r(198) in Stata, refused here as an unknown option).
- Refused: by() with three or more variables, with value labels, with a control stratum that carries no
  information, or with uninformative groups beside two or more informative ones; both notes in by(); levels that
  are not whole numbers; string variables.
