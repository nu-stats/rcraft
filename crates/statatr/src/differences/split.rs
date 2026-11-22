Guide {
    command: "split",
    about: "Cuts a string variable into pieces",
    points: &[p(
        "As Stata 19",
        "Without parse(), into words (runs of spaces as one); with parse(), at each string \
         it gives (\" \" too) after trimming the ends (notrim keeps them), the earliest one \
         winning, empty pieces kept and none after a last delimiter; limit() drops the \
         pieces past it; new variables stub1, stub2, ... as text, r(k_new) and r(varlist). \
         Parse strings that share characters and destring are declined.",
    )],
}
