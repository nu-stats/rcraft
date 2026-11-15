//! R code for installing, removing and listing packages, and reading what R
//! says about the installed ones.
//!
//! Names and versions go into R code as quoted strings after a check, so a
//! package name read from CRAN's metadata cannot become R code of its own.

use std::collections::HashMap;

use crate::import::r_string;

/// R's rule for a package name: letters, digits and dots, starting with a letter.
pub fn is_package_name(s: &str) -> bool {
    let mut chars = s.chars();
    matches!(chars.next(), Some(c) if c.is_ascii_alphabetic())
        && s.chars().all(|c| c.is_ascii_alphanumeric() || c == '.')
}

/// A version as CRAN writes it: digits, letters, dots and dashes.
pub fn is_version(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 64
        && s.chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '-')
}

/// Which kind of package to ask R for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum InstallFrom {
    /// R chooses: a binary where the system has one (Windows, macOS), else source.
    #[default]
    Default,
    /// Source packages, built here.
    Source,
    /// Binary packages only.
    Binary,
}

impl InstallFrom {
    /// The `type` argument for `install.packages`, with its leading comma.
    fn argument(self) -> &'static str {
        match self {
            InstallFrom::Default => "",
            InstallFrom::Source => ", type = \"source\"",
            InstallFrom::Binary => ", type = \"binary\"",
        }
    }
}

fn check_names(packages: &[&str]) -> Result<(), String> {
    if packages.is_empty() {
        return Err("no package named".into());
    }
    match packages.iter().find(|p| !is_package_name(p)) {
        Some(bad) => Err(format!("\"{bad}\" is not a package name")),
        None => Ok(()),
    }
}

fn names_vector(packages: &[&str]) -> String {
    let names: Vec<String> = packages.iter().map(|p| r_string(p)).collect();
    if let [one] = names.as_slice() {
        one.clone()
    } else {
        format!("c({})", names.join(", "))
    }
}

fn make_library(library: Option<&str>) -> String {
    match library {
        Some(lib) => format!(
            "dir.create({}, recursive = TRUE, showWarnings = FALSE); ",
            r_string(lib)
        ),
        None => String::new(),
    }
}

fn lib_argument(library: Option<&str>) -> String {
    library
        .map(|l| format!(", lib = {}", r_string(l)))
        .unwrap_or_default()
}

/// R code that installs `packages` (and what they need) from the repository at
/// `repo`, into `library` when given. R prefers binary packages where the
/// platform has them; `from_how` can ask for source packages (built here, so
/// a package with compiled code needs the tools to compile it) or for binary
/// packages only (Windows and macOS).
pub fn install_code(
    packages: &[&str],
    library: Option<&str>,
    repo: &str,
    from_how: InstallFrom,
) -> Result<String, String> {
    check_names(packages)?;
    Ok(format!(
        "{}install.packages({}{}, repos = {}{})",
        make_library(library),
        names_vector(packages),
        lib_argument(library),
        r_string(repo),
        from_how.argument()
    ))
}

/// Where CRAN keeps the source of an older version of a package.
pub fn archive_url(repo: &str, package: &str, version: &str) -> Result<String, String> {
    check_names(&[package])?;
    if !is_version(version) {
        return Err(format!("\"{version}\" is not a version"));
    }
    Ok(format!(
        "{}/src/contrib/Archive/{package}/{package}_{version}.tar.gz",
        repo.trim_end_matches('/')
    ))
}

/// R code that installs one older version of a package from CRAN's archive. It
/// is built from source, so it needs the tools to compile if the package has
/// compiled code. Its dependencies are installed as usual.
pub fn install_version_code(
    package: &str,
    version: &str,
    library: Option<&str>,
    repo: &str,
) -> Result<String, String> {
    let url = archive_url(repo, package, version)?;
    Ok(format!(
        "{}install.packages({}{}, repos = NULL, type = \"source\")",
        make_library(library),
        r_string(&url),
        lib_argument(library),
    ))
}

