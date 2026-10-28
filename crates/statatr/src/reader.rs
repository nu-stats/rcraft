//! P2B.1: a Stata do-file read into a tree of commands and blocks, ready for
//! translation. Built on StatCraft's front end (`lang`): its do-file reader
//! joins `///` continuations and drops comments, its syntax splits a command
//! line into its parts, and its table names abbreviated commands.
//!
//! What this adds: prefixes (`by ...:`, `bysort`, `capture`, `quietly`,
//! `noisily`), `using`, blocks with braces (`foreach`, `forvalues`, `while`,
//! `if`/`else`, and `capture {`/`quietly {`), the macros a line refers to, and a
//! line and column for every command and every problem.
//!
//! Nothing is run and nothing is expanded: a macro's value is only known when
//! the do-file runs, so references are recorded and left in the text. The
//! reader never fails as a whole and never panics: a line it cannot read
//! becomes a [`Problem`] with its place, and reading goes on.

use crate::lang::commands::resolve;
use crate::lang::script::{self, Line};
use crate::lang::syntax::{self, Opt, Parts};

/// Deepest nesting of blocks accepted.
pub const MAX_DEPTH: usize = 64;
/// Most lines kept for one mata, python or input block.
pub const MAX_RAW_LINES: usize = 1_000_000;

/// Where something is in the do-file: 1-based line and column of its first
/// character, and how many physical lines it spans (`///` joins lines).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Span {
    pub line: usize,
    pub column: usize,
    pub lines: usize,
}

/// A prefix before a command or a block.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Prefix {
    /// `by varlist:` or `bysort varlist:` (`sort` is true for `bysort` and for
    /// `by ..., sort:`), with any other options as written.
    By {
        varlist: String,
        sort: bool,
        options: String,
    },
    /// `frame name:` runs the command in that data frame.
    Frame(String),
    Capture,
    Quietly,
    Noisily,
}

/// A macro a line refers to: `` `name' `` or `$name` / `${name}`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MacroRef {
    Local(String),
    Global(String),
}

/// One command.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Command {
    pub span: Span,
    pub prefixes: Vec<Prefix>,
    /// The command word as written (`reg`).
    pub written: String,
    /// Its full name (`regress`); a word Stata's table does not know is kept.
    pub name: String,
    /// Whether the name is one the table knows (abbreviations need it).
    pub known: bool,
    /// What follows the name: the main part, `if`, `in`, weight and options text.
    pub parts: Parts,
    /// The file after `using`, as written (quotes kept).
    pub using: Option<String>,
    pub options: Vec<Opt>,
    pub macros: Vec<MacroRef>,
    /// The whole line as read (comments removed, continuations joined).
    pub text: String,
}

/// The list a `foreach` loops over.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ForeachList {
    /// `foreach x in a b c`: the words as written.
    In(Vec<String>),
    /// `foreach x of varlist|newlist|numlist|local|global ...`.
    Of { kind: String, list: String },
}

/// What opened a block.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Opener {
    Foreach {
        name: String,
        list: ForeachList,
    },
    Forvalues {
        name: String,
        range: String,
    },
    While {
        condition: String,
    },
    If {
        condition: String,
    },
    /// `capture {`, `quietly {` and the like: the prefixes apply to the body.
    Prefixed,
    /// `frame name {`: the body runs in that data frame.
    Frame {
        name: String,
    },
    /// `program [define] name` ... `end`: a Stata program, its body read as commands.
    Program {
        name: String,
    },
    /// `mata` (or `mata:`), `python` and `input varlist` ... `end`: lines that
    /// are not Stata commands (Mata code, Python code, data), kept as written.
    Raw {
        kind: String,
        header: String,
        lines: Vec<String>,
    },
}

/// A block in braces.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Group {
    pub span: Span,
    pub prefixes: Vec<Prefix>,
    pub opener: Opener,
    pub body: Vec<Item>,
    /// The `else` part of an `if` (an `else if` is an `If` group in it).
    pub otherwise: Option<Vec<Item>>,
    pub macros: Vec<MacroRef>,
    /// Made by `else if`: it is the else part of the if before it.
    pub else_if: bool,
}

/// A line that could not be read, and why.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Problem {
    pub span: Span,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Item {
    Command(Command),
    Group(Group),
    Problem(Problem),
}

/// A do-file, read.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Program {
    pub items: Vec<Item>,
}

fn walk<'a>(items: &'a [Item], out: &mut Vec<&'a Item>) {
    for item in items {
        out.push(item);
        if let Item::Group(g) = item {
            walk(&g.body, out);
            if let Some(o) = &g.otherwise {
                walk(o, out);
            }
        }
    }
}

impl Program {
    /// Every item, blocks first then what is inside them, in file order.
    pub fn all(&self) -> Vec<&Item> {
        let mut out = Vec::new();
        walk(&self.items, &mut out);
        out
    }

    /// Every command, inside blocks too, in file order.
    pub fn commands(&self) -> Vec<&Command> {
        self.all()
            .into_iter()
            .filter_map(|i| match i {
                Item::Command(c) => Some(c),
                _ => None,
            })
            .collect()
    }

