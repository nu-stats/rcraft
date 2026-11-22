Guide {
    command: "recode",
    about: "Changes values by rules",
    points: &[p(
        "Rules",
        "The rules are taken in Stata's order, the first that matches a value deciding it; \
         missing values are kept unless a rule names them, as in Stata. Labels in the \
         rules (`1 \"low\"`) make a label set for each new variable, named after it or \
         label(), as Stata does; they need generate() or prefix(). The older syntax without \
         parentheses (recode x 1=10 2=20) is taken too. Stata's count follows for each \
         variable, always plural: (1 changes made to x), or (N differences between x and \
         new) with generate().",
    )],
}
