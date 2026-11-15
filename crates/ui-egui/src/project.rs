//! Project files (P7.5): a session's script, R's objects, its log and its
//! package list kept together in one folder, with a small `.rcraft` file
//! naming them, so the work can be opened again another day or on another
//! computer.
//!
//! ```text
//! # RCraft project (version 1)
//! name<TAB>wages
//! script<TAB>script.R
//! workspace<TAB>workspace.RData
//! log<TAB>session.log
//! packages<TAB>rcraft-packages.txt
//! ```
//!
//! Each file is named relative to the project's folder and must be a plain
//! file name there (no folders, no `..`). The reader is strict and bounded.

use std::path::{Path, PathBuf};

use rcraft_rcodegen::r_string;

const HEADER: &str = "# RCraft project (version 1)";

/// Largest project file read.
pub const MAX_BYTES: u64 = 64 * 1024;

/// The ending of a project file.
pub const ENDING: &str = "rcraft";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Project {
    pub name: String,
    pub script: String,
    pub workspace: String,
    pub log: String,
    pub packages: String,
}

impl Project {
    /// A project called `name`, its script a do-file if `stata`.
    pub fn new(name: &str, stata: bool) -> Project {
        Project {
            name: name.to_string(),
            script: if stata { "script.do" } else { "script.R" }.to_string(),
            workspace: "workspace.RData".to_string(),
            log: "session.log".to_string(),
            packages: "rcraft-packages.txt".to_string(),
        }
    }

    pub fn to_text(&self) -> String {
        format!(
            "{HEADER}\nname\t{}\nscript\t{}\nworkspace\t{}\nlog\t{}\npackages\t{}\n",
            self.name.replace(['\t', '\n', '\r'], " "),
            self.script,
            self.workspace,
            self.log,
            self.packages
        )
    }

    pub fn parse(text: &str) -> Result<Project, String> {
        let mut lines = text.lines();
        if lines.next().map(str::trim_end) != Some(HEADER) {
            return Err("this is not an RCraft project file".into());
        }
        let get = |key: &str| -> Option<String> {
            text.lines()
                .filter_map(|l| l.split_once('\t'))
                .find(|(k, _)| *k == key)
                .map(|(_, v)| v.trim_end().to_string())
        };
        let file = |key: &str, v: Option<String>| -> Result<String, String> {
            let v = v.ok_or_else(|| format!("the project file names no {key}"))?;
            if is_plain_name(&v) {
                Ok(v)
            } else {
                Err(format!(
                    "the project's {key} is not a file name in its folder: {v}"
                ))
            }
        };
        let name = get("name").unwrap_or_default();
        Ok(Project {
            name,
            script: file("script", get("script"))?,
            workspace: file("workspace", get("workspace"))?,
            log: file("log", get("log"))?,
            packages: file("packages", get("packages"))?,
        })
    }

    /// Reads the project file at `path`.
    pub fn read(path: &Path) -> Result<Project, String> {
        let size = std::fs::metadata(path)
            .map_err(|e| format!("{}: {e}", path.display()))?
            .len();
        if size > MAX_BYTES {
            return Err(format!(
                "{} is too large to be a project file",
                path.display()
            ));
        }
        let text = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
        Project::parse(&text)
    }
}

/// A file name with no folder in it, not `.` or `..`, not empty.
fn is_plain_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 200
        && name != "."
        && name != ".."
        && !name.contains(['/', '\\', ':', '\n', '\r', '\t'])
}

/// The folder's name, as the project's name.
pub fn name_of(folder: &Path) -> String {
    folder
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .filter(|n| !n.is_empty())
        .unwrap_or_else(|| "project".to_string())
}

/// Where the project's file goes in `folder`.
pub fn project_file(folder: &Path) -> PathBuf {
    folder.join(format!("{}.{ENDING}", name_of(folder)))
}

