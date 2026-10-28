//! The Data browser (a virtualised grid) and the variable list. Only the
//! rows and columns in view are drawn, so a table of millions of rows costs
//! no more than a screenful.

use eframe::egui::{self, Align2, Color32, FontId, Rect, RichText, Sense, TextStyle};
use rcraft_engine::WorkspaceItem;
use rcraft_rdata::display::{labelled_text, LabelView};
use rcraft_rdata::summary::Report;
use rcraft_rdata::{Column, Data, Table};

use crate::look::Palette;

const ROW_HEIGHT: f32 = 20.0;
const MIN_COLUMN: f32 = 56.0;
const MAX_COLUMN: f32 = 320.0;
/// Rows measured to pick a column's width.
const SAMPLE_ROWS: usize = 30;
/// Width of one character, in points, for choosing column widths.
const CHAR_WIDTH: f32 = 7.4;

fn right_aligned(data: &Data) -> bool {
    matches!(
        data,
        Data::Int(_)
            | Data::Double(_)
            | Data::Date(_)
            | Data::DateTime { .. }
            | Data::Duration { .. }
    )
}

/// Which variables the Variables pane lists.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TypeFilter {
    #[default]
    All,
    Numbers,
    Text,
    Factors,
    Dates,
    Logical,
    Labelled,
}

impl TypeFilter {
    pub const ALL: [TypeFilter; 7] = [
        TypeFilter::All,
        TypeFilter::Numbers,
        TypeFilter::Text,
        TypeFilter::Factors,
        TypeFilter::Dates,
        TypeFilter::Logical,
        TypeFilter::Labelled,
    ];

    pub fn label(self) -> &'static str {
        match self {
            TypeFilter::All => "All types",
            TypeFilter::Numbers => "Numbers",
            TypeFilter::Text => "Text",
            TypeFilter::Factors => "Factors",
            TypeFilter::Dates => "Dates and times",
            TypeFilter::Logical => "Logical",
            TypeFilter::Labelled => "With labels",
        }
    }

    pub fn matches(self, col: &Column) -> bool {
        match self {
            TypeFilter::All => true,
            TypeFilter::Numbers => matches!(col.data, Data::Int(_) | Data::Double(_)),
            TypeFilter::Text => matches!(col.data, Data::Text(_)),
            TypeFilter::Factors => matches!(col.data, Data::Factor { .. }),
            TypeFilter::Dates => matches!(
                col.data,
                Data::Date(_) | Data::DateTime { .. } | Data::Duration { .. }
            ),
            TypeFilter::Logical => matches!(col.data, Data::Logical(_)),
            TypeFilter::Labelled => col.label().is_some() || !col.value_labels().is_empty(),
        }
    }
}

/// One column as the grid draws it: its value labels, and whether they are
/// being shown (labels are text, so they sit on the left).
struct Shown {
    labels: Vec<(f64, String)>,
    view: LabelView,
}

impl Shown {
    fn new(col: &Column, view: LabelView) -> Shown {
        Shown {
            labels: col.value_labels(),
            view,
        }
    }

    fn text(&self, col: &Column, row: usize) -> String {
        labelled_text(&col.data, &self.labels, row, self.view)
    }

    fn right(&self, col: &Column) -> bool {
        right_aligned(&col.data) && (self.view == LabelView::Values || self.labels.is_empty())
    }
}

/// Width in points for each column, from its name and a sample of its rows.
fn column_widths(table: &Table, shown: &[Shown]) -> Vec<f32> {
    let step = (table.nrow / SAMPLE_ROWS).max(1);
    table
        .columns
        .iter()
        .zip(shown)
        .map(|(col, shown)| {
            let mut longest = col.name.chars().count();
            for row in (0..table.nrow).step_by(step).take(SAMPLE_ROWS) {
                longest = longest.max(shown.text(col, row).chars().count());
            }
            (longest as f32 * CHAR_WIDTH + 18.0).clamp(MIN_COLUMN, MAX_COLUMN)
        })
        .collect()
}

