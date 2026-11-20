//! `nlsur name @ varlist, parameters() neq()` (the function-evaluator form) in the runner;
//! `data_r::nlsur` is the translator's side of the lines it writes.
//!
//! Stata calls the program `nlsurname` at each step with `at()` the parameters and the first
//! `neq()` variables of the varlist to fill with fitted values. RCraft runs the program's lines
//! once, at the starting values and on copies of those variables (the lines run as they are made),
//! and makes the same R the function the fit calls:
//!
//! ```text
//! _rcraft_nlsur_begin <the line>        (the rows used, as __nlsur_touse; the matrix __nlsur_at)
//! quietly generate double __nlsur_y1 = y1  ...  (the copies)
//! _rcraft_nlsur_mark
//! _rcraft_nlsur_at                      (inside the function: at() from its argument)
//! quietly nlsurname __nlsur_y1 ... x1 ... if __nlsur_touse, at(__nlsur_at)
//! _rcraft_nlsur_collect __nlsur_y1 ...
//! _rcraft_nlsur_compile                 (the lines since the mark become stata_nlsur_fn)
//! _rcraft_nlsur_fit <the line>
//! quietly drop __nlsur_y1 ... __nlsur_touse
//! ```
//!
//! The program's lines must not need a value from R while they are translated (the function runs
//! later, at each step); when one does, or one is not translated, the fit is refused.

use super::*;

impl Run<'_> {
    pub(super) fn nlsur_eval(
        &mut self,
        c: &Command,
        rest: &str,
        depth: usize,
        written: String,
    ) -> Flow {
        self.done_item(depth);
        let refuse = |run: &mut Self, why: String| {
            let line = run.t.refused(c.span, c.text.clone(), why);
            run.push(line, What::Command, None, written.clone());
            Flow::Line
        };
        let main = c.parts.main.trim();
        let Some((name, vars)) = main.split_once('@') else {
            return refuse(
                self,
                "nlsur: the evaluator form is nlsur name @ varlist".into(),
            );
        };
        let name = name.trim();
        let prog = format!("nlsur{name}");
        if !self.programs.contains_key(&prog) {
            return refuse(
                self,
                format!("nlsur {name} @: the program {prog} is not defined in this do-file"),
            );
        }
        let neq = c
            .options
            .iter()
            .find(|o| o.is("neq", 3))
            .and_then(|o| o.arg.as_deref())
            .and_then(|a| a.trim().parse::<usize>().ok());
        let Some(neq) = neq else {
            return refuse(
                self,
                "nlsur name @: neq(#) is needed (RCraft translates it given as a number)".into(),
            );
        };
        let words: Vec<&str> = vars.split_whitespace().collect();
        if words.len() <= neq || neq == 0 || words.iter().any(|w| !macros::is_name(w)) {
            return refuse(
                self,
                "nlsur name @ varlist: the dependent variables then the others, plain names".into(),
            );
        }
        let (deps, others) = words.split_at(neq);
        let copies: Vec<String> = (1..=neq).map(|i| format!("__nlsur_y{i}")).collect();
        let mut text = format!("_rcraft_nlsur_begin {}\n", rest.trim());
        for (cp, d) in copies.iter().zip(deps) {
            text.push_str(&format!("quietly generate double {cp} = {d}\n"));
        }
        text.push_str("_rcraft_nlsur_mark\n_rcraft_nlsur_at\n");
        text.push_str(&format!(
            "quietly {prog} {} {} if __nlsur_touse, at(__nlsur_at)\n",
            copies.join(" "),
            others.join(" ")
        ));
        text.push_str(&format!("_rcraft_nlsur_collect {}\n", copies.join(" ")));
        text.push_str("_rcraft_nlsur_compile\n");
        text.push_str(&format!("_rcraft_nlsur_fit {}\n", rest.trim()));
        text.push_str(&format!(
            "quietly drop {} __nlsur_touse\n",
            copies.join(" ")
        ));
        let mut items = reader::read(&text).items;
        for item in &mut items {
            if let Item::Command(cmd) = item {
                cmd.span = c.span;
            }
        }
        self.nl_failed = None;
        self.frames.push(Frame {
            items: Rc::new(items),
            at: 0,
            kind: Kind::Body,
            prefixes: c.prefixes.clone(),
        });
        Flow::Next
    }

    /// `_rcraft_nlsur_mark`: where the evaluator's lines start, and the asks by then.
    pub(super) fn nlsur_mark(&mut self, depth: usize) -> Flow {
        self.done_item(depth);
        self.nl_mark = Some((self.lines.len(), self.asks));
        Flow::Next
    }

    /// `_rcraft_nlsur_compile`: the lines since the mark become the function the fit calls.
    pub(super) fn nlsur_compile(&mut self, c: &Command, depth: usize, written: String) -> Flow {
        self.done_item(depth);
        let Some((l0, a0)) = self.nl_mark.take() else {
            self.nl_failed = Some("nlsur: the evaluator's lines were not found".into());
            return Flow::Next;
        };
        let lines = self.lines.get(l0..).unwrap_or_default();
        if let Some(bad) = lines.iter().find_map(|l| l.result.as_ref().err()) {
            self.nl_failed = Some(format!(
                "nlsur's evaluator program has a line that is not translated: {}",
                bad.reason
            ));
            return Flow::Next;
        }
        if self.asks != a0 {
            self.nl_failed = Some(
                "nlsur's evaluator program needs values from R while it is translated, which a function run at each step cannot have".into(),
            );
            return Flow::Next;
        }
        let body: Vec<String> = lines
            .iter()
            .filter_map(|l| l.result.as_ref().ok())
            .filter(|r| !r.code.is_empty())
            .map(|r| r.code.clone())
            .collect();
        let packages: Vec<&'static str> = lines
            .iter()
            .filter_map(|l| l.result.as_ref().ok())
            .flat_map(|r| r.packages.iter().copied())
            .collect();
        let line = Line {
            span: c.span,
            stata: "the evaluator program, as the function the fit calls".into(),
            result: Ok(crate::data_r::RCode {
                code: format!(
                    "stata_nlsur_fn <- function(stata_nlsur_b) {{\n{}\nstata_nlsur_out\n}}",
                    body.join("\n")
                ),
                caveats: vec![
                    "the evaluator program's lines run inside a function, once at each step of the fit, as Stata calls the program".into(),
                ],
                packages,
            }),
        };
        self.push(line, What::Command, None, written);
        Flow::Line
    }

    /// `_rcraft_nlsur_fit <the line>`: the fit, unless the evaluator was not made.
    pub(super) fn nlsur_fit(
        &mut self,
        c: &Command,
        rest: &str,
        depth: usize,
        written: String,
    ) -> Flow {
        self.done_item(depth);
        let shown = format!("nlsur {}", rest.trim());
        let mut line = match self.nl_failed.take() {
            Some(why) => self.t.refused(c.span, shown.clone(), why),
            None => self.t.command(c),
        };
        line.stata = shown;
        self.push(line, What::Command, None, written);
        Flow::Line
    }
}
