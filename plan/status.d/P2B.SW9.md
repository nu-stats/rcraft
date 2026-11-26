## STATUS

## P2B.SW9: the install panel shows the version CRAN lists, 2026-11-26

- The dialog's "needs the R package ..." install panel said only "the newest release built for your R". It now adds the version CRAN's newest is listed as, read from the package list RCraft already keeps on disk (loaded from disk once the dialog opens, no network call, nothing in the background), for each package to install, and says a release built for an older R can be one behind. With no saved list it says what it said before.
- Not done: the exact version R will install is only known when R asks the mirror, which RCraft does only when the user presses Install, so it cannot be shown before.
