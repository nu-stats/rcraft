# cluster: hierarchical linkage, kmeans and kmedians, generate, list, dir, drop
# (plan/stata/cluster.md). The objects live in the data's characteristics, as Stata keeps them.
stata_cl_num <- function(u) { u <- unclass(u); attributes(u) <- NULL; as.numeric(u) }
stata_cl_objects <- function(d) {
  v <- stata_char_get(d, "_dta", "_cluster_objects")
  if (nzchar(v)) strsplit(v, " +")[[1]] else character(0)
}
stata_cl_put <- function(d, name, value) stata_char_put(d, "_dta", name, value)
# the dissimilarity between a point and the rows of a matrix
stata_cl_dist <- function(P, x, measure) {
  df <- sweep(P, 2, x)
  switch(measure, L2 = sqrt(rowSums(df^2)), L2squared = rowSums(df^2), L1 = rowSums(abs(df)),
         Linfinity = apply(abs(df), 1, max))
}
stata_cl_measure <- function(m, default) {
  if (is.null(m)) return(default)
  switch(tolower(m), l2 = , euclidean = "L2", l2squared = "L2squared", l1 = , absolute = , cityblock = , manhattan = "L1",
         linfinity = , maximum = "Linfinity",
         stop(m, " not a valid measure (Stata: r(198))", call. = FALSE))
}
stata_cl_default <- function(d) {
  taken <- stata_cl_objects(d)
  i <- 1L
  while (paste0("_clus_", i) %in% c(taken, names(d))) i <- i + 1L
  paste0("_clus_", i)
}
stata_cl_record <- function(d, name, cmdline, vars, info) {
  objs <- c(name, setdiff(stata_cl_objects(d), name))
  d <- stata_cl_put(d, "_cluster_objects", paste(objs, collapse = " "))
  d <- stata_cl_put(d, paste0(name, "_info"), info)
  d <- stata_cl_put(d, paste0(name, "_cmd"), cmdline)
  d <- stata_cl_put(d, paste0(name, "_vars"), paste(vars, collapse = " "))
  stata_cl_put(d, name, sprintf("\"cmd %s_cmd\" \"vars %s_vars\" \"info %s_info\"", name, name, name))
}
stata_cl_points <- function(d, vars) {
  for (v in vars) {
    if (!v %in% names(d)) stop("variable ", v, " not found (Stata: r(111))", call. = FALSE)
    if (is.character(d[[v]])) stop(v, ": string variable, type mismatch (Stata: r(109))", call. = FALSE)
  }
  P <- vapply(vars, function(v) stata_cl_num(d[[v]]), numeric(nrow(d)))
  P <- matrix(P, nrow(d), length(vars))
  list(P = P, rows = which(stats::complete.cases(P)))
}
stata_cl_add <- function(d, name, values, label = NULL) {
  if (name %in% names(d)) stop("variable ", name, " already defined (Stata: r(110))", call. = FALSE)
  d[[name]] <- values
  if (!is.null(label)) attr(d[[name]], "label") <- label
  d
}
stata_cl_hier <- function(d, vars, linkage, measure = NULL, name = NULL, cmdline = "") {
  pt <- stata_cl_points(d, vars)
  P <- pt$P[pt$rows, , drop = FALSE]
  n <- nrow(P)
  if (n < 2) stop("insufficient observations (Stata: r(2001))", call. = FALSE)
  measure <- stata_cl_measure(measure, if (linkage == "wards") "L2squared" else "L2")
  defaulted <- is.null(name)
  if (defaulted) name <- stata_cl_default(d)
  D <- matrix(0, n, n)
  for (i in seq_len(n)) D[i, ] <- stata_cl_dist(P, P[i, ], measure)
  members <- lapply(seq_len(n), function(i) i)
  joins <- lapply(seq_len(n), function(i) matrix(numeric(0), 0, 3))
  size <- rep(1, n)
  active <- rep(TRUE, n)
  for (step in seq_len(n - 1)) {
    M <- D
    M[!active, ] <- Inf; M[, !active] <- Inf
    M[lower.tri(M, diag = TRUE)] <- Inf
    k <- which.min(t(M))
    # t(M) is scanned down its columns, which is M row by row
    i <- (k - 1) %/% n + 1
    j <- (k - 1) %% n + 1
    v <- D[i, j]
    keep <- if (linkage == "single") j else i
    gone <- if (linkage == "single") i else j
    si <- size[i]; sj <- size[j]
    for (kk in which(active)) {
      if (kk == i || kk == j) next
      dki <- D[kk, i]; dkj <- D[kk, j]; sk <- size[kk]
      nv <- switch(linkage, single = min(dki, dkj), complete = max(dki, dkj),
                   average = (si * dki + sj * dkj) / (si + sj),
                   wards = ((si + sk) * dki + (sj + sk) * dkj - sk * v) / (si + sj + sk))
      D[kk, keep] <- nv; D[keep, kk] <- nv
    }
    left <- members[[i]]; right <- members[[j]]
    allj <- rbind(joins[[i]], joins[[j]], c(left[length(left)], right[1], v))
    joins[[keep]] <- allj
    members[[keep]] <- c(left, right)
    size[keep] <- si + sj
    active[gone] <- FALSE
  }
  root <- which(active)
  ord <- members[[root]]
  jn <- joins[[root]]
  hgt <- c(vapply(seq_len(n - 1), function(t) {
    h <- jn[jn[, 1] == ord[t] & jn[, 2] == ord[t + 1], 3]
    if (length(h)) h[1] else NA_real_
  }, 1), NA_real_)
  nobs <- nrow(d)
  id <- rep(NA_real_, nobs); od <- id; hg <- id
  id[pt$rows] <- seq_len(n); od[pt$rows] <- ord; hg[pt$rows] <- hgt
  if (defaulted) cat("cluster name: ", name, "\n", sep = "")
  d <- stata_cl_add(d, paste0(name, "_id"), id)
  d <- stata_cl_add(d, paste0(name, "_ord"), od)
  d <- stata_cl_add(d, paste0(name, "_hgt"), hg)
  info <- sprintf("`\"t hierarchical\"' `\"m %s\"' `\"d %s\"' `\"o range 0 .\"' `\"v id %s_id\"' `\"v order %s_ord\"' `\"v height %s_hgt\"'",
                  linkage, measure, name, name, name)
  stata_cl_record(d, name, cmdline, vars, info)
}
stata_cl_k <- function(d, vars, k, medians = FALSE, start = "krandom", measure = NULL, name = NULL, cmdline = "") {
  pt <- stata_cl_points(d, vars)
  P <- pt$P[pt$rows, , drop = FALSE]
  n <- nrow(P)
  if (k < 1 || k > n) stop("k() invalid (Stata: r(198))", call. = FALSE)
  measure <- stata_cl_measure(measure, "L2")
  defaulted <- is.null(name)
  if (defaulted) name <- stata_cl_default(d)
  ctr <- switch(start,
    firstk = P[seq_len(k), , drop = FALSE],
    lastk = P[(n - k + 1):n, , drop = FALSE],
    krandom = { u <- stata_runiform(n); P[order(u)[seq_len(k)], , drop = FALSE] },
    stop("start(", start, ") is not translated yet", call. = FALSE))
  grp <- rep(0L, n)
  for (it in 1:10000) {
    dm <- vapply(seq_len(k), function(g) stata_cl_dist(P, ctr[g, ], measure), numeric(n))
    dm <- matrix(dm, n, k)
    ng <- max.col(-dm, ties.method = "first")
    if (identical(as.integer(ng), grp)) break
    grp <- as.integer(ng)
    for (g in seq_len(k)) {
      m <- P[grp == g, , drop = FALSE]
      if (!nrow(m)) next
      ctr[g, ] <- if (medians) apply(m, 2, stats::median) else colMeans(m)
    }
  }
  vals <- rep(NA_real_, nrow(d)); vals[pt$rows] <- grp
  if (defaulted) cat("cluster name: ", name, "\n", sep = "")
  d <- stata_cl_add(d, name, vals, "Cluster ID")
  info <- sprintf("`\"t partition\"' `\"m %s\"' `\"d %s\"' `\"o k %d\"' `\"o start %s\"' `\"o range 0 .\"' `\"v group %s\"'",
                  if (medians) "kmedians" else "kmeans", measure, as.integer(k), start, name)
  stata_cl_record(d, name, cmdline, vars, info)
}
stata_cl_info <- function(d, name, key) {
  info <- stata_char_get(d, "_dta", paste0(name, "_info"))
  items <- regmatches(info, gregexpr("`\"[^\"]*\"'", info))[[1]]
  items <- sub("^`\"", "", sub("\"'$", "", items))
  hit <- items[startsWith(items, paste0(key, " "))]
  if (length(hit)) substring(hit[1], nchar(key) + 2) else NA_character_
}
stata_cl_generate <- function(d, newvar, ks, name = NULL) {
  if (is.null(name)) {
    objs <- stata_cl_objects(d)
    if (!length(objs)) stop("no cluster solutions defined (Stata: r(198))", call. = FALSE)
    name <- objs[1]
  }
  ordv <- stata_cl_info(d, name, "v order")
  if (is.na(ordv)) stop(name, " is not a hierarchical cluster analysis (Stata: r(198))", call. = FALSE)
  hgtv <- stata_cl_info(d, name, "v height"); idv <- stata_cl_info(d, name, "v id")
  ids <- stata_cl_num(d[[idv]])
  rows <- which(!is.na(ids))
  pos <- rows[order(ids[rows])]
  ord <- stata_cl_num(d[[ordv]])[pos]
  order_rows <- pos[ord]
  hgt <- stata_cl_num(d[[hgtv]])[pos]
  for (k in ks) {
    idx <- which(!is.na(hgt))
    idx <- idx[order(-hgt[idx], idx)]
    cuts <- head(idx, max(k - 1, 0))
    vals <- rep(NA_real_, nrow(d)); g <- 1
    for (t in seq_along(order_rows)) {
      vals[order_rows[t]] <- g
      if (t %in% cuts) g <- g + 1
    }
    d <- stata_cl_add(d, if (length(ks) > 1) paste0(newvar, k) else newvar, vals)
  }
  d
}
stata_cl_list <- function(d, names = NULL) {
  if (is.null(names)) names <- stata_cl_objects(d)
  for (nm in names) {
    cat(sprintf("%s  (type: %s,  method: %s,  dissimilarity: %s)\n", nm, stata_cl_info(d, nm, "t"),
                stata_cl_info(d, nm, "m"), stata_cl_info(d, nm, "d")))
    first <- TRUE
    for (kw in list(c("v id", "id"), c("v order", "order"), c("v height", "height"), c("v group", "group"))) {
      v <- stata_cl_info(d, nm, kw[1])
      if (!is.na(v)) { cat(if (first) "      vars: " else "            ", v, " (", kw[2], " variable)\n", sep = ""); first <- FALSE }
    }
    cat("     other: cmd: ", stata_char_get(d, "_dta", paste0(nm, "_cmd")), "\n", sep = "")
    cat("            varlist: ", stata_char_get(d, "_dta", paste0(nm, "_vars")), "\n", sep = "")
    info <- stata_char_get(d, "_dta", paste0(nm, "_info"))
    items <- regmatches(info, gregexpr("`\"[^\"]*\"'", info))[[1]]
    items <- sub("^`\"", "", sub("\"'$", "", items))
    for (it in items[startsWith(items, "o ")]) {
      w <- strsplit(substring(it, 3), " ", fixed = TRUE)[[1]]
      cat("            ", w[1], ": ", paste(w[-1], collapse = " "), "\n", sep = "")
    }
    cat("\n")
  }
  invisible(NULL)
}
stata_cl_dir <- function(d) { for (nm in stata_cl_objects(d)) cat(nm, "\n", sep = ""); invisible(NULL) }
stata_cl_drop <- function(d, names) {
  all <- stata_cl_objects(d)
  if (identical(names, "_all")) names <- all
  for (nm in names) {
    for (key in c("v id", "v order", "v height")) {
      v <- stata_cl_info(d, nm, key)
      if (!is.na(v) && v %in% names(d)) d[[v]] <- NULL
    }
    for (sfx in c("", "_info", "_cmd", "_vars")) d <- stata_cl_put(d, paste0(nm, sfx), "")
  }
  stata_cl_put(d, "_cluster_objects", paste(setdiff(all, names), collapse = " "))
}
