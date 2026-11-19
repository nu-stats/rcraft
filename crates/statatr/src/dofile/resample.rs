//! `permute` and `simulate` in the runner (`data_r::permute_run` is the
//! translator's side of the lines it writes).
//!
//! A built-in command after the colon is one line (`_rcraft_pm_loop`,
//! `_rcraft_sm_loop`): the translator puts its replications in one R loop.
//! A program the do-file defines is run as Stata runs it, line by line:
//!
//! ```text
//! _rcraft_pm_begin program <the line>
//! quietly prog                (the data as they are; permute only)
//! _rcraft_pm_collect
//! shuffle, prog, collect      (a first replication)
//! _rcraft_rs_mark             (then one replication: shuffle, prog, collect)
//! _rcraft_rs_mark             (and a second one)
//! _rcraft_rs_compile <n>      (the other replications)
//! _rcraft_pm_end
//! ```
//!
//! When the two replications became the same R and the runner did not ask R
//! for a value on the way (a program with no `if` or loop that depends on
//! the data), the other replications are that R in one loop; otherwise they
//! are written out the same way, up to [`MOST_REPS`]. Each replication's lines
//! end at an error, which leaves its results missing, as in Stata.

use super::*;

/// Most replications written out line by line.
const MOST_REPS: usize = 2000;

/// The R of the lines, with temporary names (`__000012`) made alike, since
/// each run of a program has its own.
fn alike(lines: &[Line]) -> Option<Vec<String>> {
    lines
        .iter()
        .map(|l| {
            l.result.as_ref().ok().map(|r| {
                let mut out = String::new();
                let b = r.code.as_bytes();
                let mut i = 0;
                while let Some(&ch) = b.get(i) {
                    let temp = ch == b'_'
                        && b.get(i + 1) == Some(&b'_')
                        && (i + 2..i + 8).all(|k| b.get(k).is_some_and(u8::is_ascii_digit));
                    if temp {
                        out.push_str("__TEMP");
                        i += 8;
                    } else {
                        out.push(char::from(ch));
                        i += 1;
                    }
                }
                out
            })
        })
        .collect()
}

