//! `coefplot` (Ben Jann's SSC package): the coefficients of stored models
//! and their confidence intervals, drawn with ggplot2.
//!
//! What it shows is decoded from Stata 19's answers alone (its `gen()`
//! variables and the labels of its graphs exported as SVG), not from its
//! code: each plot's coefficients by Stata's names in `e(b)` (`_cons`
//! included; base levels with `baselevels`, terms left out with `omitted`,
//! both as 0), in the order they first appear across the plots with `_cons`
//! last; `keep()`, `drop()` and `order()` (whose `.` is an empty row) on
//! those names; intervals b ± t·se with the model's residual df (the normal
//! without one) at `levels()` (95); k plots 1/(k+1) apart around each row
//! (`offset()` moves a plot, the others then staying put); `headings()` a
//! row of their own, after an empty one below the first; subgraphs split at
//! `||`, the options after the last applying to all unless one has its own.
//! Labels are the variable's label, else its name; a factor's level is its
//! value label, else `g=2`.

use super::*;

/// The rows `coefplot` shows and their places: see the module.
pub(super) const COEFPLOT_HELPER: &str = r##"stata_coefplot_rows <- function(subs, d, keep = NULL, drop = NULL, order = NULL, baselevels = FALSE,
                                omitted = FALSE, level = 0.95, headings = character(), coeflabels = character(),
                                gap = TRUE, swap = FALSE, eqrename = character()) {
  wild <- function(p) paste0("^", gsub("\\?", ".", gsub("\\*", ".*", gsub("([.+^$(){}|\\[\\]])", "\\\\\\1", p))), "$")
  hits <- function(n, pats) {
    pats <- sub(":$", ":*", pats)
    pats <- ifelse(grepl(":", pats), sub("^[^:]*:", "", pats), pats)
    Reduce(`|`, lapply(pats, function(p) grepl(wild(p), n)), rep(FALSE, length(n)))
  }
  one <- function(name) {
    e <- if (name == ".") {
      if (!exists("stata_e")) stop("coefplot: no estimation results in memory")
      stata_e
    } else {
      s <- if (exists("stata_stored")) stata_stored[[name]] else NULL
      if (is.null(s)) stop("coefplot: estimation result ", name, " not found")
      s$stata_e
    }
    if (is.null(e$b)) stop("coefplot: ", name, " has no e(b)")
    raw <- colnames(e$b)
    if (any(grepl(":", raw))) {
      eq <- unique(sub(":.*$", "", raw[grepl(":", raw)]))
      if (length(eq) > 1) stop("coefplot of a model with several equations is not translated yet")
      raw <- sub("^[^:]*:", "", raw)
    }
    if (any(grepl("#", raw))) stop("coefplot of interactions is not translated yet")
    base <- grepl("^[0-9]+b?n?b\\.", raw)
    om <- grepl("^o\\.", raw) | grepl("^[0-9]+o\\.", raw)
    nm <- sub("^o\\.", "", sub("^([0-9]+)[bno]+\\.", "\\1.", raw))
    b <- as.numeric(e$b[1, ])
    se <- sqrt(pmax(diag(as.matrix(e$V)), 0))
    b[base | om] <- 0
    se[base | om] <- 0
    df <- if (!is.null(e$df_r) && is.finite(e$df_r)) e$df_r else Inf
    show <- (!base | baselevels) & (!om | omitted)
    data.frame(name = nm, b = b, se = se, df = df, stringsAsFactors = FALSE)[show, , drop = FALSE]
  }
  tabs <- lapply(subs, function(s) lapply(s$plots, function(p) one(p$name)))
  all <- unique(unlist(lapply(tabs, function(t) lapply(t, `[[`, "name"))))
  all <- c(setdiff(all, "_cons"), intersect(all, "_cons"))
  if (length(keep)) all <- all[hits(all, keep)]
  if (length(drop)) all <- all[!hits(all, drop)]
  rows <- all
  if (length(order)) {
    rows <- character()
    for (o in order) {
      if (o == ".") rows <- c(rows, NA) else rows <- c(rows, setdiff(all[hits(all, o)], rows))
    }
    rows <- c(rows, setdiff(all, rows))
  }
  # headings: a row each, after an empty one below the first
  pos <- numeric(length(rows))
  heads <- data.frame(at = numeric(), text = character(), stringsAsFactors = FALSE)
  at <- 0
  first <- TRUE
  for (i in seq_along(rows)) {
    h <- if (!is.na(rows[i])) headings[rows[i]] else NA
    if (!is.na(h)) {
      at <- at + if (first || !gap) 1 else 2
      # {bf:text} and the like are Stata's markup: the text alone
      heads <- rbind(heads, data.frame(at = at, text = gsub("\\{[a-z]+:([^}]*)\\}", "\\1", unname(h)), stringsAsFactors = FALSE))
    }
    if (i > 0) first <- FALSE
    at <- at + 1
    pos[i] <- at
  }
  label <- function(n) {
    if (!is.na(coeflabels[n])) return(unname(coeflabels[n]))
    if (n == "_cons") return(n)
    m <- regmatches(n, regexec("^([0-9]+)\\.(.+)$", n))[[1]]
    if (length(m)) {
      v <- m[3]
      l <- if (v %in% names(d)) attr(d[[v]], "labels", exact = TRUE) else NULL
      k <- if (!is.null(l)) names(l)[match(as.numeric(m[2]), as.numeric(l))] else NA
      return(if (!is.na(k) && length(k)) k else paste0(v, "=", m[2]))
    }
    v <- sub("^c\\.", "", n)
    if (v %in% names(d)) stata_varlab(d, v) else n
  }
  # levels(95 90): an interval each, ll1 ul1 then ll2 ul2
  q <- function(df, lv) ifelse(is.finite(df), stats::qt((1 + lv) / 2, df), stats::qnorm((1 + lv) / 2))
  nlev <- length(level)
  out <- list()
  # asequation swapnames: each coefficient a group, a row in it for each plot (1, 2, a
  # row's gap, 4, 5), the rows named after the plots (eqrename()), the groups after the
  # coefficients when there are several
  if (swap) {
    if (length(subs) > 1 || anyNA(rows) || nrow(heads)) {
      stop("RCraft: coefplot, swapnames with subgraphs, headings() or order()'s empty rows is not translated yet", call. = FALSE)
    }
    k <- length(subs[[1]]$plots)
    for (i in seq_len(k)) {
      if (!all(rows %in% tabs[[1]][[i]]$name)) {
        stop("RCraft: coefplot, swapnames of plots without the same coefficients is not translated yet", call. = FALSE)
      }
    }
    pnames <- vapply(subs[[1]]$plots, function(p) p$name, "")
    plabels <- ifelse(is.na(eqrename[pnames]), pnames, eqrename[pnames])
    swap_at <- function(r, i) (match(r, rows) - 1) * (k + 1) + i
  }
  for (j in seq_along(subs)) {
    k <- length(subs[[j]]$plots)
    given <- vapply(subs[[j]]$plots, function(p) !is.null(p$offset), TRUE)
    for (i in seq_len(k)) {
      p <- subs[[j]]$plots[[i]]
      off <- if (any(given)) { if (given[i]) p$offset else 0 } else ((k + 1) / 2 - i) / (k + 1)
      t <- tabs[[j]][[i]]
      t <- t[t$name %in% rows, , drop = FALSE]
      if (!nrow(t)) next
      t$at <- if (swap) swap_at(t$name, i) else pos[match(t$name, rows)] - off
      for (l in seq_len(nlev)) {
        t[[paste0("ll", l)]] <- t$b - q(t$df, level[l]) * t$se
        t[[paste0("ul", l)]] <- t$b + q(t$df, level[l]) * t$se
      }
      t$plot <- i
      t$plotlabel <- p$label
      t$by <- j
      t$bylabel <- subs[[j]]$label
      out[[length(out) + 1]] <- t
    }
  }
  tab <- do.call(rbind, out)
  # order()'s empty rows are rows of their own, with no estimate
  gaps <- pos[is.na(rows)]
  if (length(gaps)) {
    g <- tab[rep(1, length(gaps)), , drop = FALSE]
    g[, c("b", "se", paste0("ll", seq_len(nlev)), paste0("ul", seq_len(nlev)))] <- NA
    g$at <- gaps
    g$name <- NA
    g$plot <- 1
    g$by <- 1
    tab <- rbind(tab, g)
  }
  tab <- tab[order(tab$at), , drop = FALSE]
  shown <- !is.na(rows)
  nplots <- max(vapply(subs, function(s) length(s$plots), 1))
  if (swap) {
    at <- as.vector(outer(seq_len(nplots), (seq_along(rows) - 1) * (nplots + 1), `+`))
    return(list(tab = tab, breaks = at, labels = rep(unname(plabels), length(rows)), plots = nplots,
                subs = 1, levels = nlev,
                groups = if (length(rows) > 1) list(at = (seq_along(rows) - 1) * (nplots + 1) + (nplots + 1) / 2, labels = rows)))
  }
  list(tab = tab, breaks = c(pos[shown], heads$at),
       labels = c(vapply(rows[shown], label, ""), heads$text), plots = nplots,
       subs = length(subs), levels = nlev, groups = NULL)
}
stata_coefplot <- function(cp, vertical = FALSE, xline = NULL, title = NULL, legend = TRUE,
                           mlabel = NULL, keys = NULL) {
  tab <- cp$tab[!is.na(cp$tab$b), , drop = FALSE]
  # the legend and the panels in the order written
  tab$plotlabel <- factor(tab$plotlabel, levels = unique(tab$plotlabel[order(tab$by, tab$plot)]))
  tab$bylabel <- factor(tab$bylabel, levels = unique(tab$bylabel[order(tab$by)]))
  many <- cp$plots > 1
  nlev <- if (is.null(cp$levels)) 1 else cp$levels
  g <- ggplot2::ggplot(tab, ggplot2::aes(y = at, x = b))
  # each level's interval, the inner ones drawn thicker
  for (l in seq_len(nlev)) {
    g <- g + ggplot2::geom_errorbarh(ggplot2::aes(xmin = .data[[paste0("ll", l)]], xmax = .data[[paste0("ul", l)]]),
                                     height = 0, linewidth = 0.5 * l)
  }
  g <- g + ggplot2::geom_point()
  # mlabel(string(@b, "fmt")): each marker labelled with its b in the format
  if (!is.null(mlabel)) {
    tab$stata_mlbl <- vapply(tab$b, function(v) trimws(stata_fmt(v, mlabel)), "")
    g <- g + ggplot2::geom_text(data = tab, ggplot2::aes(label = stata_mlbl), vjust = -0.8, show.legend = FALSE)
  }
  if (many) g <- g + ggplot2::aes(colour = plotlabel)
  for (v in xline) g <- g + ggplot2::geom_vline(xintercept = v)
  # every row shows, headings and empty rows too; swapnames' groups on the other side
  sec <- if (!is.null(cp$groups)) ggplot2::dup_axis(breaks = cp$groups$at, labels = cp$groups$labels, name = NULL) else ggplot2::waiver()
  g <- g + ggplot2::expand_limits(y = range(c(cp$breaks, cp$tab$at))) +
    ggplot2::scale_y_reverse(breaks = cp$breaks, labels = cp$labels, sec.axis = sec) +
    ggplot2::labs(x = NULL, y = NULL, colour = NULL, title = title)
  # legend(order(3 "text" ...)): the plots named, in that order, with those texts
  if (many && !is.null(keys)) {
    lv <- levels(tab$plotlabel)
    g <- g + ggplot2::scale_colour_discrete(breaks = lv[keys$plot], labels = keys$text)
  }
  if (cp$subs > 1) g <- g + ggplot2::facet_wrap(~bylabel, nrow = 1)
  if (!legend) g <- g + ggplot2::theme(legend.position = "none")
  if (vertical) g <- g + ggplot2::coord_flip()
  g
}
stata_coefplot_gen <- function(d, cp, prefix) {
  t <- cp$tab
  n <- max(nrow(d), nrow(t))
  if (nrow(d) < n) d[(nrow(d) + 1):n, ] <- NA
  put <- function(v, x) {
    y <- rep(NA_real_, n)
    y[seq_along(x)] <- x
    # as Stata keeps them: floats
    d[[paste0(prefix, v)]] <<- stata_float(y)
  }
  put("by", t$by)
  put("plot", t$plot)
  put("at", t$at)
  put("b", t$b)
  put("V", t$se^2)
  put("se", t$se)
  put("t", t$b / t$se)
  put("df", ifelse(is.finite(t$df), t$df, NA))
  put("pval", ifelse(is.finite(t$df), 2 * stats::pt(-abs(t$b / t$se), t$df), 2 * stats::pnorm(-abs(t$b / t$se))))
  for (l in seq_len(if (is.null(cp$levels)) 1 else cp$levels)) {
    put(paste0("ll", l), t[[paste0("ll", l)]])
    put(paste0("ul", l), t[[paste0("ul", l)]])
  }
  d
}"##;

