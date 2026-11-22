# cluster (batch MV, 2026-11-22)

Ported from statcraft@b7d392e (its `cluster.md`, probe 248), then checked against Stata 19 on made-up data: corpus
`cluster.do` (193 values). Translated in `crates/statatr/src/data_r/multi/clusters.rs` and `multi/cluster.R`.

- `cluster singlelinkage|completelinkage|averagelinkage|wardslinkage varlist [, name() measure()]`. Dissimilarity L2
  (L2squared for Ward's; `measure(L1|L2|L2squared|Linfinity)` and their aliases). The closest active pair (first found,
  scanning the lower index first) merges, by the Lance-Williams update; the merged cluster keeps the lower index for
  complete, average and Ward's linkage and the higher one for single linkage, and the lower one's leaves come first. All four
  dendrogram orders and heights in the probe agree with Stata's.
- Variables `name_id` (1..n), `name_ord` (the observation at each dendrogram position) and `name_hgt` (the fusion height
  between a position and the next, missing for the last). Without `name()` the object is `_clus_#` and R prints
  `cluster name: _clus_#`.
- `cluster generate v = groups(k)` or `groups(k1/k2)` (variables `v<k>`), the most recent object unless `name()`: cut at the
  k - 1 largest heights, groups numbered along the dendrogram order. A partition object or an unknown one is r(198).
- `cluster kmeans|kmedians varlist, k(#) [start(firstk|lastk|krandom) name() measure()]`: Lloyd iterations, the nearest
  centre (first on ties), until nothing moves; the variable is labelled `Cluster ID`. `krandom` (the default) draws one
  `runiform()` per observation from the do-file's own stream (so after `set seed` it is Stata's) and takes the k smallest
  draws, the smallest first, as the centres. Both agree with Stata in the probe, start(krandom) included. The default
  dissimilarity of kmedians is L2, as of kmeans (observed).
- The objects live in the data's characteristics as Stata keeps them (`_dta[_cluster_objects]`, newest first, and per
  object `name`, `name_info`, `name_cmd`, `name_vars`). A hierarchical info is `t hierarchical`, `m <method>`,
  `d <dissimilarity>`, `o range 0 .`, `v id`, `v order`, `v height`; a partition's is `t partition`, `m kmeans`, `d L2`,
  `o k #`, `o start <start>`, `o range 0 .`, `v group name` (decoded here: StatCraft had the last two in another order).
- `cluster list [names]`, `cluster dir` and `cluster drop names|_all` print and act as Stata does.
- Not translated: `if`/`in`, weights, `cluster stop`, `dendrogram`, `measures`, `notes`, `rename`, `use`, similarity
  measures for binary data, `centroidlinkage`, `medianlinkage`, `waveragelinkage`, `start(prandom|segments|everykth|group())`.
  The storage type of the new variables (byte for 100 observations or fewer, then int) is not tracked by the translator, so
  `describe` after a cluster command does not know it.
