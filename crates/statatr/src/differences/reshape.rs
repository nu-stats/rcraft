Guide {
    command: "reshape",
    about: "Long to wide and back",
    points: &[p(
        "Order",
        "Long to wide and back with i() and j() as Stata takes them; the rows come out \
         sorted by i and j, as Stata leaves them. reshape long without j() names the new \
         variable _j, as Stata does. Stata's summary table (wide -> long) is not printed.",
    )],
}
