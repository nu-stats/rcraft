# complete/

One file per command the translator knows (`crates/statatr/src/complete.rs`),
gathered at build time by `crates/statatr/build.rs` into `COMMANDS` and the
tables behind Tab completion. The file name is the command: `areg.rs` is the
entry of `areg`. A file holds what Tab offers after the command's name:

```rust
Command {
    options: &["absorb(", "cluster(", "level(", "robust", "vce("],
    ..NONE
}
```

A command with subcommands (`graph bar`, `merge 1:1`) gives `subcommands: &[...]`
the same way. `..NONE` fills in whichever of the two is left out; when both are
given leave it out (clippy refuses it then). A command with neither is just
`NONE`. An option that takes an argument ends in `(`. A test checks that the
translator takes every option and subcommand listed.

A new command adds its file and edits nothing else.