/// One plot: a model's stored name (`.` for the results in memory), its
/// legend label and its offset.
struct Plot {
    name: String,
    label: String,
    offset: Option<f64>,
}

/// The options of one plot that only change its look.
const PLOT_LOOK: &[(&str, usize)] = &[
    ("mcolor", 2),
    ("msymbol", 2),
    ("msize", 3),
    ("mfcolor", 3),
    ("mlcolor", 3),
    ("mlwidth", 3),
    ("lcolor", 2),
    ("lwidth", 2),
    ("lpattern", 2),
    ("ciopts", 6),
    ("pstyle", 6),
    ("color", 3),
];

/// The options of the graph (or a subgraph) that only change its look.
const GRAPH_LOOK: &[(&str, usize)] = &[
    ("grid", 4),
    ("xlabel", 3),
    ("ylabel", 3),
    ("xtitle", 2),
    ("ytitle", 2),
    ("xscale", 3),
    ("yscale", 3),
    ("graphregion", 6),
    ("plotregion", 5),
    ("scheme", 3),
    ("xsize", 5),
    ("ysize", 5),
    ("name", 4),
    ("byopts", 6),
    ("noeqlabels", 6),
    ("subtitle", 3),
    ("note", 4),
    ("aspectratio", 6),
    ("xmtick", 3),
    ("ymtick", 3),
    ("bgcolor", 3),
    ("xmlabel", 4),
    ("ymlabel", 4),
    ("saving", 4),
];

