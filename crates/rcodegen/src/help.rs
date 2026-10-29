//! R help inside RCraft (Phase 2A): the typed forms (`?mean`, `help(mean)`,
//! `help mean`, `help search words`), the R code that fetches a help page or
//! searches the installed packages' help, and readers for what R answers.
//!
//! R does the work: the page is R's own help file for the installed package
//! (`utils::help`, rendered to text by `tools::Rd2txt`, its examples by
//! `tools::Rd2ex`), so it always matches the packages in this session and
//! works with no network. Nothing is copied into RCraft; the window shows what
//! R produced.

use crate::import::r_string;

/// Most search results kept.
pub const MAX_HITS: usize = 100;
/// Most "did you mean" names kept for a topic R does not have.
pub const MAX_SUGGESTIONS: usize = 12;

/// What a typed help line asks for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HelpCommand {
    /// The help page of one topic, in one package when given.
    Page {
        topic: String,
        package: Option<String>,
    },
    /// A search over the installed packages' help.
    Search { words: String },
    /// One package's page: its description, the index of its help topics, its
    /// vignettes and whether it has a NEWS file.
    Package { package: String },
    /// The vignettes of one installed package, or of all of them.
    Vignettes { package: Option<String> },
    /// One package's NEWS file.
    News { package: String },
    /// The text of a vignette file, by its path. Never typed: only the window
    /// asks for it, with a path from a vignette list, and R refuses a file that
    /// is not inside an installed package.
    File { path: String },
}

fn is_topic(s: &str) -> bool {
    let mut cs = s.chars();
    cs.next()
        .is_some_and(|c| c.is_alphabetic() || c == '.' || c == '[' || c == '$' || c == '%')
        && s.len() <= 100
        && !s
            .chars()
            .any(|c| c.is_whitespace() || c == '"' || c == '\'')
}

pub(crate) fn is_package(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 100
        && s.chars().all(|c| c.is_alphanumeric() || c == '.')
        && s.chars().next().is_some_and(char::is_alphabetic)
}

/// `pkg::name` or `name`, as a page request (a help link on a dialog page).
pub fn page_of(word: &str) -> Option<HelpCommand> {
    if let Some((pkg, topic)) = word.split_once("::") {
        let topic = topic.strip_prefix(':').unwrap_or(topic);
        return (is_package(pkg) && is_topic(topic)).then(|| HelpCommand::Page {
            topic: topic.to_string(),
            package: Some(pkg.to_string()),
        });
    }
    is_topic(word).then(|| HelpCommand::Page {
        topic: word.to_string(),
        package: None,
    })
}

fn unquote(s: &str) -> &str {
    let s = s.trim();
    for q in ['"', '\''] {
        if let Some(inner) = s.strip_prefix(q).and_then(|r| r.strip_suffix(q)) {
            return inner;
        }
    }
    s
}