    /// Every problem, in file order.
    pub fn problems(&self) -> Vec<&Problem> {
        self.all()
            .into_iter()
            .filter_map(|i| match i {
                Item::Problem(p) => Some(p),
                _ => None,
            })
            .collect()
    }
}

/// The full name of a word in command position, with the words the shared
/// table does not hold (prefixes and loops; abbreviations from Stata's
/// manuals: `bys`, `cap`, `qui`, `noi`, `forv`).
fn command_word(word: &str) -> String {
    const MORE: &[(&str, usize)] = &[
        ("bysort", 3),
        ("by", 2),
        ("capture", 3),
        ("quietly", 3),
        ("noisily", 3),
        ("foreach", 7),
        ("forvalues", 4),
        ("while", 5),
        ("if", 2),
        ("else", 4),
    ];
    if let Some((name, _)) = MORE
        .iter()
        .find(|(name, min)| word.len() >= *min && name.starts_with(word))
    {
        return (*name).to_string();
    }
    resolve(word).to_string()
}

fn is_known(word: &str) -> bool {
    let full = command_word(word);
    full != word
        || crate::lang::commands::COMMANDS
            .iter()
            .any(|(n, _)| *n == word)
        || matches!(word, "by" | "if" | "else" | "while" | "foreach")
}

/// The first word and what follows it. A word ends at a space, or before `,`,
/// `=`, `(` or `:` (as in `gen y=1`, `by g:`).
fn first_word(text: &str) -> (&str, &str) {
    let t = text.trim_start();
    let end = t
        .find(|c: char| c.is_whitespace() || matches!(c, ',' | '=' | '(' | ':' | '{'))
        .unwrap_or(t.len());
    (t.get(..end).unwrap_or(""), t.get(end..).unwrap_or(""))
}

/// The byte index of `target` at the top level of `text`: outside double
/// quotes, compound quotes, parentheses and brackets.
fn top_level(text: &str, target: char) -> Option<usize> {
    let mut depth = 0i32;
    let mut in_quote = false;
    let mut compound = 0i32;
    let mut prev = '\0';
    for (i, c) in text.char_indices() {
        if compound > 0 {
            if prev == '"' && c == '\'' {
                compound -= 1;
            } else if prev == '`' && c == '"' {
                compound += 1;
            }
        } else if in_quote {
            if c == '"' {
                in_quote = false;
            }
        } else if prev == '`' && c == '"' {
            compound = 1;
        } else if c == '"' {
            in_quote = true;
        } else if matches!(c, '(' | '[') {
            depth += 1;
        } else if matches!(c, ')' | ']') {
            depth -= 1;
        } else if c == target && depth == 0 {
            return Some(i);
        }
        prev = c;
    }
    None
}

/// `using` at the top level of a command's main part: what comes before it,
/// and the file after it.
fn split_using(main: &str) -> (String, Option<String>) {
    let mut search = 0;
    while let Some(rel) = main.get(search..).and_then(|s| s.find("using")) {
        let at = search + rel;
        let before_ok = at == 0
            || main
                .get(..at)
                .and_then(|b| b.chars().last())
                .is_some_and(char::is_whitespace);
        let after = main.get(at + 5..).unwrap_or("");
        let after_ok = after.starts_with(char::is_whitespace);
        let outside = main
            .get(..at)
            .is_some_and(|b| b.chars().filter(|c| *c == '"').count() % 2 == 0);
        if before_ok && after_ok && outside {
            let file = after.trim();
            return (
                main.get(..at).unwrap_or("").trim().to_string(),
                (!file.is_empty()).then(|| file.to_string()),
            );
        }
        search = at + 5;
    }
    (main.trim().to_string(), None)
}

fn is_name(s: &str) -> bool {
    let mut c = s.chars();
    matches!(c.next(), Some(f) if f.is_ascii_alphabetic() || f == '_')
        && s.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
}

/// The macros `text` refers to, each once, in order.
pub fn macro_refs(text: &str) -> Vec<MacroRef> {
    let chars: Vec<char> = text.chars().collect();
    let mut out: Vec<MacroRef> = Vec::new();
    let mut push = |m: MacroRef| {
        if !out.contains(&m) {
            out.push(m);
        }
    };
    for (i, c) in chars.iter().enumerate() {
        match c {
            '`' => {
                // `name' with a simple name inside (nested ones are found on
                // their own, from their own backtick)
                let rest: String = chars.iter().skip(i + 1).take(40).collect();
                if let Some(end) = rest.find('\'') {
                    let inside = rest.get(..end).unwrap_or("");
                    let local = inside.strip_prefix('=').map_or(inside, |_| "");
                    if !local.is_empty()
                        && (is_name(local) || local.chars().all(|c| c.is_ascii_digit()))
                    {
                        push(MacroRef::Local(local.to_string()));
                    }
                }
            }
            '$' => {
                let rest: String = chars.iter().skip(i + 1).take(40).collect();
                let name: String = if let Some(braced) = rest.strip_prefix('{') {
                    braced.split('}').next().unwrap_or("").to_string()
                } else {
                    rest.chars()
                        .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
                        .collect()
                };
                if is_name(&name) {
                    push(MacroRef::Global(name));
                }
            }
            _ => {}
        }
    }
    out
}

