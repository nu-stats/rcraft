//! A table in Results (P2.3): a data frame, matrix or table a command returned,
//! drawn as a grid of R's own formatted cells, numbers on the right, with Copy
//! (tab-separated, so it pastes as cells into a spreadsheet or a document's
//! table) and Save as CSV. R's printed text stays in the log and the script.

use eframe::egui::{self, RichText};
use rcraft_engine::ResultTable;

use crate::look::Palette;

/// Rows drawn in Results; Copy and Save take them all.
pub const SHOWN_ROWS: usize = 200;

/// Draws `table`. `salt` tells this table's widgets from other tables'.
pub fn show(ui: &mut egui::Ui, pal: &Palette, table: &ResultTable, salt: impl std::hash::Hash) {
    let id = ui.make_persistent_id(("result-table", salt));
    ui.horizontal(|ui| {
        if !table.title.is_empty() {
            ui.label(RichText::new(&table.title).strong().color(pal.text));
        }
        if ui
            .small_button("Copy")
            .on_hover_text("Copy the table; it pastes as cells into a spreadsheet or a document")
            .clicked()
        {
            ui.ctx().copy_text(table.tsv());
            ui.data_mut(|d| d.insert_temp(id, "Copied.".to_string()));
        }
        if ui.small_button("Save as CSV…").clicked() {
            let note = save_csv(table);
            ui.data_mut(|d| d.insert_temp(id, note));
        }
        if let Some(note) = ui.data(|d| d.get_temp::<String>(id)) {
            ui.label(RichText::new(note).small().color(pal.dim));
        }
    });
    egui::ScrollArea::horizontal()
        .id_salt(id)
        .auto_shrink([false, true])
        .show(ui, |ui| {
            egui::Grid::new(id.with("grid"))
                .striped(true)
                .spacing([14.0, 2.0])
                .show(ui, |ui| {
                    cell(ui, &table.corner, pal.dim, true, false);
                    for (name, numeric) in table.columns.iter().zip(&table.numeric) {
                        cell(ui, name, pal.text, true, *numeric);
                    }
                    ui.end_row();
                    for (name, cells) in table.rows.iter().take(SHOWN_ROWS) {
                        cell(ui, name, pal.dim, false, false);
                        for (value, numeric) in cells.iter().zip(&table.numeric) {
                            let color = if value == "NA" { pal.dim } else { pal.text };
                            cell(ui, value, color, false, *numeric);
                        }
                        ui.end_row();
                    }
                });
        });
    if table.rows.len() > SHOWN_ROWS {
        ui.label(
            RichText::new(format!(
                "First {SHOWN_ROWS} of {} rows shown; Copy and Save as CSV take them all.",
                table.rows.len()
            ))
            .small()
            .italics()
            .color(pal.dim),
        );
    }
}

fn cell(ui: &mut egui::Ui, text: &str, color: egui::Color32, strong: bool, right: bool) {
    let mut rich = RichText::new(text).monospace().color(color);
    if strong {
        rich = rich.strong();
    }
    let label = egui::Label::new(rich).wrap_mode(egui::TextWrapMode::Extend);
    if right {
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.add(label);
        });
    } else {
        ui.add(label);
    }
}

/// Asks where to save, writes the CSV, and says what happened.
fn save_csv(table: &ResultTable) -> String {
    let name = if table.title.is_empty() {
        "table.csv".to_string()
    } else {
        format!("{}.csv", file_stem(&table.title))
    };
    let Some(path) = rfd::FileDialog::new()
        .set_file_name(name)
        .add_filter("CSV (comma-separated)", &["csv"])
        .save_file()
    else {
        return String::new();
    };
    match std::fs::write(&path, table.csv()) {
        Ok(()) => format!("Saved {}.", path.display()),
        Err(e) => format!("Could not save: {e}"),
    }
}

/// A title as a file name: letters, digits, `-` and `_`, other characters as `_`.
fn file_stem(title: &str) -> String {
    let stem: String = title
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .take(60)
        .collect();
    if stem.is_empty() {
        "table".into()
    } else {
        stem
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_title_makes_a_safe_file_name() {
        assert_eq!(file_stem("By group: a/b"), "By_group__a_b");
        assert_eq!(file_stem(""), "table");
        assert_eq!(file_stem("é"), "é");
        assert_eq!(file_stem(&"x".repeat(100)).len(), 60);
    }
}
