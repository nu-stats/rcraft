# RCraft

RCraft is an open-source desktop app that gives R a complete point-and-click
front end: menus for data management, statistics and graphs, a command
window, a data editor, results, and reports. It drives a real R
installation, and every menu action shows the R code it ran, so you can move
from clicking to scripting at any time.

> **Status: Phase 0 (setup).** Nothing runs yet. See [ROADMAP.md](ROADMAP.md)
> and [plan/STATUS.md](plan/STATUS.md).

## How it is built

- Rust app (egui), in layered crates, on Windows, macOS and Linux.
- R runs as a separate process, never linked into the app.
- Every menu item is a declarative dialog spec; one spec renders the dialog
  and generates the R code.
- Rules for contributors and agents are in [AGENTS.md](AGENTS.md).

## Licence

Dual-licensed under MIT or Apache-2.0, at your option (licence files to be
added in P0.2). RCraft is an independent project, not affiliated with the R
Foundation or Posit.
