# Package providers beyond CRAN (Phase 1A)

Written 2026-10-28, before building. CRAN is done (index, search, install, failures,
lists, `rc`). This adds the other four sources the roadmap names, each with its own
trust label, and keeps the rule that nothing is installed without a clear yes to a
list that shows provider, source and version (AGENTS.md section 1).

## The four providers

| Provider | What the user gives | How RCraft learns what it is | How R installs it | Reviewed by |
|---|---|---|---|---|
| Bioconductor | a package name | Bioconductor's own list for this R (`config.yaml` names the release for each R; `packages/<release>/bioc/VIEWS` has names, versions, titles, needs) | `BiocManager::install()`, after installing BiocManager from CRAN if it is missing | the Bioconductor project |
| r-universe | a universe (`owner`) and a package name | the universe's `src/contrib/PACKAGES.gz` (the same reader as CRAN's) | `install.packages(pkg, repos = c(universe, CRAN mirror))` | nobody: built from the owner's repositories |
| GitHub | `owner/repo`, optionally `@ref` | its `DESCRIPTION`, fetched from `raw.githubusercontent.com` | the CRAN packages it needs first (planned with the CRAN list), then the archive from `github.com/<owner>/<repo>/archive/<ref>.tar.gz`, installed from source | nobody |
| Local file | a `.tar.gz`, `.zip`, `.tgz` or a folder | R reads the `DESCRIPTION` inside it (no network) | `install.packages(path, repos = NULL, type = ...)`, after the CRAN packages it needs | nobody |

Only base R is used for GitHub and local files (no `remotes`), so the code shown is
plain R anyone can run. Bioconductor uses BiocManager because the roadmap names it and
because it picks Bioconductor's repositories for the R in use.

## Trust labels and the yes

Every confirmation shows the provider with a one-line label:

- **CRAN**: checked by CRAN before each release.
- **Bioconductor**: reviewed by the Bioconductor project. Installed with BiocManager, which
  comes from CRAN.
- **r-universe**: built from the owner's own repositories by r-universe. Not reviewed by CRAN.
- **GitHub**: unreviewed source code from *owner/repo* as it is now. Anyone who can change that
  repository can change what you install. Built on this computer.
- **Local file**: unreviewed code from a file on this computer.

For the three unreviewed providers the yes needs one more tick ("I understand this code has
not been reviewed"), and the button stays off until it is ticked. A GitHub install names the
reference it will use; "HEAD" (the default branch now) is said to change over time, and a
commit is suggested for something repeatable.

## Network

New addresses, each asked only when the user acts, and each named in the window before it is
used: `bioconductor.org` (the release list and a release's VIEWS, then BiocManager's own
requests), `<owner>.r-universe.dev`, `raw.githubusercontent.com` (one DESCRIPTION) and
`github.com` (the archive, which redirects to `codeload.github.com`). `docs/network.md` gets
these in a commit of its own. The fetches go through the same `Fetcher` (HTTPS only, capped,
cancellable); R's own downloads during an install are R's.

## Building it, in order

1. `rpkg::provider`: the `Provider` type, its label, name checks (universe and GitHub owner
   names, repository names, references, local paths), and the Bioconductor release choice
   from `config.yaml`.
2. Local files: R code that reads the DESCRIPTION inside a file or folder (no network), the
   install code by file kind, and the plan of CRAN packages it needs. Tested in real R with
   files built on the spot.
3. GitHub: the DESCRIPTION fetch, the install code (deps from CRAN, then the archive),
   tested against a local stand-in for the network.
4. r-universe and Bioconductor: their lists through the existing readers, cached per
   source, and their install code.
5. The window: an "Other sources" tab (pick a provider, type what it needs, look it up, see
   what it is and its label, say yes), and `rc install` forms: `owner/repo, provider(github)`,
   `PKG, provider(bioc)`, `PKG, provider(universe:OWNER)` and a path.
6. Docs, tests, CI.

## Limits stated now

- GitHub packages in a subfolder of a repository, or that need building steps beyond
  `R CMD INSTALL` (vignettes, `configure` that needs tools), are not handled; R's own
  error then goes through the failure banner.
- A Bioconductor annotation or experiment data package is found by BiocManager but not by
  RCraft's search, which reads the software list only.
- Package lists (P1A.8) record these packages as `other` and do not restore them.
