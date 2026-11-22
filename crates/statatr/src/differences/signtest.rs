Guide {
    command: "signtest",
    about: "Sign test of matched pairs (signtest x = w, signtest x = 0)",
    points: &[p(
        "In R",
        "stata_signtest prints Stata 19's table of positive, negative and zero signs, and the \
         one-sided and two-sided binomial tests with their hypotheses; r() as Stata's \
         (N_pos, N_neg, N_tie, p, p_l, p_u).",
    )],
}