/// The grid. `offset_x` carries the sideways scroll position from the body to
/// the header between frames.
pub fn grid(ui: &mut egui::Ui, pal: &Palette, table: &Table, view: LabelView, offset_x: &mut f32) {
    let shown: Vec<Shown> = table.columns.iter().map(|c| Shown::new(c, view)).collect();
    let widths = column_widths(table, &shown);
    let gutter = (table.nrow.to_string().len() as f32 * CHAR_WIDTH + 18.0).max(36.0);
    let total = gutter + widths.iter().sum::<f32>();
    let font: FontId = TextStyle::Body.resolve(ui.style());
    let stripe = pal.dim.gamma_multiply(0.10);

    egui::ScrollArea::horizontal()
        .id_salt("grid-head")
        .horizontal_scroll_offset(*offset_x)
        .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysHidden)
        .enable_scrolling(false)
        .max_height(ROW_HEIGHT + 2.0)
        .show(ui, |ui| {
            let (rect, _) = ui.allocate_exact_size(egui::vec2(total, ROW_HEIGHT), Sense::hover());
            let mut x = rect.left() + gutter;
            for ((col, width), shown) in table.columns.iter().zip(&widths).zip(&shown) {
                let cell =
                    Rect::from_min_size(egui::pos2(x, rect.top()), egui::vec2(*width, ROW_HEIGHT));
                if ui.is_rect_visible(cell) {
                    let painter = ui
                        .painter()
                        .with_clip_rect(cell.shrink2(egui::vec2(3.0, 0.0)));
                    // the header sits over its numbers, on the same side
                    let (at, align) = if shown.right(col) {
                        (
                            cell.right_center() - egui::vec2(6.0, 0.0),
                            Align2::RIGHT_CENTER,
                        )
                    } else {
                        (
                            cell.left_center() + egui::vec2(6.0, 0.0),
                            Align2::LEFT_CENTER,
                        )
                    };
                    painter.text(at, align, &col.name, font.clone(), pal.accent);
                    ui.interact(cell, ui.id().with(("head", x as i64)), Sense::hover())
                        .on_hover_text(header_hint(col));
                }
                x += *width;
            }
        });

    let out = egui::ScrollArea::both()
        .id_salt("grid-body")
        .auto_shrink([false, false])
        .show_rows(ui, ROW_HEIGHT, table.nrow, |ui, rows| {
            let visible = ui.clip_rect();
            for row in rows {
                let (rect, _) =
                    ui.allocate_exact_size(egui::vec2(total, ROW_HEIGHT), Sense::hover());
                if row % 2 == 1 {
                    ui.painter().rect_filled(rect, 0.0, stripe);
                }
                let number = Rect::from_min_size(rect.min, egui::vec2(gutter, ROW_HEIGHT));
                ui.painter().text(
                    number.right_center() - egui::vec2(8.0, 0.0),
                    Align2::RIGHT_CENTER,
                    (row + 1).to_string(),
                    font.clone(),
                    pal.dim,
                );
                let mut x = rect.left() + gutter;
                for ((col, width), shown) in table.columns.iter().zip(&widths).zip(&shown) {
                    let cell = Rect::from_min_size(
                        egui::pos2(x, rect.top()),
                        egui::vec2(*width, ROW_HEIGHT),
                    );
                    x += *width;
                    if cell.right() < visible.left() || cell.left() > visible.right() {
                        continue;
                    }
                    let text = shown.text(col, row);
                    let color: Color32 = if text == "NA" || text == "NaN" {
                        pal.dim
                    } else {
                        pal.text
                    };
                    let painter = ui
                        .painter()
                        .with_clip_rect(cell.shrink2(egui::vec2(3.0, 0.0)));
                    if shown.right(col) {
                        painter.text(
                            cell.right_center() - egui::vec2(6.0, 0.0),
                            Align2::RIGHT_CENTER,
                            text,
                            font.clone(),
                            color,
                        );
                    } else {
                        painter.text(
                            cell.left_center() + egui::vec2(6.0, 0.0),
                            Align2::LEFT_CENTER,
                            text,
                            font.clone(),
                            color,
                        );
                    }
                }
            }
        });
    *offset_x = out.state.offset.x;
}

fn header_hint(col: &Column) -> String {
    let mut hint = format!("{}  ({})", col.name, col.data.kind_name());
    if let Some(label) = col.label() {
        hint.push_str(&format!("\n{label}"));
    }
    hint
}

