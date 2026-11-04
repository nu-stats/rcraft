# Attribution

Every file under `assets/` needs a row here. `cargo xtask assets` fails the
build when one is missing.

Assets must be original work, public domain or CC0, or openly licensed for
redistribution. Prefer art drawn in code over image files.

| File | Author | Source | Licence |
|---|---|---|---|

## Rust libraries for compressed data files

Not assets, but recorded here with their licences, which were checked by hand
(`Cargo.toml` of each, 2026-11-04). All are pure Rust; no C library is built.

| Crate | Used for | Licence |
|---|---|---|
| `flate2` with `miniz_oxide` | gzip and zip deflate (already used by `rpkg`) | MIT OR Apache-2.0 (miniz_oxide: MIT OR Zlib OR Apache-2.0) |
| `lzma-rust2` 0.25 | xz, with its CRC-32, CRC-64 and SHA-256 checks | Apache-2.0 |
| `sha2`, `digest`, `block-buffer`, `crypto-common`, `hybrid-array`, `typenum`, `const-oid`, `cpufeatures` | the xz SHA-256 check (through `lzma-rust2`) | MIT OR Apache-2.0 |
| `ruzstd` 0.8 | Zstandard | MIT |
| `twox-hash` | the Zstandard checksum (through `ruzstd`) | MIT |

The bzip2 decoder (`crates/rdata/src/unpack/bunzip.rs`) is ported from Husky
Explorer's own JavaScript one (MIT OR Apache-2.0, the same project family).
The test files in `crates/rdata/fixtures/unpack/` come from Husky Explorer's
tests, made from made-up text with xz 5.8.3, zstd 1.5.7 and bzip2 1.0.8.
