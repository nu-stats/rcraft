Guide {
    command: "cluster",
    about: "Hierarchical clustering, kmeans and kmedians",
    points: &[
        p(
            "As Stata 19",
            "cluster singlelinkage|completelinkage|averagelinkage|wardslinkage varlist [, name() measure()], cluster \
             kmeans|kmedians varlist, k(#) [start() name() measure()], cluster generate v = groups(k), cluster list, \
             dir and drop: the merges, their order and heights, the variables name_id, name_ord and name_hgt, the \
             group numbers (kmeans start(krandom) draws from the do-file's runiform() stream) and the objects kept in \
             the data's characteristics. Checked against Stata 19.",
        ),
        p(
            "Not yet",
            "if and in, weights, cluster stop (the stopping rules), dendrogram, measures, notes, rename, use, \
             similarity measures for binary data, centroid, median and weighted-average linkage, and the other start() \
             forms. The storage type of the new variables is not tracked, so describe after a cluster command does \
             not show it.",
        ),
    ],
}
