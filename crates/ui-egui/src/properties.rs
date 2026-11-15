//! The Properties pane (P8.1): what the data frame shown is, or the variable
//! picked in Variables is, as name and value pairs, worked out from the table
//! the window already has (no question to R).

use std::collections::HashSet;

use rcraft_rdata::{is_na, Column, Data, Table, INT_NA};

/// Most levels or value labels listed.
const LISTED: usize = 6;

/// The data frame's properties.
pub fn of_table(name: &str, table: &Table) -> Vec<(String, String)> {
    let labelled = table.columns.iter().filter(|c| c.label().is_some()).count();
    let missing: usize = table.columns.iter().map(|c| c.data.missing()).sum();
    let cells = table.nrow.saturating_mul(table.columns.len());
    let mut out = vec![
        ("Data frame".to_string(), name.to_string()),
        ("Rows".to_string(), table.nrow.to_string()),
        ("Variables".to_string(), table.columns.len().to_string()),
        ("With a label".to_string(), labelled.to_string()),
    ];
    if cells > 0 {
        out.push((
            "Missing cells".to_string(),
            format!("{missing} of {cells} ({})", percent(missing, cells)),
        ));
    }
    out
}

/// A variable's properties.
pub fn of_column(col: &Column) -> Vec<(String, String)> {
    let n = col.data.len();
    let missing = col.data.missing();
    let mut out = vec![("Variable".to_string(), col.name.clone())];
    if let Some(label) = col.label() {
        out.push(("Label".to_string(), label.to_string()));
    }
    out.push(("Type".to_string(), col.data.kind_name().to_string()));
    out.push(("R class".to_string(), col.class.clone()));
    out.push(("Values".to_string(), n.to_string()));
    out.push((
        "Missing".to_string(),
        if n == 0 {
            "0".to_string()
        } else {
            format!("{missing} ({})", percent(missing, n))
        },
    ));
    out.push(("Distinct".to_string(), distinct(&col.data).to_string()));
    match &col.data {
        Data::Int(_) | Data::Double(_) => {
            let v = numbers(&col.data);
            if let (Some(lo), Some(hi)) = (
                v.iter().copied().reduce(f64::min),
                v.iter().copied().reduce(f64::max),
            ) {
                out.push(("Smallest".to_string(), number(lo)));
                out.push(("Largest".to_string(), number(hi)));
                out.push((
                    "Mean".to_string(),
                    number(v.iter().sum::<f64>() / v.len() as f64),
                ));
            }
        }
        Data::Factor {
            levels, ordered, ..
        } => {
            let shown: Vec<String> = levels
                .iter()
                .take(LISTED)
                .map(|l| l.clone().unwrap_or_else(|| "NA".into()))
                .collect();
            let more = levels.len().saturating_sub(LISTED);
            out.push((
                if *ordered {
                    "Levels (in order)"
                } else {
                    "Levels"
                }
                .to_string(),
                format!(
                    "{}: {}{}",
                    levels.len(),
                    shown.join(", "),
                    if more > 0 {
                        format!(", and {more} more")
                    } else {
                        String::new()
                    }
                ),
            ));
        }
        _ => {}
    }
    let labels = col.value_labels();
    if !labels.is_empty() {
        let shown: Vec<String> = labels
            .iter()
            .take(LISTED)
            .map(|(v, l)| format!("{} = {l}", number(*v)))
            .collect();
        let more = labels.len().saturating_sub(LISTED);
        out.push((
            "Value labels".to_string(),
            format!(
                "{}{}",
                shown.join("; "),
                if more > 0 {
                    format!("; and {more} more")
                } else {
                    String::new()
                }
            ),
        ));
    }
    out
}

/// The values that are not missing, as numbers.
fn numbers(data: &Data) -> Vec<f64> {
    match data {
        Data::Int(v) => v
            .iter()
            .filter(|&&x| x != INT_NA)
            .map(|&x| f64::from(x))
            .collect(),
        Data::Double(v) => v.iter().copied().filter(|x| !x.is_nan()).collect(),
        _ => Vec::new(),
    }
}

