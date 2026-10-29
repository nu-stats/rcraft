//! A package's own documents (P2A.4): its page (description and the index of
//! its help topics), its vignettes, its NEWS file, and the text of a vignette
//! source or code file. Like the help pages, R finds them in the installed
//! package, so they match what is installed here and work with no network.
//!
//! Vignettes are listed with the files that exist for each: a rendered PDF or
//! HTML (opened outside RCraft, by the system, only on a click), the R code
//! extracted from it, and its source (`.Rmd`, `.Rnw`, `.qmd`), the last two
//! shown as text in the Help window.

use crate::import::r_string;

/// Most lines of a NEWS or vignette file shown.
pub const MAX_TEXT_LINES: usize = 4000;

/// How a vignette file can be used.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileKind {
    /// A rendered document: opened by the system (PDF, HTML).
    Document,
    /// R code extracted from the vignette: shown, and can be run.
    Code,
    /// The vignette's source text (`.Rmd`, `.Rnw`, `.qmd`, `.md`): shown.
    Source,
}

impl FileKind {
    pub fn label(self) -> &'static str {
        match self {
            FileKind::Document => "Open",
            FileKind::Code => "R code",
            FileKind::Source => "Source",
        }
    }
}

/// The file's ending in lower case, without the dot.
pub fn extension(path: &str) -> String {
    path.rsplit_once('.')
        .map(|(_, e)| e.to_ascii_lowercase())
        .unwrap_or_default()
}

/// What a vignette file is for, from its ending; `None` for any other.
pub fn kind_of(path: &str) -> Option<FileKind> {
    match extension(path).as_str() {
        "pdf" | "html" | "htm" => Some(FileKind::Document),
        "r" => Some(FileKind::Code),
        "rmd" | "rnw" | "qmd" | "md" | "txt" => Some(FileKind::Source),
        _ => None,
    }
}

/// The address the system opens for a local file: `file:///C:/a%20b/x.pdf`.
pub fn file_url(path: &str) -> String {
    let p = path.replace('\\', "/");
    let mut out = String::from(if p.starts_with('/') {
        "file://"
    } else {
        "file:///"
    });
    for b in p.bytes() {
        let c = char::from(b);
        if c.is_ascii_alphanumeric() || matches!(c, '/' | ':' | '-' | '_' | '.' | '~') {
            out.push(c);
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}

/// What the code builders print for a name that is not a package name, so R is
/// never asked about it.
const NO_ANSWER: &str = "cat(\"@@NONE\\n@@DONE\\n\")";
const NO_ANSWER_DONE: &str = "cat(\"@@DONE\\n\")";

/// One vignette and the files R has for it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Vignette {
    pub package: String,
    pub item: String,
    pub title: String,
    /// Full paths, only of kinds RCraft can use ([`kind_of`]).
    pub files: Vec<String>,
}

/// One topic of a package's help index.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Topic {
    pub name: String,
    pub title: String,
}

/// A package's page.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PackageDoc {
    pub package: String,
    /// The DESCRIPTION fields as R prints them.
    pub description: String,
    pub topics: Vec<Topic>,
    pub vignettes: Vec<Vignette>,
    /// Whether the package has a NEWS file.
    pub has_news: bool,
}

/// What R answered about a package.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PackageAnswer {
    Found(Box<PackageDoc>),
    /// No such installed package.
    NotInstalled,
}

/// Whether a package's DESCRIPTION (as R prints it) says it is a base package
/// (`Priority: base`): stats, utils, grid and the rest that come with R. Their
/// reference is part of R's own manual, not a PDF of their own on CRAN.
pub fn is_base_package(description: &str) -> bool {
    description.lines().any(|l| {
        l.split_once(':')
            .is_some_and(|(k, v)| k.trim() == "Priority" && v.trim() == "base")
    })
}

/// Where CRAN keeps the reference manual (a PDF) of `package`, on `mirror`;
/// for a base package, R's full reference manual, which covers it. The
/// address is opened by the user's browser on a click; RCraft sends nothing.
pub fn manual_url(mirror: &str, package: &str, base: bool) -> String {
    let root = mirror.trim_end_matches('/');
    if base {
        format!("{root}/doc/manuals/r-release/fullrefman.pdf")
    } else {
        format!("{root}/web/packages/{package}/{package}.pdf")
    }
}

