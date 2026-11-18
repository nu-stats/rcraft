//! `dyntext` and `dyndoc` (P2C.12x, batch D of the StatCraft port), as Stata
//! 19 runs them (`plan/stata/dyndoc.md`): a text with dynamic tags run into a
//! text file, or into HTML through Markdown.
//!
//! The source is read from R when the line is reached (the do-file usually
//! writes it a few lines before, with `file write`), and its tags become a
//! run of their own, the way `do` runs a file: `dd_do` blocks are their
//! commands, translated and run as any other line, with what each prints
//! kept for the document (echoed after `. `, then its output, then a blank
//! line); `dd_if`, `dd_else` and `dd_skip_if` become `if` blocks; text lines
//! become R that adds them to the document, `dd_display` worked out as
//! `display` prints it; `dd_ignore` and `dd_include` lines go in as they are.
//! The last line prints the document and writes it (`stata_dd_save`). A line
//! of the run that is not translated leaves the document unwritten (refused),
//! never half right.

use super::*;

/// The document being made: how the lines a `dd_do` command makes are kept,
/// and the first line of the run that was not translated.
#[derive(Debug, Clone, Default)]
pub(super) struct DynDoc {
    /// While a `dd_do` command runs: `true` keeps what it prints, `false`
    /// drops it (`quietly`, `nooutput`).
    pub(super) keep: Option<bool>,
    pub(super) failed: Option<String>,
}

/// Text as hex digits (`-` for none), so that no macro, quote or comma in it
/// reaches the do-file reader.
fn hex(s: &str) -> String {
    if s.is_empty() {
        return "-".into();
    }
    s.bytes().map(|b| format!("{b:02x}")).collect()
}

fn unhex(s: &str) -> Option<String> {
    if s == "-" {
        return Some(String::new());
    }
    let b = s.as_bytes();
    if b.len() % 2 != 0 {
        return None;
    }
    let bytes: Option<Vec<u8>> = b
        .chunks(2)
        .map(|p| {
            std::str::from_utf8(p)
                .ok()
                .and_then(|h| u8::from_str_radix(h, 16).ok())
        })
        .collect();
    String::from_utf8(bytes?).ok()
}

/// A tag that is a line of its own: its name and what follows the colon.
fn line_tag(line: &str) -> Option<(String, String)> {
    let inner = line.trim().strip_prefix("<<")?.strip_suffix(">>")?;
    if inner.contains(">>") {
        return None;
    }
    let (name, arg) = match inner.split_once(':') {
        Some((n, a)) => (n.trim(), a.trim()),
        None => (inner.trim(), ""),
    };
    const KNOWN: &[&str] = &[
        "dd_version",
        "dd_do",
        "/dd_do",
        "dd_if",
        "dd_else",
        "dd_endif",
        "dd_skip_if",
        "dd_skip_else",
        "dd_skip_end",
        "dd_ignore",
        "/dd_ignore",
        "dd_include",
    ];
    KNOWN
        .contains(&name)
        .then(|| (name.to_string(), arg.to_string()))
}

/// The pieces of a text line: literal text, a `dd_display` and a `dd_graph`.
enum Piece {
    Text(String),
    Display(String),
    Graph(String),
}

/// A text line cut at its inline tags. `dd_remove` stays in the text (it is
/// taken out when the file is written); a `dd_if` or `dd_endif` inside a line
/// is text (Stata 19, probe 547). Other inline tags are not translated.
fn pieces(line: &str) -> Result<Vec<Piece>, String> {
    let mut out = Vec::new();
    let mut text = String::new();
    let mut rest = line;
    while let Some(at) = rest.find("<<dd_") {
        let (before, tail) = rest.split_at(at);
        text.push_str(before);
        let Some(end) = tail.find(">>") else {
            text.push_str(tail);
            rest = "";
            break;
        };
        let inner = tail.get(2..end).unwrap_or("");
        let (name, arg) = match inner.split_once(':') {
            Some((n, a)) => (n.trim(), a.trim()),
            None => (inner.trim(), ""),
        };
        match name {
            "dd_display" | "dd_graph" => {
                if !text.is_empty() {
                    out.push(Piece::Text(std::mem::take(&mut text)));
                }
                out.push(if name == "dd_display" {
                    Piece::Display(arg.to_string())
                } else {
                    Piece::Graph(arg.to_string())
                });
            }
            "dd_remove" | "dd_if" | "dd_endif" => text.push_str(tail.get(..end + 2).unwrap_or("")),
            other => {
                return Err(format!(
                    "the tag <<{other}>> inside a line is not translated yet"
                ))
            }
        }
        rest = tail.get(end + 2..).unwrap_or("");
    }
    text.push_str(rest);
    if !text.is_empty() {
        out.push(Piece::Text(text));
    }
    Ok(out)
}

