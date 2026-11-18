stata_label_dropped <- function(d, nm) {
  for (v in names(d)) {
    x <- d[[v]]
    l <- attr(x, "vallab", exact = TRUE)
    if (!is.null(l) && l %in% nm) {
      lab <- attr(x, "label", exact = TRUE)
      fm <- attr(x, "format.stata", exact = TRUE)
      x <- haven::zap_labels(x)
      attr(x, "vallab") <- l
      if (!is.null(lab)) attr(x, "label") <- lab
      if (!is.null(fm)) attr(x, "format.stata") <- fm
      d[[v]] <- x
    }
  }
  d
}