impl Session {
    /// `coefplot [plots] [|| plots ...] [, options]`.
    pub(super) fn coefplot(&mut self, c: &Command, out: &mut Out) -> Result<(), Untranslated> {
        let written = if c.parts.options.trim().is_empty() {
            c.parts.main.clone()
        } else {
            format!("{}, {}", c.parts.main, c.parts.options)
        };
        if c.parts.if_exp.is_some() || c.parts.in_range.is_some() {
            return Err(no("coefplot with if or in is not translated yet"));
        }
        let segments = super::graph::split_plots(&written)?;
        if segments.is_empty() {
            return Err(no("coefplot needs a model or the results in memory"));
        }
        // the last subgraph's options are everyone's, unless one has its own
        let mut subs: Vec<(Vec<Plot>, Vec<crate::lang::syntax::Opt>)> = Vec::new();
        for s in &segments {
            let (plots_text, opt_text) = split_top_comma(s);
            let opts = crate::lang::syntax::options(opt_text);
            let plots = self.coefplot_plots(plots_text)?;
            subs.push((plots, opts));
        }
        let global: Vec<crate::lang::syntax::Opt> =
            subs.last().map(|(_, o)| o.clone()).unwrap_or_default();
        let g = Opts(&global);
        let mut known: Vec<(&str, usize)> = GRAPH_LOOK.to_vec();
        known.extend(super::graph::LOOK.iter().copied());
        known.extend(PLOT_LOOK.iter().copied());
        known.extend([
            ("keep", 4),
            ("drop", 4),
            ("order", 5),
            ("baselevels", 4),
            ("omitted", 4),
            ("levels", 3),
            ("headings", 4),
            ("coeflabels", 6),
            ("vertical", 4),
            ("horizontal", 3),
            ("xline", 5),
            ("title", 2),
            ("legend", 3),
            ("bylabel", 7),
            ("generate", 3),
            ("replace", 7),
            ("nodraw", 6),
            ("label", 5),
            ("offset", 6),
            ("asequation", 10),
            ("swapnames", 9),
            ("eqrename", 8),
        ]);
        for (_, o) in &subs {
            if let Some(bad) = Opts(o).other(&known) {
                return Err(no(format!(
                    "coefplot with the option {bad} is not translated yet"
                )));
            }
        }
        // options that pick what is shown are the graph's, from the last part
        for (i, (_, o)) in subs.iter().enumerate() {
            if i + 1 < subs.len() {
                let so = Opts(o);
                for w in [
                    "keep",
                    "drop",
                    "order",
                    "baselevels",
                    "omitted",
                    "levels",
                    "headings",
                ] {
                    if so.has(w, 4) {
                        return Err(no(format!(
                            "coefplot with {w}() in one subgraph only is not translated yet"
                        )));
                    }
                }
            }
        }
        let list = |name: &str, m: usize| -> Vec<String> {
            g.arg(name, m)
                .map(|a| words(&a).iter().map(|t| r_text(t)).collect())
                .unwrap_or_default()
        };
        let keep = list("keep", 4);
        let drop = list("drop", 4);
        let order = list("order", 5);
        let pairs = |name: &str, m: usize| -> Result<String, Untranslated> {
            let Some(a) = g.arg(name, m) else {
                return Ok("character()".into());
            };
            let mut items = Vec::new();
            // the pairs, then options that change their look (and nogap)
            let (a, _) = split_top_comma(&a);
            for (k, v) in equal_pairs(a)? {
                items.push(format!("{} = {}", r_text(&k), r_text(&v)));
            }
            Ok(format!("c({})", items.join(", ")))
        };
        let headings = pairs("headings", 4)?;
        let coeflabels = pairs("coeflabels", 6)?;
        // headings(..., nogap): no empty row before the headings after the first
        let gap = !g.arg("headings", 4).is_some_and(|a| {
            let (_, o) = split_top_comma(&a);
            crate::lang::syntax::options(o)
                .iter()
                .any(|x| x.is("nogap", 5))
        });
        // levels(95 90): an interval for each, as Stata's ll1 ul1, ll2 ul2
        let levels: Vec<f64> = match g.arg("levels", 3) {
            None => vec![95.0],
            Some(l) => l
                .split_whitespace()
                .map(|w| {
                    w.parse::<f64>()
                        .ok()
                        .filter(|v| *v > 0.0 && *v < 100.0)
                        .ok_or_else(|| {
                            no(format!(
                                "levels({l}): levels between 0 and 100 are translated"
                            ))
                        })
                })
                .collect::<Result<_, _>>()?,
        };
        if levels.is_empty() {
            return Err(no("levels() needs a level"));
        }
        // asequation (a no-op for one-equation models, as Stata 19 shows) and swapnames,
        // which makes each coefficient a group of a row per plot; eqrename() names them
        let swap = g.has("swapnames", 9);
        if swap && !g.has("asequation", 10) {
            return Err(no(
                "coefplot, swapnames without asequation is not translated yet",
            ));
        }
        if g.has("eqrename", 8) && !swap {
            return Err(no(
                "coefplot, eqrename() without asequation swapnames is not translated yet",
            ));
        }
        let eqrename = pairs("eqrename", 8)?;
        // mlabel(string(@b, "%9.3f")): each marker's b in that format; other marker labels
        // only change the look
        let mlabel = g.arg("mlabel", 4).and_then(|m| {
            let m: String = m.chars().filter(|c| !c.is_whitespace()).collect();
            let inner = m.strip_prefix("string(@b,\"")?.strip_suffix("\")")?;
            Some(r_text(inner))
        });
        // legend(order(3 "text" ...)): with L levels a plot has L + 1 keys, its marker the last
        let keys = match g.arg("legend", 3) {
            None => None,
            Some(l) => {
                let lo = crate::lang::syntax::options(&l);
                match lo.iter().find(|o| o.is("order", 5)) {
                    None => None,
                    Some(o) => {
                        let per = levels.len() + 1;
                        let mut plots = Vec::new();
                        let mut texts = Vec::new();
                        let w = quoted_words_kept(&o.raw.clone().unwrap_or_default());
                        let mut it = w.into_iter().peekable();
                        while let Some(n) = it.next() {
                            let k: usize = n.parse().map_err(|_| {
                                no(format!("coefplot, legend(order({n} ...)): numbers and their texts are translated"))
                            })?;
                            if k == 0 || k % per != 0 {
                                return Err(no(format!(
                                    "coefplot, legend(order({k})): a key other than a plot's marker is not translated yet"
                                )));
                            }
                            let Some(t) = it.next_if(|t| t.starts_with('"')) else {
                                return Err(no("coefplot, legend(order()) without a text for each key is not translated yet"));
                            };
                            plots.push((k / per).to_string());
                            texts.push(r_text(t.trim_matches('"')));
                        }
                        Some(format!(
                            "list(plot = c({}), text = c({}))",
                            plots.join(", "),
                            texts.join(", ")
                        ))
                    }
                }
            }
        };
        let mut xline = Vec::new();
        for o in global.iter().filter(|o| o.is("xline", 5)) {
            let a = o.arg();
            for w in a.split(',').next().unwrap_or("").split_whitespace() {
                let v: f64 = w
                    .parse()
                    .map_err(|_| no(format!("xline({a}): numbers are translated")))?;
                xline.push(r_number(v));
            }
        }
        let d = self.data()?;
        let mut sub_code = Vec::new();
        for (i, (plots, o)) in subs.iter().enumerate() {
            let so = Opts(o);
            let label = so
                .arg("bylabel", 7)
                .or_else(|| g.arg("bylabel", 7))
                .map(|l| l.trim().trim_matches('"').to_string())
                .unwrap_or_else(|| {
                    plots
                        .first()
                        .map(|p| p.name.clone())
                        .unwrap_or_else(|| format!("{}", i + 1))
                });
            let ps: Vec<String> = plots
                .iter()
                .map(|p| {
                    format!(
                        "list(name = {}, label = {}, offset = {})",
                        r_text(&p.name),
                        r_text(&p.label),
                        p.offset.map(r_number).unwrap_or_else(|| "NULL".into())
                    )
                })
                .collect();
            sub_code.push(format!(
                "list(label = {}, plots = list({}))",
                r_text(&label),
                ps.join(", ")
            ));
        }
        self.helpers.insert("stata_graph");
        self.helpers.insert("stata_coefplot");
        if mlabel.is_some() {
            self.helpers.insert("stata_fmt");
        }
        out.package("ggplot2");
        out.line(format!(
            "stata_cp <- stata_coefplot_rows(list({}), {d}, keep = c({}), drop = c({}), order = c({}), baselevels = {}, omitted = {}, level = c({}), headings = {headings}, coeflabels = {coeflabels}, gap = {}, swap = {}, eqrename = {eqrename})",
            sub_code.join(", "),
            keep.join(", "),
            drop.join(", "),
            order.join(", "),
            super::stats::r_logical(g.has("baselevels", 4)),
            super::stats::r_logical(g.has("omitted", 4)),
            levels.iter().map(|l| r_number(l / 100.0)).collect::<Vec<_>>().join(", "),
            super::stats::r_logical(gap),
            super::stats::r_logical(swap)
        ));
        let title = g
            .arg("title", 2)
            .map(|t| r_text(t.trim().trim_matches('"')))
            .unwrap_or_else(|| "NULL".into());
        let legend_off = g.arg("legend", 3).is_some_and(|l| l.trim() == "off");
        out.line(format!(
            "stata_graph <- stata_coefplot(stata_cp, vertical = {}, xline = c({}), title = {title}, legend = {}, mlabel = {}, keys = {})",
            super::stats::r_logical(g.has("vertical", 4)),
            xline.join(", "),
            super::stats::r_logical(!legend_off),
            mlabel.unwrap_or_else(|| "NULL".into()),
            keys.unwrap_or_else(|| "NULL".into())
        ));
        if let Some(p) = g.arg("generate", 3) {
            let p = p.trim().to_string();
            if !is_name(&p) {
                return Err(no(format!("generate({p}) needs a prefix")));
            }
            if !g.has("replace", 7) {
                out.line(format!(
                    "if (any(startsWith(names({d}), {}))) stop(\"variables {p}* already exist\")",
                    r_text(&p)
                ));
            }
            self.helpers.insert("stata_float");
            out.line(format!(
                "{d} <- stata_coefplot_gen({d}, stata_cp, {})",
                r_text(&p)
            ));
            self.kinds.clear();
            self.storage.clear();
        }
        // saving(name[, replace]): the graph kept in a file of that name
        if let Some(s) = g.arg("saving", 4) {
            let (name, o) = split_top_comma(&s);
            let file = file_name(name.trim(), "gph");
            let replace = crate::lang::syntax::options(o)
                .iter()
                .any(|x| x.is("replace", 7));
            if !replace {
                out.line(format!(
                    "if (file.exists({p})) stop(\"file {p2} already exists\")",
                    p = r_text(&file),
                    p2 = file.replace('"', "")
                ));
            }
            out.line(format!("saveRDS(stata_graph, {})", r_text(&file)));
            out.caveat(
                "saving() keeps the graph in R's form under the .gph name, which Stata cannot open",
            );
        }
        if !self.quiet(c) && !g.has("nodraw", 6) {
            out.line("print(stata_graph)");
        }
        out.caveat(
            "coefplot as the SSC package shows it, matched to its output (not its code); drawn by \
             ggplot2, so its look is R's",
        );
        self.graph_drawn = true;
        Ok(())
    }

