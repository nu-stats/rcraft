Guide {
    command: "markdown",
    about: "Turns a Markdown file into HTML",
    points: &[
        p(
            "As Stata 19",
            "markdown src, saving(file) [replace] writes the HTML body only, one block a \
             line: headings with an anchor named after their text (lower case, spaces as \
             -, punctuation dropped, the same id twice for the same text), the \
             typographer's quotes, dashes (--, ---), ellipses and << >> as entities, tight \
             and loose lists (in a tight list a paragraph after a blank line keeps its \
             <p>), indented and fenced code, quotes, rules, HTML blocks, pipe tables whose \
             cells keep their spaces, code spans, emphasis, links, images, autolinks and \
             backslash escapes; a soft line break is a space, two trailing spaces a <br />. \
             A source that is not there is r(601), a target that is there without replace \
             r(602).",
        ),
        p(
            "In R",
            "RCraft's own converter, written to Stata's rules: the HTML is byte for byte \
             Stata's for the corpus file and the probes checked. Markdown that Stata's \
             converter reads in ways not observed (reference links, footnotes, nested \
             quotes) may differ.",
        ),
    ],
}