/// The words after `frame` that are subcommands, not frame names.
fn is_frame_subcommand(word: &str) -> bool {
    matches!(
        word,
        "create"
            | "change"
            | "drop"
            | "copy"
            | "put"
            | "rename"
            | "reset"
            | "dir"
            | "list"
            | "pwf"
            | "describe"
            | "post"
            | "prefix"
    )
}

/// The prefixes at the start of `text`, and what follows them.
fn take_prefixes(text: &str) -> Result<(Vec<Prefix>, String), String> {
    let mut prefixes = Vec::new();
    let mut rest = text.trim().to_string();
    for _ in 0..16 {
        let (word, after) = first_word(&rest);
        let full = command_word(word);
        match full.as_str() {
            "by" | "bysort" => {
                let Some(colon) = top_level(after, ':') else {
                    return Err(format!("{word} needs a colon: {word} varlist: command"));
                };
                let spec = after.get(..colon).unwrap_or("").trim();
                let (varlist, options) = match spec.split_once(',') {
                    Some((v, o)) => (v.trim(), o.trim()),
                    None => (spec, ""),
                };
                if varlist.is_empty() {
                    return Err(format!("{word} needs variables before the colon"));
                }
                let sort =
                    full == "bysort" || syntax::options(options).iter().any(|o| o.is("sort", 4));
                prefixes.push(Prefix::By {
                    varlist: varlist.to_string(),
                    sort,
                    options: options.to_string(),
                });
                rest = after.get(colon + 1..).unwrap_or("").trim().to_string();
            }
            "frame" => {
                let Some(colon) = top_level(after, ':') else {
                    break;
                };
                let name = after.get(..colon).unwrap_or("").trim();
                if !is_name(name) || is_frame_subcommand(name) {
                    break;
                }
                prefixes.push(Prefix::Frame(name.to_string()));
                rest = after.get(colon + 1..).unwrap_or("").trim().to_string();
            }
            "capture" | "quietly" | "noisily" => {
                prefixes.push(match full.as_str() {
                    "capture" => Prefix::Capture,
                    "quietly" => Prefix::Quietly,
                    _ => Prefix::Noisily,
                });
                let after = after.trim_start();
                rest = after.strip_prefix(':').unwrap_or(after).trim().to_string();
            }
            _ => break,
        }
    }
    Ok((prefixes, rest))
}

/// An opener line's text without its final `{`, or `None` when it has none.
fn before_brace(text: &str) -> Option<&str> {
    text.trim_end().strip_suffix('{').map(str::trim_end)
}

fn read_opener(full: &str, after: &str) -> Result<Option<Opener>, String> {
    match full {
        "foreach" => {
            let Some(spec) = before_brace(after) else {
                return Err("foreach needs { at the end of its line".into());
            };
            let mut words = spec.split_whitespace();
            let name = words.next().unwrap_or("");
            if !is_name(name) {
                return Err("foreach needs a name to loop with: foreach x in a b c {".into());
            }
            match words.next() {
                Some("in") => Ok(Some(Opener::Foreach {
                    name: name.to_string(),
                    list: ForeachList::In(words.map(str::to_string).collect()),
                })),
                Some("of") => {
                    let kind = words.next().unwrap_or("");
                    let kinds = ["varlist", "newlist", "numlist", "local", "global"];
                    if !kinds.contains(&kind) {
                        return Err(format!(
                            "foreach ... of needs one of {}, not \"{kind}\"",
                            kinds.join(", ")
                        ));
                    }
                    Ok(Some(Opener::Foreach {
                        name: name.to_string(),
                        list: ForeachList::Of {
                            kind: kind.to_string(),
                            list: words.collect::<Vec<_>>().join(" "),
                        },
                    }))
                }
                _ => Err("foreach needs \"in\" or \"of\" after its name".into()),
            }
        }
        "forvalues" => {
            let Some(spec) = before_brace(after) else {
                return Err("forvalues needs { at the end of its line".into());
            };
            let Some((name, range)) = spec.split_once('=') else {
                return Err("forvalues needs name = range: forvalues i = 1/10 {".into());
            };
            let name = name.trim();
            if !is_name(name) || range.trim().is_empty() {
                return Err("forvalues needs name = range: forvalues i = 1/10 {".into());
            }
            Ok(Some(Opener::Forvalues {
                name: name.to_string(),
                range: range.trim().to_string(),
            }))
        }
        "while" => {
            let Some(cond) = before_brace(after) else {
                return Err("while needs { at the end of its line".into());
            };
            if cond.trim().is_empty() {
                return Err("while needs a condition".into());
            }
            Ok(Some(Opener::While {
                condition: cond.trim().to_string(),
            }))
        }
        // the one-line programming `if exp command` stays a command
        "if" => Ok(before_brace(after).map(|cond| Opener::If {
            condition: cond.trim().to_string(),
        })),
        // `frame name {` (not `frame create`, `frame change` and the other subcommands)
        "frame" => Ok(before_brace(after)
            .map(str::trim)
            .filter(|n| is_name(n) && !is_frame_subcommand(n))
            .map(|n| Opener::Frame {
                name: n.to_string(),
            })),
        // `program define name` opens a program; `program drop`, `dir` and `list` do not
        "program" => {
            let mut words = after.split(|c: char| c.is_whitespace() || c == ',');
            let mut sub = words.find(|w| !w.is_empty()).unwrap_or("");
            if matches!(sub, "drop" | "dir" | "list") || sub.is_empty() {
                return Ok(None);
            }
            if "define".starts_with(sub) && sub.len() >= 3 {
                sub = words.find(|w| !w.is_empty()).unwrap_or("");
            }
            if sub.is_empty() {
                return Err("program define needs a name".into());
            }
            Ok(Some(Opener::Program {
                name: sub.to_string(),
            }))
        }
        _ => Ok(None),
    }
}

