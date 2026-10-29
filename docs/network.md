# Network use

RCraft contacts these services only when the user asks, and sends only the
search text or package names (AGENTS.md section 1). Adding a host is its own
commit and updates this file. Hosts and endpoints below are the plan; each is
confirmed against the service's public documentation when its task is built.

| Purpose | Service | Task |
|---|---|---|
| CRAN package index, task views, binaries | CRAN mirror chosen by the user (default the cloud mirror): `src/contrib/PACKAGES.gz`, `web/packages/packages.rds` and `src/contrib/Views.rds` (the task views, about 70 KB) for the index (all exist, checked 2026-10-28) | P1A.1, P1A.3, P1A.5, P3.2 (the "Install" button in Use my file, for readxl and haven) |
| Bioconductor packages | `bioconductor.org`: `config.yaml` (which release is for this R) and `packages/<release>/bioc/VIEWS` (the list), when the user looks Bioconductor up; then BiocManager's own requests during an install the user said yes to | P1A providers |
| r-universe packages | `<universe>.r-universe.dev`, only for a universe the user typed: `src/contrib/PACKAGES.gz`, then R's install from it | P1A providers |
| GitHub installs | only for an `owner/repo` the user typed: `raw.githubusercontent.com` (its DESCRIPTION), then, after the yes, `github.com/<owner>/<repo>/archive/<ref>.tar.gz` (which redirects to `codeload.github.com`), fetched by R | P1A providers |
| A package's reference manual | only when the user clicks **Reference manual (PDF)** on a package page: the user's browser opens `<mirror>/web/packages/<package>/<package>.pdf` (for a package that comes with R, `<mirror>/doc/manuals/r-release/fullrefman.pdf`) on the mirror chosen in the Packages window. RCraft itself sends nothing (checked 2026-10-29: both exist) | P2A.4 |
| Download counts (optional) | public CRAN download-logs service | P1A.3 |
| Portable R download | the R project's download site | P1.2 |

Help (Phase 2A) works offline from installed packages. Online links open in
the user's browser only on click.
