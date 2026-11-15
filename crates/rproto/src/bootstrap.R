# RCraft session loop. Started as: Rscript --vanilla bootstrap.R <token>
#
# Requests arrive on standard input:   RUN <id> <nbytes>\n<code>\n   or   QUIT\n
# Frames leave on standard output as lines that start with <token> and a tab,
# mixed with whatever the user's code prints. Messages and other standard
# error output stay on standard error. See crates/rproto/src/lib.rs.
#
# Interrupts are held off everywhere except while the user's code runs, so one
# that arrives while a request is read or a run is being reported cannot lose
# the END frame. RCraft interrupts a run only after it has seen the run's
# BEGIN, so an interrupt still pending when the next run is about to begin was
# meant for an earlier one (sent just as it ended) and is dropped there.
local({
  token <- commandArgs(trailingOnly = TRUE)[[1L]]
  con <- file("stdin", "rb")

  # Plots: when RCraft names a folder, the graphics the user's code draws go to
  # one device that stays open between commands (so a plot can be added to by
  # the next command, as in any R console). A snapshot of the page is written
  # as <run id>-001.png, <run id>-002.png, ... when a command ends with the
  # page changed, and just before a new page replaces it.
  plot_dir <- Sys.getenv("RCRAFT_PLOT_DIR")
  live_dev <- NA_integer_
  last_dl <- NULL
  cur_run <- ""
  page_started <- FALSE
  page_no <- 0L
  keep_plots <- 30L
  # what a page shows: its recorded drawing calls, without par() settings, which
  # change the record but not the picture
  drawing <- function(dl) {
    Filter(function(e) {
      !identical(tryCatch(e[[2L]][[1L]]$name, error = function(err) ""), "C_par")
    }, dl)
  }
  snap <- function() {
    if (!page_started || is.na(live_dev) || !identical(unname(grDevices::dev.cur()), live_dev)) {
      return(invisible())
    }
    rec <- tryCatch(grDevices::recordPlot(), error = function(e) NULL)
    dl <- if (is.null(rec)) NULL else drawing(rec[[1L]])
    if (is.null(dl) || length(dl) == 0L || identical(dl, last_dl)) return(invisible())
    page_no <<- page_no + 1L
    file <- file.path(plot_dir, sprintf("%s-%03d.png", cur_run, page_no))
    # the recording is kept, under the snapshot's file name, so the plot can be
    # drawn again on a file device of any size (Graphs > Export plot); the
    # newest few only, as a recording can hold the data it plots
    kept <- c(getOption("rcraft.plots"), stats::setNames(list(rec), basename(file)))
    if (length(kept) > keep_plots) kept <- kept[seq.int(length(kept) - keep_plots + 1L, length(kept))]
    options(rcraft.plots = kept)
    try(
      {
        grDevices::dev.copy(grDevices::png, file, width = 1000, height = 700, res = 120)
        grDevices::dev.off()
        grDevices::dev.set(live_dev)
      },
      silent = TRUE
    )
    last_dl <<- dl
    invisible()
  }
  if (nzchar(plot_dir)) {
    options(device = function(...) {
      grDevices::png(file.path(plot_dir, "live-%04d.png"), width = 1000, height = 700, res = 120)
      grDevices::dev.control("enable")
      live_dev <<- unname(grDevices::dev.cur())
      last_dl <<- NULL
      page_started <<- FALSE
    })
    # plot.new() starts a new page, or the next figure of a multi-figure page
    # (pairs(), par(mfrow = ...)); only a new page ends the page being drawn
    setHook("before.plot.new", function(...) {
      mfg <- graphics::par("mfg")
      if (length(mfg) == 4L && (prod(mfg[3:4]) == 1L || all(mfg[1:2] == mfg[3:4]))) snap()
    })
    setHook("before.grid.newpage", function(...) snap())
    # a page counts only once plot.new() or grid.newpage() has begun it: par() and
    # similar calls open the device before that, and a blank page is not a plot
    setHook("plot.new", function(...) page_started <<- TRUE)
    setHook("grid.newpage", function(...) page_started <<- TRUE)
  }

  # closeAllConnections() in the user's code closes `con` too, and R may give
  # its number to a newer connection, so the id is compared as well
  con_ok <- function() {
    tryCatch(
      identical(attr(getConnection(as.integer(con)), "conn_id"), attr(con, "conn_id")),
      error = function(e) FALSE
    )
  }

  esc <- function(x) {
    x <- paste(as.character(x), collapse = "\n")
    x <- gsub("\\", "\\\\", x, fixed = TRUE)
    x <- gsub("\n", "\\n", x, fixed = TRUE)
    x <- gsub("\r", "\\r", x, fixed = TRUE)
    gsub("\t", "\\t", x, fixed = TRUE)
  }

  # frames written while the user's code has a sink() on (capture.output()
  # uses one) would land in the sink, so they wait until the run's end
  held <- character()

  emit <- function(...) {
    line <- paste0(token, "\t", paste(c(...), collapse = "\t"), "\n")
    if (sink.number() > 0L) {
      held <<- c(held, line)
    } else {
      cat(line, sep = "", file = stdout())
      flush(stdout())
    }
  }

  # sinks the run left on would swallow every later frame
  end_sinks <- function(id) {
    n <- sink.number()
    for (i in seq_len(n)) sink()
    diverted <- sink.number(type = "message") != 2L
    if (diverted) sink(type = "message")
    for (line in held) cat(line, sep = "", file = stdout())
    held <<- character()
    if (n > 0L || diverted) {
      emit("WARN", id, esc(paste(
        "sink() was still on when the code finished;",
        "RCraft turned it off so it can keep reading R's output"
      )))
    }
  }

  # Tables: a visible value that is a data frame, matrix or table (or a named
  # list of them) is sent as a table file in the plot folder, with what R
  # prints for it, and Results shows a grid (see crates/rproto/src/table.rs).
  # Anything else, anything larger, and anything that goes wrong prints as usual.
  # options(rcraft.tables = FALSE) turns this off.
  table_no <- 0L
  or_else <- function(x, y) if (is.null(x)) y else x
  cell_text <- function(x) {
    out <- trimws(format(x))
    out[is.na(out)] <- "NA"
    out
  }
  grid_of <- function(v) {
    if (is.data.frame(v)) {
      ok <- length(v) > 0L && length(v) <= 50L && nrow(v) <= 500L &&
        all(vapply(v, function(x) is.atomic(x) && is.null(dim(x)), TRUE))
      if (!ok) return(NULL)
      cells <- vapply(v, function(x) {
        if (is.factor(x)) x <- as.character(x)
        cell_text(x)
      }, character(nrow(v)))
      if (!is.matrix(cells)) cells <- matrix(cells, nrow = nrow(v))
      return(list(corner = "", cols = names(v), num = vapply(v, is.numeric, TRUE),
                  rows = as.character(row.names(v)), cells = cells))
    }
    if (inherits(v, "table") && length(dim(v)) == 1L) {
      if (length(v) > 500L) return(NULL)
      dn <- names(dimnames(v))
      return(list(corner = if (length(dn) == 1L) dn else "", cols = "Count", num = TRUE,
                  rows = or_else(names(v), as.character(seq_along(v))),
                  cells = matrix(cell_text(as.vector(v)), ncol = 1L)))
    }
    two_d <- is.matrix(v) || (inherits(v, "table") && length(dim(v)) == 2L)
    if (two_d && (is.numeric(v) || is.character(v) || is.logical(v))) {
      m <- unclass(v)
      if (nrow(m) > 500L || ncol(m) > 50L || ncol(m) == 0L) return(NULL)
      cells <- matrix("", nrow(m), ncol(m))
      for (j in seq_len(ncol(m))) cells[, j] <- cell_text(m[, j])
      dn <- names(dimnames(v))
      corner <- if (length(dn) == 2L && any(nzchar(dn))) paste(dn[nzchar(dn)], collapse = " \\ ") else ""
      return(list(corner = corner,
                  cols = or_else(colnames(m), sprintf("[,%d]", seq_len(ncol(m)))),
                  num = rep(is.numeric(m), ncol(m)),
                  rows = or_else(rownames(m), sprintf("[%d,]", seq_len(nrow(m)))),
                  cells = cells))
    }
    NULL
  }
  # A table inside what R printed (summary(lm)'s coefficients, an anova table):
  # the `rows` lines that start with their row names, under a header naming
  # `cols` in order. Each column ends where its name ends, as R right-aligns
  # both, so the cells are R's own text (a column after the last, such as the
  # significance stars, too). The lines above and below go with it.
  printed_grid <- function(lines, rows, cols, from = 1L) {
    n <- length(rows)
    if (n == 0L || n > 500L || length(cols) == 0L || length(cols) > 49L ||
        anyNA(rows) || anyNA(cols) || length(lines) - n < from) return(NULL)
    w <- max(nchar(rows))
    for (h in seq.int(from, length(lines) - n)) {
      body <- lines[h + seq_len(n)]
      if (!identical(trimws(substr(body, 1L, w), "right"), rows)) next
      header <- lines[h]
      if (!identical(trimws(substr(header, 1L, w)), "")) return(NULL)
      ends <- integer(length(cols))
      pos <- w
      for (j in seq_along(cols)) {
        k <- regexpr(cols[j], substring(header, pos + 1L), fixed = TRUE)
        if (k < 1L) return(NULL)
        ends[j] <- pos + k + nchar(cols[j]) - 1L
        pos <- ends[j]
      }
      starts <- c(w + 1L, ends[-length(ends)] + 1L)
      cells <- matrix("", n, length(cols))
      for (j in seq_along(cols)) cells[, j] <- trimws(substr(body, starts[j], ends[j]))
      num <- rep(TRUE, length(cols))
      extra <- trimws(substring(body, pos + 1L))
      if (any(nzchar(extra))) {
        cells <- cbind(cells, extra)
        cols <- c(cols, "")
        num <- c(num, FALSE)
      }
      joined <- function(x) if (length(x) == 0L) "" else paste(x, collapse = "\n")
      return(list(g = list(corner = "", cols = cols, num = num, rows = rows, cells = cells),
                  above = joined(lines[seq_len(h - 1L)]),
                  below = joined(lines[-seq_len(h + n)])))
    }
    NULL
  }
  # The printed objects whose table is read this way, or NULL.
  inner_grid <- function(v, lines) {
    if (inherits(v, c("summary.lm", "summary.glm"))) {
      cf <- v$coefficients
      if (!is.matrix(cf)) return(NULL)
      rows <- rownames(cf)
      # R prints a row of NAs for a coefficient left out (aliased)
      if (!is.null(v$aliased) && any(v$aliased)) rows <- names(v$aliased)
      at <- grep("^Coefficients:", lines)
      if (length(at) == 0L || is.null(rows)) return(NULL)
      return(printed_grid(lines, rows, colnames(cf), from = at[1L] + 1L))
    }
    if (inherits(v, "anova") && is.data.frame(v)) {
      return(printed_grid(lines, row.names(v), names(v)))
    }
    if (inherits(v, "coeftest") && is.matrix(v)) {
      return(printed_grid(lines, rownames(v), colnames(v)))
    }
    NULL
  }
  write_table <- function(id, title, g, text, above = "", below = "") {
    table_no <<- table_no + 1L
    name <- sprintf("%s-t%d.rct", id, table_no)
    rows <- vapply(seq_along(g$rows), function(i) {
      paste(vapply(c(g$rows[[i]], g$cells[i, ]), esc, ""), collapse = "\t")
    }, "")
    lines <- c("RCT1", esc(title), esc(g$corner),
               paste(vapply(g$cols, esc, ""), collapse = "\t"),
               paste(as.integer(g$num), collapse = "\t"),
               rows, "TEXT", esc(text), "ABOVE", esc(above), "BELOW", esc(below))
    out <- file(file.path(plot_dir, name), open = "wb")
    on.exit(close(out))
    writeLines(enc2utf8(lines), out, useBytes = TRUE)
    name
  }
  printed <- function(v) {
    text <- utils::capture.output(print(v))
    if (length(text) == 0L) "" else paste0(paste(text, collapse = "\n"), "\n")
  }
  # TRUE when `v` went out as tables; FALSE to print it as usual
  as_tables <- function(id, v) {
    if (!nzchar(plot_dir) || !isTRUE(getOption("rcraft.tables", TRUE)) || sink.number() > 0L) {
      return(FALSE)
    }
    if (inherits(v, c("summary.lm", "summary.glm", "anova", "coeftest"))) {
      lines <- utils::capture.output(print(v))
      inner <- inner_grid(v, lines)
      if (!is.null(inner)) {
        text <- paste0(paste(lines, collapse = "\n"), "\n")
        emit("TABLE", id, esc(write_table(id, "", inner$g, text, inner$above, inner$below)))
        return(TRUE)
      }
    }
    g <- grid_of(v)
    parts <- if (!is.null(g)) {
      list(list(title = "", g = g))
    } else if (is.list(v) && !is.object(v) && length(v) >= 1L && length(v) <= 20L &&
               !is.null(names(v)) && all(nzchar(names(v))) &&
               identical(names(attributes(v)), "names")) {
      gs <- lapply(v, grid_of)
      if (any(vapply(gs, is.null, TRUE))) return(FALSE)
      Map(function(t, g) list(title = t, g = g), names(v), gs)
    } else {
      return(FALSE)
    }
    # all the text goes with the first table, so the log reads as R printed it
    text <- printed(v)
    names <- character()
    for (i in seq_along(parts)) {
      p <- parts[[i]]
      names <- c(names, write_table(id, p$title, p$g, if (i == 1L) text else ""))
    }
    for (name in names) emit("TABLE", id, esc(name))
    TRUE
  }

  run_one <- function(id, n) {
    code <- readChar(con, n, useBytes = TRUE)
    Encoding(code) <- "UTF-8"
    # Sys.sleep() looks for interrupts, so a stale one is taken here
    tryCatch(allowInterrupts(Sys.sleep(0)), interrupt = function(i) NULL)
    emit("BEGIN", id)
    cur_run <<- id
    page_no <<- 0L
    table_no <<- 0L
    status <- "ok"
    tryCatch(
      withCallingHandlers(
        allowInterrupts({
          exprs <- parse(text = code, keep.source = FALSE)
          for (e in exprs) {
            r <- withVisible(eval(e, globalenv()))
            if (r$visible) {
              shown <- tryCatch(as_tables(id, r$value), error = function(err) FALSE)
              if (!shown) print(r$value)
            }
          }
        }),
        warning = function(w) {
          warn <- getOption("warn", 0)
          # warn >= 2: R itself turns the warning into an error
          if (is.numeric(warn) && length(warn) == 1L && warn >= 2) {
            return()
          }
          if (!is.numeric(warn) || length(warn) != 1L || warn >= 0) {
            emit("WARN", id, esc(conditionMessage(w)))
          }
          # a warning raised with signalCondition() has nothing to muffle
          tryInvokeRestart("muffleWarning")
        }
      ),
      interrupt = function(i) status <<- "interrupted",
      error = function(e) {
        status <<- "error"
        call <- conditionCall(e)
        text <- if (is.null(call)) "" else paste(deparse(call, nlines = 1L), collapse = " ")
        # this loop's own calls are not the user's: a top-level stop() or a
        # syntax error should read "Error: message", not name them
        if (text %in% c("eval(e, globalenv())", "parse(text = code, keep.source = FALSE)")) text <- ""
        emit("ERROR", id, esc(conditionMessage(e)), esc(text))
      }
    )
    if (nzchar(plot_dir)) try(snap(), silent = TRUE)
    end_sinks(id)
    flush(stdout())
    # tell the reader of standard error that everything for this run is written
    cat(token, "\tSYNC\t", id, "\n", sep = "", file = stderr())
    flush(stderr())
    emit("END", id, status)
    if (!con_ok()) con <<- file("stdin", "rb")
  }

  emit("READY", R.version.string, Sys.getpid())
  done <- FALSE
  while (!done) {
    tryCatch(
      suspendInterrupts({
        header <- readLines(con, n = 1L, warn = FALSE)
        if (length(header) == 0L) {
          done <- TRUE # the parent closed the pipe
        } else if (nzchar(header)) {
          parts <- strsplit(header, " ", fixed = TRUE)[[1L]]
          if (identical(parts[[1L]], "QUIT")) {
            done <- TRUE
          } else if (identical(parts[[1L]], "RUN") && length(parts) == 3L) {
            run_one(parts[[2L]], as.integer(parts[[3L]]))
          }
        }
      }),
      interrupt = function(i) NULL
    )
  }
  quit(save = "no", status = 0L)
})
