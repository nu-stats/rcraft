//! Graphs > Export plot (P5.4): a plot the Graph tab shows, drawn again on a
//! PNG, SVG or PDF file of a chosen size. The session keeps a recording of each
//! plot it captured (`options("rcraft.plots")`, the newest 30, see
//! `rproto/src/bootstrap.R`), and this replays one onto the file device, so the
//! file is drawn at its own size and resolution rather than scaled from the
//! picture on screen. Needs no package.

use rcraft_rcodegen::dialog::{Args, Opt, Problem, Spec, Style, TextDefault, TextRule, Widget};
use rcraft_rcodegen::r_string;

use crate::data::BASE;

/// Biggest side of a PNG, in pixels, so a typo cannot ask for gigabytes.
const MAX_PIXELS: f64 = 20_000.0;
/// Biggest side of any file, in inches.
const MAX_INCHES: f64 = 100.0;
const CM_PER_INCH: f64 = 2.54;

pub const EXPORT_PLOT: Spec = Spec {
    entry: "Export plot",
    menu: "Graphs > Export plot",
    about: "Save a plot as a picture file: PNG, or SVG or PDF, which stay sharp at any size. \
            The plot is drawn again at the size you give, not scaled from the Graph tab.",
    widgets: &[
        Widget::Text {
            id: "plot",
            label: "Plot",
            default: TextDefault::Empty,
            rule: TextRule::Any,
            hint: "The plot's file name in the Graph tab. Empty: the newest plot. R keeps the \
                   last 30 plots of a session.",
        },
        Widget::Text {
            id: "file",
            label: "Save to",
            default: TextDefault::Empty,
            rule: TextRule::SavePath,
            hint: "The full path of the file, with its extension.",
        },
        Widget::Choice {
            id: "format",
            label: "Format",
            options: &[
                Opt {
                    value: "png",
                    label: "PNG (.png)",
                },
                Opt {
                    value: "svg",
                    label: "SVG (.svg)",
                },
                Opt {
                    value: "pdf",
                    label: "PDF (.pdf)",
                },
            ],
            default: "png",
            hint: "",
        },
        Widget::Text {
            id: "width",
            label: "Width",
            default: TextDefault::Fixed("7"),
            rule: TextRule::Number,
            hint: "",
        },
        Widget::Text {
            id: "height",
            label: "Height",
            default: TextDefault::Fixed("5"),
            rule: TextRule::Number,
            hint: "",
        },
        Widget::Choice {
            id: "units",
            label: "Size is in",
            options: &[
                Opt {
                    value: "in",
                    label: "Inches",
                },
                Opt {
                    value: "cm",
                    label: "Centimetres",
                },
                Opt {
                    value: "px",
                    label: "Pixels (at the resolution below)",
                },
            ],
            default: "in",
            hint: "",
        },
        Widget::Text {
            id: "dpi",
            label: "Resolution (dots per inch)",
            default: TextDefault::Fixed("300"),
            rule: TextRule::Number,
            hint: "For PNG, and to turn pixels into inches for SVG and PDF.",
        },
        Widget::Check {
            id: "replace",
            label: "Replace the file if it is already there",
            default: false,
            hint: "Left unticked, nothing is written when the file exists.",
        },
    ],
    needs: &[],
    styles: BASE,
    check: export_check,
    code: export_code,
};

/// The Graphs menu's plot-less dialogs (they need a plot, not a data frame).
pub const PLOT_SPECS: &[&Spec] = &[&EXPORT_PLOT];

fn number(a: &Args, id: &str) -> f64 {
    a.number(id).parse().unwrap_or(0.0)
}

/// A size in `units` as inches.
fn inches(size: f64, units: &str, dpi: f64) -> f64 {
    match units {
        "cm" => size / CM_PER_INCH,
        "px" => size / dpi,
        _ => size,
    }
}

/// `7`, `2.7559`: at most four decimals, no trailing zeros.
fn tidy(x: f64) -> String {
    let s = format!("{x:.4}");
    s.trim_end_matches('0').trim_end_matches('.').to_string()
}

