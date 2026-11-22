Guide {
    command: "dyntext",
    about: "Runs a text with dynamic tags into a text file",
    points: &[
        p(
            "As Stata 19",
            "dyntext src, saving(file) [replace] prints the processed text and writes it. \
             <<dd_do>> blocks run their commands and show each as a do-file's log does \
             (. and the command, its output, a blank line), less what quietly, nocommands, \
             nooutput and noprompt leave out; <<dd_display: [%fmt] exp>> is what display \
             shows; dd_if, dd_else and dd_endif (and dd_skip_if, dd_skip_else, dd_skip_end) \
             show or skip their lines, a tag on its own line goes with its line and the \
             blank lines around stay; dd_ignore keeps its lines as they are, tags and all, \
             and dd_include puts a file's lines in as they are; dd_remove's text is printed \
             but taken out of the file. A source that is not there is r(601), a target that \
             is there without replace r(602) with its full path, before any tag runs. On \
             Windows the lines a dd_do command prints end in a carriage return in the file.",
        ),
        p(
            "In R",
            "The source is read when the line is reached, and its tags become lines of the \
             run: the dd_do commands are translated as any other line, with what each \
             prints kept for the document; the file is written only when every line of it \
             was translated. The text and the file are the same as Stata's for the corpus \
             file and the probes checked. Refused: a dd_if inside another (Stata does not \
             nest them), dd_version other than 2, dd_do blocks with blank lines, comments, \
             braces or lines that go on (///), dd_graph to anything but an .svg file, other \
             tags, and dyntext under capture or quietly unless Stata stops before the tags \
             run.",
        ),
    ],
}
