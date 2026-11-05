//! Programs that estimate (Step 3): `ereturn post`, `scalar`, `local`,
//! `matrix`, `clear` and `display`, as Stata 19 runs them.
//!
//! Observed in Stata 19 (`plan/stata/eclass.md`): `post` and `clear` work
//! anywhere, the others only in an e-class program (r(152) elsewhere);
//! `post` and `matrix` move the matrices they are given unless `copy`;
//! `ereturn list` keeps the first 20 scalars in the order set and puts later
//! ones first, macros newest first, and e(b) and e(V) ahead of the other
//! matrices, newest first (StatCraft's probe 147); an empty macro is removed.

use super::returned::Returned;
use super::{first_word, is_name, no, Out, Session};
use crate::expr_r::{r_text, Kind, Untranslated};
use crate::reader::Command;

/// `ereturn post`, the order `ereturn list` keeps, `_b[]` and `_se[]` of
/// posted results, and `ereturn display`.
pub(super) const ECLASS_HELPER: &str = r##"stata_epost <- function(b = NULL, V = NULL, depname = NULL, obs = NULL, dof = NULL,
                        properties = NULL, sample = FALSE) {
  e <- list()
  if (!is.null(b)) {
    if (!is.matrix(b) || nrow(b) != 1) stop("conformability error (Stata: r(503))", call. = FALSE)
    if (!is.null(V)) {
      if (!is.matrix(V) || nrow(V) != ncol(b) || ncol(V) != ncol(b)) {
        stop("conformability error (Stata: r(503))", call. = FALSE)
      }
      if (!identical(rownames(V), colnames(b)) || !identical(colnames(V), colnames(b))) {
        stop("name conflict (Stata: r(507))", call. = FALSE)
      }
      if (!isSymmetric(unname(V))) {
        stop("RCraft: a variance matrix that is not symmetric (Stata posts it with a warning) is not translated yet", call. = FALSE)
      }
    }
    rownames(b) <- "y1"
  }
  if (!is.null(obs)) e$N <- as.numeric(obs)
  if (!is.null(dof)) e$df_r <- as.numeric(dof)
  props <- if (!is.null(properties)) properties else if (is.null(b)) "" else if (is.null(V)) "b" else "b V"
  if (props != "") e$properties <- props
  if (!is.null(depname)) e$depvar <- depname
  if (!is.null(b)) e$b <- b
  if (!is.null(V)) e$V <- V
  attr(e, "stata_sample") <- sample
  e
}
stata_eput <- function(e, name, value) {
  if (is.null(e)) e <- structure(list(), stata_sample = FALSE)
  smp <- attr(e, "stata_sample")
  e[[name]] <- NULL
  kind <- vapply(e, function(v) if (is.matrix(v)) "m" else if (is.character(v)) "t" else "n", "")
  num <- e[kind == "n"]
  txt <- e[kind == "t"]
  mat <- e[kind == "m"]
  one <- stats::setNames(list(value), name)
  if (is.character(value)) {
    # newest first; an empty macro is removed
    if (!identical(value, "")) txt <- c(one, txt)
  } else if (is.matrix(value)) {
    # e(b) and e(V) first, the others newest first
    lead <- sum(cumprod(names(mat) %in% c("b", "V")))
    at <- if (name == "b") 0 else if (name == "V") as.integer(length(mat) > 0 && names(mat)[1] == "b") else lead
    mat <- append(mat, one, after = at)
  } else {
    # in order among the first 20, newest first after them
    num <- append(num, one, after = min(length(num), 20))
  }
  out <- c(num, txt, mat)
  attr(out, "stata_sample") <- smp
  out
}
stata_bpost <- function(key, se = FALSE) {
  b <- if (exists("stata_e")) stata_e[["b"]] else NULL
  if (is.null(b)) stop("_b not allowed when e(b) is not present", call. = FALSE)
  nm <- colnames(b)
  i <- match(key, nm)
  # a name without an equation is the first equation's
  if (is.na(i) && !grepl(":", key, fixed = TRUE) && grepl(":", nm[1], fixed = TRUE)) {
    i <- match(paste0(sub(":.*$", "", nm[1]), ":", key), nm)
  }
  if (is.na(i)) stop("[", key, "] not found (Stata: r(111))", call. = FALSE)
  if (!se) return(unname(b[1, i]))
  V <- stata_e[["V"]]
  if (is.null(V)) stop("_se not allowed when e(V) is not present (Stata: r(111))", call. = FALSE)
  sqrt(V[i, i])
}
stata_epost_display <- function(level = 0.95) {
  e <- if (exists("stata_e")) stata_e else NULL
  b <- e[["b"]]
  if (is.null(b)) stop("last estimates not found (Stata: r(301))", call. = FALSE)
  nm <- colnames(b)
  if (any(grepl("[.#]", sub("^[^:]*:", "", nm)))) {
    stop("RCraft: ereturn display of factor variables and interactions is not translated yet", call. = FALSE)
  }
  pad <- stata_tab_pad
  V <- e[["V"]]
  df <- e[["df_r"]]
  tdist <- !is.null(df) && !is.na(df)
  dep <- if (is.null(e[["depvar"]])) "" else e[["depvar"]]
  rule <- paste0(strrep("-", 13), "+", strrep("-", 64), "\n")
  cat(strrep("-", 78), "\n", sep = "")
  if (is.null(V)) {
    cat(pad(stata_tab_abbrev(dep), 12), " | Coefficient\n", sep = "")
  } else {
    cat(pad(stata_tab_abbrev(dep), 12), " | Coefficient  Std. err.      ", if (tdist) "t    P>|t|" else "z    P>|z|",
        "     [", format(100 * level), "% conf. interval]\n", sep = "")
  }
  cat(rule)
  eqs <- ifelse(grepl(":", nm, fixed = TRUE), sub(":.*$", "", nm), "")
  q <- if (tdist) stats::qt((1 + level) / 2, df) else stats::qnorm((1 + level) / 2)
  for (i in seq_along(nm)) {
    # an equation's name above its rows, a rule between equations
    if (eqs[i] != "" && (i == 1 || eqs[i] != eqs[i - 1])) {
      if (i > 1) cat(rule)
      cat(formatC(stata_tab_abbrev(eqs[i]), width = -12), " |\n", sep = "")
    }
    label <- pad(stata_tab_abbrev(sub("^[^:]*:", "", nm[i])), 12)
    bi <- b[1, i]
    if (is.null(V)) {
      cat(label, " |", pad(stata_tab_g(bi), 11), "\n", sep = "")
      next
    }
    si <- sqrt(V[i, i])
    z <- bi / si
    p <- if (tdist) 2 * stats::pt(-abs(z), df) else 2 * stats::pnorm(-abs(z))
    cat(label, " |", pad(stata_tab_g(bi), 11), pad(stata_tab_g(si), 11), pad(sprintf("%.2f", z), 9),
        pad(sprintf("%.3f", p), 8), pad(stata_tab_g(bi - q * si), 13), pad(stata_tab_g(bi + q * si), 12), "\n", sep = "")
  }
  cat(strrep("-", 78), "\n", sep = "")
  invisible(NULL)
}"##;

