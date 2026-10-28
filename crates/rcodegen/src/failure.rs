//! Reading what R says when installing a package fails, and saying it in plain
//! words: what went wrong, what to do about it on this system, and what can be
//! tried again from the window.
//!
//! The messages are R's own and differ by system. The Windows ones below were
//! recorded from R 4.4.3 on Windows without Rtools (see the tests); the macOS
//! and Linux ones follow the messages R, `make`, the compilers and `configure`
//! scripts print on those systems. A message not recognised gives no diagnosis,
//! and the window then shows R's text as it is.

/// The operating system the advice is for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Os {
    Windows,
    MacOs,
    Linux,
}

/// What failed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Cause {
    /// No compiler or `make` where the package had to be built from source.
    NoCompiler,
    /// A system library (or its development files) the package needs is missing.
    MissingLibrary(String),
    /// The library folder cannot be written to or is locked.
    NotWritable,
    /// The mirror could not be reached.
    Network,
    /// The package needs a newer R than this one: `(package, needed R version)`.
    NeedsNewerR(String, String),
    /// A package this one needs is not available: `(package, dependency)`.
    DependencyUnavailable(String, String),
    /// Only a source package exists for the version, and building it needs tools.
    SourceOnly,
    /// No package of that name in the repository.
    NotFound(String),
    /// R stopped the install of this package and gave no clearer reason.
    Failed(String),
}

/// Something the window can offer to try next.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Retry {
    /// The same install again (after the user has fixed something).
    Again,
    /// Ask for a binary package only (Windows and macOS).
    BinaryOnly,
    /// Build from source.
    FromSource,
    /// Install an older version of this package, which the user names.
    OlderVersion(String),
    /// Look at the mirror setting.
    ChangeMirror,
}

/// One finding, in words for the user.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnosis {
    pub cause: Cause,
    /// What went wrong, in a sentence or two.
    pub summary: String,
    /// What to do about it, one step per line.
    pub steps: Vec<String>,
    pub retry: Vec<Retry>,
}

/// Straight quotes for the curly ones some R builds print.
fn normalise(text: &str) -> String {
    text.replace(['\u{2018}', '\u{2019}'], "'")
        .replace(['\u{201c}', '\u{201d}'], "\"")
}

/// The text between `start` and the next `end` after it.
fn between<'a>(text: &'a str, start: &str, end: &str) -> Option<&'a str> {
    let from = text.find(start)? + start.len();
    let rest = text.get(from..)?;
    let to = rest.find(end)?;
    rest.get(..to)
}

/// The text between the last `start` before the first `end`, and that `end`. For
/// two lines, "dependency 'a' is not available" then "ERROR: dependency 'b' is not
/// available for package 'c'", it gives `b`, not everything from the first line on.
fn last_between<'a>(text: &'a str, start: &str, end: &str) -> Option<&'a str> {
    let stop = text.find(end)?;
    let head = text.get(..stop)?;
    let from = head.rfind(start)? + start.len();
    head.get(from..)
}

/// "Rtools44" for R 4.4, and so on; `None` for R versions older than 4.2.
fn rtools_name(r_version: &str) -> Option<String> {
    let mut parts = r_version.split('.');
    let major: u32 = parts.next()?.parse().ok()?;
    let minor: u32 = parts.next()?.trim().parse().ok()?;
    (major > 4 || (major == 4 && minor >= 2)).then(|| format!("Rtools{major}{minor}"))
}

