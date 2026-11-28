//! The RCraft manual (P2A.6): a few pages of Markdown under `docs/manual/`,
//! compiled into the program, read here into blocks the Help window draws, and
//! searched by the same box as R's help.
//!
//! The reader knows only what the pages use: `#` and `##` headings, paragraphs,
//! `-` and `1.` list items (one line each), fenced code, pictures (a line of
//! its own, `![what it shows](images/name.png)`) and, inside text,
//! `[words](target)` links, `**bold**` and `` `code` ``. A link's target is a
//! page's name (`menus`) or `help:topic`, which opens R's help page for the
//! topic. The pictures are screenshots of RCraft itself, drawn by
//! `rcraft-desktop --picture` and compiled in like the pages. Nothing here
//! talks to R, so the manual works with no session and no network.

use rcraft_rcodegen::help::{page_of, HelpCommand, Hit as RHit};

/// One page of the manual.
#[derive(Debug, PartialEq, Eq)]
pub struct Page {
    /// The name links and the Help window use.
    pub slug: &'static str,
    pub title: &'static str,
    pub text: &'static str,
}

/// The contents page's name: it is where the manual opens.
pub const CONTENTS: &str = "index";

/// Every page, the contents first. The order breaks ties in a search.
pub const PAGES: &[Page] = &[
    Page {
        slug: CONTENTS,
        title: "RCraft manual",
        text: include_str!("../../../docs/manual/index.md"),
    },
    Page {
        slug: "getting-started",
        title: "Getting started",
        text: include_str!("../../../docs/manual/getting-started.md"),
    },
    Page {
        slug: "menus",
        title: "The menus",
        text: include_str!("../../../docs/manual/menus.md"),
    },
    Page {
        slug: "data-dialogs",
        title: "The Data menu",
        text: include_str!("../../../docs/manual/data-dialogs.md"),
    },
    Page {
        slug: "statistics-dialogs",
        title: "The Statistics menu",
        text: include_str!("../../../docs/manual/statistics-dialogs.md"),
    },
    Page {
        slug: "graphs-dialogs",
        title: "The Graphs menu",
        text: include_str!("../../../docs/manual/graphs-dialogs.md"),
    },
    Page {
        slug: "data-and-properties",
        title: "The Data tab and Properties",
        text: include_str!("../../../docs/manual/data-and-properties.md"),
    },
    Page {
        slug: "command-window",
        title: "The Command window",
        text: include_str!("../../../docs/manual/command-window.md"),
    },
    Page {
        slug: "packages",
        title: "Packages",
        text: include_str!("../../../docs/manual/packages.md"),
    },
    Page {
        slug: "projects-and-reporting",
        title: "Projects and reporting",
        text: include_str!("../../../docs/manual/projects-and-reporting.md"),
    },
];

/// The page with this name.
pub fn page(slug: &str) -> Option<&'static Page> {
    PAGES.iter().find(|p| p.slug == slug)
}

/// A picture the pages can show: its file under `docs/manual/images/`.
#[derive(Debug, PartialEq, Eq)]
pub struct Picture {
    pub name: &'static str,
    pub png: &'static [u8],
}

macro_rules! picture {
    ($name:literal) => {
        Picture {
            name: $name,
            png: include_bytes!(concat!("../../../docs/manual/images/", $name)),
        }
    };
}

/// Every picture, each a screenshot of RCraft drawn by `rcraft-desktop
/// --picture` (with R's `mtcars` as the data).
pub const PICTURES: &[Picture] = &[
    picture!("window.png"),
    picture!("menu-statistics.png"),
    picture!("dialog-filter-rows.png"),
    picture!("dialog-linear-model.png"),
    picture!("dialog-histogram.png"),
    picture!("graph-histogram.png"),
    picture!("data-tab.png"),
    picture!("command-window.png"),
    picture!("packages-installed.png"),
];

