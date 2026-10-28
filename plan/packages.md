# Phase 1A plan: Packages

Drafted 2026-10-28. Decisions already made are marked **decided**; the rest
are open and listed at the end.

## Decided

- **The index is fetched and searched in Rust** (not through an R session),
  so search works offline and with no R running.
- CRAN only in the first pass. Bioconductor, r-universe, GitHub and Local
  come after P1A.11, each with its own trust label (ROADMAP "Limits").
- Install keeps running through R (`install.packages`), supervised, with
  the exact call shown and logged. Only the *index* is Rust-side.

## What CRAN serves (checked 2026-10-28 with HEAD requests, nothing downloaded)

| File | Size | Holds |
|---|---|---|
| `src/contrib/PACKAGES.gz` | 2.0 MB | DCF text: name, version, Depends, Imports, LinkingTo, Suggests, License, NeedsCompilation. No titles. |
| `web/packages/packages.rds` | 12.7 MB | R's own package database: the above plus Title, Description, Author, Maintainer, Published, URL, SystemRequirements, and so on. |
| `web/views/` | 10 KB | Task View list (HTML); each view has its own page and a machine-readable `.ctv` file. |

Titles and descriptions exist only in `packages.rds`, so search needs an RDS
reader in Rust. `PACKAGES.gz` is small and plain text: use it for the
install plan and as the fallback when the big file cannot be read.

## New pieces

1. **Crate `rcraft-rpkg`, layer L2**, depends on `core` only. Add it to the
   layer table in `AGENTS.md`. No UI, no R process.
2. **An HTTP client.** The workspace has none today. Candidate: `ureq` with
   rustls (MIT or Apache-2.0). Needs a licence check of its tree (the
   crypto library underneath) before it goes in. One dependency commit by
   itself.
3. **`docs/network.md`**: the CRAN row is already there; confirm the exact
   paths above in the same commit as the first fetch.
4. **An RDS reader** (gzip, then XDR): only what the package database uses,
   which is a data frame of character columns. Anything else it meets is an
   error, not a guess. Hostile input rules apply (caps on lengths and
   depth). `flate2` is already in the lock file through the image crates.

## P1A.1 in steps (Opus)

1. `rds` module and its tests, on tiny RDS files made by R 4.4.3 (stored in
   the repo as small fixtures; made-up packages, so no redistribution issue).
2. `dcf` module for `PACKAGES.gz`, same kind of tests.
3. `index` module: `Package` record, in-memory table, load from cache,
   merge of the two sources, staleness (age of the cache file, a clock
   passed in so tests don't depend on time).
4. `cache` module: where the file lives (the RCraft data folder next to the
   library), written to a temp name and renamed so a half download never
   replaces a good cache. A refresh that fails keeps the old cache.
5. `fetch` module: the only code that touches the network, behind a small
   trait so every other test uses a local stand-in. Runs only when the user
   asks to refresh. Reports size and progress; cancellable.
6. A tiny recorded index built in code for later steps (P1A.11 asks for it).

Done when: with the network off, a cached index loads and answers "which
packages are called X" and "list all with this prefix"; a corrupt cache and
a truncated download are errors that leave the old cache in place; CI needs
no network.

## Then

| Step | Depends on | Model |
|---|---|---|
| P1A.5 install flow (plan, confirm, supervised run, cancel) | P1A.1 | Opus |
| P1A.7 readable failures | P1A.5 | Opus |
| P1A.8 lockfile | P1A.5 | Opus |
| P1A.2 ranked search | P1A.1 | Sonnet |
| P1A.3, P1A.4 browse and describe | P1A.1 | Sonnet |
| P1A.6 installed packages | none (asks R) | Sonnet |
| P1A.9, P1A.10 `rc` commands, package-needed prompt | P1A.2, P1A.5 | Sonnet |

P1A.6 can start any time and doesn't need the index.

## Open

1. **OK to add `ureq` (rustls)** as the first network dependency, after the
   licence check? The alternative is shelling out to R or `curl` for the
   download only, which avoids the dependency but needs R or curl present.
2. **Cache and size.** The full database is 12.7 MB to download. Keep the
   raw file, or store a trimmed copy (the columns RCraft uses) to keep
   search start-up fast? Recommended: trimmed copy, with the raw file
   deleted after a successful parse.
3. **Which mirror.** `docs/network.md` says "chosen by the user, default the
   cloud mirror". Is a setting for that part of P1A.1, or later?
