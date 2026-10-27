//! How a value is shown in the Data browser. Numbers follow R's printing
//! (7 significant digits), dates and date-times are shown in ISO form, and
//! missing values read `NA` as in R.

use crate::{is_na, Data, INT_NA};

/// The text for row `row` of a column, or an empty string past the end.
pub fn cell_text(data: &Data, row: usize) -> String {
    match data {
        Data::Int(v) => match v.get(row) {
            Some(&x) if x != INT_NA => x.to_string(),
            Some(_) => "NA".into(),
            None => String::new(),
        },
        Data::Double(v) => v.get(row).map_or_else(String::new, |&x| number(x)),
        Data::Logical(v) => match v.get(row) {
            Some(1) => "TRUE".into(),
            Some(0) => "FALSE".into(),
            Some(_) => "NA".into(),
            None => String::new(),
        },
        Data::Factor { levels, codes, .. } => match codes.get(row) {
            Some(&c) if c >= 0 => levels
                .get(c as usize)
                .and_then(|l| l.clone())
                .unwrap_or_else(|| "NA".into()),
            Some(_) => "NA".into(),
            None => String::new(),
        },
        Data::Date(v) => v.get(row).map_or_else(String::new, |&d| date(d)),
        Data::DateTime { secs, tz } => secs
            .get(row)
            .map_or_else(String::new, |&s| date_time(s, tz)),
        Data::Duration { values, units } => values.get(row).map_or_else(String::new, |&x| {
            if x.is_nan() {
                "NA".into()
            } else {
                format!("{} {units}", number(x))
            }
        }),
        Data::Text(v) | Data::Other(v) => match v.get(row) {
            Some(Some(s)) => one_line(s),
            Some(None) => "NA".into(),
            None => String::new(),
        },
    }
}

/// What a labelled column shows: its codes, its value labels, or both.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum LabelView {
    Values,
    #[default]
    Labels,
    Both,
}

impl LabelView {
    pub const ALL: [LabelView; 3] = [LabelView::Labels, LabelView::Values, LabelView::Both];

    pub fn label(self) -> &'static str {
        match self {
            LabelView::Values => "Values",
            LabelView::Labels => "Labels",
            LabelView::Both => "Both",
        }
    }
}

/// Like [`cell_text`], but a numeric code that has a value label shows the
/// label as `view` asks. `labels` is the column's [`crate::Column::value_labels`];
/// codes without a label, missing values and other kinds of column show as
/// they are.
pub fn labelled_text(data: &Data, labels: &[(f64, String)], row: usize, view: LabelView) -> String {
    let plain = cell_text(data, row);
    if view == LabelView::Values || labels.is_empty() {
        return plain;
    }
    let code = match data {
        Data::Int(v) => v.get(row).copied().filter(|&x| x != INT_NA).map(f64::from),
        Data::Double(v) => v.get(row).copied().filter(|x| !x.is_nan()),
        _ => None,
    };
    let Some(code) = code else {
        return plain;
    };
    match labels
        .iter()
        .find(|(v, text)| (v - code).abs() < 1e-9 && !text.is_empty())
    {
        Some((_, text)) if view == LabelView::Both => format!("{text} ({plain})"),
        Some((_, text)) => text.clone(),
        None => plain,
    }
}

/// Line breaks and tabs would break a table cell; show them as escapes.
fn one_line(s: &str) -> String {
    s.replace('\r', "\\r")
        .replace('\n', "\\n")
        .replace('\t', "\\t")
}

/// A number as R prints it: up to 7 significant digits, no trailing zeros.
pub fn number(x: f64) -> String {
    if is_na(x) {
        return "NA".into();
    }
    if x.is_nan() {
        return "NaN".into();
    }
    if x.is_infinite() {
        return if x > 0.0 { "Inf" } else { "-Inf" }.into();
    }
    if x == 0.0 {
        return "0".into();
    }
    let exponent = x.abs().log10().floor() as i32;
    if !(-5..15).contains(&exponent) {
        let s = format!("{x:.6e}");
        let (mantissa, exp) = s.split_once('e').unwrap_or((&s, "0"));
        let mantissa = trim_zeros(mantissa);
        let exp: i32 = exp.parse().unwrap_or(0);
        return format!(
            "{mantissa}e{}{:02}",
            if exp < 0 { '-' } else { '+' },
            exp.abs()
        );
    }
    let decimals = (6 - exponent).max(0) as usize;
    trim_zeros(&format!("{x:.decimals$}")).to_string()
}

fn trim_zeros(s: &str) -> &str {
    if s.contains('.') {
        s.trim_end_matches('0').trim_end_matches('.')
    } else {
        s
    }
}

/// Days since 1970-01-01 as `YYYY-MM-DD`; `NA` for a missing or absurd value.
pub fn date(days: f64) -> String {
    if !days.is_finite() || days.abs() > 1.0e8 {
        return "NA".into();
    }
    let (y, m, d) = civil_from_days(days.floor() as i64);
    format!("{y:04}-{m:02}-{d:02}")
}

