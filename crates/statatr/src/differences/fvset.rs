Guide {
    command: "fvset",
    about: "Chooses a factor variable's base level",
    points: &[p(
        "Base levels",
        "fvset base # (and default, and fvset clear) is kept by RCraft, and later models \
         write i.x and a bare x in an interaction as ib#.x, which is what Stata does. The \
         choice stays with the data as in Stata: save and use keep it, preserve and restore \
         bring it back, and import or clear drops it. fvset base first, last, frequent and \
         none, fvset design and fvset report are not translated yet.",
    )],
}