/// The picture a page names (`images/window.png` or `window.png`).
pub fn picture(name: &str) -> Option<&'static Picture> {
    let name = name.strip_prefix("images/").unwrap_or(name);
    PICTURES.iter().find(|p| p.name == name)
}

/// A piece of text inside a block.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Span {
    Text(String),
    Bold(String),
    Code(String),
    Link { text: String, target: String },
}

/// A block of a page.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Block {
    /// A heading: level 1 for `#`, 2 for `##`, 3 for the rest.
    Heading(u8, Vec<Span>),
    Paragraph(Vec<Span>),
    /// A list item; the number is given for an ordered list.
    Item(Option<usize>, Vec<Span>),
    /// A fenced block, shown as it is.
    Code(String),
    /// A picture: what it shows (its caption), and its file as the page names it.
    Picture {
        alt: String,
        name: String,
    },
}

/// `![alt](name)` on a line of its own.
fn picture_line(line: &str) -> Option<Block> {
    let inner = line.strip_prefix("![")?.strip_suffix(')')?;
    let (alt, name) = inner.split_once("](")?;
    (!alt.contains([']', '[']) && !name.contains(['(', ')'])).then(|| Block::Picture {
        alt: alt.trim().to_string(),
        name: name.trim().to_string(),
    })
}

/// Where a link goes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Target {
    /// Another page of the manual.
    Page(&'static str),
    /// R's help page for a topic.
    Help(HelpCommand),
}

/// What a link's target means, or `None` when it names nothing that exists.
pub fn resolve(target: &str) -> Option<Target> {
    match target.strip_prefix("help:") {
        Some(topic) => page_of(topic).map(Target::Help),
        None => page(target).map(|p| Target::Page(p.slug)),
    }
}

/// The spans of one line of text.
fn spans(line: &str) -> Vec<Span> {
    let mut out: Vec<Span> = Vec::new();
    let mut plain = String::new();
    let mut rest = line;
    while let Some(c) = rest.chars().next() {
        let found = match c {
            '[' => rest.split_once("](").and_then(|(text, after)| {
                let (target, tail) = after.split_once(')')?;
                let text = text.strip_prefix('[')?;
                (!text.contains(['[', ']'])).then(|| {
                    (
                        Span::Link {
                            text: text.to_string(),
                            target: target.trim().to_string(),
                        },
                        tail,
                    )
                })
            }),
            '`' => rest[1..]
                .split_once('`')
                .map(|(code, tail)| (Span::Code(code.to_string()), tail)),
            '*' if rest.starts_with("**") => rest[2..]
                .split_once("**")
                .map(|(bold, tail)| (Span::Bold(bold.to_string()), tail)),
            _ => None,
        };
        match found {
            Some((span, tail)) => {
                if !plain.is_empty() {
                    out.push(Span::Text(std::mem::take(&mut plain)));
                }
                out.push(span);
                rest = tail;
            }
            None => {
                plain.push(c);
                rest = &rest[c.len_utf8()..];
            }
        }
    }
    if !plain.is_empty() {
        out.push(Span::Text(plain));
    }
    out
}

/// `1. text` as its number and the text.
fn ordered(line: &str) -> Option<(usize, &str)> {
    let digits = line.chars().take_while(char::is_ascii_digit).count();
    let (number, rest) = line.split_at(digits);
    let text = rest.strip_prefix(". ")?;
    Some((number.parse().ok()?, text))
}