/// Development packages for the libraries whose headers or names turn up in
/// build errors: (what to look for, library name, apt, dnf, brew).
const LIBRARIES: &[(&str, &str, &str, &str, &str)] = &[
    (
        "curl/curl.h",
        "libcurl",
        "libcurl4-openssl-dev",
        "libcurl-devel",
        "curl",
    ),
    (
        "libcurl",
        "libcurl",
        "libcurl4-openssl-dev",
        "libcurl-devel",
        "curl",
    ),
    (
        "libxml/",
        "libxml2",
        "libxml2-dev",
        "libxml2-devel",
        "libxml2",
    ),
    (
        "libxml-2.0",
        "libxml2",
        "libxml2-dev",
        "libxml2-devel",
        "libxml2",
    ),
    (
        "-lxml2",
        "libxml2",
        "libxml2-dev",
        "libxml2-devel",
        "libxml2",
    ),
    (
        "openssl/",
        "OpenSSL",
        "libssl-dev",
        "openssl-devel",
        "openssl@3",
    ),
    (
        "libssl",
        "OpenSSL",
        "libssl-dev",
        "openssl-devel",
        "openssl@3",
    ),
    ("png.h", "libpng", "libpng-dev", "libpng-devel", "libpng"),
    ("libpng", "libpng", "libpng-dev", "libpng-devel", "libpng"),
    (
        "jpeglib.h",
        "libjpeg",
        "libjpeg-dev",
        "libjpeg-turbo-devel",
        "jpeg",
    ),
    (
        "tiffio.h",
        "libtiff",
        "libtiff-dev",
        "libtiff-devel",
        "libtiff",
    ),
    (
        "ft2build.h",
        "FreeType",
        "libfreetype6-dev",
        "freetype-devel",
        "freetype",
    ),
    (
        "freetype2",
        "FreeType",
        "libfreetype6-dev",
        "freetype-devel",
        "freetype",
    ),
    (
        "hb.h",
        "HarfBuzz",
        "libharfbuzz-dev",
        "harfbuzz-devel",
        "harfbuzz",
    ),
    (
        "harfbuzz",
        "HarfBuzz",
        "libharfbuzz-dev",
        "harfbuzz-devel",
        "harfbuzz",
    ),
    (
        "fribidi",
        "FriBidi",
        "libfribidi-dev",
        "fribidi-devel",
        "fribidi",
    ),
    ("gmp.h", "GMP", "libgmp-dev", "gmp-devel", "gmp"),
    ("zlib.h", "zlib", "zlib1g-dev", "zlib-devel", "zlib"),
    (
        "sqlite3.h",
        "SQLite",
        "libsqlite3-dev",
        "sqlite-devel",
        "sqlite",
    ),
    ("cairo.h", "Cairo", "libcairo2-dev", "cairo-devel", "cairo"),
    (
        "udunits2.h",
        "UDUNITS",
        "libudunits2-dev",
        "udunits2-devel",
        "udunits",
    ),
    ("gdal", "GDAL", "libgdal-dev", "gdal-devel", "gdal"),
    ("proj", "PROJ", "libproj-dev", "proj-devel", "proj"),
    ("geos", "GEOS", "libgeos-dev", "geos-devel", "geos"),
    ("glpk", "GLPK", "libglpk-dev", "glpk-devel", "glpk"),
    ("nlopt", "NLopt", "libnlopt-dev", "NLopt-devel", "nlopt"),
];

fn library_steps(label: &str, apt: &str, dnf: &str, brew: &str, os: Os) -> Vec<String> {
    match os {
        Os::Linux => vec![
            format!("Install the development files for {label}, then try again."),
            format!("Debian and Ubuntu: sudo apt install {apt}"),
            format!("Fedora and Red Hat: sudo dnf install {dnf}"),
        ],
        Os::MacOs => vec![
            format!("Install {label} with Homebrew (brew.sh), then try again."),
            format!("In Terminal: brew install {brew}"),
        ],
        Os::Windows => vec![format!(
            "{label} comes inside Windows binary packages. Try \"Binary only\"; building \
                 from source on Windows needs Rtools, which brings its own libraries."
        )],
    }
}

fn no_compiler(os: Os, r_version: &str) -> Diagnosis {
    let steps = match os {
        Os::Windows => {
            let tools = rtools_name(r_version)
                .unwrap_or_else(|| "the Rtools that matches your R".to_string());
            vec![
                "Most CRAN packages have Windows binaries that need no compiler: try \"Binary only\"."
                    .to_string(),
                format!(
                    "To build from source, install {tools} from \
                     https://cran.r-project.org/bin/windows/Rtools/ with the default settings, \
                     restart RCraft, and try again."
                ),
            ]
        }
        Os::MacOs => vec![
            "Install Apple's command line tools: open Terminal and run  xcode-select --install"
                .to_string(),
            "Packages with Fortran code also need gfortran from \
             https://mac.r-project.org/tools/ ."
                .to_string(),
            "Then restart RCraft and try again. \"Binary only\" avoids building where a binary exists."
                .to_string(),
        ],
        Os::Linux => vec![
            "Install the build tools, then try again.".to_string(),
            "Debian and Ubuntu: sudo apt install build-essential gfortran".to_string(),
            "Fedora and Red Hat: sudo dnf groupinstall \"Development Tools\" && sudo dnf install gcc-gfortran"
                .to_string(),
        ],
    };
    let mut retry = vec![Retry::Again];
    if os != Os::Linux {
        retry.insert(0, Retry::BinaryOnly);
    }
    Diagnosis {
        cause: Cause::NoCompiler,
        summary: "This package has to be built from source here, and R cannot find the \
                  tools to build it (a compiler and make)."
            .to_string(),
        steps,
        retry,
    }
}