/// R code that installs the packages of a saved list: the older versions one by
/// one from CRAN's archive first, then the rest in one call at CRAN's current
/// versions (R installs what they need that is still missing, so a pinned
/// older package is not replaced by a newer one it was just given). One
/// statement per line.
pub fn restore_code(
    current: &[&str],
    older: &[(&str, &str)],
    library: Option<&str>,
    repo: &str,
) -> Result<String, String> {
    if current.is_empty() && older.is_empty() {
        return Err("nothing to install".into());
    }
    let mut lines = Vec::new();
    for (package, version) in older {
        lines.push(install_version_code(package, version, library, repo)?);
    }
    if !current.is_empty() {
        lines.push(install_code(current, library, repo, InstallFrom::Default)?);
    }
    Ok(lines.join("\n"))
}

/// R code that removes `packages` from `library` (or the first library).
pub fn remove_code(packages: &[&str], library: Option<&str>) -> Result<String, String> {
    check_names(packages)?;
    Ok(format!(
        "remove.packages({}{})",
        names_vector(packages),
        lib_argument(library)
    ))
}

/// R code that prints one line per installed package, `name<TAB>version<TAB>
/// library<TAB>priority<TAB>repository` (priority and repository empty for
/// packages with none; the repository is `CRAN` for packages installed from
/// CRAN), every library on R's search path, in search order.
pub fn installed_packages_code() -> &'static str {
    "local({ ip <- installed.packages(fields = \"Repository\"); \
     na0 <- function(x) ifelse(is.na(x), \"\", x); \
     cat(paste(ip[, \"Package\"], ip[, \"Version\"], ip[, \"LibPath\"], na0(ip[, \"Priority\"]), \
     na0(ip[, \"Repository\"]), sep = \"\\t\"), sep = \"\\n\") })"
}

/// R code that prints this R's own per-user library folders (`R_LIBS_USER`),
/// one per line, with `~` expanded. Nothing is created or changed.
pub fn user_libraries_code() -> &'static str {
    "cat(path.expand(strsplit(Sys.getenv(\"R_LIBS_USER\"), .Platform$path.sep, fixed = TRUE)[[1]]), \
     sep = \"\\n\")"
}

/// The folders [`user_libraries_code`] printed.
pub fn parse_user_libraries(text: &str) -> Vec<String> {
    text.lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .take(16)
        .map(str::to_string)
        .collect()
}

/// R code that prints, one per line, where `name` can be found: the attached
/// packages (and the global workspace) that hold an object of that name, then
/// the namespaces that have one that is not attached (R's `getAnywhere`).
pub fn where_code(name: &str) -> String {
    let n = r_string(name);
    format!(
        "local({{ w <- unique(c(find({n}), getAnywhere({n})$where)); \
         w <- sub('^namespace:', '', w); cat(w, sep = \"\n\") }})"
    )
}

/// R code that prints the packages loaded in this session, one per line, and
/// marks those on the search path (attached) with a tab and `attached`.
pub fn loaded_packages_code() -> &'static str {
    "local({ l <- sort(loadedNamespaces()); \
     a <- sub('^package:', '', grep('^package:', search(), value = TRUE)); \
     cat(paste0(l, ifelse(l %in% a, '\tattached', '')), sep = \"\n\") })"
}

/// Names of the loaded packages from [`loaded_packages_code`]'s output, with
/// whether each is attached.
pub fn parse_loaded(text: &str) -> Vec<(String, bool)> {
    text.lines()
        .filter_map(|l| {
            let l = l.trim_end_matches('\r');
            let (name, attached) = match l.split_once('\t') {
                Some((n, a)) => (n, a == "attached"),
                None => (l, false),
            };
            is_package_name(name).then(|| (name.to_string(), attached))
        })
        .collect()
}

/// A package R has installed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Installed {
    pub name: String,
    pub version: String,
    pub library: String,
    /// `base` or `recommended` for packages that come with R, else empty.
    pub priority: String,
    /// R's `Repository` field: `CRAN` for packages installed from CRAN, else
    /// empty or another repository.
    pub repository: String,
}

