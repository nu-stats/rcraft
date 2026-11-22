# differences/

One file per page of the differences guide (`crates/statatr/src/differences.rs`),
gathered at build time by `crates/statatr/build.rs` into `GUIDES`. The file name
is the page's command: `areg.rs` holds the page whose `command` is `"areg"`
(a test checks it). A new command adds its file here and edits nothing else.

- `<command>.rs` is one `Guide { command, about, points }` expression. It may use
  `p(...)` and the shared points (`ML_SE`, `LAYOUT_R`, ...) defined in
  `differences.rs`. The files are not compiled on their own and `rustfmt` does
  not touch them, so keep the layout of the neighbours.
- `general.rs` is the page for what holds everywhere; it is always first.
- `<page>.aliases.rs` is a list of command names that use another command's page
  instead of having their own, for example `logit.aliases.rs` holds
  `&["probit"]`. A name is either an alias or a page, not both.