/// The help request in a typed line, or `None` when it is ordinary R (which
/// then runs as typed). Forms: `?topic`, `?pkg::topic`, `??words`,
/// `help(topic)`, `help("topic", package = "pkg")`, `help topic`,
/// `help pkg::topic` and `help search words`.
pub fn parse_command(line: &str) -> Option<HelpCommand> {
    let t = line.trim();
    if let Some(words) = t.strip_prefix("??") {
        let words = words.trim();
        return (!words.is_empty()).then(|| HelpCommand::Search {
            words: unquote(words).to_string(),
        });
    }
    if let Some(rest) = t.strip_prefix('?') {
        let rest = unquote(rest);
        return page_of(rest);
    }
    if let Some(command) = browse_vignettes(t) {
        return Some(command);
    }
    let rest = t.strip_prefix("help")?;
    if let Some(args) = rest.trim_start().strip_prefix('(') {
        let args = args.trim_end().strip_suffix(')')?;
        if args.contains(['(', ')']) {
            return None;
        }
        let mut topic: Option<String> = None;
        let mut package: Option<String> = None;
        for (i, part) in args.split(',').enumerate() {
            let part = part.trim();
            match part.split_once('=') {
                Some((name, value)) => match name.trim() {
                    "topic" => topic = Some(unquote(value).to_string()),
                    "package" => package = Some(unquote(value).to_string()),
                    _ => return None,
                },
                None if i == 0 => topic = Some(unquote(part).to_string()),
                None => return None,
            }
        }
        let Some(topic) = topic else {
            return package
                .filter(|p| is_package(p))
                .map(|package| HelpCommand::Package { package });
        };
        if !is_topic(&topic) || package.as_deref().is_some_and(|p| !is_package(p)) {
            return None;
        }
        return Some(HelpCommand::Page { topic, package });
    }
    // `help topic` (a space, then a word that is not an assignment)
    if !rest.starts_with(char::is_whitespace) {
        return None;
    }
    let rest = rest.trim();
    for (word, make) in [
        (
            "package",
            package_command as fn(&str) -> Option<HelpCommand>,
        ),
        ("manual", package_command),
        ("news", news_command),
        ("vignettes", vignettes_command),
    ] {
        if rest == word && word == "vignettes" {
            return Some(HelpCommand::Vignettes { package: None });
        }
        if let Some(arg) = rest.strip_prefix(word) {
            if arg.starts_with(char::is_whitespace) {
                return make(unquote(arg.trim()));
            }
        }
    }
    if let Some(words) = rest.strip_prefix("search") {
        if words.starts_with(char::is_whitespace) && !words.trim().is_empty() {
            return Some(HelpCommand::Search {
                words: unquote(words.trim()).to_string(),
            });
        }
    }
    if rest.is_empty() || rest.contains(char::is_whitespace) {
        return None;
    }
    page_of(unquote(rest))
}

fn package_command(arg: &str) -> Option<HelpCommand> {
    is_package(arg).then(|| HelpCommand::Package {
        package: arg.to_string(),
    })
}

fn news_command(arg: &str) -> Option<HelpCommand> {
    is_package(arg).then(|| HelpCommand::News {
        package: arg.to_string(),
    })
}

fn vignettes_command(arg: &str) -> Option<HelpCommand> {
    is_package(arg).then(|| HelpCommand::Vignettes {
        package: Some(arg.to_string()),
    })
}

/// `browseVignettes()`, `browseVignettes("pkg")` or `browseVignettes(package = "pkg")`.
fn browse_vignettes(t: &str) -> Option<HelpCommand> {
    let args = t
        .strip_prefix("browseVignettes")?
        .trim_start()
        .strip_prefix('(')?
        .trim_end()
        .strip_suffix(')')?
        .trim();
    if args.is_empty() {
        return Some(HelpCommand::Vignettes { package: None });
    }
    let arg = args
        .strip_prefix("package")
        .and_then(|a| a.trim_start().strip_prefix('='))
        .unwrap_or(args);
    vignettes_command(unquote(arg.trim()))
}

/// The topic under the end of `line`, for F1: the last word of identifier
/// characters (with `::`), or `None`.
pub fn word_at_end(line: &str) -> Option<String> {
    let end = line.trim_end();
    let start = end
        .rfind(|c: char| !(c.is_alphanumeric() || matches!(c, '.' | '_' | ':')))
        .map_or(0, |i| {
            i + end
                .get(i..)
                .and_then(|r| r.chars().next())
                .map_or(1, char::len_utf8)
        });
    let word = end.get(start..)?;
    page_of(word).map(|_| word.to_string())
}

