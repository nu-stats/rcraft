# expandcl (2026-11-18)

StatCraft's note (`plan/stata/expandcl.md` there, probe 552) was the spec; RCraft's corpus `expandcl.do`
re-checks it on Stata 19 SE (158 results, exact) and the printed notes were read against Stata's log.
Translated in `crates/statatr/src/data_r/rowcol.rs` (`stata_expandcl`).

- `expandcl [=]exp [if] [in], cluster(varlist) generate(newvar)`; both options required (`option
  generate() required`, then `option cluster() required`, r(198)); no expression r(198); a string
  expression r(109); `newvar` already there r(110); an expression that is not the same for every selected
  observation of a cluster `expression is not constant within clusters` r(198); a cluster variable the
  data lack r(111).
- Each selected observation becomes `exp` copies; a missing, zero or negative count keeps the one
  observation and a fraction is rounded half up (2.5 and 2.7 give 3). Notes, in this order and counted
  per observation: `(2 missing counts ignored; observations not deleted)`, `(1 noninteger count rounded
  to integer)`, `(1 negative count ignored; observation not deleted)`, `(1 zero count ignored;
  observation not deleted)`, then `(N observations created)` (also `(0 observations created)`).
- `newvar` ("New cluster ID from expandcl") numbers the (cluster, copy) pairs: the clusters in the sort
  order of the cluster variables (missing last, a missing value a cluster of its own, `.` before `.a`),
  a cluster with k copies taking k numbers in a row, the j-th copy of each observation the j-th number;
  observations outside `if`/`in` are 1. Clusters are made of the selected observations only.
- **Row order:** Stata keeps each observation's copies next to it, but in an order of its own (a probe
  showed numbers 4,3 then 1,2 then 5,6 then 6,5: it is not stable, nor sorted). RCraft puts the copies in
  the order of their numbers; a do-file that depends on the order of the copies would differ. The corpus
  sorts by `id` and `newvar` before it looks.
- `by:`: r(190), see clonevar.md.
