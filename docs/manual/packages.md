# Packages

Many dialogs use a package from CRAN, R's library of add-on packages. A dialog that needs one you do not have tells you which, and offers to install it. Nothing is installed, updated or removed without your clear yes, and RCraft makes no network call until you ask for one: a search, a description, an install or an update.

Open the window from the Packages menu. Its tabs are these.

## Find on CRAN

Type a word and press Find: the matches are listed with their titles. Pick one for its description and version, and press **Install** to see what will be installed, from where, and which version, before you say yes.

## Task views

CRAN's task views are lists of packages by field, such as econometrics or survival analysis. Browse a field, then search inside it.

## Installed

The packages installed here, with their versions. You can look for newer versions and update one, or all that are out of date, and remove a package. Each step asks first. Packages go into RCraft's own library, kept apart for each version of R.

## Other sources

Bioconductor, r-universe, GitHub and files on your own computer. These are labelled as not reviewed: installing code can run anything, so read what the install list says.

## Project list

The list of packages a project uses. **Save list** writes it to a file, **Open list** reads one, and **Install what is missing** installs what this computer lacks, after you say yes. [Save project](projects-and-reporting) keeps the list with the project, and Open project loads it here.

## From another R

If you installed packages for an earlier version of R, this tab lists them so you can install them again for the version you now have.

## The same from the Command line

`rc search`, `rc install` and the other `rc` commands do the same as this window; see [the Command window](command-window).
