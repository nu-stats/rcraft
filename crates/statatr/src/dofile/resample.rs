//! `permute` and `simulate` in the runner: written out as the lines Stata runs,
//! one replication after another (`data_r::permute_run` is the translator's side
//! of the lines between them).
//!
//! `permute x exp, reps(3): cmd` becomes
//!
//! ```text
//! _rcraft_pm_begin builtin|program <the line>
//! quietly cmd                 (the unpermuted data)
//! _rcraft_pm_collect
//! _rcraft_pm_shuffle          (and again for each replication)
//! quietly cmd
//! _rcraft_pm_collect
//! _rcraft_pm_end
//! ```
//!
//! and `simulate exp, reps(3): prog` becomes `_rcraft_sm_begin`, then for each
//! replication `quietly prog` and `_rcraft_sm_collect`, then `_rcraft_sm_end`.

use super::*;

/// Most replications written out: each is lines of R.
const MOST_REPS: usize = 2000;

impl Run<'_> {
    pub(super) fn resample_loop(
        &mut self,
        c: &Command,
        rest: &str,
        depth: usize,
        written: String,
    ) -> Flow {
        self.done_item(depth);
        let permute = c.name == "permute";
        match crate::data_r::resample_plan(&c.name, rest) {
            Err((code, msg)) => {
                let mut fail = self.internal(c.span, &format!("_rcraft_rs_fail {code} {msg}"));
                fail.prefixes = c.prefixes.clone();
                let mut line = self.t.command(&fail);
                line.stata = c.text.clone();
                self.push(line, What::Command, None, written);
                Flow::Line
            }
            Ok(plan) if plan.reps > MOST_REPS => {
                let line = self.t.refused(
                    c.span,
                    c.text.clone(),
                    format!(
                        "{} with more than {MOST_REPS} replications is not translated: each one is written out as lines of R",
                        c.name
                    ),
                );
                self.push(line, What::Command, None, written);
                Flow::Line
            }
            Ok(plan) => {
                let word = plan.inner.split_whitespace().next().unwrap_or("");
                let kind = match self.programs.get(word) {
                    Some(def) if def.class == "eclass" => "eprogram",
                    Some(_) => "program",
                    None => "builtin",
                };
                let tag = if permute { "pm" } else { "sm" };
                let mut text = format!("_rcraft_{tag}_begin {kind} {}\n", rest.trim());
                let inner = format!("quietly {}\n", plan.inner.trim());
                if permute {
                    text.push_str(&inner);
                    text.push_str("_rcraft_pm_collect\n");
                    for _ in 0..plan.reps {
                        text.push_str("_rcraft_pm_shuffle\n");
                        text.push_str(&inner);
                        text.push_str("_rcraft_pm_collect\n");
                    }
                    text.push_str("_rcraft_pm_end\n");
                } else {
                    for _ in 0..plan.reps {
                        text.push_str(&inner);
                        text.push_str("_rcraft_sm_collect\n");
                    }
                    text.push_str("_rcraft_sm_end\n");
                }
                let program = reader::read(&text);
                let mut items = program.items;
                // every line stands for the one the do-file has
                for item in &mut items {
                    if let Item::Command(cmd) = item {
                        cmd.span = c.span;
                    }
                }
                self.frames.push(Frame {
                    items: Rc::new(items),
                    at: 0,
                    kind: Kind::Body,
                    prefixes: c.prefixes.clone(),
                });
                Flow::Next
            }
        }
    }
}