/// What the Variables pane asks for.
pub enum VariableClick {
    /// Put the variable's name in the Command line.
    Insert(String),
}

/// One line per variable: name, kind, missing count; the label on hover.
pub fn variable_list(
    ui: &mut egui::Ui,
    pal: &Palette,
    table: &Table,
    filter: TypeFilter,
    out: &mut Vec<VariableClick>,
) {
    if filter != TypeFilter::All {
        let shown = table.columns.iter().filter(|c| filter.matches(c)).count();
        ui.label(
            RichText::new(format!("{shown} of {} variables", table.columns.len()))
                .small()
                .color(pal.dim),
        );
    }
    for col in table.columns.iter().filter(|c| filter.matches(c)) {
        let missing = col.data.missing();
        let response = ui
            .horizontal(|ui| {
                let name = ui.selectable_label(false, RichText::new(&col.name).monospace());
                ui.label(RichText::new(col.data.kind_name()).small().color(pal.dim));
                if let Data::Factor { levels, .. } = &col.data {
                    ui.label(
                        RichText::new(format!("{} levels", levels.len()))
                            .small()
                            .color(pal.dim),
                    );
                }
                if missing > 0 {
                    ui.label(
                        RichText::new(format!("{missing} missing"))
                            .small()
                            .color(pal.warning),
                    );
                }
                if let Some(label) = col.label() {
                    ui.add(
                        egui::Label::new(RichText::new(label).small().italics().color(pal.dim))
                            .truncate(),
                    );
                }
                name
            })
            .inner;
        let response = response.on_hover_text(variable_hint(col));
        if response.clicked() {
            out.push(VariableClick::Insert(rcraft_rcodegen::r_name(&col.name)));
        }
    }
}

/// What the Objects pane asks for.
pub enum ObjectClick {
    /// Make this data frame the one shown in Variables and Data.
    Open(String),
    /// Put the object's name in the Command line.
    Insert(String),
    /// From the object's right-click menu: open the dialog for this action
    /// (one of `rcraft_catalog::data::OBJECT_SPECS`) on the object.
    Dialog(&'static rcraft_rcodegen::dialog::Spec, String),
}

/// Everything in R's workspace: name, class, and size for data frames. A data
/// frame opens on a click; anything else puts its name in the Command line.
pub fn object_list(
    ui: &mut egui::Ui,
    pal: &Palette,
    items: &[WorkspaceItem],
    current: Option<&str>,
    out: &mut Vec<ObjectClick>,
) {
    if items.is_empty() {
        ui.label(RichText::new("Nothing in R's workspace yet.").color(pal.dim));
        return;
    }
    for item in items {
        let chosen = current == Some(item.name.as_str());
        let response = ui
            .horizontal(|ui| {
                let name = ui.selectable_label(chosen, RichText::new(&item.name).monospace());
                ui.label(RichText::new(&item.class).small().color(pal.dim));
                if let Some((rows, cols)) = item.dims {
                    ui.label(
                        RichText::new(format!("{rows} × {cols}"))
                            .small()
                            .color(pal.dim),
                    );
                }
                name
            })
            .inner;
        response.context_menu(|ui| {
            for spec in rcraft_catalog::data::OBJECT_SPECS {
                if ui.button(format!("{}…", spec.entry)).clicked() {
                    out.push(ObjectClick::Dialog(spec, item.name.clone()));
                    ui.close_menu();
                }
            }
            // a fitted model has its own actions (P4.5)
            if rcraft_catalog::postest::is_model_class(&item.class) {
                ui.separator();
                ui.menu_button("Model", |ui| {
                    for spec in rcraft_catalog::postest::MODEL_SPECS {
                        if ui.button(format!("{}…", spec.entry)).clicked() {
                            out.push(ObjectClick::Dialog(spec, item.name.clone()));
                            ui.close_menu();
                        }
                    }
                });
            }
        });
        let hint = if item.is_data_frame() {
            "Click to show this data frame in Variables and Data. Right-click for more."
        } else {
            "Click to put the name in the Command line. Right-click for more."
        };
        if response.on_hover_text(hint).clicked() {
            out.push(if item.is_data_frame() {
                ObjectClick::Open(item.name.clone())
            } else {
                ObjectClick::Insert(rcraft_rcodegen::r_name(&item.name))
            });
        }
    }
}

/// What was kept, shown as text only or dropped when the table came over,
/// under the variable list. Shows nothing for a table with nothing to say.
pub fn conversion_report(ui: &mut egui::Ui, pal: &Palette, table: &Table) {
    let report = Report::of(table);
    if report.is_empty() {
        return;
    }
    ui.add_space(6.0);
    egui::CollapsingHeader::new(RichText::new("Conversion report").small().color(pal.dim))
        .id_salt("conversion-report")
        .default_open(true)
        .show(ui, |ui| {
            for line in report.lines() {
                let color = if line.starts_with("Dropped") {
                    pal.warning
                } else {
                    pal.dim
                };
                ui.add(egui::Label::new(RichText::new(line).small().color(color)).wrap());
            }
        });
}

fn variable_hint(col: &Column) -> String {
    let mut hint = format!("{} ({})", col.name, col.class);
    if let Some(label) = col.label() {
        hint.push_str(&format!("\nLabel: {label}"));
    }
    let labels = col.value_labels();
    if !labels.is_empty() {
        hint.push_str("\nValue labels:");
        for (value, text) in labels.iter().take(12) {
            hint.push_str(&format!(
                "\n  {} = {text}",
                rcraft_rdata::display::number(*value)
            ));
        }
        if labels.len() > 12 {
            hint.push_str("\n  …");
        }
    }
    if let Data::Factor { levels, .. } = &col.data {
        hint.push_str(&format!("\n{} levels", levels.len()));
    }
    if col.dropped_attrs > 0 {
        hint.push_str(&format!(
            "\n{} attribute(s) could not be brought over",
            col.dropped_attrs
        ));
    }
    hint.push_str("\nClick to put the name in the Command line");
    hint
}

#[cfg(test)]
mod tests {
    use super::*;