/// R code that prints the vignettes of `package` (or of every installed
/// package) as `@@VIG\tpackage\titem\ttitle` with one `@@VFILE\tpath` line per
/// file, then `@@DONE`. Shared by the package page and the vignette list.
fn vignette_lines(selector: &str) -> String {
    format!(
        "vig <- tryCatch(utils::vignette(package = {selector})$results, error = function(e) NULL)\n\
         if (!is.null(vig)) for (i in seq_len(nrow(vig))) {{\n\
           dir <- file.path(vig[i, \"LibPath\"], vig[i, \"Package\"], \"doc\")\n\
           files <- list.files(dir)\n\
           files <- files[startsWith(files, paste0(vig[i, \"Item\"], \".\"))]\n\
           cat(\"@@VIG\\t\", vig[i, \"Package\"], \"\\t\", vig[i, \"Item\"], \"\\t\", gsub(\"[\\t\\r\\n]+\", \" \", vig[i, \"Title\"]), \"\\n\", sep = \"\")\n\
           for (f in files) cat(\"@@VFILE\\t\", file.path(dir, f), \"\\n\", sep = \"\")\n\
         }}\n"
    )
}

/// R code for a package's page: see [`parse_package`]. The package name is
/// written into the code as a string, because `help(package = )` takes the
/// name of a variable as the package's name.
pub fn package_code(package: &str) -> String {
    if !crate::help::is_package(package) {
        return NO_ANSWER.to_string();
    }
    let lit = r_string(package);
    format!(
        "local({{\n\
         info <- tryCatch(utils::help(package = {lit})$info, error = function(e) NULL)\n\
         if (is.null(info) || length(info) < 2L) {{\n\
           cat(\"@@NONE\\n\")\n\
         }} else {{\n\
           cat(\"@@PACKAGE\\t\", {lit}, \"\\n\", sep = \"\")\n\
           cat(\"@@DESC\\n\"); for (l in info[[1L]]) cat(\"|\", l, \"\\n\", sep = \"\"); cat(\"@@END\\n\")\n\
           cat(\"@@INDEX\\n\"); for (l in info[[2L]]) cat(\"|\", l, \"\\n\", sep = \"\"); cat(\"@@END\\n\")\n\
           root <- system.file(package = {lit})\n\
           if (any(file.exists(file.path(root, c(\"NEWS.md\", \"NEWS\", \"NEWS.Rd\"))))) cat(\"@@NEWS\\n\")\n\
           {vignettes}\
         }}\n\
         cat(\"@@DONE\\n\")\n\
         }})",
        lit = lit,
        vignettes = vignette_lines(&lit),
    )
}

/// R code for the vignettes of one installed package, or of all of them: see
/// [`parse_vignettes`].
pub fn vignettes_code(package: Option<&str>) -> String {
    if package.is_some_and(|p| !crate::help::is_package(p)) {
        return NO_ANSWER_DONE.to_string();
    }
    let selector = package.map_or(".packages(all.available = TRUE)".to_string(), r_string);
    format!(
        "local({{\n{}cat(\"@@DONE\\n\")\n}})",
        vignette_lines(&selector)
    )
}

/// R code for a package's NEWS file as text: see [`parse_text`]. It reads
/// `NEWS.md`, `NEWS` or `NEWS.Rd` (rendered to text), whichever the package
/// has, and cuts it at [`MAX_TEXT_LINES`].
pub fn news_code(package: &str) -> String {
    if !crate::help::is_package(package) {
        return NO_ANSWER.to_string();
    }
    let lit = r_string(package);
    format!(
        "local({{\n\
         root <- system.file(package = {lit})\n\
         found <- c(\"NEWS.md\", \"NEWS\", \"NEWS.Rd\")\n\
         found <- found[file.exists(file.path(root, found))]\n\
         if (!nzchar(root) || !length(found)) {{\n\
           cat(\"@@NONE\\n\")\n\
         }} else {{\n\
           path <- file.path(root, found[1L])\n\
           lines <- if (identical(found[1L], \"NEWS.Rd\")) {{\n\
             tf <- tempfile(); tools::Rd2txt(path, out = tf); readLines(tf, warn = FALSE, encoding = \"UTF-8\")\n\
           }} else readLines(path, warn = FALSE, encoding = \"UTF-8\")\n\
           cat(\"@@TITLE\\t\", \"News for \", {lit}, \"\\n\", sep = \"\")\n\
           cat(\"@@TEXT\\n\"); for (l in head(lines, {max})) cat(\"|\", l, \"\\n\", sep = \"\"); cat(\"@@END\\n\")\n\
         }}\n\
         }})",
        lit = lit,
        max = MAX_TEXT_LINES,
    )
}