/// The R that stops where Stata stops with `message`, r(`rc`).
fn stops(message: &str, rc: u32) -> String {
    format!(
        "stop({}, call. = FALSE)",
        r_text(&format!("{message} (Stata: r({rc}))"))
    )
}

/// Whether `word` is `full` or a short form of it at least `min` long.
fn abbrev(word: &str, full: &str, min: usize) -> bool {
    word.len() >= min && full.starts_with(word)
}

impl Session {
    /// `ereturn subcommand ...`.
    pub(super) fn ereturn(&mut self, c: &Command, out: &mut Out) -> Result<(), Untranslated> {
        let (sub, rest) = first_word(&c.parts.main);
        if abbrev(sub, "list", 2) {
            return self.list_returned(c, out);
        }
        if sub == "post" {
            return self.ereturn_post(c, rest, out);
        }
        if sub == "clear" {
            self.posted();
            self.helpers.insert("stata_returned");
            out.line("stata_e <- structure(list(), stata_sample = FALSE)");
            self.e_ret = Returned::Program {
                cmd: self.poster(),
                texts: Vec::new(),
            };
            return Ok(());
        }
        if abbrev(sub, "display", 2) {
            return self.ereturn_display(c, out);
        }
        let scalar = abbrev(sub, "scalar", 3);
        let local = abbrev(sub, "local", 3);
        let matrix = abbrev(sub, "matrix", 3);
        if !(scalar || local || matrix) {
            return Err(no(format!("ereturn {sub} is not translated yet")));
        }
        // only an e-class program sets e() this way
        if !self.programs.last().is_some_and(|p| p.class == "eclass") {
            out.line(stops("non e-class program may not set e()", 152));
            return Ok(());
        }
        let (name, value, text) = if matrix {
            self.ereturn_matrix(c, rest, out)?
        } else {
            let (name, exp) = match rest.split_once('=') {
                Some((n, e)) if is_name(n.trim()) => (n.trim().to_string(), Some(e.trim())),
                _ => (first_word(rest).0.to_string(), None),
            };
            if !is_name(&name) {
                return Err(no(format!("ereturn {sub} needs a name")));
            }
            match exp {
                Some(e) => {
                    let (v, kind) = self.value(e, out)?;
                    match (scalar, kind == Kind::Text) {
                        (true, true) => {
                            out.line(stops("type mismatch", 109));
                            return Ok(());
                        }
                        (true, false) => (name, v, false),
                        // a number kept as a macro is its %18.0g
                        (false, false) => {
                            self.helpers.insert("stata_fmt");
                            (
                                name,
                                format!("stata_fmt({v}, \"%18.0g\", trim = TRUE)"),
                                true,
                            )
                        }
                        (false, true) => (name, v, true),
                    }
                }
                None if local => {
                    // the text as written, its outer quotes off
                    let t = first_word(rest).1.trim();
                    let t = t
                        .strip_prefix("`\"")
                        .and_then(|s| s.strip_suffix("\"'"))
                        .or_else(|| t.strip_prefix('"').and_then(|s| s.strip_suffix('"')))
                        .unwrap_or(t);
                    (name, r_text(t), true)
                }
                None => return Err(no("ereturn scalar needs name = exp")),
            }
        };
        self.helpers.insert("stata_eclass");
        self.helpers.insert("stata_returned");
        out.line(format!(
            "stata_e <- stata_eput(if (exists(\"stata_e\")) stata_e else NULL, {}, {value})",
            r_text(&name)
        ));
        match &mut self.e_ret {
            Returned::Program { texts, .. } => {
                texts.retain(|t| t != &name);
                if text {
                    texts.push(name);
                }
            }
            Returned::Kept { .. } => {
                if text {
                    self.e_text.insert(name);
                } else {
                    self.e_text.remove(&name);
                }
            }
            // a fresh do-file holds no e(): this is all of it
            Returned::Nothing => {
                self.e_ret = Returned::Program {
                    cmd: self.poster(),
                    texts: if text { vec![name] } else { Vec::new() },
                };
            }
            // what else e() holds is still not known
            Returned::Lost(_) => {}
        }
        Ok(())
    }

