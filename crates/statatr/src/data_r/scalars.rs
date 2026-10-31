//! Stata's scalars (P2B.6): `scalar [define] name = exp`, `scalar list`
//! (`dir`) and `scalar drop`, and a scalar in an expression.
//!
//! In R they live in the list `stata_scalars`. As in Stata a scalar's value
//! is a number or a text, a variable in its expression means the first row,
//! and `scalar list` shows the newest first, at `%10.0g`; a scalar redefined
//! keeps its place unless it changes between number and text, which makes it
//! the newest (read off Stata 19). A name in an expression is a variable
//! before it is a scalar, as in Stata: the translator knows only the
//! variables the do-file made itself, so before a command that reads a
//! scalar by its bare name R checks that the data has no variable of that
//! name, and stops saying so if it has (`scalar(name)` always means the
//! scalar).

use super::{no, Out, Session};
use crate::expr_r::{r_text, Kind, Untranslated};
use crate::reader::Command;

pub(super) const SCALAR_HELPER: &str = r##"stata_scalars <- list()
stata_scalar <- function(name) {
  v <- stata_scalars[[name]]
  if (is.null(v)) stop(name, " not found")
  v
}
# Stata takes a variable before a scalar of the same name
stata_scalar_clash <- function(d, names) {
  for (n in names) {
    if (n %in% names(d)) {
      stop(n, " is a variable as well as a scalar, and Stata takes the variable there; write scalar(", n,
           ") for the scalar")
    }
  }
  invisible(NULL)
}
stata_scalar_set <- function(name, value) {
  if (is.logical(value)) value <- as.numeric(value)
  if (length(value) != 1) value <- if (is.character(value)) "" else NA_real_
  old <- stata_scalars[[name]]
  # a change between number and text makes it the newest, as in Stata
  if (!is.null(old) && is.character(old) != is.character(value)) stata_scalars[[name]] <<- NULL
  stata_scalars[[name]] <<- value
  invisible(value)
}
stata_scalar_list <- function(names = NULL) {
  shown <- if (is.null(names)) rev(names(stata_scalars)) else names
  for (n in shown) {
    v <- stata_scalars[[n]]
    if (is.null(v)) stop("scalar ", n, " not found")
    cat(sprintf("%10s = %s\n", n, if (is.character(v)) v else stata_fmt(v, "%10.0g")))
  }
  invisible(NULL)
}
stata_scalar_drop <- function(names = NULL) {
  if (is.null(names)) {
    stata_scalars <<- list()
    return(invisible(NULL))
  }
  for (n in names) {
    if (is.null(stata_scalars[[n]])) stop("scalar ", n, " not found")
  }
  for (n in names) stata_scalars[[n]] <<- NULL
  invisible(NULL)
}"##;

impl Session {
    /// `scalar`.
    pub(super) fn scalar(&mut self, c: &Command, out: &mut Out) -> Result<(), Untranslated> {
        let rest = super::returned::after_name(c);
        let rest = rest.trim();
        let (word, tail) = match rest.split_once(char::is_whitespace) {
            Some((w, t)) => (w, t.trim()),
            None => (rest, ""),
        };
        self.helpers.insert("stata_scalar");
        if matches!(word, "de" | "def" | "defi" | "defin" | "define") && tail.contains('=') {
            return self.scalar_define(tail, out);
        }
        // scalar name = exp, without define
        if rest
            .split_once('=')
            .is_some_and(|(lhs, _)| super::is_name(lhs.trim()))
        {
            return self.scalar_define(rest, out);
        }
        let names: Vec<&str> = tail.split_whitespace().collect();
        let all = names.is_empty() || names == ["_all"];
        for n in &names {
            if *n != "_all" && !self.scalars.contains_key(*n) {
                return Err(no(format!(
                    "scalar {n} not found: no scalar {n} is defined before this line in the do-file"
                )));
            }
        }
        let list = if all {
            String::new()
        } else {
            format!(
                "c({})",
                names
                    .iter()
                    .map(|n| r_text(n))
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        };
        match word {
            "list" | "l" | "li" | "lis" | "dir" | "di" => {
                self.helpers.insert("stata_fmt");
                if Self::quiet(c) {
                    return Ok(());
                }
                out.line(format!("stata_scalar_list({list})"));
                Ok(())
            }
            "drop" => {
                if names.is_empty() {
                    return Err(no("scalar drop needs the scalars to drop, or _all"));
                }
                if all {
                    self.scalars.clear();
                } else {
                    for n in &names {
                        self.scalars.remove(*n);
                    }
                }
                out.line(format!("stata_scalar_drop({list})"));
                Ok(())
            }
            other => Err(no(format!("scalar {other} is not translated yet"))),
        }
    }

    fn scalar_define(&mut self, text: &str, out: &mut Out) -> Result<(), Untranslated> {
        let (name, exp) = text
            .split_once('=')
            .ok_or_else(|| no("scalar define needs `name = exp`"))?;
        let name = name.trim();
        if !super::is_name(name) || name.len() > 32 {
            return Err(no(format!("{name} is not a valid scalar name")));
        }
        let (code, kind) = self.display_value(exp.trim(), out)?;
        let kind = match kind {
            Kind::Text => Kind::Text,
            _ => Kind::Number,
        };
        out.line(format!("stata_scalar_set({}, {code})", r_text(name)));
        self.scalars.insert(name.to_string(), kind);
        Ok(())
    }

    /// A scalar in an expression: `bare` when read by its name alone (a
    /// variable of that name comes first), not by `scalar(name)`.
    pub(super) fn scalar_value(
        &self,
        name: &str,
        bare: bool,
    ) -> Result<(String, Kind), Untranslated> {
        let Some(kind) = self.scalars.get(name) else {
            return Err(no(format!(
                "scalar({name}): no scalar {name} is defined before it in this do-file (Stata: {name} not found)"
            )));
        };
        if bare {
            self.scalars_read.borrow_mut().push(name.to_string());
        }
        Ok((format!("stata_scalar({})", r_text(name)), *kind))
    }
}
