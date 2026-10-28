//! Where a package comes from: CRAN, Bioconductor, an r-universe, a GitHub
//! repository or a file on this computer. Each has a trust label the
//! confirmation shows (see `plan/providers.md`), and the three that nobody
//! reviews need one more tick before the yes.
//!
//! Everything a user types that ends up in an address or in R code is checked
//! here first: universe and GitHub names, repository names, references.

use crate::plan::compare_versions;

/// Bioconductor's list of releases and the R each is for.
pub const BIOC_CONFIG_URL: &str = "https://bioconductor.org/config.yaml";

/// A source of packages.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Provider {
    Cran,
    Bioconductor,
    /// The universe's owner, as in `https://<owner>.r-universe.dev`.
    Universe(String),
    GitHub {
        owner: String,
        repo: String,
        /// A branch, tag or commit; `HEAD` is the default branch as it is now.
        reference: String,
    },
    /// A file or folder on this computer.
    Local(String),
}

impl Provider {
    /// The provider's short name.
    pub fn name(&self) -> String {
        match self {
            Provider::Cran => "CRAN".into(),
            Provider::Bioconductor => "Bioconductor".into(),
            Provider::Universe(owner) => format!("r-universe ({owner})"),
            Provider::GitHub { owner, repo, .. } => format!("GitHub ({owner}/{repo})"),
            Provider::Local(_) => "a local file".into(),
        }
    }

    /// The one-line trust label shown with every confirmation.
    pub fn label(&self) -> String {
        match self {
            Provider::Cran => "CRAN: checked by CRAN before each release.".into(),
            Provider::Bioconductor => "Bioconductor: reviewed by the Bioconductor project. \
                Installed with BiocManager, which comes from CRAN."
                .into(),
            Provider::Universe(owner) => format!(
                "r-universe: built by r-universe from {owner}'s own repositories. Not reviewed by \
                 CRAN or anyone else."
            ),
            Provider::GitHub {
                owner,
                repo,
                reference,
            } => {
                let when = if reference == "HEAD" {
                    "as it is now (the default branch, which can change at any time)".to_string()
                } else {
                    format!("at {reference}")
                };
                format!(
                    "GitHub: unreviewed source code from {owner}/{repo}, {when}. Anyone who can \
                     change that repository can change what you install. Built on this computer."
                )
            }
            Provider::Local(path) => {
                format!("Local file: unreviewed code from {path} on this computer.")
            }
        }
    }

    /// Whether someone reviews what this provider offers.
    pub fn reviewed(&self) -> bool {
        matches!(self, Provider::Cran | Provider::Bioconductor)
    }
}

/// A GitHub user or organisation name: letters, digits and single dashes, not
/// at either end, at most 39 characters.
pub fn is_github_owner(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 39
        && !s.starts_with('-')
        && !s.ends_with('-')
        && !s.contains("--")
        && s.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
}

/// A GitHub repository name: letters, digits, `.`, `_` and `-`, not `.` or `..`.
pub fn is_github_repo(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 100
        && s != "."
        && s != ".."
        && s.chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-'))
}

/// A branch, tag or commit: letters, digits, `.`, `_`, `-` and `/`, with no
/// `..`, not starting with `-` or `/`, at most 100 characters.
pub fn is_git_ref(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 100
        && !s.contains("..")
        && !s.starts_with(['-', '/'])
        && !s.ends_with('/')
        && s.chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-' | '/'))
}

/// An r-universe name is a GitHub owner name (the universe is theirs).
pub fn is_universe(s: &str) -> bool {
    is_github_owner(s)
}