/// A page's text as blocks.
pub fn parse(text: &str) -> Vec<Block> {
    let mut blocks = Vec::new();
    let mut paragraph: Vec<&str> = Vec::new();
    let mut code: Option<String> = None;
    let flush = |paragraph: &mut Vec<&str>, blocks: &mut Vec<Block>| {
        if !paragraph.is_empty() {
            blocks.push(Block::Paragraph(spans(&paragraph.join(" "))));
            paragraph.clear();
        }
    };
    for line in text.lines() {
        if let Some(open) = code.as_mut() {
            if line.trim_start().starts_with("```") {
                blocks.push(Block::Code(std::mem::take(open).trim_end().to_string()));
                code = None;
            } else {
                open.push_str(line);
                open.push('\n');
            }
            continue;
        }
        let trimmed = line.trim();
        if trimmed.starts_with("```") {
            flush(&mut paragraph, &mut blocks);
            code = Some(String::new());
        } else if trimmed.is_empty() {
            flush(&mut paragraph, &mut blocks);
        } else if let Some(picture) = picture_line(trimmed) {
            flush(&mut paragraph, &mut blocks);
            blocks.push(picture);
        } else if let Some(heading) = trimmed.strip_prefix('#') {
            flush(&mut paragraph, &mut blocks);
            let more = heading.chars().take_while(|&c| c == '#').count();
            let level = u8::try_from(more + 1).unwrap_or(3).min(3);
            blocks.push(Block::Heading(
                level,
                spans(heading.trim_start_matches('#').trim()),
            ));
        } else if let Some(item) = trimmed.strip_prefix("- ") {
            flush(&mut paragraph, &mut blocks);
            blocks.push(Block::Item(None, spans(item)));
        } else if let Some((number, item)) = ordered(trimmed) {
            flush(&mut paragraph, &mut blocks);
            blocks.push(Block::Item(Some(number), spans(item)));
        } else {
            paragraph.push(trimmed);
        }
    }
    if let Some(open) = code {
        blocks.push(Block::Code(open.trim_end().to_string()));
    }
    flush(&mut paragraph, &mut blocks);
    blocks
}

/// A page the search found.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Hit {
    pub slug: &'static str,
    pub title: &'static str,
    /// How closely it fits, on the scale R's hits are ranked by (see [`rank_of_r`]):
    /// 0 its title is the words, 1 starts with them, 2 holds them (or holds every
    /// word), 3 only its text mentions them.
    pub rank: u8,
}

/// How a name fits the words, the way R's help hits are ranked
/// (`rcodegen::help::parse_search`): 0 the same, 1 starts with them, 2 holds
/// them; `None` when it does not hold them.
fn closeness(name: &str, words: &str) -> Option<u8> {
    let name = name.to_lowercase();
    if name == words {
        Some(0)
    } else if name.starts_with(words) {
        Some(1)
    } else if name.contains(words) {
        Some(2)
    } else {
        None
    }
}

/// Where one of R's hits stands on the shared scale: its topic's closeness
/// to the words, else 3 for the packages every session has, else 4.
pub fn rank_of_r(hit: &RHit, words: &str) -> u8 {
    let q = words.trim().to_lowercase();
    let core = ["base", "stats", "utils", "graphics", "methods", "datasets"];
    closeness(&hit.topic, &q).unwrap_or(if core.contains(&hit.package.as_str()) {
        3
    } else {
        4
    })
}

/// One line of a search's answer: a page of the manual or one of R's help pages.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Found {
    Manual(Hit),
    R(RHit),
}

/// The manual's hits and R's in one list, best first. Each stands where its
/// rank puts it; on a tie the manual's page comes first, and each side keeps
/// its own order. R's hits alone, or the manual's alone, come out as they were.
pub fn rank_together(words: &str, manual: Vec<Hit>, r: Vec<RHit>) -> Vec<Found> {
    let mut all: Vec<(u8, Found)> = manual
        .into_iter()
        .map(|h| (h.rank, Found::Manual(h)))
        .chain(r.into_iter().map(|h| (rank_of_r(&h, words), Found::R(h))))
        .collect();
    // stable, so the manual (listed first) wins a tie and each side keeps its order
    all.sort_by_key(|(rank, _)| *rank);
    all.into_iter().map(|(_, f)| f).collect()
}

/// The words of a search, lower case, without punctuation.
fn words_of(query: &str) -> Vec<String> {
    query
        .split_whitespace()
        .map(|w| {
            w.chars()
                .filter(|c| c.is_alphanumeric())
                .flat_map(char::to_lowercase)
                .collect::<String>()
        })
        .filter(|w| !w.is_empty())
        .collect()
}

