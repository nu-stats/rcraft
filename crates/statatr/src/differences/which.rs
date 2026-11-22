Guide {
    command: "which",
    about: "Whether a command is installed (capture which, as do-files check their add-ons)",
    points: &[
        p(
            "In R",
            "capture which name (or quietly) sets _rc as Stata 19 does, 0 when found and 111 \
             when not, printing nothing. Found means RCraft translates the command (the estout \
             package for its esttab, eststo, estadd and estpost); an add-on it does not \
             translate counts as not installed, so a do-file's own install step runs as on a \
             machine without it.",
        ),
        p(
            "Not yet",
            "which without capture or quietly (it lists the command's file and its version \
             lines), several names and options are refused.",
        ),
    ],
}