/// `<<dd_graph: saving() replace alt() height() width()>>` as the arguments
/// of `stata_dd_graph`.
fn graph_args(attrs: &str) -> Result<String, String> {
    let mut args = Vec::new();
    let mut rest = attrs.trim();
    while !rest.is_empty() {
        let name_len = rest
            .find(|c: char| !c.is_ascii_alphanumeric())
            .unwrap_or(rest.len());
        let (name, after) = rest.split_at(name_len);
        let name = name.to_ascii_lowercase();
        let (value, after) = match after.strip_prefix('(') {
            Some(a) => {
                let close = a
                    .find(')')
                    .ok_or_else(|| format!("dd_graph: {name}( without its )"))?;
                (
                    Some(a.get(..close).unwrap_or("").trim()),
                    a.get(close + 1..).unwrap_or(""),
                )
            }
            None => (None, after),
        };
        match (name.as_str(), value) {
            ("saving", Some(v)) if !v.contains([',', ' ']) && v.to_ascii_lowercase().ends_with(".svg") => {
                args.push(format!("saving = {}", r_text(v.trim_matches('"'))))
            }
            ("replace", None) => args.push("replace = TRUE".into()),
            ("alt" | "height" | "width", Some(v)) if !v.is_empty() && !v.contains(' ') => {
                args.push(format!("{name} = {}", r_text(v.trim_matches('"'))))
            }
            _ => {
                return Err(format!(
                    "dd_graph with the attribute {name} (as written here) is not translated yet: an .svg file, replace, alt(), height() and width() are"
                ))
            }
        }
        rest = after.trim_start();
    }
    Ok(args.join(", "))
}

fn r_text(s: &str) -> String {
    crate::expr_r::r_text(s)
}

