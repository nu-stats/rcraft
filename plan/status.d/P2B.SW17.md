## STATUS

## P2B.SW17: the Results tabs keep room, 2026-11-26

- In a short window with the Command panel dragged tall (900 x 420, panel 260) the Results area, tabs included, vanished. The Command and Script panels now have a largest height: the Command panel at most what is left after the Script panel's smallest (80) and 120 points for Results, the Script panel at most what is left after Results' 120. They give way, so the Results tabs and a few lines always show; at ordinary sizes nothing moves. Looked at with the picture tool (`--command-height`) at 1240x780, 900x520, 900x420 and 700x360; layout test in `crates/ui-egui/src/app.rs` over five window and panel heights.
