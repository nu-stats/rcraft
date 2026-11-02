# xtile (P2C.9)

Observed on a licensed Stata 19 SE with made-up data, 2026-11-02. Only what Stata did is written here.

- `xtile q4 = x, nquantiles(4)` prints nothing and makes a byte variable labelled `4 quantiles of x`.
  `nq(3)` is the same option. Without it there are 2 groups.
- Each value's group is 1 plus the number of cut points below it: with x from 0 to 10 and one missing,
  `nq(4)` gives 1 to 4 on the 29 rows that have x, summing to 68; `nq(3)` sums to 56; two groups to 43.
- The cut points are Stata's percentiles. `altdef` gives other ones (the sum is 71 for `nq(4)`).
- When ties make percentiles equal, each still counts: a variable that is 1 on 27 rows and 2 on 3 gives
  1 and 4 with `nq(4)`.
- `if g == 1` works the percentiles out on the rows kept and leaves the others missing (14 rows); so does
  `in 3/20` (17 rows with x).
- `cutpoints(c)` uses every value of c, in any order and repeated (3 7 5 5 gives groups 1 to 5), and
  whatever `if` keeps; the label is `x categorized by c`.
- An expression works: `xtile qe = x * 2 + 1, nq(5)`, labelled `5 quantiles of x * 2 + 1`.
- A variable that exists already: r(110). `nq(1)`: r(198).
