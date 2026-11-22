Guide {
    command: "error",
    about: "Stops with a return code",
    points: &[p(
        "Codes",
        "error # stops the do-file, or the program it is in, with return code #, which \
         capture keeps in _rc; error 0 does nothing. In a replication of simulate or \
         permute it ends that replication, whose results are then missing. An expression \
         (error _rc) is not translated yet.",
    )],
}
