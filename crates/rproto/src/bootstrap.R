# RCraft session loop. Started as: Rscript --vanilla bootstrap.R <token>
#
# Requests arrive on standard input:   RUN <id> <nbytes>\n<code>\n   or   QUIT\n
# Frames leave on standard output as lines that start with <token> and a tab,
# mixed with whatever the user's code prints. Messages and other standard
# error output stay on standard error. See crates/rproto/src/lib.rs.
#
# Interrupts are held off everywhere except while the user's code runs, so one
# that arrives while a request is read or a run is being reported cannot lose
# the END frame. One held off that way waits for the next point where R looks
# for interrupts: so an interrupt that lands just after a run has ended may
# stop the next run instead (it can no longer hang the session).
local({
  token <- commandArgs(trailingOnly = TRUE)[[1L]]
  con <- file("stdin", "rb")

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

  run_one <- function(id, n) {
    code <- readChar(con, n, useBytes = TRUE)
    Encoding(code) <- "UTF-8"
    emit("BEGIN", id)
    status <- "ok"
    tryCatch(
      withCallingHandlers(
        allowInterrupts({
          exprs <- parse(text = code, keep.source = FALSE)
          for (e in exprs) {
            r <- withVisible(eval(e, globalenv()))
            if (r$visible) print(r$value)
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
