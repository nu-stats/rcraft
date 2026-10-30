//! What is in R's workspace, as the interface shows it (the Variables and
//! Objects panes). Found by hidden queries that never reach the history, the
//! log or the Results window.

/// Most objects listed.
pub const MAX_OBJECTS: usize = 5000;

/// A table with more cells than this is not brought over again after every
/// command, only when asked.
pub const AUTO_REFRESH_CELLS: u64 = 5_000_000;

/// R code that prints one line per object in the workspace:
/// `name`, `class`, `rows`, `cols`, separated by tabs (`NA` when not a
/// data frame). Tabs and line breaks in names are escaped.
pub const LISTING_R: &str = r#"invisible(local({
  esc <- function(x) {
    x <- gsub("\\", "\\\\", x, fixed = TRUE)
    x <- gsub("\n", "\\n", x, fixed = TRUE)
    x <- gsub("\r", "\\r", x, fixed = TRUE)
    gsub("\t", "\\t", x, fixed = TRUE)
  }
  cat("\t", "wd", "\t", esc(getwd()), "\n", sep = "")
  nm <- ls(globalenv())
  nm <- nm[seq_len(min(length(nm), 5000L))]
  for (n in nm) {
    info <- tryCatch({
      x <- get(n, envir = globalenv())
      cls <- class(x)[[1L]]
      if (is.data.frame(x)) c(cls, nrow(x), ncol(x)) else c(cls, NA, NA)
    }, error = function(e) c("error", NA, NA))
    cat(esc(n), info[[1L]], info[[2L]], info[[3L]], sep = "\t")
    cat("\n")
  }
}))"#;

/// One object in the workspace.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WorkspaceItem {
    pub name: String,
    /// R's first class name (`"data.frame"`, `"tbl_df"`, `"lm"`, `"function"`).
    pub class: String,
    /// Set for data frames.
    pub dims: Option<(usize, usize)>,
}

impl WorkspaceItem {
    pub fn is_data_frame(&self) -> bool {
        self.dims.is_some()
    }

    /// Rows times columns, for data frames.
    pub fn cells(&self) -> u64 {
        self.dims
            .map_or(0, |(r, c)| (r as u64).saturating_mul(c as u64))
    }
}

/// The folder R is working in, from the listing's first line (which starts with a
/// tab, as no object name can: tabs in names are escaped).
pub fn parse_working_dir(text: &str) -> Option<String> {
    let line = text.lines().find(|l| l.starts_with('\t'))?;
    let mut f = line.split('\t').skip(1);
    (f.next()? == "wd").then_some(())?;
    let dir = unescape(f.next()?);
    (!dir.is_empty()).then_some(dir)
}

/// Reads the listing's output. A line that cannot be read is skipped.
pub fn parse_listing(text: &str) -> Vec<WorkspaceItem> {
    let mut items: Vec<WorkspaceItem> = text
        .lines()
        .filter(|l| !l.starts_with('\t'))
        .filter_map(|line| {
            let mut f = line.split('\t');
            let name = unescape(f.next()?);
            let class = f.next()?.to_string();
            let rows = f.next()?.parse::<usize>().ok();
            let cols = f.next()?.parse::<usize>().ok();
            Some(WorkspaceItem {
                name,
                class,
                dims: rows.zip(cols),
            })
        })
        .take(MAX_OBJECTS)
        .collect();
    items.sort_by(|a, b| a.name.cmp(&b.name));
    items
}

fn unescape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        if c != '\\' {
            out.push(c);
            continue;
        }
        match chars.next() {
            Some('n') => out.push('\n'),
            Some('r') => out.push('\r'),
            Some('t') => out.push('\t'),
            Some('\\') => out.push('\\'),
            Some(other) => {
                out.push('\\');
                out.push(other);
            }
            None => out.push('\\'),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_listing_is_read_and_sorted() {
        let items = parse_listing("b\tdata.frame\t3\t2\na\tfunction\tNA\tNA\n");
        assert_eq!(items.len(), 2);
        assert_eq!(items[0].name, "a");
        assert!(!items[0].is_data_frame());
        assert_eq!(items[1].dims, Some((3, 2)));
        assert_eq!(items[1].cells(), 6);
    }

    #[test]
    fn the_working_folder_is_the_line_that_starts_with_a_tab_and_is_not_an_object() {
        let text = "\twd\tC:/data/my work\nb\tdata.frame\t3\t2\n";
        assert_eq!(parse_working_dir(text).as_deref(), Some("C:/data/my work"));
        let items = parse_listing(text);
        assert_eq!(items.len(), 1, "the folder line is not an object");
        assert_eq!(items[0].name, "b");
        assert_eq!(parse_working_dir("b\tdata.frame\t3\t2\n"), None);
        assert_eq!(parse_working_dir("\twd\t\n"), None);
        assert_eq!(parse_working_dir(""), None);
        // R escapes a backslash and a tab inside the folder's name
        assert_eq!(
            parse_working_dir("\twd\tC:\\\\dir\\ttab\n").as_deref(),
            Some("C:\\dir\ttab")
        );
    }

    #[test]
    fn escaped_names_and_junk_lines() {
        let items = parse_listing("we\\tird\\\\name\tnumeric\tNA\tNA\nnot a line\n\n");
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].name, "we\tird\\name");
    }
}
