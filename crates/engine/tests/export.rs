//! Graphs > Export plot (P5.4) end to end against a real R (skipped with a note
//! when none is found): draw plots, export them as PNG, SVG and PDF with the
//! dialog's code, and look at the files themselves for the size they were asked
//! to have.

use std::path::PathBuf;
use std::time::{Duration, Instant};

use rcraft_catalog::plot_export::EXPORT_PLOT;
use rcraft_engine::{Engine, Item, Outcome, Source};
use rcraft_rcodegen::dialog::{Context, Style};
use rcraft_rhost::{find_installs, Env, SessionConfig};

fn engine() -> Option<Engine> {
    let install = find_installs(&Env::from_process())
        .into_iter()
        .find(|i| i.is_supported());
    let Some(install) = install else {
        eprintln!("skipped: no supported R on this machine");
        return None;
    };
    Engine::start(SessionConfig::new(install)).ok()
}

fn settle(e: &mut Engine) {
    let limit = Instant::now() + Duration::from_secs(120);
    loop {
        e.poll(Duration::from_millis(50));
        if !e.is_working() {
            break;
        }
        assert!(Instant::now() < limit, "the engine did not settle");
    }
}

/// Runs code; returns how it ended, the plots it drew and its messages.
fn run(e: &mut Engine, source: Source, code: &str) -> (Outcome, Vec<PathBuf>, String) {
    let id = e.submit(source, code);
    assert!(id.is_ok(), "{id:?}");
    let id = id.unwrap_or_default();
    settle(e);
    let outcome = e
        .history()
        .entries()
        .last()
        .and_then(|h| h.outcome)
        .unwrap_or(Outcome::Error);
    let mut plots = Vec::new();
    let mut text = String::new();
    for block in e.results().blocks().iter().filter(|b| b.command == id) {
        for item in &block.items {
            match item {
                Item::Plot { path } => plots.push(path.clone()),
                Item::Text { text: t, .. } => text.push_str(t),
                Item::Error { message, .. } => text.push_str(&format!("error: {message}\n")),
                Item::Warning(w) => text.push_str(&format!(
                    "warning: {w}
"
                )),
                _ => {}
            }
        }
    }
    (outcome, plots, text)
}

/// A folder of its own for one test's files, removed when it goes out of scope.
struct Folder(PathBuf);