    /// The plots of one subgraph: names, or `(name, options)`.
    fn coefplot_plots(&self, text: &str) -> Result<Vec<Plot>, Untranslated> {
        let mut plots = Vec::new();
        let text = text.trim();
        let mut rest = text;
        while !rest.trim().is_empty() {
            let t = rest.trim_start();
            if let Some(inner) = t.strip_prefix('(') {
                let close =
                    matching_paren(inner).ok_or_else(|| no("coefplot: a ( without its )"))?;
                let body = inner.get(..close).unwrap_or("");
                rest = inner.get(close + 1..).unwrap_or("");
                let (names, opt_text) = split_top_comma(body);
                let names: Vec<&str> = names.split_whitespace().collect();
                let name = match names.as_slice() {
                    [n] => n.to_string(),
                    [] => ".".to_string(),
                    _ => {
                        return Err(no(
                            "coefplot with several models in one plot is not translated yet",
                        ))
                    }
                };
                let o = crate::lang::syntax::options(opt_text);
                let po = Opts(&o);
                let mut known: Vec<(&str, usize)> = PLOT_LOOK.to_vec();
                known.extend(super::graph::LOOK.iter().copied());
                known.extend([("label", 5), ("offset", 6)]);
                if let Some(bad) = po.other(&known) {
                    return Err(no(format!(
                        "coefplot with the plot option {bad} is not translated yet"
                    )));
                }
                let offset = match po.arg("offset", 6) {
                    None => None,
                    Some(v) => Some(
                        v.trim()
                            .parse::<f64>()
                            .map_err(|_| no(format!("offset({v}) needs a number")))?,
                    ),
                };
                let label = po
                    .arg("label", 5)
                    .map(|l| l.trim().trim_matches('"').to_string())
                    .unwrap_or_else(|| name.clone());
                plots.push(self.coefplot_plot(name, label, offset)?);
            } else {
                let (word, after) = t.split_once(char::is_whitespace).unwrap_or((t, ""));
                rest = after;
                plots.push(self.coefplot_plot(word.to_string(), word.to_string(), None)?);
            }
        }
        if plots.is_empty() {
            plots.push(self.coefplot_plot(".".into(), ".".into(), None)?);
        }
        Ok(plots)
    }

