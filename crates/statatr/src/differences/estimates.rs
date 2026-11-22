Guide {
    command: "estimates",
    about: "Keeps fitted models by name and brings them back",
    points: &[
        p(
            "Store and restore",
            "estimates store keeps the model with its e(), e(sample) and what predict, test \
             and margins read; estimates restore brings it back, with Stata's note \
             (results a are active now). drop and clear forget them, and a name that is not \
             stored is Stata's error, r(111).",
        ),
        p(
            "_est_name",
            "Stata 19 keeps each stored model's e(sample) in the data, as the byte variable \
             _est_name (labelled esample() from estimates store) at the end; so does the \
             translation, and estimates drop and clear (and eststo drop and clear, eststo's \
             own) take them out again. Where R does not hold e(sample) (after cmclogit, \
             margins post), a line that reads _est_name is refused.",
        ),
        p(
            "dir, stats and replay",
            "Stata 19's tables: estimates dir (each stored model's command, dependent variable, \
             number of parameters and title), estimates stats (N, ll(null), ll(model), df, AIC \
             and BIC, with r(S)) of the active results or of the names given, and estimates \
             replay of regress, logit, probit and poisson fits (robust and clustered too), \
             under a banner as wide as the line (Model name, or active results): as Stata 19, \
             the coefficients at 95% whatever or, irr or level() the fit had, and no \
             iteration log. A model's name alone (regress, logit, probit, poisson, with \
             level(), or and irr) shows its last fit again in the same way; after another \
             command's fit it is Stata's error (last estimates not found).",
        ),
        p(
            "Not yet",
            "estimates replay of other models (or with weights), \
             estimates save and use, and options are not translated. A name stored by a line \
             that was not translated (eststo, a program) is not known, so a restore of it is \
             refused rather than guessed.",
        ),
    ],
}