fn slashes(p: &Path) -> String {
    p.display().to_string().replace('\\', "/")
}

/// R that saves R's objects and the package list of the packages this session
/// has loaded (their versions, and whether each came with R, from CRAN or
/// from elsewhere), in the form the Packages window reads.
pub fn save_code(folder: &Path, p: &Project, mirror: &str) -> String {
    let workspace = r_string(&slashes(&folder.join(&p.workspace)));
    let packages = r_string(&slashes(&folder.join(&p.packages)));
    format!(
        "local({{\n  \
         save(list = ls(globalenv(), all.names = TRUE), envir = globalenv(), file = {workspace})\n  \
         pk <- sort(loadedNamespaces())\n  \
         info <- lapply(pk, function(n) {{\n    \
         d <- tryCatch(utils::packageDescription(n), error = function(e) NULL)\n    \
         if (!is.list(d)) return(NULL)\n    \
         from <- if (identical(d$Priority, \"base\")) \"base\" \
         else if (identical(d$Repository, \"CRAN\")) \"cran\" else \"other\"\n    \
         paste(\"package\", n, d$Version, from, sep = \"\\t\")\n  \
         }})\n  \
         lines <- c(\"# RCraft package list (version 1)\", \
         paste(\"r\", paste(R.version$major, R.version$minor, sep = \".\"), sep = \"\\t\"), \
         paste(\"mirror\", {}, sep = \"\\t\"), \
         paste(\"saved\", format(Sys.Date()), sep = \"\\t\"), unlist(info))\n  \
         writeLines(lines, {packages})\n  \
         cat(\"Saved the project: \", length(ls(globalenv(), all.names = TRUE)), \" objects, \", \
         length(unlist(info)), \" packages.\\n\", sep = \"\")\n}})",
        r_string(mirror)
    )
}

/// R that moves R to the project's folder and brings its objects back.
pub fn open_code(folder: &Path, p: &Project) -> String {
    let workspace = r_string(&p.workspace);
    format!(
        "setwd({})\nif (file.exists({workspace})) {{\n  \
         loaded <- load({workspace}, envir = globalenv())\n  \
         cat(\"Opened the project: \", length(loaded), \" objects.\\n\", sep = \"\")\n  \
         rm(loaded)\n}} else cat(\"The project has no saved objects.\\n\")",
        r_string(&slashes(folder))
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_project_file_reads_back_and_odd_ones_are_refused() {
        let p = Project::new("wages\tstudy", false);
        let text = p.to_text();
        assert!(text.starts_with("# RCraft project (version 1)\nname\twages study\n"));
        let back = Project::parse(&text).unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(back.script, "script.R");
        assert_eq!(back.name, "wages study");
        assert_eq!(Project::new("x", true).script, "script.do");
        assert!(Project::parse("name\tx\n").is_err());
        for bad in ["../secret.R", "a/b.R", "C:x.R", "..", ""] {
            let t = text.replace("script\tscript.R", &format!("script\t{bad}"));
            assert!(Project::parse(&t).is_err(), "{bad}");
        }
        let missing = text.replace("log\tsession.log\n", "");
        assert!(Project::parse(&missing).is_err());
    }

    #[test]
    fn the_r_names_the_files_in_the_folder() {
        let folder = Path::new("C:\\work\\wages");
        let p = Project::new("wages", false);
        let save = save_code(folder, &p, "https://cloud.r-project.org");
        assert!(
            save.contains("file = \"C:/work/wages/workspace.RData\""),
            "{save}"
        );
        assert!(
            save.contains("writeLines(lines, \"C:/work/wages/rcraft-packages.txt\")"),
            "{save}"
        );
        let open = open_code(folder, &p);
        assert!(open.starts_with("setwd(\"C:/work/wages\")\n"), "{open}");
        assert_eq!(name_of(folder), "wages");
        assert_eq!(project_file(folder), folder.join("wages.rcraft"));
    }
}
