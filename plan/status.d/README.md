# Status fragments

A feature or batch PR does not edit `ROADMAP.md` or `plan/STATUS.md`: every PR
that did conflicted with the next merge. It adds one file here instead,
`plan/status.d/<sub-id>.md` (for example `P2C.12at.md`; pick a sub-id no open or
merged PR uses). The file has the ROADMAP rows under a `## ROADMAP` heading and
the STATUS section under a `## STATUS` heading, each written as it would go into
that file. Either part may be left out.

```text
## ROADMAP

- P2C.12at What was built (done 2026-11-20)
  (the rows)

## STATUS

## P2C.12at: What was built, 2026-11-20

- the section
```

A periodic bookkeeping PR runs `cargo xtask status-fold`, which puts each
fragment's rows at the end of its phase in `ROADMAP.md` (the phase is the part
of the sub-id between `P` and the first `.`, so `P2C.12at` goes in `## Phase 2C`),
adds its section at the end of `plan/STATUS.md`, and deletes the fragment, in
file-name order. `cargo xtask status-fold --check` (run by `cargo xtask ci`)
only checks that the fragments are well formed. This file stays.
