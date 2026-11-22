Guide {
    command: "dyndoc",
    about: "Runs a text with dynamic tags into HTML through Markdown",
    points: &[
        p(
            "As Stata 19",
            "dyndoc src [, saving(file) replace] runs the tags as dyntext does (the file is \
             src with .html without saving()), prints the text, turns it into HTML through \
             Markdown (as markdown does) and says successfully converted with the file's \
             full path. <<dd_graph>> exports the graph as SVG (Graph.svg, or Graph1.svg \
             when that is there) and puts an <img> tag for it, with saving(), replace, \
             alt(), height() and width().",
        ),
        p(
            "In R",
            "As dyntext. The graph is R's (ggplot2), written by R's svg device; the HTML is \
             Stata's for the corpus file and the probes checked.",
        ),
    ],
}
