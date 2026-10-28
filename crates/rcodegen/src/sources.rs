//! R code for installing from the sources beyond CRAN (`plan/providers.md`):
//! a local file or folder, a GitHub repository, an r-universe and
//! Bioconductor. Only base R is used for files and GitHub, and BiocManager for
//! Bioconductor, so the code shown is the code anyone could run.
//!
//! Every name and path is checked before it goes into the code, and quoted.

use rcraft_rpkg::provider::{
    github_archive_url, is_git_ref, is_github_owner, is_github_repo, is_universe, universe_url,
};

use crate::import::r_string;
use crate::packages::{install_code, is_package_name, InstallFrom};

/// What kind of local package a path is, from its ending.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LocalKind {
    /// `.tar.gz`: a source package, built here.
    Source,
    /// `.zip`: a Windows binary package.
    WindowsBinary,
    /// `.tgz`: a macOS binary package.
    MacBinary,
    /// Anything else: a folder holding a package's source.
    Folder,
}

impl LocalKind {
    pub fn of(path: &str) -> LocalKind {
        let p = path.trim().to_ascii_lowercase();
        if p.ends_with(".tar.gz") {
            LocalKind::Source
        } else if p.ends_with(".zip") {
            LocalKind::WindowsBinary
        } else if p.ends_with(".tgz") {
            LocalKind::MacBinary
        } else {
            LocalKind::Folder
        }
    }

    /// R's `type` for installing it.
    fn r_type(self) -> &'static str {
        match self {
            LocalKind::Source | LocalKind::Folder => "source",
            LocalKind::WindowsBinary => "win.binary",
            LocalKind::MacBinary => "mac.binary",
        }
    }

    /// Whether R builds it here (so compiled code needs the tools).
    pub fn is_source(self) -> bool {
        matches!(self, LocalKind::Source | LocalKind::Folder)
    }
}

/// A path R can be given: not empty, not too long, no control characters.
pub fn check_path(path: &str) -> Result<(), String> {
    let p = path.trim();
    if p.is_empty() {
        return Err("No file or folder given.".into());
    }
    if p.chars().count() > 1000 {
        return Err("That path is too long.".into());
    }
    if p.chars().any(char::is_control) {
        return Err("That path has a control character in it.".into());
    }
    Ok(())
}

fn forward(path: &str) -> String {
    path.trim().replace('\\', "/")
}

/// The fields a package's DESCRIPTION is read for.
pub const DESCRIPTION_FIELDS: &str = "c(\"Package\", \"Version\", \"Title\", \"Description\", \
    \"Depends\", \"Imports\", \"LinkingTo\", \"Suggests\", \"License\", \"NeedsCompilation\", \"Built\")";

/// R code that prints the DESCRIPTION inside a package file or folder, in its
/// own format (`Field: value` lines). Nothing is installed or run from it; a
/// file without a DESCRIPTION stops with a message.
pub fn describe_local_code(path: &str) -> Result<String, String> {
    check_path(path)?;
    let p = r_string(&forward(path));
    let inside = match LocalKind::of(path) {
        LocalKind::Folder => format!(
            "d <- file.path({p}, \"DESCRIPTION\")\n  \
             if (!dir.exists({p})) stop(\"there is no folder called \", {p})"
        ),
        LocalKind::WindowsBinary => format!(
            "inside <- grep(\"^[^/]+/DESCRIPTION$\", utils::unzip({p}, list = TRUE)$Name, value = TRUE)[1]\n  \
             if (is.na(inside)) stop(\"that file holds no package\")\n  \
             utils::unzip({p}, files = inside, exdir = tmp)\n  \
             d <- file.path(tmp, inside)"
        ),
        LocalKind::Source | LocalKind::MacBinary => format!(
            "inside <- grep(\"^[^/]+/DESCRIPTION$\", utils::untar({p}, list = TRUE), value = TRUE)[1]\n  \
             if (is.na(inside)) stop(\"that file holds no package\")\n  \
             utils::untar({p}, files = inside, exdir = tmp)\n  \
             d <- file.path(tmp, inside)"
        ),
    };
    Ok(format!(
        "local({{\n  tmp <- tempfile(\"rcraft-peek-\")\n  dir.create(tmp)\n  \
         on.exit(unlink(tmp, recursive = TRUE))\n  {inside}\n  \
         if (!file.exists(d)) stop(\"no DESCRIPTION found\")\n  \
         x <- read.dcf(d)\n  write.dcf(x[, intersect({DESCRIPTION_FIELDS}, colnames(x)), drop = FALSE])\n}})"
    ))
}

/// The CRAN packages to install first, as a statement (or nothing).
fn deps_first(
    deps: &[&str],
    library: Option<&str>,
    mirror: &str,
) -> Result<Option<String>, String> {
    if deps.is_empty() {
        return Ok(None);
    }
    install_code(deps, library, mirror, InstallFrom::Default).map(Some)
}

