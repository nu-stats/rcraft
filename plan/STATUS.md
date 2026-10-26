# Status

Last updated: 2026-10-26.

**Phase 0 (setup): scaffolded, not yet committed.** The Cargo workspace,
layered crates, licence files, `NOTICE`, `cargo xtask ci` and the CI workflow
exist on branch `p0/scaffold`.

## Done

- P0.1 Workspace and layered crates (L0 to L7, empty except `core`,
  `catalog` and `rcraft-cli`)
- P0.2 Contributor rules, MIT and Apache-2.0 licences, `NOTICE`
- P0.3 `cargo xtask ci`: fmt, clippy, tests, layers, assets, coverage
- P0.4 Coverage inventory (`crates/catalog`, `docs/coverage.md`)
- P0.6 CI workflow for Windows, macOS and Linux (not yet run: no remote)

## Not done

- P0.5 `ENVIRONMENT` still holds placeholders; pin R and the package
  snapshot once R is installed (R is not installed on this machine).
- The `xtask` crate's own unit test (in `layers.rs`) is blocked on this
  machine by a Windows Application Control policy (os error 4551), so
  `cargo xtask ci` stops at the test step here. Every other check passes
  when run on its own. It should run on CI.

## Decisions made

- Architecture: RCraft **drives a real R** as a separate child process. It
  does not reimplement R and never links `libR` (keeps the licence MIT OR
  Apache-2.0).
- Layered Rust crates and the catalog-of-dialog-specs model follow StatCraft.
- Generated R code is the product: every menu action shows and logs R.

- Added 2026-10-26: Phase 1A (Packages: search, describe, install, update;
  the `ssc install` and `net search` equivalent, with `rc install` and `rc
  search` command forms) and Phase 2A (Help: R help pages, help search,
  vignettes, per-dialog help). Network rules are in AGENTS.md section 1 and
  `docs/network.md`.

- Added 2026-10-26: Phase 2B (Stata mode): a switch and button that take
  Stata code, translate it to R, show both side by side and run the R. A
  translator, not Stata; unsupported commands are marked, never guessed.
  Clean-room rules in AGENTS.md section 1; new `statatr` crate planned at L3.
- P1.1 (find installed R) is done and committed; R is not installed on this
  machine, so `rcraft --find-r` reports none.

- Researched 2026-10-26: run an existing do-file on R (`plan/do-file-run.md`,
  Phase 2C). No mature tool exists; StatCraft's parsers are reusable.
  Recommended approach: idiomatic R plus a small shipped R runtime package
  for Stata-specific rules; verified against recorded Stata output.

## Decided 2026-10-26 (open items worked)

- P0.5: `ENVIRONMENT` pins R 4.4.3 and a package snapshot dated 2025-03-15;
  CI installs that R with `r-lib/actions/setup-r`. Neither the snapshot date
  nor the CI step has run yet (no remote), so the first CI run confirms them.
- Pinokio's conda installs are found with no setting: Pinokio's home comes
  from `PINOKIO_HOME` or `~/.pinokio/config.json`, then `bin/miniforge` and
  `bin/miniconda`. `rcraft --find-r` now reports R 4.4.3 on this machine.
- Do not install packages into another app's environment. The R found here
  belongs to Husky Eye View (`hev-r`, 62 packages, no tidyverse). RCraft uses
  its own user library for installs (P1.5, Phase 1A).
- Shared parsers: extract StatCraft's do-file reader, syntax and expression
  parsers into a shared crate when P2C.3 starts, and depend on that; do not
  copy them here. That change is made in the StatCraft repository, so it is
  not done from this one and needs the maintainer's go-ahead.
- `rcraftstata` (the shipped R runtime package) lives in this repository
  under `r/rcraftstata/` and is installed into RCraft's own user library on
  first use, with the user's yes. Created when P2C.4 starts.

## Open questions

- None blocking. Next: P1.2 (download a portable R when none is found),
  P1.3 (the R session process).

- Stata mode: share StatCraft's parser (`lang` crate) or write a new one?
  Proposed: start from StatCraft's if its licence and layering allow.
- Stata mode output: R's own output first; Stata-style tables later.

- Phase 1A providers: CRAN and Bioconductor first; r-universe and GitHub
  after? Download counts from a third-party service are optional.

- Generated code style default: tidyverse with a base-R toggle (proposed).
- Public name check for "RCraft"; the repository URL in `Cargo.toml` is a
  placeholder.
- Portable R source for first-run download on each platform.
- First milestone: Phases 0 to 3 as a usable "open a file, look at it,
  clean it, run R commands" app, before any statistics menus.

## Next tasks

- Commit the scaffold (P0.1 to P0.4, P0.6)
- P0.5 pin R and the package snapshot
- P1.1 find an installed R