/// Whether an `else` can still hang from this if block: its own else part is
/// free, or it ends in an `else if` whose else part is free.
fn open_for_else(g: &Group) -> bool {
    if !matches!(g.opener, Opener::If { .. }) {
        return false;
    }
    match g.otherwise.as_deref() {
        None => true,
        Some([Item::Group(inner)]) if inner.else_if => open_for_else(inner),
        _ => false,
    }
}

/// A block being read.
struct Frame {
    group: Group,
    /// Reading its `else` part.
    in_else: bool,
    /// An `else if` block: closing it closes the `if` it hangs from too.
    chained: bool,
}

fn command(span: Span, prefixes: Vec<Prefix>, text: &str, line_text: &str) -> Item {
    let (word, after) = first_word(text);
    if word.is_empty() {
        return Item::Problem(Problem {
            span,
            message: "a prefix with no command after it".into(),
        });
    }
    let parts = syntax::parts(after);
    let (main, using) = split_using(&parts.main);
    let options = syntax::options(&parts.options);
    Item::Command(Command {
        span,
        prefixes,
        written: word.to_string(),
        name: command_word(word),
        known: is_known(word),
        parts: Parts { main, ..parts },
        using,
        options,
        macros: macro_refs(line_text),
        text: line_text.trim().to_string(),
    })
}