fn lib_arg(library: Option<&str>) -> String {
    library
        .map(|l| format!(", lib = {}", r_string(l)))
        .unwrap_or_default()
}

fn make_lib(library: Option<&str>) -> String {
    library
        .map(|l| {
            format!(
                "dir.create({}, recursive = TRUE, showWarnings = FALSE)\n",
                r_string(l)
            )
        })
        .unwrap_or_default()
}

/// R code that installs a local package file or folder, after the CRAN
/// packages it needs (`deps`, from the mirror).
pub fn install_local_code(
    path: &str,
    deps: &[&str],
    library: Option<&str>,
    mirror: &str,
) -> Result<String, String> {
    check_path(path)?;
    let kind = LocalKind::of(path);
    let mut out = String::new();
    if let Some(first) = deps_first(deps, library, mirror)? {
        out.push_str(&first);
        out.push('\n');
    } else {
        out.push_str(&make_lib(library));
    }
    out.push_str(&format!(
        "install.packages({}{}, repos = NULL, type = \"{}\")",
        r_string(&forward(path)),
        lib_arg(library),
        kind.r_type()
    ));
    Ok(out)
}

/// R code that downloads a source archive from `url` and installs it, after
/// the CRAN packages it needs. Used for GitHub; the archive is fetched by R.
pub fn install_archive_code(
    url: &str,
    deps: &[&str],
    library: Option<&str>,
    mirror: &str,
) -> Result<String, String> {
    check_path(url)?;
    let mut out = String::new();
    if let Some(first) = deps_first(deps, library, mirror)? {
        out.push_str(&first);
        out.push('\n');
    } else {
        out.push_str(&make_lib(library));
    }
    out.push_str(&format!(
        "local({{\n  f <- tempfile(fileext = \".tar.gz\")\n  on.exit(unlink(f))\n  \
         utils::download.file({}, f, mode = \"wb\", quiet = TRUE)\n  \
         install.packages(f{}, repos = NULL, type = \"source\")\n}})",
        r_string(url),
        lib_arg(library)
    ));
    Ok(out)
}

/// R code that installs a GitHub repository at a reference, from source.
pub fn install_github_code(
    owner: &str,
    repo: &str,
    reference: &str,
    deps: &[&str],
    library: Option<&str>,
    mirror: &str,
) -> Result<String, String> {
    if !is_github_owner(owner) || !is_github_repo(repo) || !is_git_ref(reference) {
        return Err("That is not a GitHub owner/repository@reference.".into());
    }
    install_archive_code(
        &github_archive_url(owner, repo, reference),
        deps,
        library,
        mirror,
    )
}

/// R code that installs packages from an r-universe, with CRAN for what they
/// need that the universe does not have.
pub fn install_universe_code(
    universe: &str,
    packages: &[&str],
    library: Option<&str>,
    mirror: &str,
) -> Result<String, String> {
    if !is_universe(universe) {
        return Err(format!("\"{universe}\" is not an r-universe name."));
    }
    if packages.is_empty() || packages.iter().any(|p| !is_package_name(p)) {
        return Err("Name one or more packages.".into());
    }
    let names: Vec<String> = packages.iter().map(|p| r_string(p)).collect();
    let pkgs = if let [one] = names.as_slice() {
        one.clone()
    } else {
        format!("c({})", names.join(", "))
    };
    Ok(format!(
        "{}install.packages({pkgs}{}, repos = c({}, {}))",
        make_lib(library),
        lib_arg(library),
        r_string(&universe_url(universe)),
        r_string(mirror)
    ))
}