/// Reads the output of [`installed_packages_code`]. Lines that do not have
/// four or five fields are skipped.
pub fn parse_installed(text: &str) -> Vec<Installed> {
    text.lines()
        .filter_map(|line| {
            let f: Vec<&str> = line.trim_end_matches('\r').split('\t').collect();
            let (name, version, library, priority, repository) = match f.as_slice() {
                [n, v, l, p] => (n, v, l, p, ""),
                [n, v, l, p, r] => (n, v, l, p, *r),
                _ => return None,
            };
            is_package_name(name).then(|| Installed {
                name: (*name).to_string(),
                version: (*version).to_string(),
                library: (*library).to_string(),
                priority: (*priority).to_string(),
                repository: repository.to_string(),
            })
        })
        .collect()
}

/// Name to version for the packages R would load: where a package is in two
/// libraries the first (R's search order) wins.
pub fn installed_versions(list: &[Installed]) -> HashMap<String, String> {
    let mut map = HashMap::new();
    for p in list {
        map.entry(p.name.clone())
            .or_insert_with(|| p.version.clone());
    }
    map
}

/// Why a command wants a package installed (P1A.10).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Need {
    /// It is not installed.
    Missing,
    /// The installed version is older than another package needs.
    Newer,
    /// It was installed for an older R and must be installed again.
    Reinstall,
}

/// The package R's message (an error, a warning or a message) says must be
/// installed, and why: `there is no package called 'x'` (`library(x)`,
/// `x::f()`, a dependency), `package 'x' required by 'y' could not be found`,
/// the "Package x is required / needed" of packages that check first, an
/// installed version too old for another package, and a package built before
/// R 4.0.0. `None` for any other message.
pub fn package_to_install(message: &str) -> Option<(String, Need)> {
    let text = message
        .replace(['\u{2018}', '\u{2019}', '\u{201c}', '\u{201d}'], "'")
        .replace('"', "'");
    // the name R quoted right after `start` (anywhere in the text)
    let quoted_after = |start: &str| -> Option<String> {
        let mut rest = text.as_str();
        while let Some(at) = rest.find(start) {
            rest = rest.get(at + start.len()..)?;
            let name: String = rest.chars().take_while(|&c| c != '\'').collect();
            if is_package_name(&name) && rest.get(name.len()..)?.starts_with('\'') {
                return Some(name);
            }
        }
        None
    };
    let lower = text.to_lowercase();
    if let Some(name) = quoted_after("there is no package called '") {
        return Some((name, Need::Missing));
    }
    if let Some(name) = quoted_after("package '") {
        if text.contains(&format!("package '{name}' required by '"))
            && text.contains("could not be found")
        {
            return Some((name, Need::Missing));
        }
        if text.contains(&format!("package '{name}' was installed before R")) {
            return Some((name, Need::Reinstall));
        }
    }
    if let Some(name) = quoted_after("namespace '") {
        if text.contains("is being loaded, but >=") && text.contains("is required") {
            return Some((name, Need::Newer));
        }
    }
    // packages that check for another first: rlang's "The package 'x' is
    // required", and the common "Package 'x' needed for this function to work"
    if lower.contains("is required") || lower.contains("needed for this function") {
        let lowered_start = lower.find("package '")?;
        let rest = text.get(lowered_start + "package '".len()..)?;
        let name: String = rest.chars().take_while(|&c| c != '\'').collect();
        let after = rest.get(name.len()..)?;
        let tail = after.get(..after.len().min(40))?.to_lowercase();
        if is_package_name(&name)
            && after.starts_with('\'')
            && (tail.contains("is required") || tail.contains("needed for this function"))
        {
            return Some((name, Need::Missing));
        }
    }
    None
}

#[cfg(test)]
mod need_tests {
    use super::*;

    fn one(message: &str) -> Option<(String, Need)> {
        package_to_install(message)
    }

