Guide {
    command: "center",
    about: "Centers or standardizes variables (center, SSC)",
    points: &[
        p(
            "As Stata 19",
            "Each variable less its mean (over if, in and, with casewise, the rows with none of \
             them missing; other rows missing), divided by its standard deviation with \
             standardize; aweights and fweights as summarize takes them. Without double the \
             mean and standard deviation are rounded to float first, as center does: new \
             variables c_x (or prefix()) are float, inplace keeps a double double and makes \
             any other type float. The labels \"(centered)\" and \"(standardized)\", nolabel, \
             replace and the add-on's messages and errors. Observed on made-up data; the \
             add-on's program was not read.",
        ),
        p(
            "Not yet",
            "pweights and iweights, generate(), meansave() and sdsave(). With inplace, a \
             variable whose storage type RCraft does not know (one read from a file) is \
             taken as double when a value cannot be a float and otherwise as float, Stata's \
             default, with a note.",
        ),
    ],
}
