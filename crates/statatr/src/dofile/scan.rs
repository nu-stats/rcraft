//! What a do-file holds before it runs (P2C.1 and P2C.10), and what is
//! written afterwards (P2C.7): the R script, a Quarto document and the run's
//! report, which says line by line what ran, what ran with a note and what
//! did not run.

use std::collections::BTreeMap;

use super::{DoFile, FileRef, Origin, What};
use crate::data_r::{Line, STATA_COMMENT};
use crate::live::{HELPERS_BEGIN, HELPERS_END};

/// What a file the do-file names is for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum FileRole {
    Read,
    Write,
    /// A do-file it runs (`do`, `run`, `include`).
    Run,
    /// A folder it goes to (`cd`).
    Folder,
}

/// Whether it is there.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum FileStatus {
    There,
    Missing,
    /// Read after the do-file itself saved it.
    MadeHere,
    /// Written by the do-file.
    Written,
}

/// How one line went when the do-file ran.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Ran {
    /// R ran it (or there was nothing for R to do).
    Ok,
    /// R stopped with this error.
    Error(String),
    /// Not run: it is not translated, or the run went past it after a problem.
    Skipped,
    /// The run stopped before it.
    NotReached,
}

/// How a line of a dry run stands.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    /// Translated with nothing to say.
    Clean,
    /// Translated, with a note on where R or the translation differs.
    Notes,
    /// A macro, a loop or a block: done by RCraft, nothing for R.
    ByRcraft,
    /// Not translated.
    Not,
    /// Known only when the do-file runs.
    Later,
}

pub fn status(line: &Line, origin: &Origin) -> Status {
    match (&line.result, origin.what) {
        (_, What::Later) => Status::Later,
        (Err(_), _) => Status::Not,
        (Ok(_), What::Macro | What::Control) => Status::ByRcraft,
        (Ok(r), _) if r.caveats.is_empty() => Status::Clean,
        (Ok(_), _) => Status::Notes,
    }
}

/// One command and how often it came.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandUse {
    pub name: String,
    pub lines: usize,
    /// How many of them are not translated.
    pub not: usize,
}

/// The dry run, summed up.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Scan {
    pub lines: usize,
    pub clean: usize,
    pub notes: usize,
    pub by_rcraft: usize,
    pub not: usize,
    pub later: usize,
    pub commands: Vec<CommandUse>,
    /// Why lines are not translated, the most common first.
    pub reasons: Vec<(String, usize)>,
    /// R packages the translation calls.
    pub packages: Vec<&'static str>,
    /// What `ssc install` asked for.
    pub ssc: Vec<String>,
    /// Files and folders named, each once.
    pub files: Vec<FileRef>,
    /// Things Stata does that a run on R cannot do the same (P2C.10).
    pub limits: Vec<&'static str>,
}

/// The random functions whose draws are not Stata's yet (`runiform`, `rnormal` and
/// `rgamma` are, P2C.11).
const RANDOM: &[&str] = &[
    "rbinomial(",
    "rpoisson(",
    "rchi2(",
    "rbeta(",
    "rexponential(",
    "rlogistic(",
    "runiformint(",
    "rt(",
];

