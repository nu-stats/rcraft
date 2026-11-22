Guide {
    command: "frame",
    about: "Several datasets at once (frame create, change, drop, pwf)",
    points: &[
        p(
            "In R",
            "Each frame's data is kept in the list stata_frames while another is current; \
             frame change puts the current one there and takes the other out, so each keeps \
             its own data, labels, settings and pending preserve, and e() stays, as in Stata \
             19. A frame that is \
             there already is r(110), one that is not r(111), dropping the current one r(119), \
             each stopping in R so capture gives Stata's _rc.",
        ),
        p(
            "In R",
            "frlink 1:1 and m:1 sort the linked frame by its keys and keep, in a column, the \
             row of that frame each row matches (missing when none does), with Stata's note on \
             how many rows matched; frget copies variables through the link (their label and \
             format with them, text empty where unmatched, with Stata's note on missing \
             values) and frval() looks a value up through it. Stata checks at each use that the \
             linked frame still looks as it did; RCraft follows the do-file instead, so a \
             frget or frval() after a line that may have changed the linked frame is refused.",
        ),
        p(
            "Not yet",
            "frlink and frget under capture, frlink 1:m, frlink rebuild and describe, frget \
             of _all, of a variable with value labels, with both prefix() and suffix(), \
             frame commands inside a frame prefix, and frame dir after a line that changed a \
             frame's data in a way RCraft does not follow are refused.",
        ),
    ],
}