    fn coefplot_plot(
        &self,
        name: String,
        label: String,
        offset: Option<f64>,
    ) -> Result<Plot, Untranslated> {
        if name == "." {
            if matches!(self.e_ret, super::returned::Returned::Nothing) {
                return Err(no("coefplot: no model is fitted before it in this do-file"));
            }
        } else if !self.stored_fits.contains_key(&name) {
            return Err(no(format!(
                "coefplot: {name} is not stored before it in this do-file (estimates store or eststo)"
            )));
        }
        Ok(Plot {
            name,
            label,
            offset,
        })
    }
}

/// Text before its first comma outside quotes and parentheses, and the rest.
pub(super) fn split_top_comma(text: &str) -> (&str, &str) {
    let (mut depth, mut quote) = (0usize, false);
    for (i, ch) in text.char_indices() {
        match ch {
            '"' => quote = !quote,
            '(' if !quote => depth += 1,
            ')' if !quote => depth = depth.saturating_sub(1),
            ',' if !quote && depth == 0 => {
                return (text.get(..i).unwrap_or(""), text.get(i + 1..).unwrap_or(""))
            }
            _ => {}
        }
    }
    (text, "")
}

/// Where the `)` closing an opened `(` is (the text after the `(`).
fn matching_paren(text: &str) -> Option<usize> {
    let (mut depth, mut quote) = (1usize, false);
    for (i, ch) in text.char_indices() {
        match ch {
            '"' => quote = !quote,
            '(' if !quote => depth += 1,
            ')' if !quote => {
                depth -= 1;
                if depth == 0 {
                    return Some(i);
                }
            }
            _ => {}
        }
    }
    None
}