impl Run<'_> {
    /// `dyntext src, saving(file) [replace]` and `dyndoc src [, saving(file)
    /// replace]`.
    pub(super) fn dyn_command(&mut self, c: &Command, depth: usize, written: String) -> Flow {
        let refuse = |s: &mut Self, why: String, written: String| {
            s.done_item(depth);
            let line = s.t.refused(c.span, c.text.clone(), why);
            s.push(line, What::Command, None, written);
            Flow::Line
        };
        let kind = c.name.clone();
        if let Some(o) = c
            .options
            .iter()
            .find(|o| !(o.is("saving", 6) || o.is("replace", 7)))
        {
            return refuse(
                self,
                format!("{kind} with the option {} is not translated yet", o.name),
                written,
            );
        }
        if c.using.is_some() || c.parts.if_exp.is_some() || c.parts.in_range.is_some() {
            return refuse(
                self,
                format!("{kind} is translated for a file name and its options"),
                written,
            );
        }
        if self.dd.is_some() {
            return refuse(
                self,
                format!("{kind} inside a dd_do block is not translated"),
                written,
            );
        }
        let toks = macros::tokens(c.parts.main.trim());
        let [src] = toks.as_slice() else {
            return refuse(
                self,
                format!("{kind} is translated with one file name"),
                written,
            );
        };
        let src = clean_path(src);
        let replace = c.options.iter().any(|o| o.is("replace", 7));
        let saving = c
            .options
            .iter()
            .find(|o| o.is("saving", 6))
            .map(|o| o.arg());
        let target = match saving {
            Some(s) if !s.contains(',') && !s.trim().is_empty() => clean_path(s.trim()),
            Some(_) => {
                return refuse(
                    self,
                    format!("{kind}, saving(file, ...) is not translated yet"),
                    written,
                )
            }
            None if kind == "dyndoc" => {
                let stem = std::path::Path::new(&src).with_extension("html");
                stem.to_string_lossy().replace('\\', "/")
            }
            // Stata 19: "option saving() required", r(198), before it reads anything
            None => {
                self.done_item(depth);
                let mut stop = self.internal(c.span, "_rcraft_fail 198 option saving() required");
                stop.prefixes = c.prefixes.clone();
                stop.text = c.text.clone();
                let line = self.t.command(&stop);
                self.push(line, What::Command, None, written);
                return Flow::Line;
            }
        };
        self.t.need_helper("stata_dyndoc");
        let check = self.ask(Need::R(
            format!(
                "stata_dd_check({}, {}, {})",
                r_text(&src),
                r_text(&target),
                if replace { "TRUE" } else { "FALSE" }
            ),
            format!("the file {kind} reads"),
        ));
        let text = match check {
            Err(Stop::Ask(n)) => return Flow::Ask(n),
            Err(e) => return refuse(self, stop_reason(&e), written),
            Ok(t) => t,
        };
        let (status, content) = text.split_once('\n').unwrap_or((text.as_str(), ""));
        let fail = match status.trim() {
            "ok" => None,
            "601" => Some(format!("601 file {src} not found")),
            s if s.starts_with("602 ") => Some(format!(
                "602 file {} already exists",
                s.get(4..).unwrap_or("")
            )),
            other => {
                return refuse(
                    self,
                    format!("{kind}: R could not read the file ({other})"),
                    written,
                )
            }
        };
        if let Some(f) = fail {
            // Stata stops before running any tag: the source is not there, or the target is
            self.done_item(depth);
            let mut stop = self.internal(c.span, &format!("_rcraft_fail {f}"));
            stop.prefixes = c.prefixes.clone();
            stop.text = c.text.clone();
            let line = self.t.command(&stop);
            self.push(line, What::Command, None, written);
            return Flow::Line;
        }
        if c.prefixes.iter().any(|p| !matches!(p, Prefix::Noisily)) {
            return refuse(
                self,
                format!("{kind} under capture or quietly is not translated yet (only when Stata stops before the tags run)"),
                written,
            );
        }
        let content = content.to_string();
        let synthetic = match self.dyn_source(&content, &kind, &target) {
            Ok(s) => s,
            Err(Stop::Ask(n)) => return Flow::Ask(n),
            Err(Stop::Not(why)) | Err(Stop::Stata(why)) => return refuse(self, why, written),
            Err(e) => return refuse(self, stop_reason(&e), written),
        };
        let mut program = reader::read(&synthetic);
        if program.items.iter().any(|i| matches!(i, Item::Problem(_))) {
            return refuse(
                self,
                format!("{kind}: the do-file reader could not read a line of a dd_do block"),
                written,
            );
        }
        for item in &mut program.items {
            at_span(item, c.span);
        }
        self.done_item(depth);
        self.push(
            nothing(c),
            What::Control,
            Some(format!("runs the dynamic tags of {src}")),
            written,
        );
        self.frames.push(Frame {
            items: Rc::new(program.items),
            at: 0,
            kind: Kind::Body,
            prefixes: Vec::new(),
        });
        Flow::Line
    }

    /// The source's tags as a run of Stata lines: RCraft's own
    /// `_rcraft_dd` lines for the document, `if` blocks for `dd_if`, and the
    /// `dd_do` commands as they are.
    fn dyn_source(&mut self, content: &str, kind: &str, target: &str) -> Result<String, Stop> {
        let not = |why: &str| Stop::Not(why.to_string());
        let mut out = String::from("_rcraft_dd begin\n");
        let lines: Vec<&str> = content
            .lines()
            .map(|l| l.strip_suffix('\r').unwrap_or(l))
            .collect();
        // each open dd_if or dd_skip_if, and whether its else part has begun
        let mut open: Vec<(bool, bool)> = Vec::new();
        let mut i = 0;
        while let Some(&line) = lines.get(i) {
            i += 1;
            let tag = line_tag(line);
            let Some((name, arg)) = tag else {
                if let Err(why) = pieces(line) {
                    return Err(Stop::Not(why));
                }
                out.push_str(&format!("_rcraft_dd text {}\n", hex(line)));
                continue;
            };
            match name.as_str() {
                "dd_version" => {
                    if arg.trim() != "2" {
                        return Err(not(
                            "dyntext with a dd_version other than 2 is not translated yet",
                        ));
                    }
                }
                "dd_if" | "dd_skip_if" => {
                    // Stata 19 does not nest them: a dd_if inside one starts over, and the
                    // dd_endif left after it is text (corpus dyndoc.do); not reproduced
                    if !open.is_empty() {
                        return Err(not("a dd_if inside another dd_if is not translated (Stata 19 does not nest them)"));
                    }
                    if arg.trim().is_empty() {
                        return Err(not("dd_if without a condition is not translated"));
                    }
                    let skip = name == "dd_skip_if";
                    out.push_str(&if skip {
                        format!("if !({}) {{\n", arg.trim())
                    } else {
                        format!("if ({}) {{\n", arg.trim())
                    });
                    open.push((skip, false));
                }
                "dd_else" | "dd_skip_else" => match open.last_mut() {
                    Some((skip, in_else)) if *skip == (name == "dd_skip_else") && !*in_else => {
                        *in_else = true;
                        out.push_str("}\nelse {\n");
                    }
                    _ => return Err(not("a dd_else without its dd_if is not translated")),
                },
                "dd_endif" | "dd_skip_end" => match open.pop() {
                    Some((skip, _)) if skip == (name == "dd_skip_end") => out.push_str("}\n"),
                    _ => return Err(not("a dd_endif without its dd_if is not translated")),
                },
                "dd_ignore" => loop {
                    let Some(&l) = lines.get(i) else {
                        return Err(not(
                            "dd_ignore without its <</dd_ignore>> is not translated",
                        ));
                    };
                    i += 1;
                    if line_tag(l).is_some_and(|(n, _)| n == "/dd_ignore") {
                        break;
                    }
                    out.push_str(&format!("_rcraft_dd raw {}\n", hex(l)));
                },
                "dd_include" => {
                    let file = clean_path(arg.trim());
                    let read = self.ask(Need::R(
                        format!("stata_dd_read({})", r_text(&file)),
                        format!("the file dd_include reads ({file})"),
                    ))?;
                    let (status, text) = read.split_once('\n').unwrap_or((read.as_str(), ""));
                    if status.trim() != "ok" {
                        return Err(not(&format!(
                            "dd_include of a file that is not there ({file}) is not translated"
                        )));
                    }
                    // the file's lines go in exactly, a carriage return at their end kept
                    // (Stata 19 keeps the one file write puts there on Windows; corpus
                    // dyndoc.do); the answer has lost its last newline already
                    if !text.is_empty() {
                        for l in text.split('\n') {
                            out.push_str(&format!("_rcraft_dd raw {}\n", hex(l)));
                        }
                    }
                }
                "dd_do" => {
                    let mut flags = String::new();
                    for w in arg.split_whitespace() {
                        match w.to_ascii_lowercase().as_str() {
                            "quietly" => flags.push('q'),
                            "nocommands" => flags.push('c'),
                            "nooutput" => flags.push('o'),
                            "noprompt" => flags.push('p'),
                            other => {
                                return Err(Stop::Not(format!(
                                    "dd_do with the attribute {other} is not translated yet"
                                )))
                            }
                        }
                    }
                    if flags.is_empty() {
                        flags.push('-');
                    }
                    let mut closed = false;
                    while let Some(&l) = lines.get(i) {
                        i += 1;
                        if line_tag(l).is_some_and(|(n, _)| n == "/dd_do") {
                            closed = true;
                            break;
                        }
                        let t = l.trim();
                        if t.is_empty()
                            || t.starts_with('*')
                            || t.starts_with("//")
                            || l.contains("///")
                            || l.contains("/*")
                            || l.contains(['{', '}'])
                            || l.contains("<<dd_")
                        {
                            return Err(not(
                                "a dd_do block with a blank line, a comment, a block in braces or a tag in it is not translated yet",
                            ));
                        }
                        let read = reader::read(l);
                        if !matches!(read.items.as_slice(), [Item::Command(_)]) {
                            return Err(not(
                                "a dd_do line that is not one command is not translated yet",
                            ));
                        }
                        out.push_str(&format!("_rcraft_dd cmd {flags} {}\n", hex(l)));
                        if flags.contains('q') {
                            out.push_str("quietly ");
                        }
                        out.push_str(l);
                        out.push('\n');
                        out.push_str(&format!("_rcraft_dd endcmd {flags}\n"));
                    }
                    if !closed {
                        return Err(not("dd_do without its <</dd_do>> is not translated"));
                    }
                }
                _ => {
                    return Err(Stop::Not(format!(
                        "the tag <<{name}>> without the one that opens it is not translated"
                    )))
                }
            }
        }
        if !open.is_empty() {
            return Err(not("a dd_if without its dd_endif is not translated"));
        }
        out.push_str(&format!("_rcraft_dd save {kind} {}\n", hex(target)));
        Ok(out)
    }

    /// One of the run's own `_rcraft_dd` lines.
    pub(super) fn dd_line(&mut self, c: &Command, depth: usize, written: String) -> Flow {
        self.done_item(depth);
        let words: Vec<&str> = c.parts.main.split_whitespace().collect();
        let own = |s: &mut Self, stata: String, code: String| {
            s.t.own_line(c.span, stata, code, &["stata_dyndoc"], &[])
        };
        let line = match words.as_slice() {
            ["begin"] => {
                self.dd = Some(DynDoc::default());
                own(
                    self,
                    "(the document begins)".into(),
                    "stata_dd_begin()".into(),
                )
            }
            ["raw", h] => {
                let text = unhex(h).unwrap_or_default();
                own(
                    self,
                    text.clone(),
                    format!("stata_dd_put({})", r_text(&text)),
                )
            }
            ["text", h] => {
                let text = unhex(h).unwrap_or_default();
                match self.text_code(&text) {
                    Ok(code) => own(self, text, format!("stata_dd_put({code})")),
                    Err(why) => self.t.refused(c.span, text, why),
                }
            }
            ["cmd", flags, h] => {
                let cmd = unhex(h).unwrap_or_default();
                let echo = !flags.contains('q') && !flags.contains('c');
                let shown = if flags.contains('p') {
                    cmd.clone()
                } else {
                    format!(". {cmd}")
                };
                let line = own(
                    self,
                    format!("(echo of {cmd})"),
                    if echo {
                        format!("stata_dd_put({}, cr = TRUE)", r_text(&shown))
                    } else {
                        String::new()
                    },
                );
                // the echo goes in first; what the command prints is kept after it
                self.push(line, What::Command, None, written);
                if let Some(d) = self.dd.as_mut() {
                    d.keep = Some(!flags.contains('q') && !flags.contains('o'));
                }
                return Flow::Line;
            }
            ["endcmd", flags] => {
                if let Some(d) = self.dd.as_mut() {
                    d.keep = None;
                }
                // the blank line that ends a command's output (none with quietly,
                // nor with nooutput and nocommands together)
                let blank = !flags.contains('q') && !(flags.contains('o') && flags.contains('c'));
                own(
                    self,
                    "(end of the command's output)".into(),
                    if blank {
                        "stata_dd_put(\"\", cr = TRUE)".into()
                    } else {
                        String::new()
                    },
                )
            }
            ["save", kind, h] => {
                let target = unhex(h).unwrap_or_default();
                let failed = self.dd.take().and_then(|d| d.failed);
                match failed {
                    Some(why) => self.t.refused(
                        c.span,
                        format!("({kind} writes {target})"),
                        format!(
                            "{kind} is not written: a line of its tags was not translated ({why})"
                        ),
                    ),
                    None => own(
                        self,
                        format!("({kind} prints the document and writes {target})"),
                        format!("stata_dd_save({}, {})", r_text(kind), r_text(&target)),
                    ),
                }
            }
            _ => self.t.refused(
                c.span,
                c.text.clone(),
                "a line RCraft made for dyntext was not read back",
            ),
        };
        self.push(line, What::Command, None, written);
        Flow::Line
    }

    /// The R for a text line: its text, each `dd_display` as `display` prints
    /// it, each `dd_graph` as its `<img>` tag.
    fn text_code(&mut self, text: &str) -> Result<String, String> {
        let mut parts = Vec::new();
        for p in pieces(text)? {
            match p {
                Piece::Text(t) => parts.push(r_text(&t)),
                Piece::Display(rest) => {
                    let v = self
                        .t
                        .display_code(&rest)
                        .map_err(|e| format!("dd_display: {}", e.reason))?;
                    parts.push(format!(
                        "paste(utils::capture.output({{\n{}\n}}), collapse = \"\\n\")",
                        v.code
                    ));
                }
                Piece::Graph(attrs) => {
                    if !self.t.graph_drawn() {
                        return Err("dd_graph needs a graph drawn earlier in this do-file".into());
                    }
                    parts.push(format!("stata_dd_graph({})", graph_args(&attrs)?));
                }
            }
        }
        Ok(match parts.len() {
            0 => "\"\"".to_string(),
            1 => parts.remove(0),
            _ => format!("paste0({})", parts.join(", ")),
        })
    }

    /// While a `dd_do` command of a dyntext runs: its R keeps what it prints
    /// for the document (or drops it). A line that is not translated marks
    /// the document as not written.
    pub(super) fn dd_wrap(&mut self, line: &mut Line) {
        let Some(d) = self.dd.as_mut() else {
            return;
        };
        match &mut line.result {
            Err(e) => {
                d.failed
                    .get_or_insert_with(|| format!("{}: {}", line.stata.trim(), e.reason));
            }
            Ok(r) if !r.code.is_empty() => match d.keep {
                Some(true) => {
                    r.code = format!("stata_dd_out(utils::capture.output({{\n{}\n}}))", r.code)
                }
                Some(false) => {
                    r.code = format!("invisible(utils::capture.output({{\n{}\n}}))", r.code)
                }
                None => {}
            },
            Ok(_) => {}
        }
    }
}

