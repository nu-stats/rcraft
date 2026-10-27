//! How the window looks: accent colours, light and dark, fonts and sizes.
//!
//! The accent is the one colour RCraft adds to an otherwise neutral window:
//! the logo, selections, the command line's outline, the Run button and the
//! echo of each command in Results. Seven are offered under Edit > Accent;
//! Teal is the default. Each has a light-mode and a dark-mode shade, checked
//! for contrast against white and against the dark window (`design/layouts.html`).
//! Fonts are the computer's own where they exist; nothing is bundled.

use eframe::egui::{self, Color32, FontFamily, FontId, Rounding, Stroke, TextStyle};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Accent {
    Blue,
    Indigo,
    #[default]
    Teal,
    Plum,
    Forest,
    Amber,
    Rose,
}

impl Accent {
    pub const ALL: [Accent; 7] = [
        Accent::Blue,
        Accent::Indigo,
        Accent::Teal,
        Accent::Plum,
        Accent::Forest,
        Accent::Amber,
        Accent::Rose,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Accent::Blue => "Blue",
            Accent::Indigo => "Indigo",
            Accent::Teal => "Teal",
            Accent::Plum => "Plum",
            Accent::Forest => "Forest",
            Accent::Amber => "Amber",
            Accent::Rose => "Rose",
        }
    }

    /// The accent for a light window.
    pub fn light(self) -> Color32 {
        let (r, g, b) = match self {
            Accent::Blue => (0x1f, 0x5f, 0x99),
            Accent::Indigo => (0x4b, 0x4d, 0xa8),
            Accent::Teal => (0x0e, 0x76, 0x6e),
            Accent::Plum => (0x7a, 0x3e, 0x9d),
            Accent::Forest => (0x2d, 0x6a, 0x3e),
            Accent::Amber => (0x9c, 0x5a, 0x00),
            Accent::Rose => (0xb0, 0x34, 0x5f),
        };
        Color32::from_rgb(r, g, b)
    }

    /// The accent for a dark window.
    pub fn dark(self) -> Color32 {
        let (r, g, b) = match self {
            Accent::Blue => (0x7f, 0xb3, 0xe6),
            Accent::Indigo => (0xa5, 0xa8, 0xf0),
            Accent::Teal => (0x5f, 0xcb, 0xbe),
            Accent::Plum => (0xc7, 0x9b, 0xe6),
            Accent::Forest => (0x85, 0xcc, 0x95),
            Accent::Amber => (0xf0, 0xb3, 0x5a),
            Accent::Rose => (0xf0, 0x9a, 0xb8),
        };
        Color32::from_rgb(r, g, b)
    }

    pub fn color(self, dark: bool) -> Color32 {
        if dark {
            self.dark()
        } else {
            self.light()
        }
    }
}

/// Colours for the Results window and the panes, for one mode.
#[derive(Clone, Copy, Debug)]
pub struct Palette {
    pub dark: bool,
    pub accent: Color32,
    pub text: Color32,
    pub dim: Color32,
    pub command: Color32,
    pub error: Color32,
    pub warning: Color32,
    pub on_accent: Color32,
}

impl Palette {
    pub fn new(accent: Accent, dark: bool) -> Palette {
        Palette {
            dark,
            accent: accent.color(dark),
            text: if dark {
                rgb(227, 232, 238)
            } else {
                rgb(29, 39, 51)
            },
            dim: if dark {
                rgb(154, 166, 178)
            } else {
                rgb(86, 98, 112)
            },
            command: if dark {
                rgb(250, 250, 250)
            } else {
                rgb(16, 20, 26)
            },
            error: if dark {
                rgb(255, 112, 112)
            } else {
                rgb(196, 40, 40)
            },
            warning: if dark {
                rgb(240, 179, 90)
            } else {
                rgb(156, 90, 0)
            },
            on_accent: if dark {
                rgb(16, 20, 24)
            } else {
                Color32::WHITE
            },
        }
    }
}

