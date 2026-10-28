//! A table of text columns: what both source files are reduced to before the
//! index is built. Only the columns asked for are kept, which is what makes
//! the cached copy small.

use crate::rds::{Data, Node};
use crate::{Error, Result};

/// Text columns of equal length. `None` is a missing value.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Table {
    pub names: Vec<String>,
    pub columns: Vec<Vec<Option<String>>>,
    pub rows: usize,
}

impl Table {
    /// The column called `name`.
    pub fn column(&self, name: &str) -> Option<&[Option<String>]> {
        let i = self.names.iter().position(|n| n == name)?;
        self.columns.get(i).map(|c| c.as_slice())
    }

    /// The value in row `row` of column `name`.
    pub fn get(&self, name: &str, row: usize) -> Option<&str> {
        self.column(name)?.get(row)?.as_deref()
    }
}

/// Reduces an R value to a [`Table`] keeping the columns in `keep` (those that
/// exist). The value must be a text matrix with column names (what
/// `tools::CRAN_package_db()` returns) or a list of text columns with names (a
/// data frame).
pub fn from_node(node: &Node, keep: &[&str]) -> Result<Table> {
    match &node.data {
        Data::Strings(cells) => from_matrix(node, cells, keep),
        Data::List(cols) => from_frame(node, cols, keep),
        _ => Err(Error::Format(
            "expected a table of text, found something else".into(),
        )),
    }
}

fn names_of(node: &Node) -> Option<Vec<String>> {
    match &node.data {
        Data::Strings(v) => Some(v.iter().map(|s| s.clone().unwrap_or_default()).collect()),
        _ => None,
    }
}

fn from_matrix(node: &Node, cells: &[Option<String>], keep: &[&str]) -> Result<Table> {
    let dim = match node.attr("dim").map(|d| &d.data) {
        Some(Data::Ints(d)) => d.as_slice(),
        _ => return Err(Error::Format("a text vector with no dimensions".into())),
    };
    let (nr, nc) = match dim {
        [r, c] => (
            usize::try_from(*r).map_err(|_| Error::Format("a negative dimension".into()))?,
            usize::try_from(*c).map_err(|_| Error::Format("a negative dimension".into()))?,
        ),
        _ => return Err(Error::Format("expected a table with two dimensions".into())),
    };
    if nr.checked_mul(nc) != Some(cells.len()) {
        return Err(Error::Format(
            "the dimensions do not match the cells".into(),
        ));
    }
    let colnames = match node.attr("dimnames").map(|d| &d.data) {
        Some(Data::List(l)) => l.get(1).and_then(names_of),
        _ => None,
    }
    .ok_or_else(|| Error::Format("the table has no column names".into()))?;
    if colnames.len() != nc {
        return Err(Error::Format("the column names do not match".into()));
    }
    let mut table = Table {
        rows: nr,
        ..Table::default()
    };
    for want in keep {
        let Some(j) = colnames.iter().position(|n| n == want) else {
            continue;
        };
        // column-major: column j is the cells j*nr .. (j+1)*nr
        let start = j * nr;
        let col = cells
            .get(start..start + nr)
            .ok_or_else(|| Error::Format("the dimensions do not match the cells".into()))?;
        table.names.push((*want).to_string());
        table.columns.push(col.to_vec());
    }
    Ok(table)
}

fn from_frame(node: &Node, cols: &[Node], keep: &[&str]) -> Result<Table> {
    let names = node
        .attr("names")
        .and_then(names_of)
        .ok_or_else(|| Error::Format("the table has no column names".into()))?;
    if names.len() != cols.len() {
        return Err(Error::Format("the column names do not match".into()));
    }
    let mut table = Table::default();
    let mut rows = None;
    for want in keep {
        let Some(j) = names.iter().position(|n| n == want) else {
            continue;
        };
        let Some(Node {
            data: Data::Strings(col),
            ..
        }) = cols.get(j)
        else {
            return Err(Error::Format(format!("column {want} is not text")));
        };
        match rows {
            None => rows = Some(col.len()),
            Some(n) if n != col.len() => {
                return Err(Error::Format("columns of different lengths".into()))
            }
            Some(_) => {}
        }
        table.names.push((*want).to_string());
        table.columns.push(col.clone());
    }
    table.rows = match rows {
        Some(n) => n,
        // no kept column was found: the length of the first column, if text
        None => match cols.first().map(|c| &c.data) {
            Some(Data::Strings(c)) => c.len(),
            _ => 0,
        },
    };
    Ok(table)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s(items: &[Option<&str>]) -> Node {
        Node {
            data: Data::Strings(items.iter().map(|s| s.map(str::to_string)).collect()),
            attrs: Vec::new(),
        }
    }

    fn matrix() -> Node {
        // 2 rows, 3 columns, column-major
        let mut n = s(&[Some("a"), Some("b"), Some("1"), None, Some("x"), Some("y")]);
        n.attrs.push((
            "dim".into(),
            Node {
                data: Data::Ints(vec![2, 3]),
                attrs: vec![],
            },
        ));
        n.attrs.push((
            "dimnames".into(),
            Node {
                data: Data::List(vec![
                    Node {
                        data: Data::Null,
                        attrs: vec![],
                    },
                    s(&[Some("Package"), Some("Version"), Some("Title")]),
                ]),
                attrs: vec![],
            },
        ));
        n
    }

    #[test]
    fn a_matrix_keeps_only_the_columns_asked_for_in_the_order_asked() {
        let t = from_node(&matrix(), &["Title", "Package", "Nope"]);
        let t = t.unwrap_or_default();
        assert_eq!(t.names, vec!["Title", "Package"]);
        assert_eq!(t.rows, 2);
        assert_eq!(t.get("Package", 1), Some("b"));
        assert_eq!(t.get("Title", 0), Some("x"));
        assert_eq!(t.get("Version", 0), None);
        // missing value in the Version column, not asked for here
        let t = from_node(&matrix(), &["Version"]).unwrap_or_default();
        assert_eq!(t.get("Version", 0), Some("1"));
        assert_eq!(t.get("Version", 1), None);
    }

    #[test]
    fn a_data_frame_of_text_columns_is_read_the_same_way() {
        let mut frame = Node {
            data: Data::List(vec![s(&[Some("a"), Some("b")]), s(&[Some("1"), None])]),
            attrs: vec![],
        };
        frame
            .attrs
            .push(("names".into(), s(&[Some("Package"), Some("Version")])));
        let t = from_node(&frame, &["Version", "Package"]).unwrap_or_default();
        assert_eq!(t.rows, 2);
        assert_eq!(t.get("Package", 0), Some("a"));
        assert_eq!(t.get("Version", 1), None);
    }

    #[test]
    fn wrong_shapes_are_errors() {
        let mut bad = matrix();
        bad.attrs.retain(|(k, _)| k != "dimnames");
        assert!(from_node(&bad, &["Package"]).is_err());
        let mut bad = matrix();
        bad.attrs[0].1.data = Data::Ints(vec![5, 5]);
        assert!(from_node(&bad, &["Package"]).is_err());
        assert!(from_node(
            &Node {
                data: Data::Ints(vec![1]),
                attrs: vec![]
            },
            &["Package"]
        )
        .is_err());
    }
}
