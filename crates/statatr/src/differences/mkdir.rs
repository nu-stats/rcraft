Guide {
    command: "mkdir",
    about: "Makes a folder (rmdir removes an empty one)",
    points: &[
        p(
            "As Stata 19",
            "mkdir makes one folder whose parent is there. Anything that stops it (the folder \
             or a file is already there, the parent is missing) is could not create directory, \
             r(693): unlike the shell's mkdir -p, a nested path is not made in one go. No name \
             is invalid syntax (r(198)), two names varlist not allowed (r(101)), and , public \
             is accepted. rmdir removes an empty folder, and could not remove directory \
             (r(693)) for one that is not empty, not there, or a file.",
        ),
    ],
}
