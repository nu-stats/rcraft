//! `snapshot save|list|label|restore|erase`, as Stata 19 does them
//! (`plan/stata/snapshot.md`). Snapshots live in R (`stata_snap`, a list of
//! id, label, time and the data) and outlive `clear`; the translator keeps
//! what it follows of each snapshot's data so `restore` can put it back.
//!
//! - the next number is the highest one there plus one (1 when none);
//! - `restore` and `label` outside 1 to the highest number: `snapshot number
//!   must be between 1 and N` (r(198)); no snapshots at all: `no snapshots
//!   exist` (r(198)); a number in range that was erased: `snapshot N not
//!   found` (r(111)), which `erase` gives too, before it erases any;
//! - lines wrap at the line size, continued with two blanks.

use super::*;

pub(super) const SNAPSHOT_HELPER: &str = r##"if (!exists("stata_snap")) stata_snap <- list()
stata_snap_ids <- function() vapply(stata_snap, function(s) s$id, 0)
stata_snap_wrap <- function(txt) {
  ls <- if (exists("stata_linesize")) stata_linesize else 79
  words <- strsplit(txt, " ", fixed = TRUE)[[1]]
  lines <- character()
  cur <- ""
  for (w in words) {
    if (cur == "") { cur <- w; next }
    if (nchar(cur) + 1 + nchar(w) <= ls) { cur <- paste(cur, w); next }
    lines <- c(lines, cur)
    cur <- ""
    if (nchar(w) + 2 <= ls) { cur <- paste0("  ", w); next }
    first <- TRUE
    while (nchar(w) > 0) {
      room <- if (first) ls - 2 else ls - 4
      piece <- substr(w, 1, room)
      w <- substring(w, room + 1)
      if (nchar(w) > 0) lines <- c(lines, paste0(if (first) "  " else "  > ", piece)) else cur <- paste0(if (first) "  " else "  > ", piece)
      first <- FALSE
    }
  }
  c(lines, cur)
}
stata_snap_show <- function(s) {
  txt <- paste0("snapshot ", s$id, if (nzchar(s$label)) paste0(" (", s$label, ")") else "", " created at ", s$time)
  cat(stata_snap_wrap(txt), sep = "\n")
}
stata_snap_int <- function(arg) {
  tok <- strsplit(trimws(arg), "[[:space:]]+")[[1]]
  tok <- tok[nzchar(tok)]
  if (length(tok) == 0) stop("'' found where integer expected (Stata: r(7))", call. = FALSE)
  if (!grepl("^[0-9]+$", tok[1])) stop("'", tok[1], "' found where integer expected (Stata: r(7))", call. = FALSE)
  if (length(tok) > 1) stop("invalid syntax (Stata: r(198))", call. = FALSE)
  as.integer(tok[1])
}
stata_snap_numlist <- function(arg) {
  tok <- strsplit(trimws(arg), "[[:space:]]+")[[1]]
  tok <- tok[nzchar(tok)]
  if (length(tok) == 0) stop("invalid numlist has too few elements (Stata: r(122))", call. = FALSE)
  unlist(lapply(tok, function(t) {
    if (grepl("^[0-9]+$", t)) return(as.integer(t))
    if (grepl("^[0-9]+/[0-9]+$", t)) { p <- as.integer(strsplit(t, "/", fixed = TRUE)[[1]]); return(seq(p[1], p[2])) }
    stop("invalid numlist (Stata: r(121))", call. = FALSE)
  }))
}
stata_snap_find <- function(n) {
  ids <- stata_snap_ids()
  if (!length(ids)) stop("no snapshots exist (Stata: r(198))", call. = FALSE)
  if (n < 1 || n > max(ids)) stop("snapshot number must be between 1 and ", max(ids), " (Stata: r(198))", call. = FALSE)
  k <- match(n, ids)
  if (is.na(k)) stop("snapshot ", n, " not found (Stata: r(111))", call. = FALSE)
  k
}
stata_snapshot_save <- function(d, label) {
  ids <- stata_snap_ids()
  id <- if (length(ids)) max(ids) + 1 else 1
  now <- Sys.time()
  stamp <- paste(as.integer(format(now, "%d")), month.abb[as.integer(format(now, "%m"))], format(now, "%Y %H:%M"))
  s <- list(id = id, label = label, time = stamp, data = d)
  stata_snap[[length(stata_snap) + 1]] <<- s
  stata_snap_show(s)
  list(snapshot = id)
}
stata_snapshot_list <- function(arg) {
  if (trimws(arg) %in% c("", "_all")) {
    for (s in stata_snap) stata_snap_show(s)
  } else {
    for (n in stata_snap_numlist(arg)) {
      k <- match(n, stata_snap_ids())
      if (is.na(k)) stop("snapshot ", n, " not found (Stata: r(111))", call. = FALSE)
      stata_snap_show(stata_snap[[k]])
    }
  }
  invisible(NULL)
}
stata_snapshot_label <- function(arg, text) {
  k <- stata_snap_find(stata_snap_int(arg))
  stata_snap[[k]]$label <<- text
  invisible(NULL)
}
stata_snapshot_get <- function(arg) {
  stata_snap[[stata_snap_find(stata_snap_int(arg))]]$data
}
stata_snapshot_erase <- function(arg) {
  if (trimws(arg) == "_all") {
    stata_snap <<- list()
    return(invisible(NULL))
  }
  nums <- stata_snap_numlist(arg)
  ids <- stata_snap_ids()
  for (n in nums) if (!(n %in% ids)) stop("snapshot ", n, " not found (Stata: r(111))", call. = FALSE)
  stata_snap <<- stata_snap[!(ids %in% nums)]
  invisible(NULL)
}"##;