/// R code that prints the help page of `topic`: see [`parse_page`].
pub fn page_code(topic: &str, package: Option<&str>) -> String {
    let package = package.map_or("NULL".to_string(), r_string);
    format!(
        "local({{\n\
         topic <- {topic}\n\
         h <- tryCatch(utils::help(topic, package = {package}), error = function(e) character())\n\
         lines <- function(prefix, x) for (l in x) cat(prefix, l, \"\\n\", sep = \"\")\n\
         if (length(h) == 0L) {{\n\
           cat(\"@@NONE\\n\")\n\
           for (s in head(utils::apropos(topic), {sug})) cat(\"@@SUGGEST\\t\", s, \"\\n\", sep = \"\")\n\
         }} else {{\n\
           paths <- as.character(h)\n\
           pkgs <- basename(dirname(dirname(paths)))\n\
           rd <- utils:::.getHelpFile(paths[1L])\n\
           tags <- vapply(rd, function(x) {{ t <- attr(x, \"Rd_tag\"); if (is.null(t)) \"\" else t }}, \"\")\n\
           title <- paste(trimws(unlist(rd[tags == \"\\\\title\"])), collapse = \" \")\n\
           cat(\"@@PACKAGE\\t\", pkgs[1L], \"\\n\", sep = \"\")\n\
           cat(\"@@TOPIC\\t\", topic, \"\\n\", sep = \"\")\n\
           cat(\"@@TITLE\\t\", gsub(\"[\\t\\r\\n]+\", \" \", title), \"\\n\", sep = \"\")\n\
           for (p in unique(pkgs[-1L])) cat(\"@@ALSO\\t\", p, \"\\n\", sep = \"\")\n\
           links <- character()\n\
           walk <- function(e) {{\n\
             if (identical(attr(e, \"Rd_tag\"), \"\\\\link\")) links <<- c(links, paste(unlist(e), collapse = \"\"))\n\
             if (is.list(e)) for (i in e) walk(i)\n\
           }}\n\
           walk(rd[tags == \"\\\\seealso\"])\n\
           for (l in unique(links)) cat(\"@@LINK\\t\", l, \"\\n\", sep = \"\")\n\
           tf <- tempfile()\n\
           tools::Rd2txt(rd, out = tf, options = list(underline_titles = FALSE))\n\
           cat(\"@@TEXT\\n\"); lines(\"|\", readLines(tf, warn = FALSE, encoding = \"UTF-8\")); cat(\"@@END\\n\")\n\
           ex <- tempfile()\n\
           tools::Rd2ex(rd, out = ex)\n\
           cat(\"@@EXAMPLES\\n\"); lines(\"|\", if (file.exists(ex)) readLines(ex, warn = FALSE, encoding = \"UTF-8\") else character()); cat(\"@@END\\n\")\n\
         }}\n\
         }})",
        topic = r_string(topic),
        package = package,
        sug = MAX_SUGGESTIONS,
    )
}

/// A help page as R gave it.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Page {
    pub topic: String,
    pub package: String,
    pub title: String,
    /// R's rendering of the page: title, description, usage, arguments, value,
    /// details, references, see also and examples, as plain text.
    pub text: String,
    /// The page's examples as code, without R's header comments. Empty when
    /// the page has none.
    pub examples: String,
    /// Topics named in "See also".
    pub links: Vec<String>,
    /// Other packages with a page for the same topic.
    pub also_in: Vec<String>,
}

/// What R answered to [`page_code`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PageAnswer {
    Found(Box<Page>),
    /// R has no such topic; these topics have it in their name.
    NotFound {
        suggestions: Vec<String>,
    },
}

/// Reads R's answer. `None` when it is not an answer at all.
pub fn parse_page(answer: &str) -> Option<PageAnswer> {
    let mut page = Page::default();
    let mut suggestions: Vec<String> = Vec::new();
    let mut none = false;
    let mut found = false;
    // 0: header lines, 1: text, 2: examples
    let mut block = 0u8;
    let mut text: Vec<&str> = Vec::new();
    let mut examples: Vec<&str> = Vec::new();
    for l in answer.lines() {
        match block {
            1 | 2 => {
                if l == "@@END" {
                    block = 0;
                } else if let Some(body) = l.strip_prefix('|') {
                    if block == 1 {
                        text.push(body);
                    } else {
                        examples.push(body);
                    }
                }
            }
            _ => {
                if l == "@@NONE" {
                    none = true;
                } else if l == "@@TEXT" {
                    block = 1;
                } else if l == "@@EXAMPLES" {
                    block = 2;
                } else if let Some(v) = l.strip_prefix("@@PACKAGE\t") {
                    page.package = v.to_string();
                    found = true;
                } else if let Some(v) = l.strip_prefix("@@TOPIC\t") {
                    page.topic = v.to_string();
                } else if let Some(v) = l.strip_prefix("@@TITLE\t") {
                    page.title = v.to_string();
                } else if let Some(v) = l.strip_prefix("@@LINK\t") {
                    page.links.push(v.to_string());
                } else if let Some(v) = l.strip_prefix("@@ALSO\t") {
                    page.also_in.push(v.to_string());
                } else if let Some(v) = l.strip_prefix("@@SUGGEST\t") {
                    suggestions.push(v.to_string());
                }
            }
        }
    }
    if none {
        return Some(PageAnswer::NotFound { suggestions });
    }
    if !found || text.is_empty() {
        return None;
    }
    page.text = text.join("\n");
    page.examples = clean_examples(&examples);
    Some(PageAnswer::Found(Box::new(page)))
}