/// How many different values that are not missing.
fn distinct(data: &Data) -> usize {
    match data {
        Data::Int(v) => v
            .iter()
            .filter(|&&x| x != INT_NA)
            .collect::<HashSet<_>>()
            .len(),
        Data::Double(v) | Data::Date(v) => v
            .iter()
            .filter(|x| !x.is_nan())
            .map(|x| x.to_bits())
            .collect::<HashSet<_>>()
            .len(),
        Data::Logical(v) => v.iter().filter(|&&x| x >= 0).collect::<HashSet<_>>().len(),
        Data::Factor { codes, .. } => codes
            .iter()
            .filter(|&&x| x >= 0)
            .collect::<HashSet<_>>()
            .len(),
        Data::DateTime { secs: v, .. } | Data::Duration { values: v, .. } => v
            .iter()
            .filter(|x| !x.is_nan() && !is_na(**x))
            .map(|x| x.to_bits())
            .collect::<HashSet<_>>()
            .len(),
        Data::Text(v) | Data::Other(v) => v.iter().flatten().collect::<HashSet<_>>().len(),
    }
}

fn percent(part: usize, whole: usize) -> String {
    if whole == 0 {
        return "0%".into();
    }
    let p = 100.0 * part as f64 / whole as f64;
    if p > 0.0 && p < 0.1 {
        "under 0.1%".into()
    } else {
        format!("{}%", trim(format!("{p:.1}")))
    }
}

/// A number as a person reads it: up to 6 significant digits, no trailing zeros.
fn number(x: f64) -> String {
    if x == x.trunc() && x.abs() < 1e15 {
        return format!("{x:.0}");
    }
    let digits = 6 - (x.abs().log10().floor() as i32 + 1);
    trim(format!("{x:.*}", digits.clamp(0, 10) as usize))
}

fn trim(s: String) -> String {
    if s.contains('.') {
        s.trim_end_matches('0').trim_end_matches('.').to_string()
    } else {
        s
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rcraft_rdata::{Attr, AttrValue};

    fn col(name: &str, data: Data, attrs: Vec<Attr>) -> Column {
        Column {
            name: name.into(),
            class: "numeric".into(),
            attrs,
            dropped_attrs: 0,
            data,
        }
    }

    fn get<'a>(p: &'a [(String, String)], key: &str) -> Option<&'a str> {
        p.iter().find(|(k, _)| k == key).map(|(_, v)| v.as_str())
    }

    #[test]
    fn a_number_variable_with_a_label_and_value_labels() {
        let c = col(
            "score",
            Data::Double(vec![1.0, 2.0, 2.0, f64::NAN, 9.5]),
            vec![
                Attr {
                    key: "label".into(),
                    value: AttrValue::Text(vec![Some("Test score".into())]),
                    names: None,
                },
                Attr {
                    key: "labels".into(),
                    value: AttrValue::Number(vec![1.0, 2.0]),
                    names: Some(vec![Some("Low".into()), Some("High".into())]),
                },
            ],
        );
        let p = of_column(&c);
        assert_eq!(get(&p, "Label"), Some("Test score"));
        assert_eq!(get(&p, "Values"), Some("5"));
        assert_eq!(get(&p, "Missing"), Some("1 (20%)"));
        assert_eq!(get(&p, "Distinct"), Some("3"));
        assert_eq!(get(&p, "Smallest"), Some("1"));
        assert_eq!(get(&p, "Largest"), Some("9.5"));
        assert_eq!(get(&p, "Mean"), Some("3.625"));
        assert_eq!(get(&p, "Value labels"), Some("1 = Low; 2 = High"));
    }

    #[test]
    fn factors_text_and_the_whole_table() {
        let levels: Vec<Option<String>> = (1..=8).map(|i| Some(format!("g{i}"))).collect();
        let f = col(
            "group",
            Data::Factor {
                levels,
                codes: vec![0, 1, 1, -1, 7],
                ordered: true,
            },
            Vec::new(),
        );
        let p = of_column(&f);
        assert_eq!(get(&p, "Distinct"), Some("3"));
        assert_eq!(
            get(&p, "Levels (in order)"),
            Some("8: g1, g2, g3, g4, g5, g6, and 2 more")
        );
        assert_eq!(get(&p, "Smallest"), None);
        let t = col(
            "town",
            Data::Text(vec![
                Some("a".into()),
                None,
                Some("a".into()),
                Some("b".into()),
                None,
            ]),
            Vec::new(),
        );
        assert_eq!(get(&of_column(&t), "Distinct"), Some("2"));
        let table = Table {
            nrow: 5,
            columns: vec![f, t],
        };
        let p = of_table("d", &table);
        assert_eq!(get(&p, "Rows"), Some("5"));
        assert_eq!(get(&p, "Variables"), Some("2"));
        assert_eq!(get(&p, "Missing cells"), Some("3 of 10 (30%)"));
        assert_eq!(number(0.000123456789), "0.000123457");
        assert_eq!(number(1234567.0), "1234567");
        assert_eq!(percent(1, 5000), "under 0.1%");
    }
}
