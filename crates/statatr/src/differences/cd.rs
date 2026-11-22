Guide {
    command: "cd",
    about: "Changes the folder the do-file works in",
    points: &[
        p("In R", "setwd(), with the folder as written: a relative one is taken from the folder the do-file is in then, as in Stata."),
        p("Paths", "A \\ in a path is read as /, which R takes on every system. A folder the do-file names that is somewhere else on this machine can be pointed to before the run, in the do-file window."),
        p("No folder", "cd alone, or cd \"\" (as a global that was never set leaves it), prints the folder and stays there, as Stata does on Windows. On macOS and Linux Stata goes to the home folder instead; RCraft stops there with a message rather than guess."),
    ],
}