/// Reads a do-file.
pub fn read(source: &str) -> Program {
    let lines = script::read(source);
    let mut program = Program::default();
    let mut stack: Vec<Frame> = Vec::new();
    let mut physical = 1usize;
    // a mata, python or input block being read (its lines are not Stata commands)
    let mut raw: Option<Group> = None;

    fn target<'a>(program: &'a mut Program, stack: &'a mut [Frame]) -> &'a mut Vec<Item> {
        match stack.last_mut() {
            Some(f) if f.in_else => f.group.otherwise.get_or_insert_with(Vec::new),
            Some(f) => &mut f.group.body,
            None => &mut program.items,
        }
    }

    // closes the innermost block, and the `if` an `else if` hangs from
    fn close(program: &mut Program, stack: &mut Vec<Frame>) {
        while let Some(frame) = stack.pop() {
            let chained = frame.chained;
            target(program, stack).push(Item::Group(frame.group));
            if !chained {
                break;
            }
        }
    }

    for Line {
        physical: lines_of,
        text,
    } in lines
    {
        let first = lines_of.first().map(String::as_str).unwrap_or("");
        let span = Span {
            line: physical,
            column: first.chars().take_while(|c| c.is_whitespace()).count() + 1,
            lines: lines_of.len().max(1),
        };
        physical += lines_of.len().max(1);
        let t = text.trim();
        // inside mata, python or input: lines are kept as written until `end`
        if let Some(r) = raw.as_mut() {
            if t == "end" {
                if let Some(r) = raw.take() {
                    target(&mut program, &mut stack).push(Item::Group(r));
                }
            } else if !t.is_empty() {
                if let Group {
                    opener: Opener::Raw { lines, .. },
                    ..
                } = r
                {
                    if lines.len() < MAX_RAW_LINES {
                        lines.push(text.trim_end().to_string());
                    }
                }
            }
            continue;
        }
        if t.is_empty() || t.starts_with('*') {
            continue;
        }
        // `end` closes a program
        if t == "end" {
            if matches!(stack.last(), Some(f) if matches!(f.group.opener, Opener::Program { .. })) {
                close(&mut program, &mut stack);
            } else {
                target(&mut program, &mut stack).push(Item::Problem(Problem {
                    span,
                    message: "end with no program, mata, python or input block to end".into(),
                }));
            }
            continue;
        }

        // a closing brace, maybe with else after it
        if let Some(after) = t.strip_prefix('}') {
            let after = after.trim();
            if stack.is_empty() {
                program.items.push(Item::Problem(Problem {
                    span,
                    message: "a } with no { to close".into(),
                }));
                continue;
            }
            let else_part = after.strip_prefix("else").filter(|r| {
                r.is_empty() || r.starts_with(char::is_whitespace) || r.starts_with('{')
            });
            if let Some(rest) = else_part {
                let rest = rest.trim();
                let can_else = stack
                    .last()
                    .is_some_and(|f| matches!(f.group.opener, Opener::If { .. }) && !f.in_else);
                if !can_else {
                    close(&mut program, &mut stack);
                    target(&mut program, &mut stack).push(Item::Problem(Problem {
                        span,
                        message: "else with no if block before it".into(),
                    }));
                    continue;
                }
                if rest == "{" {
                    if let Some(f) = stack.last_mut() {
                        f.in_else = true;
                        f.group.otherwise = Some(Vec::new());
                    }
                    continue;
                }
                let (w, cond_rest) = first_word(rest);
                if command_word(w) == "if" {
                    if let Some(cond) = before_brace(cond_rest) {
                        if let Some(f) = stack.last_mut() {
                            f.in_else = true;
                            f.group.otherwise = Some(Vec::new());
                        }
                        stack.push(Frame {
                            group: Group {
                                span,
                                prefixes: Vec::new(),
                                opener: Opener::If {
                                    condition: cond.trim().to_string(),
                                },
                                body: Vec::new(),
                                otherwise: None,
                                macros: macro_refs(t),
                                else_if: true,
                            },
                            in_else: false,
                            chained: true,
                        });
                        continue;
                    }
                }
                target(&mut program, &mut stack).push(Item::Problem(Problem {
                    span,
                    message: "else needs { or if ... { after it".into(),
                }));
                continue;
            }
            close(&mut program, &mut stack);
            if !after.is_empty() {
                target(&mut program, &mut stack).push(Item::Problem(Problem {
                    span,
                    message: format!("text after a closing brace: \"{after}\""),
                }));
            }
            continue;
        }

        let (prefixes, rest) = match take_prefixes(t) {
            Ok(x) => x,
            Err(message) => {
                target(&mut program, &mut stack).push(Item::Problem(Problem { span, message }));
                continue;
            }
        };
        let (word, after) = first_word(&rest);
        let full = command_word(word);

        // `else` on its own line, after the } of an if block (as Stata's manual
        // writes it): the if block is opened again for its else part
        if full == "else" && prefixes.is_empty() {
            let after = after.trim();
            let last_is_if = matches!(
                target(&mut program, &mut stack).last(),
                Some(Item::Group(g)) if open_for_else(g)
            );
            if !last_is_if {
                // after a one-line `if exp command`, a one-line else is kept for the translator
                let after_one_line_if = matches!(
                    target(&mut program, &mut stack).last(),
                    Some(Item::Command(c)) if c.name == "if"
                );
                let item = if after_one_line_if && before_brace(after).is_none() {
                    command(span, Vec::new(), &rest, t)
                } else {
                    Item::Problem(Problem {
                        span,
                        message: "else with no if block before it".into(),
                    })
                };
                target(&mut program, &mut stack).push(item);
                continue;
            }
            let Some(Item::Group(top)) = target(&mut program, &mut stack).pop() else {
                continue;
            };
            // open the if again, and the else-if chain hanging from it, down to
            // the if whose else part is still free
            let mut chain = vec![top];
            while let Some(last) = chain.last_mut() {
                let inner = match last.otherwise.as_mut() {
                    Some(items) => match items.as_slice() {
                        [Item::Group(g)] if g.else_if => items.pop(),
                        _ => None,
                    },
                    None => None,
                };
                match inner {
                    Some(Item::Group(g)) => chain.push(g),
                    _ => break,
                }
            }
            let deepest = chain.len().saturating_sub(1);
            for (i, group) in chain.into_iter().enumerate() {
                stack.push(Frame {
                    group,
                    in_else: i < deepest,
                    chained: i > 0,
                });
            }
            let (w2, cond_rest) = first_word(after);
            if after == "{" {
                if let Some(f) = stack.last_mut() {
                    f.in_else = true;
                    f.group.otherwise = Some(Vec::new());
                }
            } else if let (true, Some(cond)) = (command_word(w2) == "if", before_brace(cond_rest)) {
                if let Some(f) = stack.last_mut() {
                    f.in_else = true;
                    f.group.otherwise = Some(Vec::new());
                }
                stack.push(Frame {
                    group: Group {
                        span,
                        prefixes: Vec::new(),
                        opener: Opener::If {
                            condition: cond.trim().to_string(),
                        },
                        body: Vec::new(),
                        otherwise: None,
                        macros: macro_refs(t),
                        else_if: true,
                    },
                    in_else: false,
                    chained: true,
                });
            } else if !after.is_empty() && before_brace(after).is_none() {
                // `else command` on one line
                if let Some(f) = stack.last_mut() {
                    f.group.otherwise = Some(vec![command(span, Vec::new(), after, t)]);
                }
                close(&mut program, &mut stack);
            } else {
                close(&mut program, &mut stack);
                target(&mut program, &mut stack).push(Item::Problem(Problem {
                    span,
                    message: "else needs { or if ... { or a command after it".into(),
                }));
            }
            continue;
        }

        // mata (or mata:), python (or python:) and input start blocks of other lines
        let starts_raw = match full.as_str() {
            "mata" | "python" => matches!(after.trim(), "" | ":"),
            "input" => true,
            _ => false,
        };
        if starts_raw {
            raw = Some(Group {
                span,
                prefixes,
                opener: Opener::Raw {
                    kind: full.clone(),
                    header: t.to_string(),
                    lines: Vec::new(),
                },
                body: Vec::new(),
                otherwise: None,
                macros: macro_refs(t),
                else_if: false,
            });
            continue;
        }

        let opener = if rest == "{" && !prefixes.is_empty() {
            Ok(Some(Opener::Prefixed))
        } else {
            read_opener(&full, after)
        };
        match opener {
            Ok(Some(opener)) => {
                if stack.len() >= MAX_DEPTH {
                    target(&mut program, &mut stack).push(Item::Problem(Problem {
                        span,
                        message: format!("blocks nested deeper than {MAX_DEPTH}"),
                    }));
                    continue;
                }
                stack.push(Frame {
                    group: Group {
                        span,
                        prefixes,
                        opener,
                        body: Vec::new(),
                        otherwise: None,
                        macros: macro_refs(t),
                        else_if: false,
                    },
                    in_else: false,
                    chained: false,
                });
            }
            Ok(None) => {
                let item = command(span, prefixes, &rest, t);
                target(&mut program, &mut stack).push(item);
            }
            Err(message) => {
                target(&mut program, &mut stack).push(Item::Problem(Problem { span, message }));
            }
        }
    }

    // a mata, python or input block with no end
    if let Some(r) = raw.take() {
        let opened = r.span;
        target(&mut program, &mut stack).push(Item::Group(r));
        target(&mut program, &mut stack).push(Item::Problem(Problem {
            span: opened,
            message: format!("the block opened on line {} has no end", opened.line),
        }));
    }
    // blocks left open at the end: say so, keep what was read
    while let Some(frame) = stack.last() {
        let opened = frame.group.span;
        close(&mut program, &mut stack);
        target(&mut program, &mut stack).push(Item::Problem(Problem {
            span: opened,
            message: format!("the block opened on line {} is not closed", opened.line),
        }));
    }
    program
}

