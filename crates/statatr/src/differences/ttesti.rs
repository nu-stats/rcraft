Guide {
    command: "ttesti",
    about: "t tests from the counts, means and standard deviations typed in",
    points: &[p(
        "As Stata 19",
        "ttesti #obs #mean #sd #val (one sample) and ttesti #obs1 #mean1 #sd1 #obs2 #mean2 \
         #sd2 (two samples, with unequal or welch), with level(): ttest's table and lines, \
         the two samples together as Combined, and r() as Stata's.",
    )],
}