/// Reads `owner/repo` or `owner/repo@ref` (the reference is `HEAD` when not
/// given). A GitHub address (`https://github.com/owner/repo`) is accepted too.
pub fn parse_github(spec: &str) -> Result<Provider, String> {
    let t = spec.trim();
    let t = t
        .strip_prefix("https://github.com/")
        .or_else(|| t.strip_prefix("github.com/"))
        .unwrap_or(t)
        .trim_end_matches('/');
    let t = t.strip_suffix(".git").unwrap_or(t);
    let (path, reference) = match t.split_once('@') {
        Some((p, r)) => (p, r),
        None => (t, "HEAD"),
    };
    let Some((owner, repo)) = path.split_once('/') else {
        return Err(format!(
            "\"{spec}\" is not owner/repository, for example: rstudio/gt"
        ));
    };
    if !is_github_owner(owner) {
        return Err(format!(
            "\"{owner}\" is not a GitHub user or organisation name."
        ));
    }
    if !is_github_repo(repo) {
        return Err(format!("\"{repo}\" is not a GitHub repository name."));
    }
    if !is_git_ref(reference) {
        return Err(format!("\"{reference}\" is not a branch, tag or commit."));
    }
    Ok(Provider::GitHub {
        owner: owner.to_string(),
        repo: repo.to_string(),
        reference: reference.to_string(),
    })
}

/// Where a GitHub repository's DESCRIPTION is, at a reference.
pub fn github_description_url(owner: &str, repo: &str, reference: &str) -> String {
    format!("https://raw.githubusercontent.com/{owner}/{repo}/{reference}/DESCRIPTION")
}

/// Where GitHub serves a repository as a source archive, at a reference.
pub fn github_archive_url(owner: &str, repo: &str, reference: &str) -> String {
    format!("https://github.com/{owner}/{repo}/archive/{reference}.tar.gz")
}

/// The address of an r-universe (a CRAN-like repository).
pub fn universe_url(owner: &str) -> String {
    format!("https://{owner}.r-universe.dev")
}

/// The list of Bioconductor's software packages for a release.
pub fn bioc_views_url(release: &str) -> String {
    format!("https://bioconductor.org/packages/{release}/bioc/VIEWS")
}

/// The Bioconductor release for this R, from Bioconductor's `config.yaml`: the
/// newest release made for this R's major.minor that is not the development
/// one. `None` when the file names none.
///
/// The part read looks like this (only these lines matter):
///
/// ```text
/// release_version: "3.20"
/// devel_version: "3.21"
/// r_ver_for_bioc_ver:
///     "3.19": "4.4"
///     "3.20": "4.4"
///     "3.21": "4.5"
/// ```
pub fn bioc_release_for(config: &str, r_version: &str) -> Option<String> {
    let unquote = |s: &str| s.trim().trim_matches(|c| c == '"' || c == '\'').to_string();
    let minor = |v: &str| v.split('.').take(2).collect::<Vec<_>>().join(".");
    let r = minor(r_version);
    let mut devel: Option<String> = None;
    let mut pairs: Vec<(String, String)> = Vec::new();
    let mut in_map = false;
    for line in config.lines().take(100_000) {
        let indented = line.starts_with([' ', '\t']);
        let t = line.trim();
        if !indented {
            in_map = t.starts_with("r_ver_for_bioc_ver:");
            if let Some(v) = t.strip_prefix("devel_version:") {
                devel = Some(unquote(v));
            }
            continue;
        }
        if in_map {
            if let Some((bioc, rv)) = t.split_once(':') {
                pairs.push((unquote(bioc), unquote(rv)));
            }
        }
    }
    pairs
        .into_iter()
        .filter(|(b, rv)| minor(rv) == r && devel.as_deref() != Some(b.as_str()))
        .filter(|(b, _)| b.chars().all(|c| c.is_ascii_digit() || c == '.') && !b.is_empty())
        .map(|(b, _)| b)
        .max_by(|a, b| compare_versions(a, b))
}

#[cfg(test)]
mod tests {
    use super::*;

    const CONFIG: &str = "\
## a made-up config in Bioconductor's form
release_version: \"3.20\"
devel_version: \"3.21\"
r_version_associated_with_release: \"4.4.0\"
r_ver_for_bioc_ver:
    \"3.18\": \"4.3\"
    \"3.19\": \"4.4\"
    \"3.20\": \"4.4\"
    \"3.21\": \"4.5\"
release_dates:
    \"3.20\": \"10/30/2024\"
";

