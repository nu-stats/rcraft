Guide {
    command: "xtunitroot",
    about: "Panel unit-root tests",
    points: &[
        p(
            "As Stata 19",
            "xtunitroot ips, ht, breitung, fisher, hadri and llc on strongly balanced panels, with if and in. ips: the mean of the panels' Dickey-Fuller t statistics (t-bar with Table 2's exact critical values, Z-t-tilde-bar), or W-t-bar with lags(0 to 8). ht: the pooled AR coefficient (altt shortens T by one). breitung: lambda, with noconstant, trend or demean. fisher: Choi's P, Z, L* and Pm from each panel's dfuller (lags(), trend, drift) or pperron p-values. hadri: the LM z statistic, robust or with a Bartlett or Parzen kernel of a given length. llc: Levin, Lin and Chu's adjusted t* (unadjusted t, delta, sbar, with lags(#) and kernel(bartlett # or parzen #, or ... llc) and the adjustment table by the number of periods; the default bandwidth follows each panel's rows in the data). demean removes the cross-sectional means. When the sample starts after the data do, the first period's lag is read from the period before (but not for a demeaned series). r() holds N, N_t, N_g, the statistics and p-values, and the texts test, deterministics, demean, urtest and options.",
        ),
        p(
            "Errors",
            "No test or an unknown one, an unknown option, noconstant with ips, hadri and trend, lags() or a test missing for fisher r(198); no variable r(100), two variables r(103); without xtset r(459); a name the data lack r(111); a string variable r(109). Unbalanced panels with llc r(498). Refused: llc with lags(aic or bic or hqic) or kernel(... nwest), breitung with robust or lags(), ips with lags() above 8 or lags(aic), hadri kernels other than bartlett # and parzen #, fisher with noconstant, a panel whose p-value is 0 or 1, the Phillips-Perron form with trend, demean or a sample that starts late, ht with trend on a sample that starts late, panels that are not strongly balanced or have gaps in time, fewer than 4 periods.",
        ),
    ],
}