/// `a = "text" b = "text"` as pairs.
fn equal_pairs(text: &str) -> Result<Vec<(String, String)>, Untranslated> {
    let mut out = Vec::new();
    let mut rest = text.trim();
    while !rest.is_empty() {
        let (key, after) = rest
            .split_once('=')
            .ok_or_else(|| no(format!("{text}: name = \"text\" pairs are translated")))?;
        let after = after.trim_start();
        let inner = after
            .strip_prefix('"')
            .ok_or_else(|| no(format!("{text}: the text needs quotes")))?;
        let end = inner
            .find('"')
            .ok_or_else(|| no(format!("{text}: a quote without its end")))?;
        out.push((
            key.trim().to_string(),
            inner.get(..end).unwrap_or("").to_string(),
        ));
        rest = inner.get(end + 1..).unwrap_or("").trim();
    }
    Ok(out)
}

/// Words split at spaces outside quotes, the quotes taken off.
fn words(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut quote = false;
    for ch in text.chars() {
        match ch {
            '"' => quote = !quote,
            c if c.is_whitespace() && !quote => {
                if !cur.is_empty() {
                    out.push(std::mem::take(&mut cur));
                }
            }
            c => cur.push(c),
        }
    }
    if !cur.is_empty() {
        out.push(cur);
    }
    out
}

/// Words split at blanks outside quotes, a quoted one keeping its quotes.
fn quoted_words_kept(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut quote = false;
    for ch in text.chars() {
        match ch {
            '"' => {
                quote = !quote;
                cur.push(ch);
            }
            c if c.is_whitespace() && !quote => {
                if !cur.is_empty() {
                    out.push(std::mem::take(&mut cur));
                }
            }
            c => cur.push(c),
        }
    }
    if !cur.is_empty() {
        out.push(cur);
    }
    out
}