/// The examples without the generated header (`### Name:` and so on) and
/// without blank lines at either end.
fn clean_examples(lines: &[&str]) -> String {
    let after_header = lines
        .iter()
        .position(|l| l.starts_with("### ** Examples"))
        .map_or(lines, |i| lines.get(i + 1..).unwrap_or(&[]));
    let joined = after_header.join("\n");
    joined.trim().to_string()
}

/// R code that searches the installed packages' help for `words`: see
/// [`parse_search`].
pub fn search_code(words: &str) -> String {
    format!(
        "local({{\n\
         clean <- function(x) gsub(\"[\\t\\r\\n]+\", \" \", x)\n\
         words <- strsplit(trimws({words}), \"[[:space:]]+\")[[1L]]\n\
         words <- words[nzchar(words)]\n\
         found <- lapply(words, function(w) {{\n\
           m <- tryCatch(utils::help.search(w, fields = c(\"alias\", \"title\", \"keyword\"), ignore.case = TRUE)$matches, error = function(e) NULL)\n\
           if (is.null(m)) return(NULL)\n\
           m[m[, \"Type\"] == \"help\", c(\"Package\", \"Topic\", \"Title\"), drop = FALSE]\n\
         }})\n\
         found <- found[!vapply(found, is.null, TRUE)]\n\
         if (length(found)) {{\n\
           keyof <- function(m) paste(m[, \"Package\"], m[, \"Topic\"], sep = \"\\r\")\n\
           n <- table(unlist(lapply(found, function(m) unique(keyof(m)))))\n\
           keep <- names(n)[n == length(found)]\n\
           if (!length(keep)) keep <- names(n)[n == max(n)]\n\
           all <- do.call(rbind, found)\n\
           k <- keyof(all)\n\
           all <- all[!duplicated(k) & k %in% keep, , drop = FALSE]\n\
           for (i in seq_len(min(nrow(all), {max}))) cat(\"@@HIT\\t\", clean(all[i, \"Package\"]), \"\\t\", clean(all[i, \"Topic\"]), \"\\t\", clean(all[i, \"Title\"]), \"\\n\", sep = \"\")\n\
         }}\n\
         cat(\"@@DONE\\n\")\n\
         }})",
        words = r_string(words),
        max = MAX_HITS * 5,
    )
}

/// One search result.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Hit {
    pub package: String,
    pub topic: String,
    pub title: String,
}

