# verified/

One file per Verified command (`crates/statatr/src/verified.rs`), gathered at
build time by `crates/statatr/build.rs` into `VERIFIED`, sorted by command name.
The file name is the command: `areg.rs` is the entry of `areg`, and holds the
corpus files that check it, a slice of the names in `engine/tests/stata_corpus`
(without `.do`):

```rust
&["areg", "contrast_areg"]
```

`functions/` does the same for `VERIFIED_FUNCTIONS`, one file per function. Write
each capital letter of a function's name as `_` and the small letter (`Clock` is
`_clock.rs`, `clock` is `clock.rs`) so two names never differ only in case on a
case-insensitive file system.

A new command or function adds its file and edits nothing else.
