Guide {
    command: "compress",
    about: "Makes variables' storage types smaller",
    points: &[p(
        "Nothing to do",
        "R has no storage types (byte, int, float, str20), so compress changes nothing in R \
         and the values stay as they are. Stata lists each variable it makes smaller and the \
         bytes saved; RCraft prints nothing.",
    )],
}