/// Reads R's answer to [`search_code`], ranked for `words`: a topic equal to
/// the words first, then topics that start with them or contain them, then
/// pages of the packages every session has (base, stats, utils, graphics),
/// then the rest, each group in R's order. `None` when R did not finish.
pub fn parse_search(words: &str, answer: &str) -> Option<Vec<Hit>> {
    let mut done = false;
    let mut hits: Vec<Hit> = Vec::new();
    for l in answer.lines() {
        if l == "@@DONE" {
            done = true;
        } else if let Some(rest) = l.strip_prefix("@@HIT\t") {
            let mut f = rest.splitn(3, '\t');
            if let (Some(p), Some(t), Some(ti)) = (f.next(), f.next(), f.next()) {
                hits.push(Hit {
                    package: p.to_string(),
                    topic: t.to_string(),
                    title: ti.to_string(),
                });
            }
        }
    }
    if !done {
        return None;
    }
    let q = words.trim().to_lowercase();
    let core = ["base", "stats", "utils", "graphics", "methods", "datasets"];
    let rank = |h: &Hit| -> u8 {
        let t = h.topic.to_lowercase();
        if t == q {
            0
        } else if t.starts_with(&q) {
            1
        } else if t.contains(&q) {
            2
        } else if core.contains(&h.package.as_str()) {
            3
        } else {
            4
        }
    };
    // a stable sort keeps R's order within a rank
    hits.sort_by_key(rank);
    hits.truncate(MAX_HITS);
    Some(hits)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn page(topic: &str, package: Option<&str>) -> Option<HelpCommand> {
        Some(HelpCommand::Page {
            topic: topic.to_string(),
            package: package.map(str::to_string),
        })
    }

    #[test]
    fn typed_forms_are_understood() {
        assert_eq!(parse_command("?mean"), page("mean", None));
        assert_eq!(parse_command("  ? mean "), page("mean", None));
        assert_eq!(parse_command("?stats::lm"), page("lm", Some("stats")));
        assert_eq!(parse_command("help(mean)"), page("mean", None));
        assert_eq!(parse_command("help(\"mean\")"), page("mean", None));
        assert_eq!(
            parse_command("help(\"lm\", package = \"stats\")"),
            page("lm", Some("stats"))
        );
        assert_eq!(
            parse_command("help(topic = 'lm', package = stats)"),
            page("lm", Some("stats"))
        );
        assert_eq!(parse_command("help mean"), page("mean", None));
        assert_eq!(parse_command("help stats::lm"), page("lm", Some("stats")));
        assert_eq!(
            parse_command("help search linear regression"),
            Some(HelpCommand::Search {
                words: "linear regression".into()
            })
        );
        assert_eq!(
            parse_command("??regression"),
            Some(HelpCommand::Search {
                words: "regression".into()
            })
        );
        assert_eq!(parse_command("?`[`"), None);
        assert_eq!(parse_command("?\"[\""), page("[", None));
    }

    #[test]
    fn package_vignette_and_news_forms_are_understood() {
        let package = |p: &str| Some(HelpCommand::Package { package: p.into() });
        let vignettes = |p: Option<&str>| {
            Some(HelpCommand::Vignettes {
                package: p.map(str::to_string),
            })
        };
        assert_eq!(parse_command("help(package = \"stats\")"), package("stats"));
        assert_eq!(parse_command("help(package = stats)"), package("stats"));
        assert_eq!(parse_command("help package stats"), package("stats"));
        assert_eq!(parse_command("help manual stats"), package("stats"));
        assert_eq!(parse_command("help vignettes"), vignettes(None));
        assert_eq!(
            parse_command("help vignettes grid"),
            vignettes(Some("grid"))
        );
        assert_eq!(parse_command("browseVignettes()"), vignettes(None));
        assert_eq!(
            parse_command("browseVignettes(\"grid\")"),
            vignettes(Some("grid"))
        );
        assert_eq!(
            parse_command("browseVignettes(package = \"grid\")"),
            vignettes(Some("grid"))
        );
        assert_eq!(
            parse_command("help news survival"),
            Some(HelpCommand::News {
                package: "survival".into()
            })
        );
        // a topic that happens to be called one of these words is still a topic
        assert_eq!(parse_command("help(\"package\")"), page("package", None));
        assert_eq!(parse_command("help package"), page("package", None));
        assert_eq!(parse_command("help news"), page("news", None));
        assert_eq!(parse_command("help package a b"), None);
        assert_eq!(parse_command("help(package = 3)"), None);
        assert_eq!(parse_command("browseVignettes(x)"), vignettes(Some("x")));
        assert_eq!(parse_command("browseVignettes(f(x))"), None);
    }

    #[test]
    fn ordinary_r_is_left_alone() {
        for line in [
            "help <- 3",
            "help = 3",
            "helper(1)",
            "help(mean) + 1",
            "help(f(x))",
            "help(1, 2, 3)",
            "help(foo = 1)",
            "?",
            "??",
            "help",
            "help a b",
            "x <- help(mean)",
            "print(help)",
            "?;",
        ] {
            assert_eq!(parse_command(line), None, "{line}");
        }
    }

    #[test]
    fn the_word_for_f1_is_the_last_name() {
        assert_eq!(word_at_end("x <- mean").as_deref(), Some("mean"));
        assert_eq!(word_at_end("stats::rnorm").as_deref(), Some("stats::rnorm"));
        assert_eq!(word_at_end("read.csv").as_deref(), Some("read.csv"));
        assert_eq!(word_at_end("lm(y ~ x, data").as_deref(), Some("data"));
        assert_eq!(word_at_end("mean(").as_deref(), None);
        assert_eq!(word_at_end("").as_deref(), None);
        assert_eq!(word_at_end("1 + 2").as_deref(), None);
    }

    #[test]
    fn the_code_quotes_what_was_typed() {
        let code = page_code("a\"b\\c", Some("pk"));
        assert!(code.contains(r#"topic <- "a\"b\\c""#), "{code}");
        assert!(code.contains(r#"package = "pk""#));
        assert!(page_code("mean", None).contains("package = NULL"));
        assert!(search_code("it's \"x\"").contains(r#""it's \"x\"""#));
    }

    #[test]
    fn a_page_is_read() {
        let answer = "@@PACKAGE\tbase\n@@TOPIC\tmean\n@@TITLE\tArithmetic Mean\n@@ALSO\tfoo\n\
                      @@LINK\tweighted.mean\n@@LINK\tcolMeans\n\
                      @@TEXT\n|Arithmetic Mean\n|\n|Description:\n@@END\n\
                      @@EXAMPLES\n|### Name: mean\n|### ** Examples\n|\n|x <- c(0:10, 50)\n|mean(x)\n|\n@@END\n";
        let Some(PageAnswer::Found(p)) = parse_page(answer) else {
            panic!("not a page");
        };
        assert_eq!(p.package, "base");
        assert_eq!(p.topic, "mean");
        assert_eq!(p.title, "Arithmetic Mean");
        assert_eq!(p.links, vec!["weighted.mean", "colMeans"]);
        assert_eq!(p.also_in, vec!["foo"]);
        assert_eq!(p.text, "Arithmetic Mean\n\nDescription:");
        assert_eq!(p.examples, "x <- c(0:10, 50)\nmean(x)");
    }

    #[test]
    fn a_missing_topic_and_a_broken_answer() {
        assert_eq!(
            parse_page("@@NONE\n@@SUGGEST\tmean\n@@SUGGEST\tmean.Date\n"),
            Some(PageAnswer::NotFound {
                suggestions: vec!["mean".into(), "mean.Date".into()]
            })
        );
        assert_eq!(parse_page(""), None);
        assert_eq!(parse_page("Error in x"), None);
        assert_eq!(parse_page("@@PACKAGE\tbase\n"), None);
        // a text line that looks like a marker stays text
        let answer = "@@PACKAGE\tp\n@@TEXT\n|@@END\n|x\n@@END\n";
        let Some(PageAnswer::Found(p)) = parse_page(answer) else {
            panic!("not a page");
        };
        assert_eq!(p.text, "@@END\nx");
        assert_eq!(p.examples, "");
    }

    #[test]
    fn search_results_are_ranked() {
        let answer = "@@HIT\tcoda\tline\tSimple linear regression\n\
                      @@HIT\tstats\tlm\tFitting Linear Models\n\
                      @@HIT\tgraphics\tabline\tAdd Straight Lines\n\
                      @@HIT\tmypkg\tlinear\tA linear thing\n\
                      @@DONE\n";
        let hits = parse_search("linear", answer).unwrap_or_default();
        let order: Vec<&str> = hits.iter().map(|h| h.topic.as_str()).collect();
        assert_eq!(order, vec!["linear", "lm", "abline", "line"]);
        assert_eq!(parse_search("x", "@@HIT\ta\tb\tc\n"), None);
        assert_eq!(parse_search("x", "@@DONE\n"), Some(Vec::new()));
    }
}
