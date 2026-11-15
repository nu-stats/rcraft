//! Example data (P8.3): data frames to practise on, taken from R's own
//! `datasets` package, which comes with every R. RCraft ships no data of its
//! own: the dialog copies a data frame from that package into the workspace,
//! and R's help page for it (`?mtcars`) gives its source. The descriptions
//! here are RCraft's own.

use rcraft_rcodegen::dialog::{Opt, Spec, Widget};

use crate::data::BASE;

/// Each example: its name in `datasets`, then a line saying what it is.
pub const EXAMPLES: &[Opt] = &[
    Opt {
        value: "mtcars",
        label: "mtcars: fuel use, power and weight of 32 cars (32 rows, 11 variables)",
    },
    Opt {
        value: "iris",
        label: "iris: flower measurements of three iris species (150 rows, 5 variables)",
    },
    Opt {
        value: "airquality",
        label: "airquality: daily ozone, sun, wind and heat in New York, with gaps (153 rows, 6 variables)",
    },
    Opt {
        value: "ToothGrowth",
        label: "ToothGrowth: tooth length by vitamin C dose and how it was given (60 rows, 3 variables)",
    },
    Opt {
        value: "PlantGrowth",
        label: "PlantGrowth: plant weights under a control and two treatments (30 rows, 2 variables)",
    },
    Opt {
        value: "warpbreaks",
        label: "warpbreaks: breaks in yarn by wool and tension (54 rows, 3 variables)",
    },
    Opt {
        value: "esoph",
        label: "esoph: cases and controls by age, alcohol and tobacco groups (88 rows, 5 variables)",
    },
    Opt {
        value: "infert",
        label: "infert: a matched case-control study of infertility (248 rows, 8 variables)",
    },
    Opt {
        value: "swiss",
        label: "swiss: fertility and social measures of Swiss provinces (47 rows, 6 variables)",
    },
    Opt {
        value: "USArrests",
        label: "USArrests: arrest rates by US state (50 rows, 4 variables)",
    },
    Opt {
        value: "faithful",
        label: "faithful: eruption length and waiting time of a geyser (272 rows, 2 variables)",
    },
    Opt {
        value: "cars",
        label: "cars: speed and stopping distance (50 rows, 2 variables)",
    },
    Opt {
        value: "trees",
        label: "trees: girth, height and volume of cherry trees (31 rows, 3 variables)",
    },
    Opt {
        value: "sleep",
        label: "sleep: extra sleep under two drugs, the same people twice (20 rows, 3 variables)",
    },
    Opt {
        value: "chickwts",
        label: "chickwts: chick weights by feed (71 rows, 2 variables)",
    },
    Opt {
        value: "InsectSprays",
        label: "InsectSprays: insect counts after six sprays (72 rows, 2 variables)",
    },
    Opt {
        value: "quakes",
        label: "quakes: place, depth and size of earthquakes near Fiji (1000 rows, 5 variables)",
    },
    Opt {
        value: "attitude",
        label: "attitude: survey ratings of a firm's departments (30 rows, 7 variables)",
    },
    Opt {
        value: "stackloss",
        label: "stackloss: a chemical plant's loss by air flow, water and acid (21 rows, 4 variables)",
    },
    Opt {
        value: "morley",
        label: "morley: speed-of-light measurements in five experiments (100 rows, 3 variables)",
    },
];

pub const EXAMPLE_DATA: Spec = Spec {
    entry: "Example data",
    menu: "File > Example data",
    about: "Copy a data frame to practise on into the workspace, under its own name. They come \
            with R itself (its datasets package); R's help page on each (type ?mtcars, say) says \
            where it is from.",
    widgets: &[Widget::Choice {
        id: "result",
        label: "Data",
        options: EXAMPLES,
        default: "mtcars",
        hint: "",
    }],
    needs: &[],
    styles: BASE,
    check: |_| Vec::new(),
    code: |a, _| {
        let name = a.choice("result");
        format!("{name} <- as.data.frame(datasets::{name})")
    },
};

/// The example data dialog, for the File menu.
pub const SPECS: &[&Spec] = &[&EXAMPLE_DATA];

#[cfg(test)]
mod tests {
    use super::*;
    use rcraft_rcodegen::dialog::{Context, Style};

    #[test]
    fn each_example_is_copied_under_its_own_name() {
        let c = Context {
            dataset: "(examples)".into(),
            frames: Vec::new(),
            columns: Vec::new(),
        };
        for ex in EXAMPLES {
            let mut v = EXAMPLE_DATA.defaults(&c);
            v.set_text("result", ex.value);
            let code = EXAMPLE_DATA
                .generate(&c, &v, Style::Base)
                .unwrap_or_default();
            assert_eq!(
                code,
                format!("{0} <- as.data.frame(datasets::{0})", ex.value)
            );
            assert!(
                ex.label.starts_with(&format!("{}: ", ex.value)),
                "{}",
                ex.label
            );
        }
    }
}
