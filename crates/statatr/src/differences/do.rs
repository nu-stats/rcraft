Guide {
    command: "do",
    about: "Runs another do-file (run quietly; include as if its lines were here)",
    points: &[
        p("Files", "The file is found from the folder the do-file is in then (after any cd), with .do added when it has no extension, as Stata does. RCraft reads it and runs its lines in turn; a file that is not there is said so and not run."),
        p("Macros", "do and run give the file its own locals, with its arguments in `0', `1', ...; include shares the locals of the do-file that includes it."),
    ],
}