/// Seconds since the epoch, shown in UTC with the zone R reported.
pub fn date_time(secs: f64, tz: &str) -> String {
    if !secs.is_finite() || secs.abs() > 8.0e15 {
        return "NA".into();
    }
    let whole = secs.floor() as i64;
    let (days, rem) = (whole.div_euclid(86_400), whole.rem_euclid(86_400));
    let (y, m, d) = civil_from_days(days);
    let zone = if tz.is_empty() { "UTC" } else { tz };
    format!(
        "{y:04}-{m:02}-{d:02} {:02}:{:02}:{:02} {zone}",
        rem / 3600,
        rem % 3600 / 60,
        rem % 60
    )
}

/// Year, month and day for a day count since 1970-01-01 (proleptic
/// Gregorian calendar).
fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    let y = yoe + era * 400 + i64::from(m <= 2);
    (y, m, d)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numbers_print_like_r() {
        assert_eq!(number(1.0), "1");
        assert_eq!(number(1.5), "1.5");
        assert_eq!(number(0.1 + 0.2), "0.3");
        assert_eq!(number(1234567.891), "1234568");
        assert_eq!(number(123456.7891), "123456.8");
        assert_eq!(number(-0.000123456789), "-0.0001234568");
        assert_eq!(number(1e-7), "1e-07");
        assert_eq!(number(1.5e20), "1.5e+20");
        assert_eq!(number(0.0), "0");
        assert_eq!(number(f64::NAN), "NaN");
        assert_eq!(number(f64::INFINITY), "Inf");
        assert_eq!(number(f64::NEG_INFINITY), "-Inf");
        assert_eq!(number(f64::from_bits(0x7FF0_0000_0000_07A2)), "NA");
    }

    #[test]
    fn dates_are_iso() {
        assert_eq!(date(0.0), "1970-01-01");
        assert_eq!(date(18321.0), "2020-02-29");
        assert_eq!(date(-1.0), "1969-12-31");
        assert_eq!(date(11016.0), "2000-02-29");
        assert_eq!(date(f64::NAN), "NA");
        assert_eq!(date(1e12), "NA");
    }

    #[test]
    fn date_times_are_shown_in_utc_with_the_zone() {
        assert_eq!(date_time(1_614_834_367.0, "UTC"), "2021-03-04 05:06:07 UTC");
        assert_eq!(date_time(946_684_800.0, ""), "2000-01-01 00:00:00 UTC");
        assert_eq!(date_time(-1.0, "UTC"), "1969-12-31 23:59:59 UTC");
        assert_eq!(date_time(f64::NAN, "UTC"), "NA");
    }

    #[test]
    fn value_labels_replace_codes_only_where_one_exists() {
        let labels = vec![(1.0, "Male".to_string()), (2.0, "Female".to_string())];
        let codes = Data::Double(vec![1.0, 2.0, 3.0, f64::NAN]);
        let show = |row, view| labelled_text(&codes, &labels, row, view);
        assert_eq!(show(0, LabelView::Labels), "Male");
        assert_eq!(show(1, LabelView::Both), "Female (2)");
        assert_eq!(show(0, LabelView::Values), "1");
        assert_eq!(show(2, LabelView::Labels), "3", "no label for 3");
        assert_eq!(show(3, LabelView::Labels), "NaN");
        assert_eq!(show(9, LabelView::Labels), "", "past the end");
        let ints = Data::Int(vec![2, INT_NA]);
        assert_eq!(
            labelled_text(&ints, &labels, 0, LabelView::Labels),
            "Female"
        );
        assert_eq!(labelled_text(&ints, &labels, 1, LabelView::Labels), "NA");
        let blank = vec![(1.0, String::new())];
        assert_eq!(labelled_text(&codes, &blank, 0, LabelView::Labels), "1");
        assert_eq!(labelled_text(&codes, &[], 0, LabelView::Labels), "1");
    }

    #[test]
    fn cells_of_every_kind() {
        assert_eq!(cell_text(&Data::Int(vec![4, INT_NA]), 0), "4");
        assert_eq!(cell_text(&Data::Int(vec![4, INT_NA]), 1), "NA");
        assert_eq!(cell_text(&Data::Int(vec![4]), 9), "");
        assert_eq!(cell_text(&Data::Logical(vec![1, 0, -1]), 0), "TRUE");
        assert_eq!(cell_text(&Data::Logical(vec![1, 0, -1]), 1), "FALSE");
        assert_eq!(cell_text(&Data::Logical(vec![1, 0, -1]), 2), "NA");
        let f = Data::Factor {
            levels: vec![Some("a".into()), Some("b".into())],
            codes: vec![1, -1, 7],
            ordered: false,
        };
        assert_eq!(cell_text(&f, 0), "b");
        assert_eq!(cell_text(&f, 1), "NA");
        assert_eq!(cell_text(&f, 2), "NA", "a bad code is not a crash");
        let t = Data::Text(vec![Some("x\ny".into()), None]);
        assert_eq!(cell_text(&t, 0), "x\\ny");
        assert_eq!(cell_text(&t, 1), "NA");
        let d = Data::Duration {
            values: vec![3.0],
            units: "mins".into(),
        };
        assert_eq!(cell_text(&d, 0), "3 mins");
    }
}