/// R code that prints a vignette file (its source or R code) as text, only
/// when the file is inside an installed package's folder: see [`parse_text`].
pub fn file_text_code(path: &str) -> String {
    format!(
        "local({{\n\
         path <- {path}\n\
         norm <- function(p) normalizePath(p, winslash = \"/\", mustWork = FALSE)\n\
         libs <- paste0(norm(.libPaths()), \"/\")\n\
         p <- norm(path)\n\
         if (!file.exists(p) || !any(startsWith(p, libs)) || file.size(p) > 5e6) {{\n\
           cat(\"@@NONE\\n\")\n\
         }} else {{\n\
           lines <- readLines(p, warn = FALSE, encoding = \"UTF-8\")\n\
           cat(\"@@TITLE\\t\", basename(p), \"\\n\", sep = \"\")\n\
           cat(\"@@TEXT\\n\"); for (l in head(lines, {max})) cat(\"|\", l, \"\\n\", sep = \"\"); cat(\"@@END\\n\")\n\
         }}\n\
         }})",
        path = r_string(path),
        max = MAX_TEXT_LINES,
    )
}

/// The title of a vignette as R lists it, without the "(source, pdf)" note.
fn clean_title(title: &str) -> String {
    let t = title.trim();
    match t.rfind(" (") {
        Some(i) if t.ends_with(')') && t.get(i..).is_some_and(|s| s.contains("source")) => {
            t.get(..i).unwrap_or(t).trim().to_string()
        }
        _ => t.to_string(),
    }
}

/// Reads `@@VIG` and `@@VFILE` lines.
fn read_vignettes(answer: &str) -> Vec<Vignette> {
    let mut out: Vec<Vignette> = Vec::new();
    for l in answer.lines() {
        if let Some(rest) = l.strip_prefix("@@VIG\t") {
            let mut f = rest.splitn(3, '\t');
            if let (Some(p), Some(i), Some(t)) = (f.next(), f.next(), f.next()) {
                out.push(Vignette {
                    package: p.to_string(),
                    item: i.to_string(),
                    title: clean_title(t),
                    files: Vec::new(),
                });
            }
        } else if let Some(path) = l.strip_prefix("@@VFILE\t") {
            if let (Some(v), Some(_)) = (out.last_mut(), kind_of(path)) {
                v.files.push(path.to_string());
            }
        }
    }
    out
}

/// Reads R's answer to [`vignettes_code`]; `None` when R did not finish.
/// Vignettes come in R's order, which is by package.
pub fn parse_vignettes(answer: &str) -> Option<Vec<Vignette>> {
    answer
        .lines()
        .any(|l| l == "@@DONE")
        .then(|| read_vignettes(answer))
}

/// The lines of a `|`-prefixed block that starts at the line `start`, up to
/// `@@END`.
fn block<'a>(answer: &'a str, start: &str) -> Vec<&'a str> {
    let mut out = Vec::new();
    let mut inside = false;
    for l in answer.lines() {
        if inside {
            if l == "@@END" {
                break;
            }
            if let Some(body) = l.strip_prefix('|') {
                out.push(body);
            }
        } else if l == start {
            inside = true;
        }
    }
    out
}