impl Run<'_> {
    /// The lines of `text`, each standing for the line `c` of the do-file; the
    /// runs of the command are marked as replications.
    fn resample_items(&self, text: &str, c: &Command, first_is_plain: bool) -> Vec<Item> {
        let mut items = reader::read(text).items;
        let mut runs = 0;
        for item in &mut items {
            if let Item::Command(cmd) = item {
                cmd.span = c.span;
                if !cmd.name.starts_with("_rcraft_") {
                    runs += 1;
                    if !(first_is_plain && runs == 1) {
                        cmd.prefixes.push(Prefix::Replication);
                    }
                }
            }
        }
        items
    }

    pub(super) fn resample_loop(
        &mut self,
        c: &Command,
        rest: &str,
        depth: usize,
        written: String,
    ) -> Flow {
        self.done_item(depth);
        let permute = c.name == "permute";
        let tag = if permute { "pm" } else { "sm" };
        let plan = match crate::data_r::resample_plan(&c.name, rest) {
            Err((code, msg)) => {
                let mut fail = self.internal(c.span, &format!("_rcraft_rs_fail {code} {msg}"));
                fail.prefixes = c.prefixes.clone();
                let mut line = self.t.command(&fail);
                line.stata = c.text.clone();
                self.push(line, What::Command, None, written);
                return Flow::Line;
            }
            Ok(plan) => plan,
        };
        let word = plan.inner.split_whitespace().next().unwrap_or("");
        let kind = match self.programs.get(word) {
            Some(def) if def.class == "eclass" => "eprogram",
            Some(_) => "program",
            None => "builtin",
        };
        // a built-in command: one line, its replications in one R loop
        if kind == "builtin" {
            let mut cmd = self.internal(
                c.span,
                &format!("_rcraft_{tag}_loop builtin {}", rest.trim()),
            );
            cmd.prefixes = c.prefixes.clone();
            let mut line = self.t.command(&cmd);
            line.stata = c.text.clone();
            self.push(line, What::Command, None, written);
            return Flow::Line;
        }
        let inner = format!("quietly {}\n", plan.inner.trim());
        let unit = if permute {
            format!("_rcraft_pm_shuffle\n{inner}_rcraft_pm_collect\n")
        } else {
            format!("{inner}_rcraft_sm_collect\n")
        };
        let mut text = format!("_rcraft_{tag}_begin {kind} {}\n", rest.trim());
        if permute {
            text.push_str(&inner);
            text.push_str("_rcraft_pm_collect\n");
        }
        // the first replication may start from other data (none yet): the second
        // and third are compared
        if plan.reps <= 3 {
            for _ in 0..plan.reps {
                text.push_str(&unit);
            }
        } else {
            text.push_str(&unit);
            text.push_str("_rcraft_rs_mark\n");
            text.push_str(&unit);
            text.push_str("_rcraft_rs_mark\n");
            text.push_str(&unit);
            text.push_str(&format!("_rcraft_rs_compile {}\n", plan.reps - 3));
            self.rs_unit = Some(unit);
            self.rs_marks.clear();
        }
        text.push_str(&format!("_rcraft_{tag}_end\n"));
        let items = self.resample_items(&text, c, permute);
        self.frames.push(Frame {
            items: Rc::new(items),
            at: 0,
            kind: Kind::Body,
            prefixes: c.prefixes.clone(),
        });
        Flow::Next
    }

    /// `_rcraft_rs_mark`: where a replication starts, and how many values the
    /// runner had asked R for by then.
    pub(super) fn resample_mark(&mut self, depth: usize) -> Flow {
        self.done_item(depth);
        self.rs_marks.push((self.lines.len(), self.asks));
        Flow::Next
    }

    /// `_rcraft_rs_compile n`: the other `n` replications, as one R loop when
    /// the two before were the same R and needed no value from R; else line
    /// by line.
    pub(super) fn resample_compile(
        &mut self,
        c: &Command,
        rest: &str,
        depth: usize,
        written: String,
    ) -> Flow {
        self.done_item(depth);
        let n: usize = rest.trim().parse().unwrap_or(0);
        let marks = std::mem::take(&mut self.rs_marks);
        let unit = self.rs_unit.take().unwrap_or_default();
        let (Some(&(l0, a0)), Some(&(l1, _))) = (marks.first(), marks.get(1)) else {
            return Flow::Next;
        };
        let l2 = self.lines.len();
        let first = self.lines.get(l0..l1).and_then(alike);
        let second = self.lines.get(l1..l2).and_then(alike);
        if self.asks == a0 && first.is_some() && first == second && n > 0 {
            let body: Vec<String> = self
                .lines
                .get(l1..l2)
                .unwrap_or_default()
                .iter()
                .filter_map(|l| l.result.as_ref().ok())
                .filter(|r| !r.code.is_empty())
                .map(|r| r.code.clone())
                .collect();
            let line = Line {
                span: c.span,
                stata: format!("the other {n} replications"),
                result: Ok(crate::data_r::RCode {
                    code: format!("for (stata_rs_i in seq_len({n})) {{\n{}\n}}", body.join("\n")),
                    caveats: vec![
                        "each replication's R is the same as the one before, so the others run in one loop".into(),
                    ],
                    packages: Vec::new(),
                }),
            };
            self.push(line, What::Command, None, written);
            return Flow::Line;
        }
        if n + 3 > MOST_REPS {
            // the end of the line is not reached: nothing after it is known
            if let Some(top) = self.frames.get_mut(depth) {
                top.at = top.items.len();
            }
            let line = self.t.refused(
                c.span,
                c.text.clone(),
                format!(
                    "more than {MOST_REPS} replications of a program whose R differs from one replication to the next (it asks R for values) are not translated: each one is written out as lines of R"
                ),
            );
            self.push(line, What::Command, None, written);
            return Flow::Line;
        }
        let text = unit.repeat(n);
        let items = self.resample_items(&text, c, false);
        let prefixes = self
            .frames
            .get(depth)
            .map(|f| f.prefixes.clone())
            .unwrap_or_default();
        self.frames.push(Frame {
            items: Rc::new(items),
            at: 0,
            kind: Kind::Body,
            prefixes,
        });
        Flow::Next
    }
}