/// R code that installs Bioconductor packages with BiocManager, installing
/// BiocManager from CRAN first when it is missing. CRAN is set to the chosen
/// mirror for the install only.
pub fn install_bioc_code(
    packages: &[&str],
    library: Option<&str>,
    mirror: &str,
) -> Result<String, String> {
    if packages.is_empty() || packages.iter().any(|p| !is_package_name(p)) {
        return Err("Name one or more packages.".into());
    }
    let names: Vec<String> = packages.iter().map(|p| r_string(p)).collect();
    let m = r_string(mirror);
    Ok(format!(
        "{}local({{\n  old <- options(repos = c(CRAN = {m}))\n  on.exit(options(old))\n  \
         if (!requireNamespace(\"BiocManager\", quietly = TRUE)) install.packages(\"BiocManager\"{lib}, repos = {m})\n  \
         BiocManager::install(c({}){lib}, update = FALSE, ask = FALSE)\n}})",
        make_lib(library),
        names.join(", "),
        lib = lib_arg(library),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_kind_of_a_local_package_comes_from_its_ending() {
        assert_eq!(LocalKind::of("C:/a/x_1.0.tar.gz"), LocalKind::Source);
        assert_eq!(LocalKind::of("x_1.0.ZIP"), LocalKind::WindowsBinary);
        assert_eq!(LocalKind::of("x_1.0.tgz"), LocalKind::MacBinary);
        assert_eq!(LocalKind::of("/home/me/x"), LocalKind::Folder);
        assert!(LocalKind::Folder.is_source() && !LocalKind::WindowsBinary.is_source());
    }

    #[test]
    fn describing_a_file_reads_its_description_and_installs_nothing() {
        let tar = describe_local_code("C:\\pkgs\\x_1.0.tar.gz").unwrap_or_default();
        assert!(
            tar.contains("utils::untar(\"C:/pkgs/x_1.0.tar.gz\", list = TRUE)"),
            "{tar}"
        );
        assert!(tar.contains("read.dcf(d)") && tar.contains("write.dcf"));
        assert!(!tar.contains("install.packages"));
        let zip = describe_local_code("x.zip").unwrap_or_default();
        assert!(
            zip.contains("utils::unzip(\"x.zip\", list = TRUE)"),
            "{zip}"
        );
        let dir = describe_local_code("/home/me/pkg").unwrap_or_default();
        assert!(
            dir.contains("file.path(\"/home/me/pkg\", \"DESCRIPTION\")"),
            "{dir}"
        );
        assert!(describe_local_code("").is_err());
        assert!(describe_local_code("a\nb").is_err());
    }

    #[test]
    fn a_local_install_comes_after_its_cran_needs_with_the_right_type() {
        let code = install_local_code(
            "C:/x_1.0.zip",
            &["cli", "rlang"],
            Some("/lib"),
            "https://m.org",
        )
        .unwrap_or_default();
        let lines: Vec<&str> = code.lines().collect();
        assert!(
            lines[0].contains("install.packages(c(\"cli\", \"rlang\")"),
            "{code}"
        );
        assert!(
            lines[1].ends_with("install.packages(\"C:/x_1.0.zip\", lib = \"/lib\", repos = NULL, type = \"win.binary\")"),
            "{code}"
        );
        let plain =
            install_local_code("x_1.0.tar.gz", &[], None, "https://m.org").unwrap_or_default();
        assert_eq!(
            plain,
            "install.packages(\"x_1.0.tar.gz\", repos = NULL, type = \"source\")"
        );
        // a path with quotes stays one string
        let odd =
            install_local_code("C:/a \"b\".tar.gz", &[], None, "https://m.org").unwrap_or_default();
        assert!(odd.contains("\"C:/a \\\"b\\\".tar.gz\""), "{odd}");
    }

    #[test]
    fn github_downloads_the_archive_and_builds_it_after_its_needs() {
        let code = install_github_code(
            "r-lib",
            "cli",
            "v3.6.0",
            &["glue"],
            Some("/lib"),
            "https://m.org",
        )
        .unwrap_or_default();
        assert!(code.starts_with("dir.create(\"/lib\""), "{code}");
        assert!(
            code.contains("install.packages(\"glue\", lib = \"/lib\", repos = \"https://m.org\")")
        );
        assert!(code.contains(
            "utils::download.file(\"https://github.com/r-lib/cli/archive/v3.6.0.tar.gz\""
        ));
        assert!(
            code.contains("install.packages(f, lib = \"/lib\", repos = NULL, type = \"source\")")
        );
        assert!(install_github_code("a b", "c", "HEAD", &[], None, "https://m.org").is_err());
        assert!(install_github_code("a", "c", "../x", &[], None, "https://m.org").is_err());
    }

    #[test]
    fn a_universe_is_a_repository_tried_before_cran() {
        let code =
            install_universe_code("ropensci", &["rtweet"], None, "https://cloud.r-project.org")
                .unwrap_or_default();
        assert_eq!(
            code,
            "install.packages(\"rtweet\", repos = c(\"https://ropensci.r-universe.dev\", \"https://cloud.r-project.org\"))"
        );
        assert!(install_universe_code("bad name", &["x"], None, "https://m.org").is_err());
        assert!(install_universe_code("ok", &[], None, "https://m.org").is_err());
    }

    #[test]
    fn bioconductor_goes_through_biocmanager_with_the_mirror_for_the_install_only() {
        let code = install_bioc_code(&["limma", "edgeR"], Some("/lib"), "https://m.org")
            .unwrap_or_default();
        assert!(
            code.contains("old <- options(repos = c(CRAN = \"https://m.org\"))"),
            "{code}"
        );
        assert!(code.contains("on.exit(options(old))"));
        assert!(code.contains(
            "install.packages(\"BiocManager\", lib = \"/lib\", repos = \"https://m.org\")"
        ));
        assert!(code.contains("BiocManager::install(c(\"limma\", \"edgeR\"), lib = \"/lib\", update = FALSE, ask = FALSE)"));
        assert!(install_bioc_code(&["no good"], None, "https://m.org").is_err());
    }
}
