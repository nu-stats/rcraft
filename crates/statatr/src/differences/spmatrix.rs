Guide {
    command: "spmatrix",
    about: "Spatial weighting matrices",
    points: &[
        p(
            "As Stata 19",
            "create idistance with normalize(spectral, row, minmax or none), vtruncate(), if \
             and in; dir; summarize and its r(); drop, copy and note; export and import in \
             Stata's text form; fromdata. The weights agree with Stata's to the last digit or \
             two, and the matrices are kept across clear, as in Stata. Stata's errors keep \
             their return codes.",
        ),
        p(
            "Differences",
            "save and use write and read RCraft's own file: a .stswm file Stata saved is not \
             read. Contiguity matrices (which need a shapefile), matafromsp and spfrommata \
             (Mata), normalize and userdefined are not translated, and create stops on missing \
             coordinates or two places at the same point.",
        ),
    ],
}