pub fn scan(d: &DoFile) -> Scan {
    let mut s = Scan {
        lines: d.translation.lines.len(),
        packages: d.translation.packages(),
        ssc: d.translation.ssc.clone(),
        ..Scan::default()
    };
    let mut commands: BTreeMap<String, CommandUse> = BTreeMap::new();
    let mut reasons: BTreeMap<String, usize> = BTreeMap::new();
    let limit = |s: &mut Scan, l: &'static str| {
        if !s.limits.contains(&l) {
            s.limits.push(l);
        }
    };
    for (line, origin) in d.translation.lines.iter().zip(&d.origins) {
        let st = status(line, origin);
        match st {
            Status::Clean => s.clean += 1,
            Status::Notes => s.notes += 1,
            Status::ByRcraft => s.by_rcraft += 1,
            Status::Not => s.not += 1,
            Status::Later => s.later += 1,
        }
        if let Err(e) = &line.result {
            if st == Status::Not {
                *reasons.entry(e.reason.clone()).or_default() += 1;
            }
        }
        let text = line.stata.trim();
        let word = command_word(text);
        if origin.what == What::Command && !word.is_empty() {
            let u = commands.entry(word.clone()).or_insert(CommandUse {
                name: word.clone(),
                lines: 0,
                not: 0,
            });
            u.lines += 1;
            if line.result.is_err() {
                u.not += 1;
            }
        }
        let lower = text.to_ascii_lowercase();
        // a function name on its own (rt( is not the end of sqrt()
        let calls = |f: &str| {
            lower.match_indices(f).any(|(i, _)| {
                !lower
                    .get(..i)
                    .and_then(|b| b.chars().next_back())
                    .is_some_and(|c| c.is_ascii_alphanumeric() || c == '_')
            })
        };
        if RANDOM.iter().any(|r| calls(r))
            || matches!(
                word.as_str(),
                "bootstrap" | "simulate" | "permute" | "sample"
            )
        {
            limit(&mut s, "Random numbers: runiform(), rnormal(), rgamma() and bsample give Stata's own draws, but the other random functions and the commands that draw (bootstrap, simulate, ...) do not, and the draws after them wait for a set seed.");
        }
        if lower.starts_with("mata") {
            limit(&mut s, "Mata: its blocks are not run.");
        }
        if lower.starts_with("program define") {
            limit(&mut s, "Programs the do-file defines (program define ... end) are not run, nor the lines that call them.");
        }
        if lower.contains("plugin") {
            limit(&mut s, "Compiled plugins are not run.");
        }
        if matches!(
            word.as_str(),
            "browse" | "edit" | "pause" | "more" | "window" | "db" | "keyboard"
        ) {
            limit(
                &mut s,
                "Interactive commands (browse, edit, pause, window) do nothing in a run on R.",
            );
        }
    }
    s.commands = commands.into_values().collect();
    s.commands
        .sort_by(|a, b| b.lines.cmp(&a.lines).then(a.name.cmp(&b.name)));
    let mut r: Vec<(String, usize)> = reasons.into_iter().collect();
    r.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    s.reasons = r;
    for f in &d.refs {
        if !s
            .files
            .iter()
            .any(|g| g.path.eq_ignore_ascii_case(&f.path) && g.role == f.role)
        {
            s.files.push(f.clone());
        }
    }
    s
}

/// The first word of a Stata line, after its prefixes.
pub fn command_word(text: &str) -> String {
    let mut rest = text;
    loop {
        let t = rest.trim_start();
        let (w, after) = t.split_once(char::is_whitespace).unwrap_or((t, ""));
        let w = w.trim_end_matches(':');
        let prefix = [
            "capture", "cap", "capt", "quietly", "qui", "quie", "quiet", "noisily", "noi",
        ]
        .contains(&w);
        if prefix {
            rest = after;
            continue;
        }
        if matches!(w, "by" | "bys" | "bysort") {
            match after.split_once(':') {
                Some((_, a)) => {
                    rest = a;
                    continue;
                }
                None => return w.to_string(),
            }
        }
        return w.split([',', '(']).next().unwrap_or("").to_string();
    }
}

