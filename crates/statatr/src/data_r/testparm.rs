//! `testparm varlist [, equal]`: the joint test that the coefficients the
//! varlist names are zero (or, with `equal`, equal), as Stata 19 does it
//! after a model (`plan/stata/testparm.md`).
//!
//! The varlist may hold factor variables (`i.g`), interactions (`i.g#c.x`,
//! `i.g##i.h`) and wildcards (`x*`). Each names the model's coefficients
//! whose terms it matches, in either order, base levels left out, and the
//! test is `test`'s (`stata_test`), so its lines and r() are the same: the
//! coefficients in e(b)'s order, labelled as Stata labels them (`[y]x`
//! after the likelihood models).

use super::model::Fit;
use super::*;

pub(super) const TESTPARM_HELPER: &str = r##"stata_testparm <- function(terms, equal = FALSE, eq = "", data_names = character(0), ret = FALSE) {
  st <- stata_bnames()
  rn <- names(stata_b)[seq_along(st)]
  # each coefficient's parts as kind:variable (f a factor's level, c a number), and its label
  parts_of <- function(n) {
    out <- list()
    for (p in strsplit(n, ":", fixed = TRUE)[[1]]) {
      m <- regmatches(p, regexec("^stata_cells\\(([^,]+), ([^,]+), \"[^\"]*\", \"[^\"]*\"\\)([^#]+)#(.+)$", p))[[1]]
      if (length(m)) {
        out[[length(out) + 1]] <- c("f", m[2], m[4])
        out[[length(out) + 1]] <- c("f", m[3], m[5])
        next
      }
      m <- regmatches(p, regexec("^(factor|stata_ibn)\\(([^()]+)\\)(.+)$", p))[[1]]
      if (length(m)) { out[[length(out) + 1]] <- c("f", m[3], m[4]); next }
      m <- regmatches(p, regexec("^stata_ib\\(([^,]+), \"[^\"]*\"\\)(.+)$", p))[[1]]
      if (length(m)) { out[[length(out) + 1]] <- c("f", m[2], m[3]); next }
      m <- regmatches(p, regexec("^I\\(([^()^]+)\\^([0-9]+)\\)$", p))[[1]]
      if (length(m)) {
        for (k in seq_len(as.integer(m[3]))) out[[length(out) + 1]] <- c("c", m[2], "")
        next
      }
      out[[length(out) + 1]] <- c(if (p == "(Intercept)") "_" else "c", p, "")
    }
    out
  }
  coefs <- lapply(rn, parts_of)
  label <- function(ps) {
    one <- length(ps) == 1
    paste(vapply(ps, function(p) if (p[1] == "f") paste0(p[3], ".", p[2]) else if (one) p[2] else paste0("c.", p[2]), ""),
          collapse = "#")
  }
  glob <- function(s) grepl("[*?]", s)
  like <- function(v, pat) if (glob(pat)) grepl(utils::glob2rx(pat), v) else v == pat
  for (t in terms) for (p in t) {
    v <- sub("^[fc]:", "", p)
    found <- if (glob(v)) any(grepl(utils::glob2rx(v), data_names)) else v %in% data_names
    if (!found) stop("variable ", v, " not found (Stata: r(111))", call. = FALSE)
  }
  # a term matches a coefficient with as many parts, each matched once
  fits <- function(ps, t) {
    if (length(ps) != length(t)) return(FALSE)
    used <- rep(FALSE, length(ps))
    for (p in t) {
      k <- substr(p, 1, 1)
      v <- substring(p, 3)
      j <- which(!used & vapply(ps, function(q) q[1] == k && like(q[2], v), TRUE))
      if (!length(j)) return(FALSE)
      used[j[1]] <- TRUE
    }
    TRUE
  }
  hit <- which(vapply(coefs, function(ps) any(vapply(terms, function(t) fits(ps, t), TRUE)), TRUE))
  if (!length(hit)) stop("no such variables;\nthe specified varlist does not identify any testable coefficients (Stata: r(111))", call. = FALSE)
  if (equal && length(hit) < 2) stop("too few variables specified (Stata: r(102))", call. = FALSE)
  pre <- if (nzchar(eq)) paste0("[", eq, "]") else ""
  lab <- paste0(pre, vapply(coefs[hit], label, ""))
  if (equal) {
    rows <- lapply(hit[-1], function(i) stats::setNames(c(-1, 1), rn[c(hit[1], i)]))
    labels <- paste0("- ", lab[1], " + ", lab[-1], " = 0")
  } else {
    rows <- lapply(hit, function(i) stats::setNames(1, rn[i]))
    labels <- paste0(lab, " = 0")
  }
  stata_test(rows, rep(0, length(rows)), labels, ret = ret)
  if (ret) assign("stata_r", stata_r, envir = parent.frame())
  invisible(NULL)
}"##;