/// A package's help index as R prints it: a topic starts at the left margin,
/// its title may continue on indented lines.
fn read_index(lines: &[&str]) -> Vec<Topic> {
    let mut out: Vec<Topic> = Vec::new();
    for l in lines {
        if l.starts_with(char::is_whitespace) {
            if let Some(last) = out.last_mut() {
                if !l.trim().is_empty() {
                    last.title.push(' ');
                    last.title.push_str(l.trim());
                }
            }
        } else if let Some((name, title)) = l.split_once(char::is_whitespace) {
            out.push(Topic {
                name: name.to_string(),
                title: title.trim().to_string(),
            });
        } else if !l.trim().is_empty() {
            out.push(Topic {
                name: l.trim().to_string(),
                title: String::new(),
            });
        }
    }
    // section headings some packages put in their index (`Functions:`, `=====`)
    // are not topics
    out.retain(|t| {
        let heading = t.title.is_empty()
            && (t.name.ends_with(':') || t.name.chars().all(|c| matches!(c, '=' | '-' | '_')));
        !heading
    });
    out
}

/// Reads R's answer to [`package_code`].
pub fn parse_package(answer: &str) -> Option<PackageAnswer> {
    if answer.lines().any(|l| l == "@@NONE") {
        return Some(PackageAnswer::NotInstalled);
    }
    if !answer.lines().any(|l| l == "@@DONE") {
        return None;
    }
    let package = answer.lines().find_map(|l| l.strip_prefix("@@PACKAGE\t"))?;
    Some(PackageAnswer::Found(Box::new(PackageDoc {
        package: package.to_string(),
        description: block(answer, "@@DESC").join("\n"),
        topics: read_index(&block(answer, "@@INDEX")),
        vignettes: read_vignettes(answer),
        has_news: answer.lines().any(|l| l == "@@NEWS"),
    })))
}

/// A text R printed: a NEWS file, a vignette source.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextDoc {
    pub title: String,
    pub text: String,
}