/// Every command of an item at the dyntext line's place in the do-file.
fn at_span(item: &mut Item, span: Span) {
    match item {
        Item::Command(c) => c.span = span,
        Item::Group(g) => {
            g.span = span;
            for i in &mut g.body {
                at_span(i, span);
            }
            if let Some(o) = &mut g.otherwise {
                for i in o {
                    at_span(i, span);
                }
            }
        }
        Item::Problem(p) => p.span = span,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tags_are_read_as_stata_reads_them() {
        assert_eq!(unhex(&hex("a <<dd_x>> é")).as_deref(), Some("a <<dd_x>> é"));
        assert_eq!(unhex(&hex("")).as_deref(), Some(""));
        assert!(unhex("zz").is_none() && unhex("a").is_none());
        assert_eq!(
            line_tag("<<dd_do: quietly>>"),
            Some(("dd_do".into(), "quietly".into()))
        );
        assert_eq!(
            line_tag(" <</dd_do>> "),
            Some(("/dd_do".into(), String::new()))
        );
        assert!(line_tag("Inline <<dd_if: 1>>yes<<dd_endif>>").is_none());
        assert!(line_tag("<<dd_display: 1>>").is_none());
        let p =
            pieces("a <<dd_display: %4.1f x>> b <<dd_remove>>c<</dd_remove>>").unwrap_or_default();
        assert_eq!(p.len(), 3);
        assert!(pieces("<<dd_docx_display: 1>>").is_err());
        assert_eq!(
            graph_args("saving(g.svg) replace alt(plot) height(200)").as_deref(),
            Ok("saving = \"g.svg\", replace = TRUE, alt = \"plot\", height = \"200\"")
        );
        assert!(graph_args("alt(The plot)").is_err() && graph_args("saving(g.png)").is_err());
    }
}