/// A word of the varlist as the terms it names: each term its parts as
/// `f:var` (a factor) or `c:var` (a number); `a##b` is a, b and a#b.
fn terms_of(word: &str) -> Result<Vec<Vec<String>>, Untranslated> {
    let full = word.contains("##");
    let interaction = word.contains('#');
    let parts: Vec<&str> = if full {
        word.split("##").collect()
    } else {
        word.split('#').collect()
    };
    let mut parsed = Vec::new();
    for p in &parts {
        parsed.push(part(p, interaction, word)?);
    }
    if !full {
        return Ok(vec![parsed]);
    }
    if parsed.len() > 8 {
        return Err(no(format!("testparm {word}: too many terms interacted")));
    }
    // every non-empty choice of the parts, in Stata's order (mains, then pairs, ...)
    let n = parsed.len();
    let mut out: Vec<Vec<String>> = Vec::new();
    for size in 1..=n {
        for mask in 1u32..(1 << n) {
            if mask.count_ones() as usize != size {
                continue;
            }
            out.push(
                (0..n)
                    .filter(|i| mask & (1 << i) != 0)
                    .filter_map(|i| parsed.get(i).cloned())
                    .collect(),
            );
        }
    }
    Ok(out)
}

fn part(p: &str, interaction: bool, word: &str) -> Result<String, Untranslated> {
    let name_ok = |v: &str| {
        let plain: String = v.chars().filter(|c| *c != '*' && *c != '?').collect();
        !v.is_empty()
            && v.chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '*' || c == '?')
            && (plain.is_empty() || !plain.starts_with(|c: char| c.is_ascii_digit()))
    };
    let (kind, var) = match p.split_once('.') {
        None => (if interaction { 'f' } else { 'c' }, p),
        Some(("c", v)) => ('c', v),
        Some((op, v))
            if op == "i"
                || op == "ibn"
                || (op.starts_with("ib") && op[2..].chars().all(|c| c.is_ascii_digit())) =>
        {
            ('f', v)
        }
        Some(_) => {
            return Err(no(format!(
                "testparm {word}: only i., ib#., ibn. and c. are translated in its varlist"
            )))
        }
    };
    if !name_ok(var) {
        return Err(no(format!(
            "testparm {word}: a list in parentheses or a level of its own is not translated yet"
        )));
    }
    Ok(format!("{kind}:{var}"))
}

impl Session {
    pub(super) fn testparm(
        &mut self,
        c: &Command,
        opts: &Opts,
        out: &mut Out,
    ) -> Result<(), Untranslated> {
        self.no_by(c, "testparm")?;
        let est = self.estimates_for("testparm")?;
        if let Some(o) = opts.other(&[("equal", 5)]) {
            return Err(no(format!(
                "testparm with the option {o} is not translated yet"
            )));
        }
        // Stata labels the coefficients with the equation after the likelihood models
        let eq = match est.fit {
            Fit::Regress | Fit::Xtreg | Fit::Ivregress | Fit::Stcox => String::new(),
            Fit::Logit
            | Fit::Probit
            | Fit::Poisson
            | Fit::Nbreg
            | Fit::Ologit
            | Fit::Oprobit
            | Fit::Clogit => est.dep.clone(),
            f => {
                return Err(no(format!(
                    "testparm after {} is not translated yet",
                    f.name()
                )))
            }
        };
        let words: Vec<&str> = c.parts.main.split_whitespace().collect();
        if words.is_empty() {
            return Err(no("testparm needs a varlist (Stata: varlist required)"));
        }
        let mut terms = Vec::new();
        for w in words {
            for t in terms_of(w)? {
                let parts: Vec<String> = t.iter().map(|p| r_text(p)).collect();
                terms.push(format!("c({})", parts.join(", ")));
            }
        }
        let d = self.data()?;
        for h in [
            "stata_testparm",
            "stata_test",
            "stata_zero_term",
            "stata_names",
        ] {
            self.helpers.insert(h);
        }
        self.r_new = Some((super::returned::R_TEST, "testparm".into()));
        let call = format!(
            "stata_testparm(list({}){}{}, data_names = names({d}), ret = TRUE)",
            terms.join(", "),
            if opts.has("equal", 5) {
                ", equal = TRUE"
            } else {
                ""
            },
            if eq.is_empty() {
                String::new()
            } else {
                format!(", eq = {}", r_text(&eq))
            }
        );
        out.line(super::returned::quietly(c, call));
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_varlist_is_its_terms() {
        let t = |w: &str| terms_of(w).map_err(|e| e.reason);
        assert_eq!(t("x1"), Ok(vec![vec!["c:x1".to_string()]]));
        assert_eq!(t("i.g"), Ok(vec![vec!["f:g".to_string()]]));
        assert_eq!(
            t("c.age#c.age"),
            Ok(vec![vec!["c:age".into(), "c:age".into()]])
        );
        assert_eq!(t("g#h"), Ok(vec![vec!["f:g".into(), "f:h".into()]]));
        assert_eq!(
            t("i.g##c.x"),
            Ok(vec![
                vec!["f:g".to_string()],
                vec!["c:x".to_string()],
                vec!["f:g".into(), "c:x".into()]
            ])
        );
        assert_eq!(t("ib2.g"), Ok(vec![vec!["f:g".to_string()]]));
        assert_eq!(t("x*"), Ok(vec![vec!["c:x*".to_string()]]));
        assert!(t("L.x").is_err());
        assert!(t("i.(g h)").is_err());
        assert!(t("2.g").is_err());
    }
}
