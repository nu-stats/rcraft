//! What came over from R with a data frame: which labels were kept, which
//! columns are shown only as text, and what could not be carried. The
//! Variables pane shows it under the list, so nothing is lost in silence.

use crate::{Column, Data, Table};

/// Counts for one table.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Report {
    pub columns: usize,
    /// Columns with a variable label (the `label` attribute).
    pub variable_labels: usize,
    /// Columns whose codes have value labels, and the labels in all.
    pub value_labelled: usize,
    pub value_labels: usize,
    /// Of those columns, the ones whose codes are text.
    pub text_labelled: usize,
    /// Columns that are not numbers, text, dates or factors, shown as text.
    pub shown_as_text: Vec<String>,
    /// Columns that lost attributes, with the number lost.
    pub lost_attrs: Vec<(String, u32)>,
    /// Columns with SPSS-style user-defined missing values.
    pub user_missing: usize,
}

impl Report {
    pub fn of(table: &Table) -> Report {
        let mut r = Report {
            columns: table.columns.len(),
            ..Report::default()
        };
        for col in &table.columns {
            if col.label().is_some() {
                r.variable_labels += 1;
            }
            let labels = col.value_label_count();
            if labels > 0 {
                r.value_labelled += 1;
                r.value_labels += labels;
                if !col.text_value_labels().is_empty() {
                    r.text_labelled += 1;
                }
            }
            if matches!(col.data, Data::Other(_)) {
                r.shown_as_text.push(col.name.clone());
            }
            if col.dropped_attrs > 0 {
                r.lost_attrs.push((col.name.clone(), col.dropped_attrs));
            }
            if has_user_missing(col) {
                r.user_missing += 1;
            }
        }
        r
    }

    /// Whether there is anything worth showing.
    pub fn is_empty(&self) -> bool {
        self.variable_labels == 0
            && self.value_labelled == 0
            && self.shown_as_text.is_empty()
            && self.lost_attrs.is_empty()
            && self.user_missing == 0
    }

    /// Lines of plain text, kept things first, then changed, then dropped.
    pub fn lines(&self) -> Vec<String> {
        let mut out = Vec::new();
        if self.variable_labels > 0 {
            out.push(format!(
                "Kept: variable labels on {} of {} columns",
                self.variable_labels, self.columns
            ));
        }
        if self.value_labelled > 0 {
            let codes = if self.text_labelled == 0 {
                "codes stay numbers"
            } else if self.text_labelled == self.value_labelled {
                "codes stay text"
            } else {
                "codes stay as they are"
            };
            out.push(format!(
                "Kept: {} on {} ({codes})",
                count(self.value_labels, "value label"),
                count(self.value_labelled, "column")
            ));
        }
        if self.user_missing > 0 {
            out.push(format!(
                "Kept: user-defined missing values on {}",
                count(self.user_missing, "column")
            ));
        }
        if !self.shown_as_text.is_empty() {
            out.push(format!(
                "Changed: shown as text only: {}",
                names(&self.shown_as_text)
            ));
        }
        if !self.lost_attrs.is_empty() {
            let total: u32 = self.lost_attrs.iter().map(|(_, n)| n).sum();
            let who: Vec<String> = self.lost_attrs.iter().map(|(c, _)| c.clone()).collect();
            out.push(format!(
                "Dropped: {total} attribute(s) RCraft cannot carry, on {}",
                names(&who)
            ));
        }
        out
    }
}

fn has_user_missing(col: &Column) -> bool {
    col.class.contains("haven_labelled_spss")
        || col
            .attrs
            .iter()
            .any(|a| a.key == "na_values" || a.key == "na_range")
}

/// `1 column`, `2 columns`.
fn count(n: usize, thing: &str) -> String {
    if n == 1 {
        format!("1 {thing}")
    } else {
        format!("{n} {thing}s")
    }
}