fn export_check(a: &Args) -> Vec<Problem> {
    let mut out = Vec::new();
    let dpi = number(a, "dpi");
    if !(36.0..=1200.0).contains(&dpi) {
        out.push(Problem::new(
            "dpi",
            "The resolution is between 36 and 1200 dots per inch.",
        ));
        return out;
    }
    let units = a.choice("units");
    for (id, name) in [("width", "width"), ("height", "height")] {
        let size = number(a, id);
        let inch = inches(size, units, dpi);
        if size <= 0.0 {
            out.push(Problem::new(id, format!("The {name} must be above 0.")));
        } else if inch > MAX_INCHES {
            out.push(Problem::new(
                id,
                format!("The {name} can be at most {MAX_INCHES} inches."),
            ));
        } else if a.choice("format") == "png" && inch * dpi > MAX_PIXELS {
            out.push(Problem::new(
                id,
                format!("A PNG can have at most {MAX_PIXELS} pixels on a side at this resolution."),
            ));
        } else if a.choice("format") == "png" && inch * dpi < 10.0 {
            out.push(Problem::new(
                id,
                "A PNG needs at least 10 pixels on a side.",
            ));
        } else if inch < 0.1 {
            out.push(Problem::new(
                id,
                format!("The {name} must be at least 0.1 inch."),
            ));
        }
    }
    let format = a.choice("format");
    let file = a.name_text("file").to_ascii_lowercase();
    if !file.ends_with(&format!(".{format}")) {
        out.push(Problem::new(
            "file",
            format!("The file name should end in .{format}."),
        ));
    }
    out
}

