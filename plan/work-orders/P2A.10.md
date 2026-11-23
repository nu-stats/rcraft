# P2A.10 RCraft's own PDF manuals (open → done) · 10 h · decision 5 (a font download)

**Goal:** as the roadmap says, RCraft lays out its own PDFs, with no LaTeX and no other program.

**Build:**
- **The writer:** `crates/report/src/pdf.rs`, ported from StatCraft's `crates/report/src/put.rs` (the owner's code), which has pages, paragraphs and tables. It adds:
  - **an embedded TrueType font:** one permissively licensed family (Noto Sans and Noto Sans Mono, OFL), the full file embedded, listed in `ATTRIBUTION.md`;
  - **a contents page** with page numbers, and **a topic index**;
  - **code blocks** in the mono font.
- **What it prints:**
  - **the RCraft manual**, from P2A.6's pages;
  - **a package's reference manual**, from R's own help text for each topic (as the Help window shows it), with front matter giving the RCraft, R and package versions. Each function's entry names the RCraft dialogs that write it (from `dialog_help`).
- **Save as PDF buttons:** on the Help window's package page and on the manual.

**Test:**
- `fonts_are_embedded` and `a_package_manual_has_contents_and_index`, in `pdf.rs`;
- `the_rcraft_manual_saves_as_pdf`, in `crates/engine/tests/manual_pdf.rs`: the PDF parses, every font is embedded, and the page count is checked.

**Done:** the three tests are live.

**Decision 5:** downloading the Noto font files (OFL) needs the owner's yes.