#[cfg(test)]
mod tests {
    use super::*;

    fn only_command(src: &str) -> Command {
        let p = read(src);
        match p.items.as_slice() {
            [Item::Command(c)] => c.clone(),
            other => panic!("{src:?} gave {other:?}"),
        }
    }

    #[test]
    fn a_command_comes_apart_with_its_full_name_and_place() {
        let c = only_command("  reg price mpg weight if foreign == 1 [aw = w], robust\n");
        assert_eq!(c.written, "reg");
        assert_eq!(c.name, "regress");
        assert!(c.known);
        assert_eq!(
            c.span,
            Span {
                line: 1,
                column: 3,
                lines: 1
            }
        );
        assert_eq!(c.parts.main, "price mpg weight");
        assert_eq!(
            c.parts.if_exp.as_deref().map(str::trim),
            Some("foreign == 1")
        );
        assert!(c.parts.weight.is_some());
        assert!(c.options.iter().any(|o| o.is("robust", 6)));
        let g = only_command("gen y=x^2");
        assert_eq!(
            (g.name.as_str(), g.parts.main.as_str()),
            ("generate", "y=x^2")
        );
        let unknown = only_command("ivreghdfe y (x = z), absorb(id)");
        assert_eq!(unknown.name, "ivreghdfe");
        assert!(!unknown.known);
    }

    #[test]
    fn using_is_taken_out_of_the_main_part() {
        let c = only_command("merge 1:1 id using \"data/other file.dta\", nogen");
        assert_eq!(c.parts.main, "1:1 id");
        assert_eq!(c.using.as_deref(), Some("\"data/other file.dta\""));
        let u = only_command("use \"my using file.dta\", clear");
        assert_eq!(u.using, None, "using inside quotes is part of the name");
        let v = only_command("use x y using data.dta");
        assert_eq!(
            (v.parts.main.as_str(), v.using.as_deref()),
            ("x y", Some("data.dta"))
        );
    }

    #[test]
    fn prefixes_are_read_in_order() {
        let c = only_command("capture quietly bysort g (t): gen n = _n");
        assert_eq!(
            c.prefixes,
            vec![
                Prefix::Capture,
                Prefix::Quietly,
                Prefix::By {
                    varlist: "g (t)".into(),
                    sort: true,
                    options: String::new()
                }
            ]
        );
        assert_eq!(c.name, "generate");
        let b = only_command("by g, sort: summarize x");
        assert!(matches!(&b.prefixes[0], Prefix::By { sort: true, varlist, .. } if varlist == "g"));
        let q = only_command("qui: su x");
        assert_eq!(q.prefixes, vec![Prefix::Quietly]);
        assert!(
            matches!(read("by g summarize x").items.as_slice(), [Item::Problem(p)] if p.message.contains("colon"))
        );
    }