    /// A matrix moved into e(): gone, or under capture (the line may have
    /// stopped before it moved) gone only if R says so.
    fn moved(&mut self, c: &Command, name: &str) {
        if c.prefixes.contains(&crate::reader::Prefix::Capture) {
            self.maybe_moved.insert(name.to_string());
        } else {
            self.user_mats.remove(name);
        }
    }

    /// The name e() posted here is said to come from: the program, or
    /// `ereturn` itself.
    fn poster(&self) -> String {
        self.programs
            .last()
            .map(|p| p.name.clone())
            .unwrap_or_else(|| "ereturn".into())
    }

    /// New results: the model before them is gone for `predict` and the rest.
    fn posted(&mut self) {
        self.estimates = None;
        self.estimates_lost = Some(
            "e() was posted by ereturn, and predict, test, lincom and margins are not translated after posted results yet"
                .into(),
        );
        self.active_store = None;
        self.e_text.clear();
        self.esample = false;
    }

    /// `ereturn post [b [V]] [, depname() obs() dof() esample() properties()]`.
    fn ereturn_post(&mut self, c: &Command, rest: &str, out: &mut Out) -> Result<(), Untranslated> {
        let opts = super::Opts(&c.options);
        if let Some(o) = opts.other(&[
            ("depname", 3),
            ("obs", 3),
            ("dof", 3),
            ("esample", 3),
            ("properties", 5),
        ]) {
            return Err(no(format!(
                "ereturn post with the option {o} is not translated yet"
            )));
        }
        let names: Vec<&str> = rest.split_whitespace().collect();
        if names.len() > 2 || names.iter().any(|n| !is_name(n)) {
            return Err(no("ereturn post takes a b matrix and a V matrix"));
        }
        let mut args = Vec::new();
        for (n, arg) in names.iter().zip(["b", "V"]) {
            args.push(format!("{arg} = {}", self.user_matrix(n)?));
        }
        if let Some(d) = opts.arg("depname", 3) {
            args.push(format!("depname = {}", r_text(d.trim())));
        }
        for (opt, arg) in [("obs", "obs"), ("dof", "dof")] {
            if let Some(v) = opts.arg(opt, 3) {
                let (v, kind) = self.value(v.trim(), out)?;
                if kind == Kind::Text {
                    return Err(no(format!(
                        "ereturn post, {opt}() of text (Stata: invalid syntax)"
                    )));
                }
                args.push(format!("{arg} = {v}"));
            }
        }
        if let Some(p) = opts.arg("properties", 5) {
            args.push(format!("properties = {}", r_text(p.trim())));
        }
        let esample = opts.arg("esample", 3).map(|v| v.trim().to_string());
        if let Some(v) = &esample {
            if !is_name(v) {
                return Err(no("ereturn post, esample() takes a variable"));
            }
            args.push("sample = TRUE".into());
        }
        self.posted();
        self.helpers.insert("stata_eclass");
        self.helpers.insert("stata_returned");
        out.line(format!("stata_e <- stata_epost({})", args.join(", ")));
        // e(sample): the variable's nonzero rows, and it leaves the data;
        // without esample() no row is in it
        if let Some(d) = self.data.clone() {
            match &esample {
                Some(v) => {
                    let v_r = r_text(v);
                    out.line(format!(
                        "if (!{v_r} %in% names({d})) {}",
                        stops(&format!("variable {v} not found"), 111)
                    ));
                    out.line(format!(
                        "stata_esample <- local({{ v <- {d}[[{v_r}]]; is.na(v) | v != 0 }})"
                    ));
                    out.line(format!("{d} <- {d}[setdiff(names({d}), {v_r})]"));
                    self.kinds.remove(v);
                }
                None => out.line(format!("stata_esample <- rep(FALSE, nrow({d}))")),
            }
            self.esample = true;
        } else if esample.is_some() {
            return Err(no("ereturn post, esample(): no data is in memory here"));
        }
        // the matrices are moved into e()
        for n in &names {
            out.line(format!("stata_mat[[{}]] <- NULL", r_text(n)));
            self.moved(c, n);
        }
        let mut texts = vec!["properties".to_string()];
        if opts.has("depname", 3) {
            texts.push("depvar".into());
        }
        self.e_ret = Returned::Program {
            cmd: self.poster(),
            texts,
        };
        Ok(())
    }