impl Scan {
    /// The scan in words, as the window and a copy show it.
    pub fn text(&self, name: &str) -> String {
        let mut out = format!("{name}: {} lines when it runs\n", self.lines);
        out.push_str(&format!(
            "  {} translated, {} of them with a note on how R differs\n",
            self.clean + self.notes,
            self.notes
        ));
        out.push_str(&format!(
            "  {} macros, loops and blocks, which RCraft runs itself\n",
            self.by_rcraft
        ));
        out.push_str(&format!("  {} not translated\n", self.not));
        if self.later > 0 {
            out.push_str(&format!(
                "  {} that need a value known only when the do-file runs (R works it out then)\n",
                self.later
            ));
        }
        if !self.reasons.is_empty() {
            out.push_str("\nNot translated:\n");
            for (why, n) in &self.reasons {
                out.push_str(&format!("  {n} x {why}\n"));
            }
        }
        if !self.files.is_empty() {
            out.push_str("\nFiles and folders it names:\n");
            for f in &self.files {
                let role = match f.role {
                    FileRole::Read => "reads",
                    FileRole::Write => "writes",
                    FileRole::Run => "runs",
                    FileRole::Folder => "goes to",
                };
                let st = match f.status {
                    FileStatus::There => "there",
                    FileStatus::Missing => "NOT FOUND",
                    FileStatus::MadeHere => "made by the do-file",
                    FileStatus::Written => "written",
                };
                out.push_str(&format!("  {role} {} ({st})\n", f.path));
            }
        }
        if !self.packages.is_empty() {
            out.push_str(&format!("\nR packages: {}\n", self.packages.join(", ")));
        }
        if !self.ssc.is_empty() {
            out.push_str(&format!("ssc install asks for: {}\n", self.ssc.join(", ")));
        }
        if !self.limits.is_empty() {
            out.push_str("\nWhat a run on R does not do as Stata does:\n");
            for l in &self.limits {
                out.push_str(&format!("  {l}\n"));
            }
        }
        out
    }
}

/// The comment lines above a line's R that say where it came from.
fn about(origin: &Origin) -> String {
    let mut out = String::new();
    if !origin.loops.is_empty() {
        out.push_str(&format!("#   in: {}\n", origin.loops.join("; ")));
    }
    if let Some(info) = &origin.info {
        out.push_str(&format!("#   {info}\n"));
    }
    out
}

fn helpers_block(d: &DoFile) -> String {
    if d.translation.helpers.is_empty() {
        return String::new();
    }
    let mut out = format!("{HELPERS_BEGIN}\n");
    for h in &d.translation.helpers {
        out.push_str(h);
        out.push('\n');
    }
    out.push_str(HELPERS_END);
    out.push_str("\n\n");
    out
}

/// The do-file as an R script: each Stata line as a comment above its R
/// (P2C.7). `folder` is where it ran.
pub fn r_script(d: &DoFile, name: &str, folder: &str) -> String {
    let mut out = format!(
        "# {name}, run on R by RCraft's Stata mode.\n\
         # Each Stata line is a comment above the R it became; a line that is not\n\
         # translated says why. Stata runs those lines, so what comes after one may\n\
         # differ from Stata's results. Loops are written out round by round, and a\n\
         # macro whose value R worked out holds the value it had in this run.\n"
    );
    if !folder.is_empty() {
        out.push_str(&format!(
            "# It ran from the do-file's folder; start R there to run it again:\n# setwd(\"{folder}\")\n"
        ));
    }
    out.push('\n');
    out.push_str(&helpers_block(d));
    for (line, origin) in d.translation.lines.iter().zip(&d.origins) {
        let script = line.script();
        let (first, rest) = script.split_once('\n').unwrap_or((&script, ""));
        out.push_str(first);
        out.push('\n');
        out.push_str(&about(origin));
        out.push_str(rest);
    }
    out
}