    #[test]
    fn loops_and_ifs_nest_with_their_bodies() {
        let src = "\
foreach v of varlist price mpg {
    forvalues i = 1/3 {
        di `i'
    }
    if `v' > 0 {
        replace `v' = 0
    }
    else if $flag == 1 {
        drop `v'
    }
    else {
        keep `v'
    }
}
";
        // else on its own line after the } (as Stata's manual writes it) reads the same
        // as `} else {`
        let own_line = read(src);
        assert!(own_line.problems().is_empty(), "{:?}", own_line.problems());
        let names: Vec<&str> = own_line
            .commands()
            .iter()
            .map(|c| c.name.as_str())
            .collect();
        assert_eq!(names, vec!["display", "replace", "drop", "keep"]);

        let src = "\
foreach v of varlist price mpg {
    forvalues i = 1/3 {
        di `i'
    }
    if `v' > 0 {
        replace `v' = 0
    } else if $flag == 1 {
        drop `v'
    } else {
        keep `v'
    }
}
";
        let p = read(src);
        assert!(p.problems().is_empty(), "{:?}", p.problems());
        let [Item::Group(outer)] = p.items.as_slice() else {
            panic!("{:?}", p.items)
        };
        assert!(
            matches!(&outer.opener, Opener::Foreach { name, list: ForeachList::Of { kind, list } }
            if name == "v" && kind == "varlist" && list == "price mpg")
        );
        assert_eq!(outer.body.len(), 2);
        let Some(Item::Group(cond)) = outer.body.get(1) else {
            panic!("no if")
        };
        assert!(matches!(&cond.opener, Opener::If { condition } if condition == "`v' > 0"));
        let otherwise = cond.otherwise.as_deref().unwrap_or(&[]);
        let [Item::Group(else_if)] = otherwise else {
            panic!("{otherwise:?}")
        };
        assert!(matches!(&else_if.opener, Opener::If { condition } if condition == "$flag == 1"));
        assert!(else_if.otherwise.as_ref().is_some_and(|o| o.len() == 1));
        let names: Vec<&str> = p.commands().iter().map(|c| c.name.as_str()).collect();
        assert_eq!(names, vec!["display", "replace", "drop", "keep"]);
        // the line numbers are the file's
        let lines: Vec<usize> = p.commands().iter().map(|c| c.span.line).collect();
        assert_eq!(lines, vec![3, 6, 8, 10]);
    }

    #[test]
    fn capture_and_quietly_blocks_and_one_line_ifs() {
        let p = read("capture {\n  drop x\n}\nquietly {\n  su y\n}\nif _rc di \"failed\"\n");
        assert!(p.problems().is_empty(), "{:?}", p.problems());
        assert!(
            matches!(&p.items[0], Item::Group(g) if g.opener == Opener::Prefixed && g.prefixes == vec![Prefix::Capture])
        );
        assert!(matches!(&p.items[1], Item::Group(g) if g.prefixes == vec![Prefix::Quietly]));
        assert!(matches!(&p.items[2], Item::Command(c) if c.name == "if"));
    }

    #[test]
    fn continuations_and_comments_keep_the_line_numbers_right() {
        let src =
            "* a comment\nsummarize price ///\n    mpg  // why\n/* a\nblock */\nlist in 1/5\n";
        let p = read(src);
        let spans: Vec<Span> = p.commands().iter().map(|c| c.span).collect();
        assert_eq!(
            spans[0],
            Span {
                line: 2,
                column: 1,
                lines: 2
            }
        );
        assert_eq!(spans[1].line, 6);
        assert_eq!(
            p.commands()[0]
                .parts
                .main
                .split_whitespace()
                .collect::<Vec<_>>(),
            vec!["price", "mpg"]
        );
    }

    #[test]
    fn macros_are_recorded_and_left_in_the_text() {
        let c = only_command("regress `y' $controls ${more} `x`i'' if `touse'");
        assert_eq!(
            c.macros,
            vec![
                MacroRef::Local("y".into()),
                MacroRef::Global("controls".into()),
                MacroRef::Global("more".into()),
                MacroRef::Local("i".into()),
                MacroRef::Local("touse".into()),
            ]
        );
        assert!(c.parts.main.contains("`y'"));
        // a compound quote and a dollar amount are not macros
        assert!(macro_refs("di `\"say \"hi\"\"' \"$5\"").is_empty());
    }

    #[test]
    fn broken_blocks_are_problems_with_places_and_reading_goes_on() {
        let p = read("foreach x in a b\n  di 1\n}\n}\nforvalues i 1/3 {\nwhile {\n}\nlist\n");
        let msgs: Vec<(usize, String)> = p
            .problems()
            .iter()
            .map(|p| (p.span.line, p.message.clone()))
            .collect();
        assert!(
            msgs.iter()
                .any(|(l, m)| *l == 1 && m.contains("{ at the end")),
            "{msgs:?}"
        );
        assert!(
            msgs.iter()
                .any(|(l, m)| *l == 3 && m.contains("no { to close")),
            "{msgs:?}"
        );
        assert!(
            msgs.iter()
                .any(|(l, m)| *l == 5 && m.contains("name = range")),
            "{msgs:?}"
        );
        assert!(
            msgs.iter().any(|(l, m)| *l == 6 && m.contains("condition")),
            "{msgs:?}"
        );
        assert!(
            p.commands().iter().any(|c| c.name == "list"),
            "reading went on"
        );
        let open = read("foreach x in a {\n  di 1\n");
        assert!(open
            .problems()
            .iter()
            .any(|p| p.message.contains("line 1 is not closed")));
        assert_eq!(open.commands().len(), 1, "what was read is kept");
    }

    #[test]
    fn hostile_input_never_panics() {
        let deep = "foreach x in a {\n".repeat(200) + &"}\n".repeat(200);
        let p = read(&deep);
        assert!(p
            .problems()
            .iter()
            .any(|p| p.message.contains("nested deeper")));
        for junk in [
            "",
            "}",
            "{",
            "} else {",
            "else",
            "by",
            "by:",
            "bysort , sort:",
            ":",
            "\"",
            "`",
            "$",
            "`'",
            "${",
            "if",
            "if {",
            "foreach",
            "forv",
            "while 1",
            "capture",
            "capture {\n",
            "\u{0}\u{1}",
            "é è ü",
            &"(".repeat(10_000),
            &"`".repeat(10_000),
        ] {
            let _ = read(junk);
        }
    }

    #[test]
    fn delimit_semicolon_files_read_too() {
        let p = read("#delimit ;\nsummarize price\n  mpg;\nlist in 1/3;\n#delimit cr\ndescribe\n");
        let names: Vec<&str> = p.commands().iter().map(|c| c.name.as_str()).collect();
        assert_eq!(names, vec!["summarize", "list", "describe"]);
    }
}