const fn rgb(r: u8, g: u8, b: u8) -> Color32 {
    Color32::from_rgb(r, g, b)
}

/// `a` over `b` at `percent` (0 to 100), as `color-mix(in srgb)` does.
pub fn mix(a: Color32, b: Color32, percent: u8) -> Color32 {
    let p = u16::from(percent.min(100));
    let m = |x: u8, y: u8| ((u16::from(x) * p + u16::from(y) * (100 - p)) / 100) as u8;
    Color32::from_rgb(m(a.r(), b.r()), m(a.g(), b.g()), m(a.b(), b.b()))
}

/// Sets fonts and sizes once, when the window opens.
pub fn apply_static(ctx: &egui::Context) {
    system_fonts(ctx);
    ctx.all_styles_mut(|style| {
        use FontFamily::{Monospace, Proportional};
        style.text_styles = [
            (TextStyle::Small, FontId::new(11.5, Proportional)),
            (TextStyle::Body, FontId::new(14.0, Proportional)),
            (TextStyle::Button, FontId::new(14.0, Proportional)),
            (TextStyle::Heading, FontId::new(15.5, Proportional)),
            (TextStyle::Monospace, FontId::new(13.5, Monospace)),
        ]
        .into();
        style.spacing.item_spacing = egui::vec2(8.0, 5.0);
        style.spacing.button_padding = egui::vec2(10.0, 4.0);
        style.spacing.menu_margin = egui::Margin::same(6.0);
        style.spacing.interact_size.y = 24.0;
    });
}

/// Sets the colours for both modes from the chosen accent.
pub fn apply_colors(ctx: &egui::Context, accent: Accent) {
    ctx.set_visuals_of(egui::Theme::Light, visuals(accent, false));
    ctx.set_visuals_of(egui::Theme::Dark, visuals(accent, true));
}

fn visuals(accent: Accent, dark: bool) -> egui::Visuals {
    let p = Palette::new(accent, dark);
    let mut v = if dark {
        egui::Visuals::dark()
    } else {
        egui::Visuals::light()
    };
    let (bg, panel, line) = if dark {
        (rgb(18, 22, 27), rgb(26, 32, 39), rgb(44, 53, 63))
    } else {
        (Color32::WHITE, rgb(244, 246, 248), rgb(214, 219, 224))
    };
    v.panel_fill = panel;
    v.window_fill = bg;
    v.extreme_bg_color = bg;
    v.faint_bg_color = mix(p.accent, bg, 6);
    v.selection.bg_fill = mix(p.accent, panel, if dark { 28 } else { 18 });
    v.selection.stroke = Stroke::new(1.0_f32, p.accent);
    v.hyperlink_color = p.accent;
    v.override_text_color = Some(p.text);
    v.widgets.noninteractive.bg_stroke = Stroke::new(1.0_f32, line);
    v.widgets.inactive.bg_stroke = Stroke::new(1.0_f32, line);
    v.widgets.noninteractive.fg_stroke = Stroke::new(1.0_f32, p.text);
    v.widgets.inactive.fg_stroke = Stroke::new(1.0_f32, p.text);
    v.widgets.inactive.weak_bg_fill = mix(p.text, panel, 6);
    v.widgets.hovered.weak_bg_fill = mix(p.accent, panel, 20);
    v.widgets.hovered.fg_stroke = Stroke::new(1.0_f32, p.command);
    v.widgets.hovered.bg_stroke = Stroke::new(1.0_f32, p.accent);
    v.widgets.active.fg_stroke = Stroke::new(1.0_f32, p.command);
    let r = Rounding::same(5.0);
    v.widgets.noninteractive.rounding = r;
    v.widgets.inactive.rounding = r;
    v.widgets.hovered.rounding = r;
    v.widgets.active.rounding = r;
    v.widgets.open.rounding = r;
    v.window_rounding = Rounding::same(8.0);
    v.menu_rounding = Rounding::same(6.0);
    v
}