/// Plain-language findings in what R printed during an install (its output,
/// warnings and errors, joined). Empty when nothing is recognised.
pub fn diagnose(text: &str, os: Os, r_version: &str) -> Vec<Diagnosis> {
    let t = normalise(text);
    let lower = t.to_lowercase();
    let has = |s: &str| lower.contains(&s.to_lowercase());
    let mut out: Vec<Diagnosis> = Vec::new();

    // an unreachable mirror explains the "not available" messages that follow it
    if has("unable to access index for repository")
        || has("could not resolve host")
        || has("failed to connect")
        || has("ssl certificate problem")
        || has("timed out")
        || has("connection reset")
    {
        out.push(Diagnosis {
            cause: Cause::Network,
            summary: "R could not reach the package mirror.".to_string(),
            steps: vec![
                "Check that this computer is online, and that a proxy or firewall is not \
                 blocking R (it must reach the mirror address shown in the Packages window)."
                    .to_string(),
                "If the mirror itself is down or slow, choose another one under \"Change…\" \
                 in the Packages window."
                    .to_string(),
            ],
            retry: vec![Retry::Again, Retry::ChangeMirror],
        });
        return out;
    }

    let compiler_missing = has("'make' not found")
        || has("make: command not found")
        || has("make: not found")
        || has("gcc: command not found")
        || has("g++: command not found")
        || has("gfortran: command not found")
        || has("unable to execute 'gcc'")
        || has("c compiler cannot create executables")
        || has("no acceptable c compiler")
        || has("invalid active developer path")
        || has("xcrun: error")
        || has("command line tools")
        || has("error: compilation failed") && has("not found");
    if compiler_missing {
        out.push(no_compiler(os, r_version));
    }

    // a header or library that is missing
    let missing_lib = has("fatal error:") && has("no such file or directory")
        || has("cannot find -l")
        || has("configuration failed because")
        || has("configure: error:")
        || has("was not found") && has("pkg-config")
        || has("package '") && has("' was not found") && has("pkg_config");
    if missing_lib && !compiler_missing {
        let found = LIBRARIES.iter().find(|(needle, ..)| has(needle));
        let (label, steps) = match found {
            Some((_, label, apt, dnf, brew)) => (
                (*label).to_string(),
                library_steps(label, apt, dnf, brew, os),
            ),
            None => {
                let name = between(&t, "Configuration failed because ", " was not found")
                    .or_else(|| between(&t, "cannot find -l", "\n"))
                    .map(str::trim)
                    .filter(|n| !n.is_empty() && n.len() < 60)
                    .unwrap_or("a system library")
                    .to_string();
                let steps = match os {
                    Os::Linux => vec![
                        format!(
                            "Install the development package for {name} (its name usually \
                             ends in -dev on Debian and Ubuntu, or -devel on Fedora), then try \
                             again."
                        ),
                        "R's output below names the file or library it looked for.".to_string(),
                    ],
                    Os::MacOs => vec![format!(
                        "Install {name} with Homebrew (brew install ...), then try again."
                    )],
                    Os::Windows => vec![
                        "Try \"Binary only\": Windows binaries bring their libraries with them."
                            .to_string(),
                    ],
                };
                (name, steps)
            }
        };
        let mut retry = vec![Retry::Again];
        if os != Os::Linux {
            retry.insert(0, Retry::BinaryOnly);
        }
        out.push(Diagnosis {
            cause: Cause::MissingLibrary(label.clone()),
            summary: format!(
                "Building this package needs {label}, which is not installed on this computer."
            ),
            steps,
            retry,
        });
    }

    if has("is not writable")
        || has("permission denied")
        || has("cannot create dir")
        || has("failed to lock directory")
        || has("cannot remove prior installation")
    {
        out.push(Diagnosis {
            cause: Cause::NotWritable,
            summary: "R cannot write to the package folder, or a package in it is in use."
                .to_string(),
            steps: vec![
                "RCraft installs into its own library folder. Check that the folder is not \
                 read-only and that no antivirus or sync program is holding files in it."
                    .to_string(),
                "If a package is loaded in this R session, restart R (Session > Restart R) and \
                 try again before loading it."
                    .to_string(),
            ],
            retry: vec![Retry::Again],
        });
    }

    // the package needs a newer R than this
    // "package 'x' is not available ... depends on R (>= 99.0)" (from the index) or
    // "this R is version 4.4.3, package 'x' requires R >= 99.0" (from the build)
    let newer_r = between(&t, "package '", "' is not available for this version of R")
        .zip(between(&t, "depends on R (>= ", ")"))
        .or_else(|| {
            between(&t, ", package '", "' requires R").zip(
                between(&t, "requires R >= ", "\n")
                    .or_else(|| t.split("requires R >= ").nth(1))
                    .map(|v| v.split_whitespace().next().unwrap_or(v)),
            )
        });
    if let Some((pkg, needs)) = newer_r {
        let pkg = pkg.trim().to_string();
        let needs = needs.trim().trim_end_matches('.').to_string();
        out.push(Diagnosis {
            cause: Cause::NeedsNewerR(pkg.clone(), needs.clone()),
            summary: format!(
                "The newest version of \"{pkg}\" needs R {needs} or newer, and this is R {r_version}."
            ),
            steps: vec![
                "Update R (RCraft uses the R it found: install a newer R from \
                 https://cloud.r-project.org/ and restart RCraft), or install an older version \
                 of the package that works with this R."
                    .to_string(),
            ],
            retry: vec![Retry::OlderVersion(pkg)],
        });
    }

    if let Some(dep) = last_between(&t, "dependency '", "' is not available for package '") {
        let pkg = between(&t, "' is not available for package '", "'").unwrap_or("");
        if !has("is in the repositories but depends on") {
            out.push(Diagnosis {
                cause: Cause::DependencyUnavailable(pkg.to_string(), dep.to_string()),
                summary: format!(
                    "\"{pkg}\" needs the package \"{dep}\", which the mirror does not offer for \
                     this version of R (it may have been removed from CRAN, or need a newer R)."
                ),
                steps: vec![
                    format!("Look for \"{dep}\" in the Packages window to see whether CRAN still has it."),
                    format!("An older version of \"{pkg}\" may not need it."),
                ],
                retry: vec![Retry::OlderVersion(pkg.to_string())],
            });
        }
    }

    if has("available as a source package but not as a binary")
        || has("only available in source form")
        || has("source package newer than binary")
    {
        let tools = match os {
            Os::Windows => format!(
                "Build it from source: install {} first.",
                rtools_name(r_version).unwrap_or_else(|| "Rtools".to_string())
            ),
            Os::MacOs => "Build it from source: install the command line tools first \
                          (xcode-select --install)."
                .to_string(),
            Os::Linux => {
                "Linux builds from source; install the build tools if they are missing.".to_string()
            }
        };
        out.push(Diagnosis {
            cause: Cause::SourceOnly,
            summary: "CRAN has this version only as source, and no binary for this system yet."
                .to_string(),
            steps: vec![
                tools,
                "Binaries usually appear a few days after a release: you can also wait, or \
                 install the previous version."
                    .to_string(),
            ],
            retry: vec![Retry::FromSource],
        });
    }

    if out.is_empty() {
        if let Some(pkg) = between(&t, "package '", "' is not available") {
            // "for this version of R" too: the name is not in the repository
            out.push(Diagnosis {
                cause: Cause::NotFound(pkg.trim().to_string()),
                summary: format!(
                    "The mirror has no package called \"{}\" for this version of R.",
                    pkg.trim()
                ),
                steps: vec![
                    "Check the spelling (package names are case sensitive), or search for it in \
                     the Packages window."
                        .to_string(),
                ],
                retry: vec![],
            });
        } else if let Some(pkg) = between(
            &t,
            "installation of package '",
            "' had non-zero exit status",
        ) {
            out.push(Diagnosis {
                cause: Cause::Failed(pkg.trim().to_string()),
                summary: format!(
                    "R stopped installing \"{}\" and did not give a clearer reason.",
                    pkg.trim()
                ),
                steps: vec![
                    "R's own output is above: its last lines usually name what failed.".to_string(),
                ],
                retry: if os == Os::Linux {
                    vec![Retry::Again]
                } else {
                    vec![Retry::BinaryOnly, Retry::Again]
                },
            });
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// What R 4.4.3 on Windows without Rtools printed for a package with C code.
    const WIN_NO_MAKE: &str = "* installing *source* package 'pfcompiled' ...\n** using staged installation\n** libs\nError in system(paste(MAKE, p1(paste(\"-f\", shQuote(makefiles))), \"compilers\"),  : \n  'make' not found\n* removing 'C:/Users/me/lib/pfcompiled'\n\ninstallation of package 'pfcompiled' had non-zero exit status";

    /// A configure script that fails, on Windows without Rtools.
    const WIN_CONFIG: &str = "C:\\R\\bin/config.sh: line 187: make: command not found\nWarning in system2(file.path(R.home(\"bin\"), \"Rcmd.exe\"), c(\"config\", x),  :\n  running command 'Rcmd.exe config CC' had status 127\n* removing 'lib/pfconfig'\ninstallation of package 'pfconfig' had non-zero exit status";

    const NEWER_R: &str = "package 'pfnewr' is not available for this version of R\n'pfnewr' version 1.0 is in the repositories but depends on R (>= 99.0)\n\nA version of this package for your version of R might be available elsewhere,\nsee the ideas at\nhttps://cran.r-project.org/doc/manuals/r-patched/R-admin.html#Installing-packages";

    const MISSING_DEP: &str = "dependency 'pfnothere' is not available\nERROR: dependency 'pfnothere' is not available for package 'pfdep'\n* removing 'lib/pfdep'\ninstallation of package 'pfdep' had non-zero exit status";

    const NOT_FOUND: &str = "package 'pfabsent' is not available for this version of R\n\nA version of this package for your version of R might be available elsewhere,\nsee the ideas at\nhttps://cran.r-project.org/doc/manuals/r-patched/R-admin.html#Installing-packages";

    const OFFLINE: &str = "unable to access index for repository https://x.invalid/src/contrib:\n  cannot open URL 'https://x.invalid/src/contrib/PACKAGES'\npackage 'x' is not available for this version of R\n\nA version of this package for your version of R might be available elsewhere,\nsee the ideas at\nhttps://cran.r-project.org/doc/manuals/r-patched/R-admin.html#Installing-packages\nunable to access index for repository https://x.invalid/bin/windows/contrib/4.4:\n  cannot open URL 'https://x.invalid/bin/windows/contrib/4.4/PACKAGES'";

    const NOT_WRITABLE: &str =
        "'lib = \"C:/Windows/System32/x\"' is not writable\nunable to install packages";

    fn causes(text: &str, os: Os) -> Vec<Cause> {
        diagnose(text, os, "4.4.3")
            .into_iter()
            .map(|d| d.cause)
            .collect()
    }

    #[test]
    fn a_missing_make_on_windows_names_rtools_for_this_r_and_the_binary_way_out() {
        let d = diagnose(WIN_NO_MAKE, Os::Windows, "4.4.3");
        assert_eq!(d.len(), 1, "{d:?}");
        assert_eq!(d[0].cause, Cause::NoCompiler);
        let steps = d[0].steps.join("\n");
        assert!(steps.contains("Rtools44"), "{steps}");
        assert!(steps.contains("https://cran.r-project.org/bin/windows/Rtools/"));
        assert_eq!(d[0].retry, vec![Retry::BinaryOnly, Retry::Again]);
        // the same text on the other systems gets that system's tools
        let mac = diagnose(WIN_NO_MAKE, Os::MacOs, "4.4.3");
        assert!(mac[0].steps.join("\n").contains("xcode-select --install"));
        let linux = diagnose(WIN_NO_MAKE, Os::Linux, "4.4.3");
        assert!(linux[0]
            .steps
            .join("\n")
            .contains("sudo apt install build-essential"));
        assert_eq!(linux[0].retry, vec![Retry::Again]);
    }

    #[test]
    fn a_configure_script_with_no_make_is_a_missing_compiler_too() {
        assert_eq!(causes(WIN_CONFIG, Os::Windows), vec![Cause::NoCompiler]);
    }

    #[test]
    fn a_missing_header_names_the_system_package_for_each_system() {
        let text = "gcc -I/usr/share/R/include -fpic -c xml.c -o xml.o\nxml.c:1:10: fatal error: libxml/parser.h: No such file or directory\ncompilation terminated.\nmake: *** [xml.o] Error 1\nERROR: compilation failed for package 'xml'";
        let linux = diagnose(text, Os::Linux, "4.4.3");
        assert_eq!(linux.len(), 1, "{linux:?}");
        assert_eq!(linux[0].cause, Cause::MissingLibrary("libxml2".into()));
        let steps = linux[0].steps.join("\n");
        assert!(steps.contains("sudo apt install libxml2-dev"), "{steps}");
        assert!(steps.contains("sudo dnf install libxml2-devel"), "{steps}");
        let mac = diagnose(text, Os::MacOs, "4.4.3");
        assert!(mac[0].steps.join("\n").contains("brew install libxml2"));
        let win = diagnose(text, Os::Windows, "4.4.3");
        assert!(win[0].steps.join("\n").contains("Binary only"));
    }

    #[test]
    fn a_configure_error_for_an_unknown_library_still_names_it() {
        let text = "checking for foo... no\nConfiguration failed because libbarbaz-1.0 was not found. Try installing:\n * deb: libbarbaz-dev\nERROR: configuration failed for package 'baz'";
        let d = diagnose(text, Os::Linux, "4.4.3");
        assert_eq!(
            d[0].cause,
            Cause::MissingLibrary("libbarbaz-1.0".into()),
            "{d:?}"
        );
        assert!(d[0].steps[0].contains("-dev"));
    }

    #[test]
    fn a_package_that_needs_a_newer_r_offers_an_older_version_and_says_what_is_needed() {
        let d = diagnose(NEWER_R, Os::Windows, "4.4.3");
        assert_eq!(d.len(), 1, "{d:?}");
        assert_eq!(
            d[0].cause,
            Cause::NeedsNewerR("pfnewr".into(), "99.0".into())
        );
        assert!(d[0].summary.contains("R 99.0 or newer") && d[0].summary.contains("R 4.4.3"));
        assert_eq!(d[0].retry, vec![Retry::OlderVersion("pfnewr".into())]);
    }

    #[test]
    fn a_missing_dependency_names_both_packages() {
        let d = diagnose(MISSING_DEP, Os::Windows, "4.4.3");
        assert_eq!(
            d.first().map(|d| d.cause.clone()),
            Some(Cause::DependencyUnavailable(
                "pfdep".into(),
                "pfnothere".into()
            )),
            "{d:?}"
        );
    }

    #[test]
    fn a_misspelt_name_is_not_found_and_an_unreachable_mirror_is_a_network_problem() {
        assert_eq!(
            causes(NOT_FOUND, Os::Windows),
            vec![Cause::NotFound("pfabsent".into())]
        );
        // the "not available" lines after an unreachable index are not blamed on the name
        assert_eq!(causes(OFFLINE, Os::Windows), vec![Cause::Network]);
        let d = diagnose(OFFLINE, Os::Windows, "4.4.3");
        assert_eq!(d[0].retry, vec![Retry::Again, Retry::ChangeMirror]);
    }

    #[test]
    fn an_unwritable_library_is_said_plainly() {
        assert_eq!(causes(NOT_WRITABLE, Os::Windows), vec![Cause::NotWritable]);
    }

    #[test]
    fn source_only_packages_say_so() {
        let text = "Warning: package 'abc' is available as a source package but not as a binary";
        assert_eq!(causes(text, Os::Windows), vec![Cause::SourceOnly]);
        let d = diagnose(text, Os::Windows, "4.4.3");
        assert!(d[0].steps[0].contains("Rtools44"));
        assert_eq!(d[0].retry, vec![Retry::FromSource]);
    }

    #[test]
    fn an_unexplained_failure_falls_back_to_the_package_name_and_success_gives_nothing() {
        let text = "* installing *source* package 'odd' ...\nsomething unexpected\ninstallation of package 'odd' had non-zero exit status";
        assert_eq!(causes(text, Os::Linux), vec![Cause::Failed("odd".into())]);
        assert!(diagnose(
            "package 'x' successfully unpacked and MD5 sums checked",
            Os::Linux,
            "4.4.3"
        )
        .is_empty());
        assert!(diagnose("", Os::Windows, "4.4.3").is_empty());
    }

    #[test]
    fn curly_quotes_and_other_r_versions_are_handled() {
        let curly = NEWER_R.replace('\'', "\u{2019}");
        assert_eq!(causes(&curly, Os::Linux).len(), 1);
        assert_eq!(rtools_name("4.2.1").as_deref(), Some("Rtools42"));
        assert_eq!(rtools_name("4.5.0").as_deref(), Some("Rtools45"));
        assert_eq!(rtools_name("4.1.3"), None);
        assert_eq!(rtools_name("garbage"), None);
        let d = diagnose(WIN_NO_MAKE, Os::Windows, "4.1.3");
        assert!(d[0]
            .steps
            .join("\n")
            .contains("the Rtools that matches your R"));
    }
}