    #[test]
    fn the_newest_release_for_this_r_is_chosen_and_devel_is_not() {
        assert_eq!(bioc_release_for(CONFIG, "4.4.3").as_deref(), Some("3.20"));
        assert_eq!(bioc_release_for(CONFIG, "4.3.1").as_deref(), Some("3.18"));
        // 3.21 is the development release, so R 4.5 has none yet here
        assert_eq!(bioc_release_for(CONFIG, "4.5.0"), None);
        assert_eq!(bioc_release_for(CONFIG, "3.6.3"), None);
        assert_eq!(bioc_release_for("", "4.4.3"), None);
        // the dates map after it is not read as releases
        let odd = format!("{CONFIG}\nr_ver_for_bioc_ver:\n    \"3.9x\": \"4.4\"\n");
        assert_eq!(bioc_release_for(&odd, "4.4.3").as_deref(), Some("3.20"));
    }

    #[test]
    fn github_specs_are_read_and_checked() {
        assert_eq!(
            parse_github("rstudio/gt"),
            Ok(Provider::GitHub {
                owner: "rstudio".into(),
                repo: "gt".into(),
                reference: "HEAD".into()
            })
        );
        assert_eq!(
            parse_github(" https://github.com/r-lib/cli.git "),
            Ok(Provider::GitHub {
                owner: "r-lib".into(),
                repo: "cli".into(),
                reference: "HEAD".into()
            })
        );
        assert!(matches!(
            parse_github("tidyverse/dplyr@v1.1.4"),
            Ok(Provider::GitHub { reference, .. }) if reference == "v1.1.4"
        ));
        for bad in [
            "dplyr",
            "/x",
            "a/",
            "-a/b",
            "a--b/c",
            "a/b c",
            "a/..",
            "a/b@",
            "a/b@../../x",
            "a/b@-x",
            "a/b@x;rm",
            "a\"/b",
            &format!("{}/b", "a".repeat(40)),
        ] {
            assert!(parse_github(bad).is_err(), "{bad:?}");
        }
    }

    #[test]
    fn addresses_are_built_from_checked_parts() {
        assert_eq!(
            github_description_url("r-lib", "cli", "HEAD"),
            "https://raw.githubusercontent.com/r-lib/cli/HEAD/DESCRIPTION"
        );
        assert_eq!(
            github_archive_url("r-lib", "cli", "v3.6.0"),
            "https://github.com/r-lib/cli/archive/v3.6.0.tar.gz"
        );
        assert_eq!(universe_url("ropensci"), "https://ropensci.r-universe.dev");
        assert_eq!(
            bioc_views_url("3.20"),
            "https://bioconductor.org/packages/3.20/bioc/VIEWS"
        );
        assert!(is_universe("ropensci") && !is_universe("rOpen sci"));
    }

    #[test]
    fn labels_name_the_source_and_unreviewed_ones_say_so() {
        let gh = parse_github("me/thing").unwrap_or(Provider::Cran);
        assert!(!gh.reviewed());
        assert!(gh.label().contains("unreviewed") && gh.label().contains("can change at any time"));
        let pinned = parse_github("me/thing@abc123").unwrap_or(Provider::Cran);
        assert!(pinned.label().contains("at abc123"));
        assert!(Provider::Cran.reviewed() && Provider::Bioconductor.reviewed());
        assert!(!Provider::Universe("x".into()).reviewed());
        assert!(!Provider::Local("C:/a.tar.gz".into()).reviewed());
        assert!(Provider::Local("C:/a.tar.gz".into())
            .label()
            .contains("C:/a.tar.gz"));
        assert_eq!(
            Provider::Universe("ropensci".into()).name(),
            "r-universe (ropensci)"
        );
    }
}