/// What R answered to [`news_code`] or [`file_text_code`]: the text, or
/// `Ok(None)` when there is none (no NEWS file, a file outside the packages).
/// `Err(())` when the answer is not an answer.
#[allow(clippy::result_unit_err)]
pub fn parse_text(answer: &str) -> Result<Option<TextDoc>, ()> {
    if answer.lines().any(|l| l == "@@NONE") {
        return Ok(None);
    }
    let Some(title) = answer.lines().find_map(|l| l.strip_prefix("@@TITLE\t")) else {
        return Err(());
    };
    if !answer.lines().any(|l| l == "@@TEXT") {
        return Err(());
    }
    Ok(Some(TextDoc {
        title: title.to_string(),
        text: block(answer, "@@TEXT").join("\n"),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn file_kinds_and_addresses() {
        assert_eq!(
            kind_of("C:/lib/grid/doc/grid.pdf"),
            Some(FileKind::Document)
        );
        assert_eq!(kind_of("x/intro.HTML"), Some(FileKind::Document));
        assert_eq!(kind_of("x/intro.R"), Some(FileKind::Code));
        assert_eq!(kind_of("x/intro.Rmd"), Some(FileKind::Source));
        assert_eq!(kind_of("x/intro.Rnw"), Some(FileKind::Source));
        assert_eq!(kind_of("x/changes.xyz"), None);
        assert_eq!(kind_of("noending"), None);
        assert_eq!(
            file_url("C:\\Users\\a b\\lib\\x.pdf"),
            "file:///C:/Users/a%20b/lib/x.pdf"
        );
        assert_eq!(file_url("/home/me/é.pdf"), "file:///home/me/%C3%A9.pdf");
    }

    #[test]
    fn the_cran_manual_address_and_base_packages() {
        assert_eq!(
            manual_url("https://cloud.r-project.org/", "MASS", false),
            "https://cloud.r-project.org/web/packages/MASS/MASS.pdf"
        );
        assert_eq!(
            manual_url("https://cran.example.org", "grid", true),
            "https://cran.example.org/doc/manuals/r-release/fullrefman.pdf"
        );
        assert!(is_base_package(
            "Package:  grid
Priority: base
Version: 4.4.3"
        ));
        assert!(!is_base_package(
            "Package: MASS
Priority: recommended"
        ));
        assert!(!is_base_package(
            "Package: x
Title: Priority: base things"
        ));
        assert!(!is_base_package(""));
    }

    #[test]
    fn code_names_the_package_as_a_string() {
        let code = package_code("stats");
        assert!(code.contains(r#"utils::help(package = "stats")"#), "{code}");
        // a name that is not a package name never reaches R
        for bad in ["a\"b", "", "x y", "1x", r"a\b"] {
            assert_eq!(package_code(bad), NO_ANSWER, "{bad}");
            assert_eq!(news_code(bad), NO_ANSWER, "{bad}");
            assert_eq!(vignettes_code(Some(bad)), NO_ANSWER_DONE, "{bad}");
        }
        assert!(vignettes_code(None).contains(".packages(all.available = TRUE)"));
        assert!(vignettes_code(Some("grid")).contains(r#"vignette(package = "grid")"#));
        assert!(file_text_code("x\\y").contains(r#"path <- "x\\y""#));
        assert!(news_code("MASS").contains("NEWS.Rd"));
    }

    #[test]
    fn a_package_page_is_read() {
        let answer = "@@PACKAGE\tdemo\n@@DESC\n|Package:  demo\n|Version:  1.0\n@@END\n\
                      @@INDEX\n|alpha        The first thing\n|beta         A second thing with a long\n|             title\n|gamma\n@@END\n\
                      @@NEWS\n\
                      @@VIG\tdemo\tintro\tAn Introduction (source, pdf)\n\
                      @@VFILE\tC:/lib/demo/doc/intro.pdf\n@@VFILE\tC:/lib/demo/doc/intro.R\n\
                      @@VFILE\tC:/lib/demo/doc/intro.Rmd\n@@VFILE\tC:/lib/demo/doc/intro.xyz\n\
                      @@DONE\n";
        let Some(PackageAnswer::Found(p)) = parse_package(answer) else {
            panic!("not a package");
        };
        assert_eq!(p.package, "demo");
        assert_eq!(p.description, "Package:  demo\nVersion:  1.0");
        assert_eq!(
            p.topics,
            vec![
                Topic {
                    name: "alpha".into(),
                    title: "The first thing".into()
                },
                Topic {
                    name: "beta".into(),
                    title: "A second thing with a long title".into()
                },
                Topic {
                    name: "gamma".into(),
                    title: String::new()
                },
            ]
        );
        assert!(p.has_news);
        assert_eq!(p.vignettes.len(), 1);
        assert_eq!(p.vignettes[0].title, "An Introduction");
        assert_eq!(p.vignettes[0].files.len(), 3, "the .xyz file is not kept");
    }

    #[test]
    fn index_headings_are_not_topics() {
        let answer = "@@PACKAGE	p
@@INDEX
|Functions:
|=========
|alpha  First
|---
|Data:
|beta   Second
@@END
@@DONE
";
        let Some(PackageAnswer::Found(p)) = parse_package(answer) else {
            panic!("not a package");
        };
        let names: Vec<&str> = p.topics.iter().map(|t| t.name.as_str()).collect();
        assert_eq!(names, vec!["alpha", "beta"]);
    }

    #[test]
    fn a_missing_package_and_a_broken_answer() {
        assert_eq!(
            parse_package("@@NONE\n@@DONE\n"),
            Some(PackageAnswer::NotInstalled)
        );
        assert_eq!(parse_package(""), None);
        assert_eq!(parse_package("@@PACKAGE\tx\n"), None);
        assert_eq!(parse_vignettes("@@VIG\ta\tb\tc\n"), None);
        assert_eq!(parse_vignettes("@@DONE\n"), Some(Vec::new()));
    }

    #[test]
    fn titles_lose_only_the_formats_note() {
        assert_eq!(
            clean_title("Introduction to grid (source, pdf)"),
            "Introduction to grid"
        );
        assert_eq!(
            clean_title("Using x (the new way) (source, html)"),
            "Using x (the new way)"
        );
        assert_eq!(clean_title("About (things)"), "About (things)");
        assert_eq!(clean_title("Plain"), "Plain");
    }

    #[test]
    fn a_text_is_read_or_is_absent() {
        let answer = "@@TITLE\tNews for demo\n@@TEXT\n|# demo 1.0\n|\n|* first release\n@@END\n";
        assert_eq!(
            parse_text(answer),
            Ok(Some(TextDoc {
                title: "News for demo".into(),
                text: "# demo 1.0\n\n* first release".into()
            }))
        );
        assert_eq!(parse_text("@@NONE\n"), Ok(None));
        assert_eq!(parse_text("Error: boom"), Err(()));
        assert_eq!(parse_text("@@TITLE\tx\n"), Err(()));
    }
}
