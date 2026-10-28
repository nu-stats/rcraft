# Network use

RCraft contacts these services only when the user asks, and sends only the
search text or package names (AGENTS.md section 1). Adding a host is its own
commit and updates this file. Hosts and endpoints below are the plan; each is
confirmed against the service's public documentation when its task is built.

| Purpose | Service | Task |
|---|---|---|
| CRAN package index, task views, binaries | CRAN mirror chosen by the user (default the cloud mirror): `src/contrib/PACKAGES.gz`, `web/packages/packages.rds` and `src/contrib/Views.rds` (the task views, about 70 KB) for the index (all exist, checked 2026-10-28) | P1A.1, P1A.3, P1A.5, P3.2 (the "Install" button in Use my file, for readxl and haven) |
| Bioconductor packages | Bioconductor repositories | P1A.5 |
| r-universe packages | `<universe>.r-universe.dev` | P1A.2, P1A.5 |
| GitHub installs | github.com, only for an `owner/repo` the user typed | P1A.5 |
| Download counts (optional) | public CRAN download-logs service | P1A.3 |
| Portable R download | the R project's download site | P1.2 |

Help (Phase 2A) works offline from installed packages. Online links open in
the user's browser only on click.
