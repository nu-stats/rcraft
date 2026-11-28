//! The RCraft manual (P2A.6): a few pages of Markdown under `docs/manual/`,
//! compiled into the program, read here into blocks the Help window draws, and
//! searched by the same box as R's help.
//!
//! The reader knows only what the pages use: `#` and `##` headings, paragraphs,
//! `-` and `1.` list items (one line each), fenced code and, inside text,
//! `[words](target)` links, `**bold**` and `` `code` ``. A link's target is a
//! page's name (`menus`) or `help:topic`, which opens R's help page for the
//! topic. Nothing here talks to R, so the manual works with no session and no
//! network.

use rcraft_rcodegen::help::{page_of, HelpCommand};

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

/// The pages that mention the words, best first: a word in a page's title
/// counts far more than the same word in its text, and a page in the list's
/// order wins a tie. A page needs only one of the words.
pub fn search(query: &str) -> Vec<Hit> {
    let words = words_of(query);
    if words.is_empty() {
        return Vec::new();
    }
    let mut scored: Vec<(usize, usize, &'static Page)> = PAGES
        .iter()
        .enumerate()
        .filter_map(|(order, p)| {
            let title = p.title.to_lowercase();
            let body = p.text.to_lowercase();
            let score: usize = words
                .iter()
                .map(|w| {
                    let in_title = if title.contains(w.as_str()) { 20 } else { 0 };
                    in_title + body.matches(w.as_str()).count().min(10)
                })
                .sum();
            (score > 0).then_some((score, order, p))
        })
        .collect();
    scored.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
    scored
        .into_iter()
        .map(|(_, _, p)| Hit {
            slug: p.slug,
            title: p.title,
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