impl Folder {
    fn new(name: &str) -> Folder {
        let dir = std::env::temp_dir().join(format!("rcraft-export-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        assert!(std::fs::create_dir_all(&dir).is_ok());
        Folder(dir)
    }

    fn file(&self, name: &str) -> String {
        self.0.join(name).to_string_lossy().replace('\\', "/")
    }

    fn bytes(&self, name: &str) -> Vec<u8> {
        std::fs::read(self.0.join(name)).unwrap_or_default()
    }
}

impl Drop for Folder {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// Runs the Export plot dialog with these values.
fn export(e: &mut Engine, set: &[(&str, &str)], replace: bool) -> (Outcome, Vec<PathBuf>, String) {
    let ctx = Context {
        dataset: "(plot)".into(),
        frames: Vec::new(),
        columns: Vec::new(),
    };
    let mut values = EXPORT_PLOT.defaults(&ctx);
    for (id, text) in set {
        values.set_text(id, *text);
    }
    values.set_flag("replace", replace);
    let code = EXPORT_PLOT.generate(&ctx, &values, Style::Base);
    assert!(code.is_ok(), "{code:?}");
    run(
        e,
        Source::Menu(EXPORT_PLOT.menu.into()),
        &code.unwrap_or_default(),
    )
}

/// Whether this R can load cairo, which SVG needs (a Mac without XQuartz cannot).
fn has_cairo(e: &mut Engine) -> bool {
    let (_, _, text) = run(e, Source::Typed, "cat(isTRUE(capabilities('cairo')))");
    text.trim() == "TRUE"
}

/// A PNG's width and height from its header.
fn png_size(bytes: &[u8]) -> Option<(u32, u32)> {
    let be = |at: usize| -> Option<u32> {
        let four: [u8; 4] = bytes.get(at..at + 4)?.try_into().ok()?;
        Some(u32::from_be_bytes(four))
    };
    if bytes.get(..8)? != [0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a] {
        return None;
    }
    Some((be(16)?, be(20)?))
}

fn contains(bytes: &[u8], needle: &str) -> bool {
    String::from_utf8_lossy(bytes).contains(needle)
}

#[test]
fn a_plot_is_drawn_again_at_the_size_asked_for_in_each_format() {
    let Some(mut e) = engine() else { return };
    let dir = Folder::new("sizes");
    let (outcome, drawn, text) = run(&mut e, Source::Typed, "plot(1:10, main = 'ten')");
    assert_eq!((outcome, drawn.len()), (Outcome::Ok, 1), "{text}");

    // PNG: inches, centimetres and pixels
    let mut png = |name: &str, set: &[(&str, &str)]| {
        let file = dir.file(name);
        let mut all = vec![("file", file.as_str()), ("format", "png")];
        all.extend_from_slice(set);
        export(&mut e, &all, false)
    };
    let (o, plots, text) = png("a.png", &[]);
    assert_eq!(o, Outcome::Ok, "{text}");
    assert!(plots.is_empty(), "an export must not make a new plot");
    assert_eq!(png_size(&dir.bytes("a.png")), Some((2100, 1500)));
    let (o, _, text) = png(
        "b.png",
        &[("width", "10"), ("height", "5"), ("units", "cm")],
    );
    assert_eq!(o, Outcome::Ok, "{text}");
    let (w, h) = png_size(&dir.bytes("b.png")).unwrap_or_default();
    assert!(
        (1180..=1182).contains(&w) && (590..=592).contains(&h),
        "{w}x{h}"
    );
    let (o, _, text) = png(
        "c.png",
        &[
            ("width", "800"),
            ("height", "600"),
            ("units", "px"),
            ("dpi", "100"),
        ],
    );
    assert_eq!(o, Outcome::Ok, "{text}");
    assert_eq!(png_size(&dir.bytes("c.png")), Some((800, 600)));

    // SVG and PDF: 7 by 5 inches is 504 by 360 points
    let (o, _, text) = export(
        &mut e,
        &[("file", &dir.file("a.svg")), ("format", "svg")],
        false,
    );
    if has_cairo(&mut e) {
        assert_eq!(o, Outcome::Ok, "{text}");
        let svg = dir.bytes("a.svg");
        assert!(
            contains(&svg, "<svg"),
            "not an SVG ({} bytes). R said: {text}",
            svg.len()
        );
        // cairo writes `width="504"` or, in older versions, `width="504pt"`
        assert!(contains(&svg, "width=\"504") && contains(&svg, "height=\"360"));
        assert!(contains(&svg, "viewBox=\"0 0 504 360\""));
    } else {
        // R cannot load cairo (a Mac without XQuartz, such as the CI runner): SVG says so
        // and writes nothing, rather than reporting success with an empty file
        assert_eq!(o, Outcome::Error, "{text}");
        assert!(text.contains("cairo") && text.contains("XQuartz"), "{text}");
        assert!(dir.bytes("a.svg").is_empty());
    }
    let (o, _, text) = export(
        &mut e,
        &[("file", &dir.file("a.pdf")), ("format", "pdf")],
        false,
    );
    assert_eq!(o, Outcome::Ok, "{text}");
    let pdf = dir.bytes("a.pdf");
    assert!(pdf.starts_with(b"%PDF"), "not a PDF");
    assert!(contains(&pdf, "/MediaBox [0 0 504 360]"));
    // an A4 page, given in centimetres (R writes whole points)
    let (o, _, text) = export(
        &mut e,
        &[
            ("file", &dir.file("a4.pdf")),
            ("format", "pdf"),
            ("width", "21"),
            ("height", "29.7"),
            ("units", "cm"),
        ],
        false,
    );
    assert_eq!(o, Outcome::Ok, "{text}");
    assert!(contains(&dir.bytes("a4.pdf"), "/MediaBox [0 0 595 841]"));
}

#[test]
fn the_newest_plot_a_named_one_and_one_that_is_gone() {
    let Some(mut e) = engine() else { return };
    let dir = Folder::new("which");
    let (_, first, _) = run(&mut e, Source::Typed, "plot(1:3, main = 'first')");
    let (_, second, _) = run(
        &mut e,
        Source::Typed,
        "plot(3:1, main = 'second', col = 'red', pch = 19)",
    );
    let name = |p: &[PathBuf]| {
        p.first()
            .and_then(|p| p.file_name())
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default()
    };
    let newest = export(
        &mut e,
        &[("file", &dir.file("newest.png")), ("format", "png")],
        false,
    );
    assert_eq!(newest.0, Outcome::Ok, "{}", newest.2);
    let by_name = export(
        &mut e,
        &[
            ("file", &dir.file("first.png")),
            ("format", "png"),
            ("plot", &name(&second)),
        ],
        false,
    );
    assert_eq!(by_name.0, Outcome::Ok, "{}", by_name.2);
    assert_eq!(
        dir.bytes("newest.png"),
        dir.bytes("first.png"),
        "empty means the newest"
    );
    let older = export(
        &mut e,
        &[
            ("file", &dir.file("older.png")),
            ("format", "png"),
            ("plot", &name(&first)),
        ],
        false,
    );
    assert_eq!(older.0, Outcome::Ok, "{}", older.2);
    assert_ne!(
        dir.bytes("older.png"),
        dir.bytes("newest.png"),
        "the older plot is a different picture ({} bytes). R said: {}",
        dir.bytes("older.png").len(),
        older.2
    );
    // a name R does not have says so, and writes nothing
    let gone = export(
        &mut e,
        &[
            ("file", &dir.file("gone.png")),
            ("format", "png"),
            ("plot", "nope.png"),
        ],
        false,
    );
    assert_eq!(gone.0, Outcome::Error);
    assert!(gone.2.contains("no longer kept"), "{}", gone.2);
    assert!(dir.bytes("gone.png").is_empty());
}

#[test]
fn nothing_is_written_over_unless_asked_and_with_no_plot_it_says_so() {
    let Some(mut e) = engine() else { return };
    let dir = Folder::new("guard");
    let none = export(&mut e, &[("file", &dir.file("x.png"))], true);
    assert_eq!(none.0, Outcome::Error);
    assert!(none.2.contains("no longer kept"), "{}", none.2);
    assert!(dir.bytes("x.png").is_empty());
    run(&mut e, Source::Typed, "plot(1:10)");
    let file = dir.file("keep.png");
    assert_eq!(export(&mut e, &[("file", &file)], false).0, Outcome::Ok);
    let first = dir.bytes("keep.png");
    assert!(!first.is_empty());
    run(
        &mut e,
        Source::Typed,
        "plot(10:1, col = 'blue', pch = 19, cex = 3)",
    );
    let again = export(&mut e, &[("file", &file)], false);
    assert_eq!(again.0, Outcome::Error);
    assert_eq!(dir.bytes("keep.png"), first, "the old file is untouched");
    assert_eq!(export(&mut e, &[("file", &file)], true).0, Outcome::Ok);
    assert_ne!(dir.bytes("keep.png"), first, "Replace writes the new plot");
}

#[test]
fn grid_plots_export_too_and_plotting_goes_on_afterwards() {
    let Some(mut e) = engine() else { return };
    let dir = Folder::new("grid");
    let (o, drawn, text) = run(
        &mut e,
        Source::Typed,
        "grid::grid.newpage(); grid::grid.rect(gp = grid::gpar(fill = 'tomato')); grid::grid.text('hello')",
    );
    assert_eq!((o, drawn.len()), (Outcome::Ok, 1), "{text}");
    let cairo = has_cairo(&mut e);
    for format in ["png", "svg", "pdf"] {
        if format == "svg" && !cairo {
            continue;
        }
        let r = export(
            &mut e,
            &[
                ("file", &dir.file(&format!("g.{format}"))),
                ("format", format),
            ],
            false,
        );
        assert_eq!(r.0, Outcome::Ok, "{format}: {}", r.2);
        assert!(
            !dir.bytes(&format!("g.{format}")).is_empty(),
            "{format} is empty. R said: {}",
            r.2
        );
    }
    // the live device still takes the next plot, and it is the newest
    let (o, drawn, text) = run(&mut e, Source::Typed, "plot(1:5)");
    assert_eq!((o, drawn.len()), (Outcome::Ok, 1), "{text}");
}

#[test]
fn only_the_newest_thirty_plots_are_kept() {
    let Some(mut e) = engine() else { return };
    let (o, drawn, text) = run(
        &mut e,
        Source::Typed,
        "for (i in 1:35) plot(i, main = paste('plot', i))",
    );
    assert_eq!((o, drawn.len()), (Outcome::Ok, 35), "{text}");
    let (o, _, text) = run(
        &mut e,
        Source::Typed,
        "stopifnot(length(getOption('rcraft.plots')) == 30L)",
    );
    assert_eq!(o, Outcome::Ok, "{text}");
    // the 31st from the end is gone, the newest is there
    let first_gone = drawn
        .first()
        .and_then(|p| p.file_name())
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    let (o, _, _) = run(
        &mut e,
        Source::Typed,
        &format!("stopifnot(is.null(getOption('rcraft.plots')[['{first_gone}']]))"),
    );
    assert_eq!(o, Outcome::Ok);
}

#[test]
fn svg_without_cairo_says_so_and_writes_nothing() {
    let Some(mut e) = engine() else { return };
    let dir = Folder::new("nocairo");
    run(&mut e, Source::Typed, "plot(1:10)");
    // an R that cannot load cairo: a stand-in for `capabilities()` in the workspace says so
    run(
        &mut e,
        Source::Typed,
        "capabilities <- function(what = NULL) c(cairo = FALSE)",
    );
    let (o, _, text) = export(
        &mut e,
        &[("file", &dir.file("x.svg")), ("format", "svg")],
        false,
    );
    assert_eq!(o, Outcome::Error, "{text}");
    assert!(text.contains("cairo") && text.contains("XQuartz"), "{text}");
    assert!(dir.bytes("x.svg").is_empty());
    // PNG does not need cairo and still works
    let (o, _, text) = export(
        &mut e,
        &[("file", &dir.file("x.png")), ("format", "png")],
        false,
    );
    assert_eq!(o, Outcome::Ok, "{text}");
    assert!(!dir.bytes("x.png").is_empty());
    run(&mut e, Source::Typed, "rm(capabilities)");
}

#[test]
fn a_device_that_writes_no_file_is_reported_not_called_success() {
    let Some(mut e) = engine() else { return };
    let dir = Folder::new("nofile");
    run(&mut e, Source::Typed, "plot(1:10)");
    // a folder that does not exist: the device opens nothing and writes nothing
    let missing = dir.file("no-such-folder/x.pdf");
    let (o, _, text) = export(&mut e, &[("file", &missing), ("format", "pdf")], false);
    assert_eq!(o, Outcome::Error, "{text}");
}