    #[test]
    fn missing_packages_are_named() {
        let missing = |s: &str| Some((s.to_string(), Need::Missing));
        // recorded from R 4.4.3: library(), pkg::f(), loadNamespace(), require()'s warning
        assert_eq!(
            one("there is no package called 'zzpkgq'"),
            missing("zzpkgq")
        );
        assert_eq!(
            one("there is no package called \u{2018}zz.pkg2\u{2019}"),
            missing("zz.pkg2")
        );
        // requireNamespace()'s message
        assert_eq!(
            one("Failed with error:  'there is no package called \u{2018}sf\u{2019}'"),
            missing("sf")
        );
        // a dependency
        assert_eq!(
            one("package \u{2018}vctrs\u{2019} required by \u{2018}dplyr\u{2019} could not be found"),
            missing("vctrs")
        );
        // packages that check first
        assert_eq!(one("The package \"ragg\" is required."), missing("ragg"));
        assert_eq!(
            one("Package \"MASS\" needed for this function to work. Please install it."),
            missing("MASS")
        );
        assert_eq!(
            one("\u{2139} The package `ggrepel` is required"),
            None,
            "backticks are not R's quotes for a package"
        );
    }

    #[test]
    fn old_versions_are_named() {
        assert_eq!(
            one("namespace \u{2018}rlang\u{2019} 1.0.6 is being loaded, but >= 1.1.0 is required"),
            Some(("rlang".to_string(), Need::Newer))
        );
        assert_eq!(
            one("package \u{2018}foo\u{2019} was installed before R 4.0.0: please re-install it"),
            Some(("foo".to_string(), Need::Reinstall))
        );
    }