/// The do-file as a Quarto document: its R in chunks, the Stata above each,
/// and the lines that are not translated as text with the reason (P2C.7).
pub fn quarto(d: &DoFile, name: &str) -> String {
    let mut out = format!(
        "---\ntitle: \"{}\"\nformat: html\n---\n\n\
         The Stata do-file `{name}`, run on R by RCraft's Stata mode. Each chunk \
         shows a Stata line as a comment above the R it became.\n\n",
        name.replace('"', "'")
    );
    let helpers = helpers_block(d);
    if !helpers.is_empty() {
        out.push_str("```{r}\n#| label: rcraft-stata-helpers\n#| include: false\n");
        out.push_str(helpers.trim_end());
        out.push_str("\n```\n\n");
    }
    for (line, origin) in d.translation.lines.iter().zip(&d.origins) {
        let stata = line.stata.replace('\n', " ");
        let loops = if origin.loops.is_empty() {
            String::new()
        } else {
            format!(" (in: {})", origin.loops.join("; "))
        };
        match (&line.result, origin.what) {
            (Ok(r), What::Command) if !r.code.is_empty() => {
                out.push_str("```{r}\n");
                out.push_str(&format!("{STATA_COMMENT}{stata}\n"));
                if !loops.is_empty() {
                    out.push_str(&format!("#  {loops}\n"));
                }
                out.push_str(&r.code);
                out.push('\n');
                for c in &r.caveats {
                    out.push_str(&format!("#   note: {c}\n"));
                }
                out.push_str("```\n\n");
            }
            (Ok(r), _) => {
                let info = origin.info.clone().unwrap_or_default();
                let mut text = format!("`{}`{loops}", stata.replace('`', "'"));
                if !info.is_empty() {
                    text.push_str(&format!(": {info}"));
                }
                for c in &r.caveats {
                    text.push_str(&format!(" (note: {c})"));
                }
                out.push_str(&format!("{text}\n\n"));
            }
            (Err(e), _) => {
                out.push_str(&format!(
                    "**Not run:** `{}`{loops}: {}\n\n",
                    stata.replace('`', "'"),
                    e.reason
                ));
            }
        }
    }
    out
}

/// How the run went, line by line, with what R printed for each: the run's
/// log (P2C.6) and report (P2C.7). `ran` and `printed` go with the lines.
pub fn report(d: &DoFile, name: &str, ran: &[Ran], printed: &[String]) -> String {
    let mut counts = [0usize; 5];
    for (i, (line, origin)) in d.translation.lines.iter().zip(&d.origins).enumerate() {
        let r = ran.get(i).unwrap_or(&Ran::NotReached);
        let k = match r {
            Ran::Ok => match status(line, origin) {
                Status::Notes => 1,
                _ => 0,
            },
            Ran::Error(_) => 2,
            Ran::Skipped => 3,
            Ran::NotReached => 4,
        };
        if let Some(c) = counts.get_mut(k) {
            *c += 1;
        }
    }
    let [ok, notes, errors, skipped, not_reached] = counts;
    let mut out = format!("RCraft ran {name} on R.\n\n");
    out.push_str(&format!(
        "{} lines: {} ran ({notes} of them with a note on how R differs), {errors} stopped with an error in R, {skipped} were not run",
        d.translation.lines.len(),
        ok + notes
    ));
    if not_reached > 0 {
        out.push_str(&format!(", {not_reached} were not reached"));
    }
    out.push_str(".\n\n");
    for (i, (line, origin)) in d.translation.lines.iter().zip(&d.origins).enumerate() {
        out.push_str(&format!(". {}\n", line.stata.replace('\n', " ")));
        if !origin.loops.is_empty() {
            out.push_str(&format!("  (in: {})\n", origin.loops.join("; ")));
        }
        if let Some(info) = &origin.info {
            out.push_str(&format!("  ({info})\n"));
        }
        if let Some(p) = printed.get(i).filter(|p| !p.trim().is_empty()) {
            out.push_str(p.trim_end_matches('\n'));
            out.push('\n');
        }
        match (ran.get(i).unwrap_or(&Ran::NotReached), &line.result) {
            (Ran::Error(e), _) => out.push_str(&format!("  R stopped with an error: {e}\n")),
            (Ran::Skipped, Err(e)) => out.push_str(&format!("  not run: {}\n", e.reason)),
            (Ran::Skipped, Ok(_)) => out.push_str("  not run: skipped after a problem\n"),
            (Ran::NotReached, _) => out.push_str("  not reached\n"),
            (Ran::Ok, Ok(r)) => {
                for c in &r.caveats {
                    out.push_str(&format!("  note: {c}\n"));
                }
            }
            (Ran::Ok, Err(_)) => {}
        }
    }
    out
}