    fn column(name: &str, data: Data) -> Column {
        Column {
            name: name.into(),
            class: "x".into(),
            attrs: Vec::new(),
            dropped_attrs: 0,
            data,
        }
    }

    #[test]
    fn the_type_filter_picks_columns_by_kind_and_labels() {
        let number = column("n", Data::Double(vec![1.0]));
        let text = column("t", Data::Text(vec![None]));
        let factor = column(
            "f",
            Data::Factor {
                levels: vec![Some("a".into())],
                codes: vec![0],
                ordered: false,
            },
        );
        let date = column("d", Data::Date(vec![0.0]));
        let flag = column("l", Data::Logical(vec![1]));
        let mut labelled = column("v", Data::Int(vec![1]));
        labelled.attrs.push(rcraft_rdata::Attr {
            key: "label".into(),
            value: rcraft_rdata::AttrValue::Text(vec![Some("Age".into())]),
            names: None,
        });
        let all = [&number, &text, &factor, &date, &flag, &labelled];
        let picked = |f: TypeFilter| -> Vec<&str> {
            all.iter()
                .filter(|c| f.matches(c))
                .map(|c| c.name.as_str())
                .collect()
        };
        assert_eq!(picked(TypeFilter::All).len(), 6);
        assert_eq!(picked(TypeFilter::Numbers), ["n", "v"]);
        assert_eq!(picked(TypeFilter::Text), ["t"]);
        assert_eq!(picked(TypeFilter::Factors), ["f"]);
        assert_eq!(picked(TypeFilter::Dates), ["d"]);
        assert_eq!(picked(TypeFilter::Logical), ["l"]);
        assert_eq!(picked(TypeFilter::Labelled), ["v"]);
    }

    #[test]
    fn widths_stay_within_bounds() {
        let table = Table {
            nrow: 2,
            columns: vec![Column {
                name: "x".repeat(500),
                class: "character".into(),
                attrs: Vec::new(),
                dropped_attrs: 0,
                data: Data::Text(vec![Some("a".into()), None]),
            }],
        };
        let shown: Vec<Shown> = table
            .columns
            .iter()
            .map(|c| Shown::new(c, LabelView::Labels))
            .collect();
        let w = column_widths(&table, &shown);
        assert_eq!(w, vec![MAX_COLUMN]);
        let empty = Table {
            nrow: 0,
            columns: Vec::new(),
        };
        assert!(column_widths(&empty, &[]).is_empty());
    }
}