    #[test]
    fn other_messages_name_nothing() {
        for m in [
            "",
            "object 'x' not found",
            "there is no package called ''",
            "there is no package called 'a b'",
            "there is no package called 'x; system(\"rm\")'",
            "package 'stats' is not in use",
            "argument \"x\" is missing, with no default",
            "a value is required",
            "package 'x' required by 'y'",
        ] {
            assert_eq!(one(m), None, "{m}");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn install_code_quotes_and_names_the_library_and_repository() {
        assert_eq!(
            install_code(
                &["fixest"],
                None,
                "https://cloud.r-project.org",
                InstallFrom::Default
            ),
            Ok("install.packages(\"fixest\", repos = \"https://cloud.r-project.org\")".into())
        );
        let code = install_code(
            &["a", "b.c"],
            Some("C:/Users/me/R lib"),
            "https://m.example.org/cran",
            InstallFrom::Default,
        );
        assert_eq!(
            code,
            Ok(
                "dir.create(\"C:/Users/me/R lib\", recursive = TRUE, showWarnings = FALSE); \
                install.packages(c(\"a\", \"b.c\"), lib = \"C:/Users/me/R lib\", \
                repos = \"https://m.example.org/cran\")"
                    .into()
            )
        );
    }

    #[test]
    fn from_source_is_asked_for_in_the_call() {
        assert_eq!(
            install_code(&["a"], None, "https://x.org", InstallFrom::Source),
            Ok("install.packages(\"a\", repos = \"https://x.org\", type = \"source\")".into())
        );
    }

    #[test]
    fn a_name_that_is_not_a_package_name_is_refused_not_quoted() {
        for bad in ["", "9lives", "a b", "a\";system(\"x\")", "a-b", "a_b", "é"] {
            assert!(
                install_code(&[bad], None, "https://x.org", InstallFrom::Default).is_err(),
                "{bad:?}"
            );
            assert!(remove_code(&[bad], None).is_err(), "{bad:?}");
        }
        assert!(install_code(&[], None, "https://x.org", InstallFrom::Default).is_err());
        assert!(install_code(
            &["ok", "not ok"],
            None,
            "https://x.org",
            InstallFrom::Default
        )
        .is_err());
    }

    #[test]
    fn an_older_version_comes_from_the_archive_as_source() {
        assert_eq!(
            install_version_code("dplyr", "1.0.10", Some("/lib"), "https://cloud.r-project.org/"),
            Ok("dir.create(\"/lib\", recursive = TRUE, showWarnings = FALSE); \
                install.packages(\"https://cloud.r-project.org/src/contrib/Archive/dplyr/dplyr_1.0.10.tar.gz\", \
                lib = \"/lib\", repos = NULL, type = \"source\")"
                .into())
        );
        assert!(install_version_code("dplyr", "1.0/../x", None, "https://x.org").is_err());
        assert!(install_version_code("dplyr", "", None, "https://x.org").is_err());
        assert!(install_version_code("d plyr", "1.0", None, "https://x.org").is_err());
        assert!(is_version("0.12-3"));
    }

    #[test]
    fn removing_names_the_library() {
        assert_eq!(
            remove_code(&["a"], Some("/lib")),
            Ok("remove.packages(\"a\", lib = \"/lib\")".into())
        );
        assert_eq!(
            remove_code(&["a", "b"], None),
            Ok("remove.packages(c(\"a\", \"b\"))".into())
        );
    }

    #[test]
    fn the_installed_list_is_read_and_the_first_library_wins() {
        let text = "stats\t4.4.3\tC:/R/library\tbase\r\n\
                    fixest\t0.12.1\tC:/mine\t\n\
                    garbage line\n\
                    fixest\t0.11.0\tC:/R/library\t\n\
                    bad name\t1\tx\t\n";
        let list = parse_installed(text);
        assert_eq!(list.len(), 3);
        assert_eq!(list[0].priority, "base");
        assert_eq!(list[1].library, "C:/mine");
        let v = installed_versions(&list);
        assert_eq!(v.get("fixest").map(String::as_str), Some("0.12.1"));
        assert_eq!(v.get("stats").map(String::as_str), Some("4.4.3"));
        assert_eq!(v.len(), 2);
    }

    #[test]
    fn the_listing_code_asks_for_the_columns() {
        let c = installed_packages_code();
        assert!(c.contains("installed.packages("));
        assert!(c.contains("LibPath") && c.contains("Priority"));
    }
}

#[cfg(test)]
mod where_tests {
    use super::*;

    #[test]
    fn the_where_code_quotes_the_name() {
        let c = where_code("a\"b");
        assert!(c.contains("find(\"a\\\"b\")"), "{c}");
        assert!(c.contains("getAnywhere"));
    }

    #[test]
    fn the_loaded_list_is_read_with_attached_marks() {
        let text = "base\tattached\r\ncompiler\nstats\tattached\nbad name\n";
        assert_eq!(
            parse_loaded(text),
            vec![
                ("base".to_string(), true),
                ("compiler".to_string(), false),
                ("stats".to_string(), true)
            ]
        );
        assert!(loaded_packages_code().contains("loadedNamespaces()"));
    }
}

#[cfg(test)]
mod restore_tests {
    use super::*;

    #[test]
    fn a_list_is_restored_older_versions_first_one_statement_per_line() {
        let code = restore_code(
            &["dplyr", "tidyr"],
            &[("Rcpp", "1.0.14")],
            Some("/lib"),
            "https://cloud.r-project.org",
        )
        .unwrap_or_default();
        let lines: Vec<&str> = code.lines().collect();
        assert_eq!(lines.len(), 2, "{code}");
        assert!(
            lines[0].contains("Archive/Rcpp/Rcpp_1.0.14.tar.gz"),
            "{code}"
        );
        assert!(
            lines[1].contains("install.packages(c(\"dplyr\", \"tidyr\")"),
            "{code}"
        );
        assert!(restore_code(&[], &[], None, "https://x.org").is_err());
        assert!(restore_code(&["a b"], &[], None, "https://x.org").is_err());
        assert!(restore_code(&[], &[("ok", "1/../x")], None, "https://x.org").is_err());
    }

    #[test]
    fn the_installed_list_reads_the_repository_when_there_is_one() {
        let list = parse_installed(
            "dplyr\t1.1.4\tC:/lib\t\tCRAN\nstats\t4.4.3\tC:/R\tbase\t\nold\t1.0\tC:/lib\t\n",
        );
        assert_eq!(list.len(), 3);
        assert_eq!(list[0].repository, "CRAN");
        assert_eq!(list[1].priority, "base");
        assert_eq!(list[2].repository, "");
        assert!(installed_packages_code().contains("Repository"));
    }
}