/// Puts the computer's interface and fixed-width fonts in front of egui's.
fn system_fonts(ctx: &egui::Context) {
    const PROPORTIONAL: &[&str] = &[
        "C:/Windows/Fonts/segoeui.ttf",
        "/System/Library/Fonts/Helvetica.ttc",
        "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
        "/usr/share/fonts/dejavu/DejaVuSans.ttf",
        "/usr/share/fonts/TTF/DejaVuSans.ttf",
        "/usr/share/fonts/truetype/noto/NotoSans-Regular.ttf",
    ];
    const MONOSPACE: &[&str] = &[
        "C:/Windows/Fonts/consola.ttf",
        "/System/Library/Fonts/Menlo.ttc",
        "/usr/share/fonts/truetype/dejavu/DejaVuSansMono.ttf",
        "/usr/share/fonts/dejavu/DejaVuSansMono.ttf",
        "/usr/share/fonts/TTF/DejaVuSansMono.ttf",
        "/usr/share/fonts/truetype/noto/NotoSansMono-Regular.ttf",
    ];
    let mut fonts = egui::FontDefinitions::default();
    let mut changed = false;
    for (family, list, key) in [
        (FontFamily::Proportional, PROPORTIONAL, "system-ui"),
        (FontFamily::Monospace, MONOSPACE, "system-mono"),
    ] {
        // a font file is read whole; skip anything unreasonably large
        let found = list.iter().find_map(|path| {
            let meta = std::fs::metadata(path).ok()?;
            if meta.len() > 64 * 1024 * 1024 {
                return None;
            }
            std::fs::read(path).ok()
        });
        if let Some(bytes) = found {
            fonts
                .font_data
                .insert(key.to_string(), egui::FontData::from_owned(bytes));
            if let Some(names) = fonts.families.get_mut(&family) {
                names.insert(0, key.to_string());
            }
            changed = true;
        }
    }
    if changed {
        ctx.set_fonts(fonts);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn luminance(c: Color32) -> f64 {
        let f = |v: u8| {
            let v = f64::from(v) / 255.0;
            if v <= 0.03928 {
                v / 12.92
            } else {
                ((v + 0.055) / 1.055).powf(2.4)
            }
        };
        0.2126 * f(c.r()) + 0.7152 * f(c.g()) + 0.0722 * f(c.b())
    }

    fn contrast(a: Color32, b: Color32) -> f64 {
        let (x, y) = (luminance(a), luminance(b));
        (x.max(y) + 0.05) / (x.min(y) + 0.05)
    }

    #[test]
    fn every_accent_reads_as_text_in_both_modes() {
        for accent in Accent::ALL {
            assert!(
                contrast(accent.light(), Color32::WHITE) >= 4.5,
                "{accent:?} light"
            );
            assert!(
                contrast(accent.dark(), rgb(18, 22, 27)) >= 4.5,
                "{accent:?} dark"
            );
        }
    }

    #[test]
    fn text_on_the_accent_button_is_readable() {
        for accent in Accent::ALL {
            for dark in [false, true] {
                let p = Palette::new(accent, dark);
                assert!(
                    contrast(p.on_accent, p.accent) >= 4.5,
                    "{accent:?} dark={dark}"
                );
            }
        }
    }

    #[test]
    fn mixing_is_a_blend() {
        let a = Color32::from_rgb(200, 100, 0);
        let b = Color32::from_rgb(0, 100, 200);
        assert_eq!(mix(a, b, 0), b);
        assert_eq!(mix(a, b, 100), a);
        assert_eq!(mix(a, b, 50), Color32::from_rgb(100, 100, 100));
        assert_eq!(mix(a, b, 250), a);
    }

    #[test]
    fn teal_is_the_default() {
        assert_eq!(Accent::default(), Accent::Teal);
        assert_eq!(Accent::ALL.len(), 7);
    }
}
