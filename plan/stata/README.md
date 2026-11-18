# Behaviour notes (Stata mode)

One file per command or area, written in our own words from public documentation and from
black-box runs of a licensed Stata 19 on made-up data (`AGENTS.md`, section 1). These notes are
how Stata's behaviour reaches the translator; the values themselves are checked in the corpus
(`crates/engine/tests/stata_corpus`).

Each note records:

- **Rule:** what Stata does, stated plainly.
- **Source:** a manual entry (by name and section), a probe file, or a corpus file, with the Stata
  version and edition it ran on.
- **Cases:** the inputs tried and what came back, as short values or one-line messages.
- **Open:** anything not yet pinned down, and what RCraft refuses.

Two optional sections follow Cases and Open, for notes shared with StatCraft (our own Stata clone,
a separate private project):

- **R cross-check:** the R function or package that agrees with Stata or not, and where it
  differs. Behaviour and documentation only, never R package source.
- **Port hints:** the arithmetic as plain, language-neutral steps, for the other project to
  implement from.

A note about an SSC package says in its header line how it was learned: "from output only" (the
package's behaviour seen in runs) or "ado read while debugging" (the program file was read to find
a bug in our own code). RCraft's rule is to observe only and never read an `.ado` file
(`AGENTS.md`, section 1); the second wording is for notes ported from StatCraft, which may say
so, and it is never to be written in an RCraft-decoded note.

A note ported from StatCraft's private repository is written in our own words and cites its
source as "ported from statcraft@<sha>" (`AGENTS.md`, section 1). Never paste whole logs, Stata
output or StatCraft's text here; short quotes of single messages are fine where the wording itself
has to match.

The shared command table `C:\pinokio\api\stata-matrix.csv` (made by `cargo xtask stata-matrix`,
outside the repository, not committed) is the status board, and the lanes ledger
`C:\pinokio\api\stata-lanes.md` the claim board. The table lists, per command, where each project
stands and which notes cover it; read it, and the other project's note for the command, before
decoding anything.

Files in this folder are named after the command (`clonevar.md`) or the area (`esttab-options.md`).