/// A page's rank for the words (see [`Hit::rank`]): its title, or its name
/// with spaces for dashes, against the words as typed; then 2 when its title
/// holds every word, else 3.
fn rank_of_page(p: &Page, phrase: &str, words: &[String]) -> u8 {
    let title = p.title.to_lowercase();
    let near = closeness(&title, phrase)
        .into_iter()
        .chain(closeness(&p.slug.replace('-', " "), phrase))
        // "The menus" is the words "menus" as closely as "menus" is
        .chain(
            title
                .strip_prefix("the ")
                .and_then(|t| closeness(t, phrase)),
        )
        .min();
    near.unwrap_or(if words.iter().all(|w| title.contains(w.as_str())) {
        2
    } else {
        3
    })
}

/// The pages that mention the words, best first: by rank, then a word in a
/// page's title counts far more than the same word in a heading, which counts
/// more than in its text, and a page in the list's order wins a tie. A page
/// needs only one of the words.
pub fn search(query: &str) -> Vec<Hit> {
    let words = words_of(query);
    if words.is_empty() {
        return Vec::new();
    }
    let phrase = words.join(" ");
    let mut scored: Vec<(u8, usize, usize, &'static Page)> = PAGES
        .iter()
        .enumerate()
        .filter_map(|(order, p)| {
            let title = p.title.to_lowercase();
            let body = p.text.to_lowercase();
            let headings: Vec<&str> = body
                .lines()
                .filter(|l| l.trim_start().starts_with('#'))
                .collect();
            let score: usize = words
                .iter()
                .map(|w| {
                    let in_title = if title.contains(w.as_str()) { 20 } else { 0 };
                    let in_heading = 5 * headings.iter().filter(|h| h.contains(w.as_str())).count();
                    in_title + in_heading.min(10) + body.matches(w.as_str()).count().min(10)
                })
                .sum();
            (score > 0).then(|| (rank_of_page(p, &phrase, &words), score, order, p))
        })
        .collect();
    scored.sort_by(|a, b| a.0.cmp(&b.0).then(b.1.cmp(&a.1)).then(a.2.cmp(&b.2)));
    scored
        .into_iter()
        .map(|(rank, _, _, p)| Hit {
            slug: p.slug,
            title: p.title,
            rank,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spans_read_links_bold_and_code_and_leave_the_rest() {
        assert_eq!(
            spans("See [the menus](menus), **bold** and `x <- 1`."),
            vec![
                Span::Text("See ".into()),
                Span::Link {
                    text: "the menus".into(),
                    target: "menus".into()
                },
                Span::Text(", ".into()),
                Span::Bold("bold".into()),
                Span::Text(" and ".into()),
                Span::Code("x <- 1".into()),
                Span::Text(".".into()),
            ]
        );
        // an unfinished mark is plain text, not a crash
        for odd in ["a [b](c", "a [b] c", "a `b", "a **b", "[", "*", "é[é](é)é"] {
            let text: String = spans(odd)
                .iter()
                .map(|s| match s {
                    Span::Text(t) | Span::Bold(t) | Span::Code(t) => t.clone(),
                    Span::Link { text, .. } => text.clone(),
                })
                .collect();
            assert!(!text.is_empty(), "{odd}");
        }
    }

    #[test]
    fn blocks_are_headings_paragraphs_items_and_code() {
        let blocks = parse(
            "# Title\n\nOne line\ncontinues.\n\n- first\n2. second\n\n```\nx <- 1\n```\n## Next\n",
        );
        assert_eq!(
            blocks,
            vec![
                Block::Heading(1, vec![Span::Text("Title".into())]),
                Block::Paragraph(vec![Span::Text("One line continues.".into())]),
                Block::Item(None, vec![Span::Text("first".into())]),
                Block::Item(Some(2), vec![Span::Text("second".into())]),
                Block::Code("x <- 1".into()),
                Block::Heading(2, vec![Span::Text("Next".into())]),
            ]
        );
        // a fence left open still ends the page
        assert_eq!(parse("```\nx"), vec![Block::Code("x".into())]);
    }

    #[test]
    fn a_picture_is_a_line_of_its_own() {
        assert_eq!(
            parse("Text.\n![The window](images/window.png)\nMore."),
            vec![
                Block::Paragraph(vec![Span::Text("Text.".into())]),
                Block::Picture {
                    alt: "The window".into(),
                    name: "images/window.png".into()
                },
                Block::Paragraph(vec![Span::Text("More.".into())]),
            ]
        );
        assert!(picture("images/window.png").is_some());
        assert!(picture("window.png").is_some());
        assert!(picture("images/nothing.png").is_none());
        // half a picture is text, not a crash
        for odd in ["![", "![a](b", "![a]b)", "![a](b) and more", "![[a]](b)"] {
            assert!(
                !parse(odd)
                    .iter()
                    .any(|b| matches!(b, Block::Picture { .. })),
                "{odd}"
            );
        }
    }

    fn r_hit(package: &str, topic: &str) -> RHit {
        RHit {
            package: package.into(),
            topic: topic.into(),
            title: String::new(),
        }
    }

    #[test]
    fn manual_and_r_hits_share_one_ranking() {
        // R's ranks: the same name, starts with, holds, a core package, the rest
        assert_eq!(rank_of_r(&r_hit("stats", "lm"), "lm"), 0);
        assert_eq!(rank_of_r(&r_hit("stats", "lm.fit"), "LM "), 1);
        assert_eq!(rank_of_r(&r_hit("stats", "glm"), "lm"), 2);
        assert_eq!(rank_of_r(&r_hit("stats", "anova"), "lm"), 3);
        assert_eq!(rank_of_r(&r_hit("lme4", "lmer"), "x"), 4);
        // the manual's: a page whose title is the words, or starts with them, ranks high
        let rank = |q: &str, slug: &str| search(q).iter().find(|h| h.slug == slug).map(|h| h.rank);
        assert_eq!(rank("menus", "menus"), Some(0));
        assert_eq!(rank("packages", "packages"), Some(0));
        assert_eq!(rank("projects", "projects-and-reporting"), Some(1));
        assert_eq!(rank("graphs menu", "graphs-dialogs"), Some(0));
        assert_eq!(rank("graphs", "graphs-dialogs"), Some(1));
        assert_eq!(rank("menu graphs", "graphs-dialogs"), Some(2));
        assert_eq!(rank("histogram", "graphs-dialogs"), Some(3));
        // within a rank the manual comes first, and each side keeps its own order
        let manual = vec![
            Hit {
                slug: "a",
                title: "A",
                rank: 3,
            },
            Hit {
                slug: "b",
                title: "B",
                rank: 1,
            },
        ];
        let r = vec![
            r_hit("stats", "lm"),
            r_hit("stats", "glm"),
            r_hit("base", "other"),
            r_hit("pkg", "thing"),
        ];
        let together = rank_together("lm", manual, r);
        let names: Vec<&str> = together
            .iter()
            .map(|f| match f {
                Found::Manual(h) => h.slug,
                Found::R(h) => h.topic.as_str(),
            })
            .collect();
        assert_eq!(names, ["lm", "b", "glm", "a", "other", "thing"]);
        // either side alone comes out as it was
        assert!(rank_together("lm", Vec::new(), Vec::new()).is_empty());
        let alone = rank_together("project", search("project"), Vec::new());
        assert_eq!(alone.len(), search("project").len());
    }

    #[test]
    fn a_search_ranks_the_title_over_the_text() {
        let hits = search("project");
        assert_eq!(hits.first().map(|h| h.slug), Some("projects-and-reporting"));
        assert!(search("").is_empty());
        assert!(search("zzzzqqq").is_empty());
        assert_eq!(
            search("history").first().map(|h| h.slug),
            Some("command-window")
        );
    }
}