/// The numbers a numlist (`3`, `1/3`, `1 4`) stands for, None when it is
/// not one.
fn numlist(text: &str) -> Option<Vec<u32>> {
    let mut out = Vec::new();
    for t in text.split_whitespace() {
        match t.split_once('/') {
            Some((a, b)) => {
                let (a, b) = (a.parse::<u32>().ok()?, b.parse::<u32>().ok()?);
                out.extend(a..=b.min(a.saturating_add(10_000)));
            }
            None => out.push(t.parse().ok()?),
        }
    }
    Some(out)
}

impl Session {
    pub(super) fn snapshot(
        &mut self,
        c: &Command,
        opts: &Opts,
        out: &mut Out,
    ) -> Result<(), Untranslated> {
        let stop = |out: &mut Out, msg: &str| {
            out.line(format!("stop({}, call. = FALSE)", r_text(msg)));
        };
        if c.parts.if_exp.is_some() || c.parts.in_range.is_some() {
            return Err(no("snapshot with if or in is not translated yet"));
        }
        let main = c.parts.main.trim();
        let (sub, rest) = first_word(main);
        let rest = rest.trim();
        self.helpers.insert("stata_snapshot");
        match sub {
            "save" => {
                if let Some(o) = opts.other(&[("label", 5)]) {
                    stop(out, &format!("option {o} not allowed (Stata: r(198))"));
                    return Ok(());
                }
                if !rest.is_empty() {
                    stop(out, "varlist not allowed (Stata: r(101))");
                    return Ok(());
                }
                let label = opts
                    .raw("label", 5)
                    .or_else(|| opts.arg("label", 5))
                    .map(|l| crate::dofile::macros::unquote(l.trim()))
                    .unwrap_or_default();
                let data = match &self.data {
                    Some(d) => d.clone(),
                    None => "NULL".to_string(),
                };
                out.line(format!(
                    "stata_r <- stata_snapshot_save({data}, {})",
                    r_text(&label)
                ));
                self.helpers.insert("stata_returned");
                self.r_new = Some((returned::R_SNAPSHOT, "snapshot".into()));
                let next = self.snaps.keys().max().map_or(1, |m| m + 1);
                let state = Box::new(self.data_state());
                self.snaps.insert(next, state);
                Ok(())
            }
            "list" | "label" | "restore" | "erase" => {
                if sub != "label" {
                    if let Some(o) = opts.other(&[]) {
                        stop(out, &format!("option {o} not allowed (Stata: r(198))"));
                        return Ok(());
                    }
                }
                match sub {
                    "list" => {
                        out.line(format!("stata_snapshot_list({})", r_text(rest)));
                    }
                    "label" => {
                        let (num, text) = first_word(rest);
                        let text = crate::dofile::macros::unquote(text.trim());
                        out.line(format!(
                            "stata_snapshot_label({}, {})",
                            r_text(num),
                            r_text(&text)
                        ));
                    }
                    "erase" => {
                        out.line(format!("stata_snapshot_erase({})", r_text(rest)));
                        if rest == "_all" {
                            self.snaps.clear();
                        } else if let Some(ids) = numlist(rest) {
                            // all or none: a number that is not there stops it first
                            if ids.iter().all(|i| self.snaps.contains_key(i)) {
                                for i in ids {
                                    self.snaps.remove(&i);
                                }
                            }
                        }
                    }
                    _ => return self.snapshot_restore(rest, out),
                }
                Ok(())
            }
            _ => {
                stop(out, "invalid syntax (Stata: r(198))");
                Ok(())
            }
        }
    }

    fn snapshot_restore(&mut self, rest: &str, out: &mut Out) -> Result<(), Untranslated> {
        let n = rest.parse::<u32>().ok();
        let Some(state) = n.and_then(|n| self.snaps.get(&n).cloned()) else {
            // Stata's own error (no number, a bad one, one that is not there)
            out.line(format!("stata_snapshot_get({})", r_text(rest)));
            // it was there after all (a save RCraft did not follow): the data cannot be followed
            if n.is_some() {
                out.line(format!(
                    "stop({}, call. = FALSE)",
                    r_text(&format!(
                        "RCraft: what snapshot {rest} held is not known here (a snapshot save was not translated); not translated yet"
                    ))
                ));
            }
            return Ok(());
        };
        let arg = r_text(rest);
        match &state.data {
            Some(d) => {
                out.line(format!("{d} <- stata_snapshot_get({arg})"));
                if let Some(now) = self.data.as_ref().filter(|now| *now != d) {
                    out.line(format!("rm({now})"));
                }
            }
            None => {
                out.line(format!("stata_snapshot_get({arg})"));
                if let Some(now) = &self.data {
                    out.line(format!("rm({now})"));
                }
            }
        }
        self.set_data_state(*state);
        Ok(())
    }
}
