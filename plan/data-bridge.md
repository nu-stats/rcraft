# Data bridge spike: moving a data frame from R to RCraft

Done 2026-10-26, before Phase 3 (Data window, Variables pane, Data editor).
Question: how does a data frame in R's memory reach RCraft's tables?

## What was measured

1,000,000 rows, 7 columns: integer, double, double with 10% missing, factor,
text (2% with commas, quotes and line breaks; 2% missing), logical, Date, and
one column carrying a `label` attribute. R 4.4.3 on Windows, base R only.
Scratch scripts (not in the repository) were `bridge.R` (writer) and
`read.rs` (a Rust reader for each format).

| Format | R write | Size | Rust read | Types kept | Missing vs text "NA" | Labels |
|---|---|---|---|---|---|---|
| `write.csv` (base) | 7.4 s | 82 MB | 1.1 s | no (all text) | ambiguous | lost |
| RCraft columnar, base R only | 0.38 s | 54 MB | 0.09 s | yes | exact | needs an extension |

Both readers matched R's own sums, missing counts and quote counts.

## Not measured, and why

- **Arrow IPC and Parquet** need the `arrow` package, which is not installed
  here and is a large compiled package. Installing it goes into RCraft's own
  library and needs the user's yes (network rule, AGENTS.md section 1).
- **`data.table::fwrite`** is installed but Windows Application Control blocked
  loading `data_table.dll`. This is a finding: on a locked-down machine any
  compiled R package, `arrow` included, may fail to load. The bridge must not
  depend on one.

## Findings

1. Delimited text is the wrong bridge. It loses types (integer, factor, Date,
   logical all arrive as text), cannot tell a missing value from the text
   "NA", drops attributes such as variable labels, and is 20 times slower to
   write. It stays for the user's own CSV import and export.
2. A small binary column format written by base R is fast (the write is
   mostly `writeBin` on whole vectors) and needs no package, so it works with
   any R RCraft finds, including a locked-down one.
3. Reading it in Rust is a handful of slice copies; no parsing, no guessing.
4. The framed protocol (`rproto`) is for small messages. Tables go through a
   file in the session's own temporary folder; a frame carries the path. The
   file is deleted after reading, and on session end.

## Recommendation

- **Primary bridge: "RCraft columnar" (`RCB1`)**, versioned, little-endian,
  writer in `bootstrap.R` (base R only), reader in `rdata` (L2). Kinds so far:
  integer, double, logical, factor (levels plus codes), Date, text.
  Missing: R's own integer and double missing patterns, length -1 for text.
- **Extensions needed before Phase 3 uses it**: per-column attributes (label,
  value labels, units) as a key/value block; POSIXct and difftime; list
  columns (marked, shown as text, never guessed); a row-count and size cap;
  a checksum so a truncated file is refused, not half-read.
- **Optional fast path later**: Arrow IPC when `arrow` is present and loads.
  It is an optimisation behind the same Rust trait, never a requirement.
- **Parquet**: for the user's own import and export (File menu), not the
  bridge.
- **Reverse direction** (RCraft edits, filters, sorts, then back to R) is not
  measured; plan to send only the changed columns or an index vector, and
  generate R code for every change so the log stays the product.

## Open

- Measure `arrow` IPC and Parquet once the user agrees to install it into
  RCraft's library (and see whether it loads under Application Control).
- Measure on Linux and macOS (CI could run a small version of this).
- Big-endian machines: refused with a message; none of the supported
  platforms is big-endian.