fn export_code(a: &Args, _: Style) -> String {
    let path = r_string(&a.name_text("file").replace('\\', "/"));
    let guard = if a.flag("replace") {
        String::new()
    } else {
        format!(
            "if (file.exists({path})) stop({})\n",
            r_string("the file already exists; tick Replace to write over it")
        )
    };
    let lookup = if a.name_text("plot").is_empty() {
        "if (length(plots)) plots[[length(plots)]]".to_string()
    } else {
        format!("plots[[{}]]", r_string(a.name_text("plot")))
    };
    let dpi = number(a, "dpi");
    let (w, h) = (number(a, "width"), number(a, "height"));
    let units = a.choice("units");
    let device = match a.choice("format") {
        "svg" => format!(
            "grDevices::svg({path}, width = {}, height = {})",
            tidy(inches(w, units, dpi)),
            tidy(inches(h, units, dpi))
        ),
        "pdf" => format!(
            "grDevices::pdf({path}, width = {}, height = {}, useDingbats = FALSE)",
            tidy(inches(w, units, dpi)),
            tidy(inches(h, units, dpi))
        ),
        _ => format!(
            "grDevices::png({path}, width = {}, height = {}, units = {}, res = {})",
            a.number("width"),
            a.number("height"),
            r_string(units),
            a.number("dpi")
        ),
    };
    // a device that cannot open (svg() on a Mac without XQuartz only warns, and
    // capabilities("cairo") still says TRUE) leaves the device as it was: say so,
    // and never close the device that was current before (the Graph tab's own)
    let no_device = if a.choice("format") == "svg" {
        "SVG needs R's cairo graphics, which this R cannot load. On a Mac this usually \
         means XQuartz is not installed (xquartz.org). PNG and PDF still work"
    } else {
        "R could not open the file to draw in; check that the folder exists and can be written to"
    };
    format!(
        "{guard}local({{\n  plots <- getOption(\"rcraft.plots\")\n  rec <- {lookup}\n  \
         if (is.null(rec)) stop({})\n  before <- grDevices::dev.cur()\n  {device}\n  \
         if (grDevices::dev.cur() == before) stop({})\n  opened <- grDevices::dev.cur()\n  \
         on.exit(grDevices::dev.off(opened))\n  \
         grDevices::replayPlot(rec)\n}})\n\
         if (!isTRUE(file.size({path}) > 0)) stop({})",
        r_string("that plot is no longer kept; draw it again, then export it"),
        r_string(no_device),
        r_string("R wrote no file; check that the folder exists and can be written to")
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use rcraft_rcodegen::dialog::Context;

    fn ctx() -> Context {
        Context {
            dataset: "(plot)".into(),
            frames: Vec::new(),
            columns: Vec::new(),
        }
    }

    fn code(set: &[(&str, &str)], replace: bool) -> Result<String, Vec<Problem>> {
        let c = ctx();
        let mut v = EXPORT_PLOT.defaults(&c);
        for (id, t) in set {
            v.set_text(id, *t);
        }
        v.set_flag("replace", replace);
        EXPORT_PLOT.generate(&c, &v, Style::Base)
    }

    fn message(set: &[(&str, &str)]) -> String {
        code(set, true)
            .err()
            .and_then(|p| p.first().map(|p| p.message.clone()))
            .unwrap_or_default()
    }

    #[test]
    fn png_gets_its_size_units_and_resolution_as_typed() {
        let c = code(
            &[("file", "C:\\p\\a.png"), ("width", "10"), ("units", "cm")],
            true,
        );
        let c = c.unwrap_or_default();
        assert!(
            c.contains(
                "grDevices::png(\"C:/p/a.png\", width = 10, height = 5, units = \"cm\", res = 300)"
            ),
            "{c}"
        );
        assert!(
            c.contains("rec <- if (length(plots)) plots[[length(plots)]]"),
            "{c}"
        );
        assert!(
            c.contains(
                "grDevices::replayPlot(rec)\n})\nif (!isTRUE(file.size(\"C:/p/a.png\") > 0))"
            ),
            "{c}"
        );
        assert!(c.ends_with("can be written to\")"), "{c}");
        assert!(!c.contains("XQuartz"), "only SVG needs cairo: {c}");
        // only the device opened here is closed, never the one that was current
        assert!(c.contains("before <- grDevices::dev.cur()"), "{c}");
        assert!(
            c.contains("if (grDevices::dev.cur() == before) stop("),
            "{c}"
        );
        assert!(c.contains("on.exit(grDevices::dev.off(opened))"), "{c}");
        assert!(!c.contains("file.exists"), "{c}");
    }

    #[test]
    fn svg_and_pdf_get_inches_whatever_the_units() {
        let svg = code(&[("file", "a.svg"), ("format", "svg")], true).unwrap_or_default();
        assert!(
            svg.contains("grDevices::svg(\"a.svg\", width = 7, height = 5)")
                && svg.contains("XQuartz"),
            "{svg}"
        );
        let pdf = code(
            &[
                ("file", "a.pdf"),
                ("format", "pdf"),
                ("width", "21"),
                ("height", "29.7"),
                ("units", "cm"),
            ],
            true,
        )
        .unwrap_or_default();
        assert!(
            pdf.contains("width = 8.2677, height = 11.6929, useDingbats = FALSE"),
            "{pdf}"
        );
        let px = code(
            &[
                ("file", "a.svg"),
                ("format", "svg"),
                ("width", "1200"),
                ("height", "600"),
                ("units", "px"),
                ("dpi", "150"),
            ],
            true,
        )
        .unwrap_or_default();
        assert!(px.contains("width = 8, height = 4)"), "{px}");
    }

    #[test]
    fn a_named_plot_is_looked_up_by_name_and_the_file_is_guarded() {
        let c = code(&[("file", "a.png"), ("plot", "12-002.png")], false).unwrap_or_default();
        assert!(c.starts_with("if (file.exists(\"a.png\")) stop("), "{c}");
        assert!(c.contains("rec <- plots[[\"12-002.png\"]]"), "{c}");
    }

    #[test]
    fn sizes_endings_and_resolution_are_checked() {
        assert!(message(&[("file", "a.png"), ("width", "0")]).contains("above 0"));
        assert!(message(&[("file", "a.png"), ("height", "-2")]).contains("above 0"));
        assert!(message(&[("file", "a.png"), ("dpi", "10")]).contains("between 36 and 1200"));
        assert!(message(&[("file", "a.png"), ("width", "500")]).contains("at most 100 inches"));
        // 90 inches at 300 dpi is 27,000 pixels
        assert!(message(&[("file", "a.png"), ("width", "90")]).contains("pixels on a side"));
        assert!(
            message(&[("file", "a.png"), ("width", "0.01"), ("dpi", "300")])
                .contains("at least 10 pixels")
        );
        assert!(message(&[("file", "a.pdf")]).contains("should end in .png"));
        assert!(message(&[("file", "a.png"), ("format", "svg")]).contains("should end in .svg"));
        assert!(message(&[]).contains("where to save"));
        // a big vector file is fine where a big PNG is not
        assert!(code(
            &[("file", "a.pdf"), ("format", "pdf"), ("width", "90")],
            true
        )
        .is_ok());
    }
}
