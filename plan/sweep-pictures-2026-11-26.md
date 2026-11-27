# Picture pass, 2026-11-26

With `rcraft-desktop --picture` (the real window drawn to a PNG, the user's R 4.6.2): Results with
a plot, the Graph tab, the Data tab, the do-file window run on a made-up do-file, the Packages
window (find, installed), the Session menu (sessions), and menu dialogs, at 1240x780, and
smaller. Dark mode once.

| Where | What the picture showed | Result |
|---|---|---|
| Results, Graph, Data tabs | plot drawn and fitted, table laid out, history, properties | fine |
| Session menu | stop, restart, working folder, sessions list, new, rename, close | fine |
| Packages window | tabs, search, installed list, filter, update, "where does a function come from" | fine at 1240 wide; at 820 wide it is cut by the window edge but usable |
| Do-file window | lines, status, buttons, run table | fine at 1240 wide; at 820 wide wider than the window: the close button and the Change... button are off the right edge (low) |
| Menu dialogs (Panel model, Bayesian regression) | no scroll: a long dialog grew past the window's bottom, Run, Cancel and Help unreachable | real bug, fixed in SW19 (the dialog scrolls and fits the window) |
| Data tab with `--data` and a plot in the demo | picture shows the Graph tab, not Data | picture tool only: a plot switches to Graph; not the app |

Not covered by a picture: the project window (no picture option for it), the data editor's
editing, drag and drop, Print. They need a hands-on pass.
