# Dialog specs

AGENTS.md section 3: every menu action is a catalog entry, and a menu item is a
declarative dialog spec. One spec gives the dialog and the R it runs. This is the
format, first used by the Data menu in P3.5; P4.0 extends it rather than
replacing it.

## Where things live

| Layer | Crate | Holds |
|---|---|---|
| L3 | `rcodegen::dialog` | the format: `Spec`, `Widget`, `Values`, `Context`, `Style`, `Problem`, the checks and `Args` (safe R pieces for code functions) |
| L4 | `catalog::data` (one module per menu area) | the specs themselves: widgets plus a `code` function and an optional `check` |
| L6 | `ui-egui::spec_dialog` | one renderer for every spec |

The interface never builds R. It draws the widgets, keeps the `Values`, and
shows what `Spec::generate` returns. Below L6 nothing knows about egui.

## A spec

```rust
pub const SORT: Spec = Spec {
    entry: "Sort rows",            // the catalog entry (ENTRIES) it belongs to
    menu: "Data > Sort rows",      // shown in History, Results and the log
    about: "Put the rows in order …",
    widgets: &[ /* Variable, Text, Choice, Check */ ],
    styles: &[Style::Base, Style::Tidyverse], // first is the default
    check: sort_check,             // rules beyond each widget's own
    code: sort_code,               // fn(&Args, Style) -> String
};
```

Widgets, each with an `id` the code function reads it by, a label and a hint:

- `Variable { kinds, required }`: one column of the data frame; `kinds` filters
  the list (`Number`, `Text`, `Factor`, `Logical`, `Date`, `Other`; empty for any).
- `Text { default, rule }`: `default` is empty, fixed text or the data frame's
  name; `rule` is `Any`, `Name` (quoted with backticks when R needs them) or
  `Expression` (inserted as typed).
- `Choice { options, default }`: a fixed list of (value, label).
- `Check { default }`.

Nearly every Data dialog ends with the shared `RESULT` text box: "Put the result
in", defaulting to the data frame's own name (which replaces it). A new name
keeps the original, and the window switches to the new data frame once R lists it.

## Code functions

A code function gets `Args` and a `Style` and returns the R, which runs only after
every check has passed. `Args` hands out names as R code (`a.var("by1")` →
`` `home town` ``), names as R strings (`a.var_string("by1")` →
`"home town"`), the data frame (`a.data()`), and the user's expressions as typed
(`a.expr("condition")`). Building R by joining raw names is not allowed: names
always go through `Args`, so quoting is done in one place (`r_name`, `r_string`).

An expression the user types is their own R, as it would be in the Command line.
`expression_problem` stops the cases that would make the generated call wrong or
misleading: unbalanced brackets (a stray `)` would close the call it sits in),
open quotes, `;`, comments and line breaks. R's parser has the last word when the
code runs.

## Styles

`Style::Base` uses base R only and works in any R. `Style::Tidyverse` calls
`dplyr::` functions by their full names, so the code needs no `library()` line and
still runs when copied into plain R with dplyr installed. `Style::packages()`
names what a style needs; the dialog says so next to the style choice.

## Tests

- Snapshot tests in `catalog` give the exact R for each spec in each style,
  including names that need backticks and results put in a new data frame.
- `engine/tests/data_dialogs.rs` runs each base-R spec on a small made-up data
  frame in a real R and checks the table R gives back.
- An app test opens a dialog through the menu action and checks the command goes
  into History as a menu command.

An entry stays `partial` until both styles are run in the pinned R: CI has no
dplyr yet, so the tidyverse code is snapshot-tested only.

## Not yet

- Multi-variable widgets (`Variables { min }`), numbers with bounds, and widgets
  shown only when another has a given value (P4.0).
- Checking an expression with R's own parser before Run (a hidden query).
- The missing-package panel from "Use my file" for the tidyverse style.
- Tests that run the tidyverse code (needs dplyr in CI's package step).