#[cfg(test)]
mod else_tests {
    use super::*;

    #[test]
    fn else_forms_attach_to_their_if() {
        // else with a command on its line
        let p = read("if x > 1 {\n  di 1\n}\nelse di 2\nlist\n");
        assert!(p.problems().is_empty(), "{:?}", p.problems());
        let Some(Item::Group(g)) = p.items.first() else {
            panic!("{:?}", p.items)
        };
        assert!(g
            .otherwise
            .as_ref()
            .is_some_and(|o| matches!(o.as_slice(), [Item::Command(c)] if c.name == "display")));
        assert!(matches!(p.items.get(1), Some(Item::Command(c)) if c.name == "list"));
        // after a one-line if, a one-line else is kept as a command
        let q = read("if x > 1 di 1\nelse di 2\n");
        assert!(q.problems().is_empty());
        assert_eq!(
            q.commands()
                .iter()
                .map(|c| c.name.as_str())
                .collect::<Vec<_>>(),
            vec!["if", "else"]
        );
        // else with nothing before it
        let r = read("di 1\nelse {\n");
        assert!(r
            .problems()
            .iter()
            .any(|p| p.message.contains("no if block")));
        // a second else on the same if
        let s = read("if a {\n}\nelse {\n}\nelse {\n}\n");
        assert!(
            s.problems()
                .iter()
                .any(|p| p.message.contains("no if block")),
            "{:?}",
            s.problems()
        );
    }
}

#[cfg(test)]
mod block_tests {
    use super::*;

    #[test]
    fn programs_mata_python_and_input_are_blocks_up_to_end() {
        let src = "\
capture program drop twice
program define twice, rclass
    syntax varname
    return scalar n = 2 * `varlist'[1]
end
mata:
class Base {
    real scalar n
}
end
input x y
1 2
3 4
end
mata: st_numscalar(\"a\", 1)
twice price
";
        let p = read(src);
        assert!(p.problems().is_empty(), "{:?}", p.problems());
        let kinds: Vec<String> = p
            .items
            .iter()
            .map(|i| match i {
                Item::Command(c) => format!("command {}", c.name),
                Item::Group(g) => match &g.opener {
                    Opener::Program { name } => format!("program {name} ({} lines)", g.body.len()),
                    Opener::Raw { kind, lines, .. } => format!("{kind} ({} lines)", lines.len()),
                    other => format!("{other:?}"),
                },
                Item::Problem(p) => format!("problem {}", p.message),
            })
            .collect();
        assert_eq!(
            kinds,
            vec![
                "command program",
                "program twice (2 lines)",
                "mata (3 lines)",
                "input (2 lines)",
                "command mata",
                "command twice",
            ]
        );
        // the data lines are kept as written
        let Some(Item::Group(g)) = p.items.get(3) else {
            panic!()
        };
        assert!(
            matches!(&g.opener, Opener::Raw { header, lines, .. } if header == "input x y" && lines == &vec!["1 2".to_string(), "3 4".to_string()])
        );
    }

    #[test]
    fn a_stray_end_and_a_block_with_no_end_are_problems() {
        let p = read("end\nmata\nx = 1\n");
        let msgs: Vec<String> = p.problems().iter().map(|p| p.message.clone()).collect();
        assert!(
            msgs.iter().any(|m| m.contains("end with no program")),
            "{msgs:?}"
        );
        assert!(
            msgs.iter().any(|m| m.contains("line 2 has no end")),
            "{msgs:?}"
        );
        let q = read("program define f\n di 1\n");
        assert!(q
            .problems()
            .iter()
            .any(|p| p.message.contains("not closed")));
    }
}

#[cfg(test)]
mod frame_tests {
    use super::*;

    #[test]
    fn frame_blocks_and_prefixes_are_read_and_subcommands_stay_commands() {
        let p = read("frame create rates\nframe rates {\n  set obs 6\n}\nframe rates: gen x = 1\nframe change default\n");
        assert!(p.problems().is_empty(), "{:?}", p.problems());
        assert!(
            matches!(&p.items[0], Item::Command(c) if c.name == "frame" && c.parts.main == "create rates")
        );
        assert!(
            matches!(&p.items[1], Item::Group(g) if g.opener == Opener::Frame { name: "rates".into() } && g.body.len() == 1)
        );
        assert!(
            matches!(&p.items[2], Item::Command(c) if c.prefixes == vec![Prefix::Frame("rates".into())] && c.name == "generate")
        );
        assert!(matches!(&p.items[3], Item::Command(c) if c.name == "frame"));
    }
}
