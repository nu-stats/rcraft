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
/// platform has them; `from_source` asks for source packages (built here, so
/// a package with compiled code needs the tools to compile it).
pub fn install_code(
    packages: &[&str],
    library: Option<&str>,
    repo: &str,
    from_source: bool,
) -> Result<String, String> {
    check_names(packages)?;
    Ok(format!(
        "{}install.packages({}{}, repos = {}{})",
        make_library(library),
        names_vector(packages),
        lib_argument(library),
        r_string(repo),
        if from_source {
            ", type = \"source\""
        } else {
            ""
        }
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
/// library<TAB>priority` (priority empty for packages with none), every library
/// on R's search path, in search order.
pub fn installed_packages_code() -> &'static str {
    "local({ ip <- installed.packages(); \
     cat(paste(ip[, \"Package\"], ip[, \"Version\"], ip[, \"LibPath\"], \
     ifelse(is.na(ip[, \"Priority\"]), \"\", ip[, \"Priority\"]), sep = \"\\t\"), sep = \"\\n\") })"
}

/// A package R has installed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Installed {
    pub name: String,
    pub version: String,
    pub library: String,
    /// `base` or `recommended` for packages that come with R, else empty.
    pub priority: String,
}

/// Reads the output of [`installed_packages_code`]. Lines that do not have
/// the four fields are skipped.
pub fn parse_installed(text: &str) -> Vec<Installed> {
    text.lines()
        .filter_map(|line| {
            let f: Vec<&str> = line.trim_end_matches('\r').split('\t').collect();
            match f.as_slice() {
                [name, version, library, priority] if is_package_name(name) => Some(Installed {
                    name: (*name).to_string(),
                    version: (*version).to_string(),
                    library: (*library).to_string(),
                    priority: (*priority).to_string(),
                }),
                _ => None,
            }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn install_code_quotes_and_names_the_library_and_repository() {
        assert_eq!(
            install_code(&["fixest"], None, "https://cloud.r-project.org", false),
            Ok("install.packages(\"fixest\", repos = \"https://cloud.r-project.org\")".into())
        );
        let code = install_code(
            &["a", "b.c"],
            Some("C:/Users/me/R lib"),
            "https://m.example.org/cran",
            false,
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
            install_code(&["a"], None, "https://x.org", true),
            Ok("install.packages(\"a\", repos = \"https://x.org\", type = \"source\")".into())
        );
    }

    #[test]
    fn a_name_that_is_not_a_package_name_is_refused_not_quoted() {
        for bad in ["", "9lives", "a b", "a\";system(\"x\")", "a-b", "a_b", "é"] {
            assert!(
                install_code(&[bad], None, "https://x.org", false).is_err(),
                "{bad:?}"
            );
            assert!(remove_code(&[bad], None).is_err(), "{bad:?}");
        }
        assert!(install_code(&[], None, "https://x.org", false).is_err());
        assert!(install_code(&["ok", "not ok"], None, "https://x.org", false).is_err());
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
    fn the_listing_code_asks_for_the_four_columns() {
        let c = installed_packages_code();
        assert!(c.contains("installed.packages()"));
        assert!(c.contains("LibPath") && c.contains("Priority"));
    }
}