/// Up to four names, then a count of the rest.
fn names(all: &[String]) -> String {
    const SHOWN: usize = 4;
    let head: Vec<&str> = all.iter().take(SHOWN).map(String::as_str).collect();
    let mut s = head.join(", ");
    if all.len() > SHOWN {
        s.push_str(&format!(" and {} more", all.len() - SHOWN));
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Attr, AttrValue};

    fn col(name: &str, data: Data) -> Column {
        Column {
            name: name.into(),
            class: "numeric".into(),
            attrs: Vec::new(),
            dropped_attrs: 0,
            data,
        }
    }

    fn table(columns: Vec<Column>) -> Table {
        Table { nrow: 1, columns }
    }

    #[test]
    fn a_plain_table_has_nothing_to_report() {
        let r = Report::of(&table(vec![col("x", Data::Double(vec![1.0]))]));
        assert!(r.is_empty());
        assert!(r.lines().is_empty());
        assert_eq!(r.columns, 1);
    }

    #[test]
    fn labels_text_columns_and_lost_attributes_are_counted() {
        let mut sex = col("sex", Data::Double(vec![1.0]));
        sex.attrs.push(Attr {
            key: "label".into(),
            value: AttrValue::Text(vec![Some("Respondent sex".into())]),
            names: None,
        });
        sex.attrs.push(Attr {
            key: "labels".into(),
            value: AttrValue::Number(vec![1.0, 2.0]),
            names: Some(vec![Some("Male".into()), Some("Female".into())]),
        });
        let mut spss = col("score", Data::Double(vec![1.0]));
        spss.class = "haven_labelled_spss haven_labelled vctrs_vctr double".into();
        let mut odd = col("when", Data::Other(vec![Some("x".into())]));
        odd.dropped_attrs = 2;
        let r = Report::of(&table(vec![sex, spss, odd]));
        assert_eq!(r.variable_labels, 1);
        assert_eq!((r.value_labelled, r.value_labels), (1, 2));
        assert_eq!(r.user_missing, 1);
        assert_eq!(r.shown_as_text, vec!["when".to_string()]);
        assert_eq!(r.lost_attrs, vec![("when".to_string(), 2)]);
        let lines = r.lines();
        assert_eq!(lines.len(), 5);
        assert!(lines[0].starts_with("Kept: variable labels on 1 of 3"));
        assert!(lines[3].starts_with("Changed:"));
        assert!(lines[4].starts_with("Dropped: 2 attribute(s)"));
    }

    #[test]
    fn text_codes_with_labels_are_counted_and_said_so() {
        let mut gender = col("gender", Data::Text(vec![Some("f".into())]));
        gender.attrs.push(Attr {
            key: "labels".into(),
            value: AttrValue::Text(vec![Some("f".into()), Some("m".into())]),
            names: Some(vec![Some("Female".into()), Some("Male".into())]),
        });
        let r = Report::of(&table(vec![gender.clone()]));
        assert_eq!(
            (r.value_labelled, r.value_labels, r.text_labelled),
            (1, 2, 1)
        );
        assert_eq!(
            r.lines(),
            vec!["Kept: 2 value labels on 1 column (codes stay text)"]
        );

        let mut workshop = col("workshop", Data::Double(vec![1.0]));
        workshop.attrs.push(Attr {
            key: "labels".into(),
            value: AttrValue::Number(vec![1.0]),
            names: Some(vec![Some("R".into())]),
        });
        let r = Report::of(&table(vec![workshop, gender]));
        assert_eq!(
            r.lines(),
            vec!["Kept: 3 value labels on 2 columns (codes stay as they are)"]
        );
        assert_eq!(count(1, "column"), "1 column");
        assert_eq!(count(0, "column"), "0 columns");
    }

    #[test]
    fn long_name_lists_are_cut() {
        let all: Vec<String> = (1..=6).map(|i| format!("c{i}")).collect();
        assert_eq!(names(&all), "c1, c2, c3, c4 and 2 more");
        assert_eq!(names(&all[..2]), "c1, c2");
    }
}
