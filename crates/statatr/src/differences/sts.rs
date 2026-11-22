Guide {
    command: "sts",
    about: "Kaplan-Meier lists, new variables and tests of equal survival",
    points: &[
        p(
            "Forms",
            "sts list (survivor, failure or na, by(), at(), level()), sts generate (s, se(s), \
             lb(s), ub(s), na, se(na), lb(na), ub(na), h, n, d, by()) and sts test (logrank, \
             wilcoxon, tware, peto, fh(p q), strata()); r(chi2) and r(df) after sts test.",
        ),
        p(
            "Not yet",
            "sts graph, the cox and trend tests, adjustfor(), enter and compare, and any of \
             these with late entry are refused with the reason.",
        ),
        LAYOUT_R,
    ],
}