    /// `ereturn matrix name [=] matname [, copy]`: the name, its R and that
    /// it is not text.
    fn ereturn_matrix(
        &mut self,
        c: &Command,
        rest: &str,
        out: &mut Out,
    ) -> Result<(String, String, bool), Untranslated> {
        let opts = super::Opts(&c.options);
        if let Some(o) = opts.other(&[("copy", 4)]) {
            return Err(no(format!(
                "ereturn matrix with the option {o} is not translated yet"
            )));
        }
        let words: Vec<&str> = rest
            .split(|ch: char| ch.is_whitespace() || ch == '=')
            .filter(|w| !w.is_empty())
            .collect();
        let [name, from] = words.as_slice() else {
            return Err(no(
                "ereturn matrix needs name = matname (Stata: invalid syntax)",
            ));
        };
        if !is_name(name) || !is_name(from) {
            return Err(no("ereturn matrix needs name = matname"));
        }
        let m = self.user_matrix(from)?;
        if !opts.has("copy", 4) {
            // moved: the do-file's matrix is gone after the line
            self.moved(c, from);
            out.line(format!("stata_emove <- {m}"));
            out.line(format!("stata_mat[[{}]] <- NULL", r_text(from)));
            return Ok((name.to_string(), "stata_emove".into(), false));
        }
        Ok((name.to_string(), m, false))
    }

    /// `ereturn display [, level(#)]`: the coefficient table of e(b) and e(V).
    fn ereturn_display(&mut self, c: &Command, out: &mut Out) -> Result<(), Untranslated> {
        let opts = super::Opts(&c.options);
        if let Some(o) = opts.other(&[("level", 1)]) {
            return Err(no(format!(
                "ereturn display with the option {o} is not translated yet"
            )));
        }
        match &self.e_ret {
            Returned::Nothing => {
                return Err(no(
                    "ereturn display: nothing before it in this do-file leaves e()",
                ))
            }
            Returned::Lost(why) => return Err(no(format!("ereturn display: {why}"))),
            Returned::Program { .. } | Returned::Kept { .. } => {}
        }
        let level = match opts.arg("level", 1) {
            Some(l) => {
                let x: f64 = l
                    .trim()
                    .parse()
                    .map_err(|_| no("ereturn display, level() takes a number"))?;
                if !(10.0..=99.99).contains(&x) {
                    return Err(no("level() must be between 10 and 99.99 (Stata: r(198))"));
                }
                x / 100.0
            }
            None => 0.95,
        };
        self.helpers.insert("stata_eclass");
        self.helpers.insert("stata_tables");
        self.helpers.insert("stata_fmt");
        out.line(format!("stata_epost_display({level})"));
        // it leaves r(table), which is not kept
        self.r_ret = Returned::Lost(
            "ereturn display leaves r(table), which RCraft does not keep yet".into(),
        );
        Ok(())
    }
}
