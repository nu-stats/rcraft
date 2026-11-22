Guide {
    command: "constraint",
    about: "Define and list linear constraints",
    points: &[
        p(
            "As Stata 19",
            "constraint [define] # exp keeps the text as typed (1 to 1999), without checking it \
             until a command uses it; constraint dir and list [numlist] print `     1:  x1 = x2`; \
             get # sets r(defined) and r(contents); free sets r(free), the highest number not in \
             use; drop takes a numlist or _all. Only get and free change r(). cnsreg reads them.",
        ),
        p(
            "Not yet",
            "Numbers above 1999 (Stata accepts them silently), quoted text, and the \
             constraints() option of other models (logit, poisson, sspace and the rest).",
        ),
    ],
}
