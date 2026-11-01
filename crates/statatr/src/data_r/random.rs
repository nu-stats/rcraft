//! Stata's random numbers in R (P2C.11): `set seed`, `runiform()`,
//! `rnormal()` and `rgamma()` give Stata 19's own draws.
//!
//! The generator is Stata's default, the 64-bit Mersenne Twister (mt64) as
//! Matsumoto and Nishimura published it: `set seed #` is `init_genrand64`
//! of |#|, and a uniform is the top 52 bits of an output plus one half, over
//! 2^52. R has no unsigned 64-bit integers, so each word is four 16-bit
//! pieces in integer vectors (`bitwXor` and `bitwAnd` work on them, and a
//! shift moves bits between pieces). Its state lives in R (`stata_rng_state`,
//! made at Stata's starting seed 123456789 when first used), so it follows
//! the do-file through every line, `preserve` and `restore` included, and
//! typed lines in Stata mode go on from where the last draw left it.
//!
//! The variates are StatCraft's decoding of Stata's (our own project, its
//! `plan/stata/random-numbers.md`): `rnormal()` is a rectangle-wedge-tail
//! method with 1/8-wide strips and an alias table (the constants are the
//! doubles Stata's draws show, written as exact hexadecimal literals), and
//! `rgamma()` is Best's RGS below 1, −ln u at 1 and Schmeiser and Lal's G4PE
//! above. The design and what was observed in Stata 19 for it, with made-up
//! data, are in this repository's `plan/stata/random-numbers.md`: a row draws
//! only when the `if` and `in`
//! select it and its arguments are valid (a missing argument, a ≥ b,
//! s < 0 or a shape or scale not above 0 give missing and use no uniform;
//! `rnormal(m, 0)` is m), `runiform(a, b)` is a + b·u − a·u, and a G4PE
//! left-tail point at or below 0 is turned down before the second uniform
//! is drawn. 240,000 draws of every kind matched Stata's bit for bit.
//!
//! The variates draw a pool of uniforms ahead and hand back what they did
//! not use (`stata_rng_done`), so the next draw gets them in order.
//! `rnormal()` is worked out for whole vectors: only its rare wedge and tail
//! draws, which take more than three uniforms, go one at a time.

/// `stata_rng`: the generator and the variates, defined once.
pub(super) const RNG_HELPER: &str = r#"stata_set_seed <- function(seed) {
  seed <- abs(seed)
  w <- c(seed %% 65536, seed %/% 65536, 0, 0)
  m <- matrix(0L, 312, 4)
  m[1, ] <- as.integer(w)
  for (i in 2:312) {
    x <- w
    x[1] <- bitwXor(x[1], w[4] %/% 16384)
    p <- c(x[1] * 32557 + (i - 1), x[1] * 19605 + x[2] * 32557,
      x[1] * 62509 + x[2] * 19605 + x[3] * 32557,
      x[1] * 22609 + x[2] * 62509 + x[3] * 19605 + x[4] * 32557)
    for (k in 1:3) {
      p[k + 1] <- p[k + 1] + p[k] %/% 65536
      p[k] <- p[k] %% 65536
    }
    p[4] <- p[4] %% 65536
    w <- p
    m[i, ] <- as.integer(w)
  }
  s <- new.env()
  s$w <- lapply(1:4, function(k) m[, k])
  s$i <- 312L
  s$buf <- NULL
  stata_rng_state <<- s
  invisible(NULL)
}
stata_rng_twist <- function(w) {
  step <- function(k, n0, n1, f) {
    x1 <- bitwAnd(w[[2]][k], 32768L) + bitwAnd(n1, 32767L)
    x2 <- w[[3]][k]
    x3 <- w[[4]][k]
    odd <- -bitwAnd(n0, 1L)
    list(bitwXor(bitwXor(f[[1]], n0 %/% 2L + bitwAnd(x1, 1L) * 32768L), bitwAnd(6633L, odd)),
      bitwXor(bitwXor(f[[2]], x1 %/% 2L + bitwAnd(x2, 1L) * 32768L), bitwAnd(43366L, odd)),
      bitwXor(bitwXor(f[[3]], x2 %/% 2L + bitwAnd(x3, 1L) * 32768L), bitwAnd(28506L, odd)),
      bitwXor(bitwXor(f[[4]], x3 %/% 2L), bitwAnd(46338L, odd)))
  }
  at <- function(i) lapply(w, function(v) v[i])
  for (b in list(list(1:156, 2:157, 157:312), list(157:311, 158:312, 1:155), list(312L, 1L, 156L))) {
    y <- step(b[[1]], w[[1]][b[[2]]], w[[2]][b[[2]]], at(b[[3]]))
    for (j in 1:4) w[[j]][b[[1]]] <- y[[j]]
  }
  w
}
stata_rng_temper <- function(x) {
  x0 <- x[[1]]
  x1 <- x[[2]]
  x2 <- x[[3]]
  x3 <- x[[4]]
  x0 <- bitwXor(x0, bitwAnd(x1 %/% 8192L + (x2 %% 8192L) * 8L, 21845L))
  x1 <- bitwXor(x1, bitwAnd(x2 %/% 8192L + (x3 %% 8192L) * 8L, 21845L))
  x2 <- bitwXor(x2, bitwAnd(x3 %/% 8192L, 21845L))
  x3 <- bitwXor(x3, bitwAnd((x2 %% 32768L) * 2L + x1 %/% 32768L, 29142L))
  x2 <- bitwXor(x2, bitwAnd((x1 %% 32768L) * 2L + x0 %/% 32768L, 32767L))
  x1 <- bitwXor(x1, bitwAnd((x0 %% 32768L) * 2L, 60838L))
  x3 <- bitwXor(x3, bitwAnd((x1 %% 2048L) * 32L + x0 %/% 2048L, 65527L))
  x2 <- bitwXor(x2, bitwAnd((x0 %% 2048L) * 32L, 61152L))
  x0 <- bitwXor(x0, x2 %/% 2048L + (x3 %% 2048L) * 32L)
  x1 <- bitwXor(x1, x3 %/% 2048L)
  list(x0, x1, x2, x3)
}
stata_rng_words <- function(n) {
  if (!exists("stata_rng_state")) stata_set_seed(123456789)
  s <- stata_rng_state
  old <- NULL
  if (!is.null(s$buf)) {
    have <- min(n, length(s$buf[[1]]))
    old <- lapply(s$buf, function(v) v[seq_len(have)])
    s$buf <- if (have < length(s$buf[[1]])) lapply(s$buf, function(v) v[-seq_len(have)])
    if (have == n) return(old)
    n <- n - have
  }
  raw <- lapply(1:4, function(k) integer(n))
  done <- 0L
  while (done < n) {
    if (s$i >= 312L) {
      s$w <- stata_rng_twist(s$w)
      s$i <- 0L
    }
    take <- min(n - done, 312L - s$i)
    for (k in 1:4) raw[[k]][done + seq_len(take)] <- s$w[[k]][s$i + seq_len(take)]
    s$i <- s$i + take
    done <- done + take
  }
  new <- stata_rng_temper(raw)
  if (is.null(old)) new else lapply(1:4, function(k) c(old[[k]], new[[k]]))
}
stata_rng_u <- function(w) (w[[4]] * 68719476736 + w[[3]] * 1048576 + w[[2]] * 16 + w[[1]] %/% 4096L + 0.5) / 4503599627370496
stata_rng_pool <- function(n) {
  p <- new.env()
  p$w <- stata_rng_words(n)
  p$u <- stata_rng_u(p$w)
  p
}
stata_rng_more <- function(p, n) {
  w <- stata_rng_words(n)
  p$w <- lapply(1:4, function(k) c(p$w[[k]], w[[k]]))
  p$u <- c(p$u, stata_rng_u(w))
}
stata_rng_done <- function(p, used) {
  if (used >= length(p$u)) return(invisible(NULL))
  w <- lapply(p$w, function(v) v[-seq_len(used)])
  s <- stata_rng_state
  s$buf <- if (is.null(s$buf)) w else lapply(1:4, function(k) c(w[[k]], s$buf[[k]]))
  invisible(NULL)
}
stata_rng_rows <- function(rows) if (is.logical(rows)) !is.na(rows) & rows else rep(TRUE, rows)
stata_rng_out <- function(x) {
  x[!is.finite(x)] <- NA_real_
  x
}
stata_runiform <- function(rows, a = 0, b = 1) {
  sel <- stata_rng_rows(rows)
  n <- length(sel)
  a <- rep_len(as.numeric(a), n)
  b <- rep_len(as.numeric(b), n)
  out <- rep(NA_real_, n)
  need <- sel & !is.na(a) & !is.na(b) & a < b
  out[need] <- a[need] + (b[need] - a[need]) * stata_rng_u(stata_rng_words(sum(need)))
  stata_rng_out(out)
}
stata_rnormal <- function(rows, m = 0, s = 1) {
  sel <- stata_rng_rows(rows)
  n <- length(sel)
  m <- rep_len(as.numeric(m), n)
  s <- rep_len(as.numeric(s), n)
  out <- rep(NA_real_, n)
  ok <- sel & !is.na(m) & !is.na(s) & s >= 0
  out[ok & s == 0] <- m[ok & s == 0]
  need <- ok & s > 0
  out[need] <- m[need] + s[need] * stata_rnorm_std(sum(need))
  stata_rng_out(out)
}
stata_rgamma <- function(rows, a, b) {
  sel <- stata_rng_rows(rows)
  n <- length(sel)
  a <- rep_len(as.numeric(a), n)
  b <- rep_len(as.numeric(b), n)
  out <- rep(NA_real_, n)
  need <- sel & !is.na(a) & !is.na(b) & a > 0 & b > 0
  out[need] <- b[need] * stata_rgamma_std(a[need])
  stata_rng_out(out)
}
stata_rnorm_std <- function(n) {
  z <- numeric(n)
  if (n == 0) return(z)
  tb <- stata_rnorm_tables
  p <- stata_rng_pool(3L * n + 16L)
  pos <- 1L
  k <- 0L
  starts <- integer(n)
  repeat {
    P <- length(p$u)
    j <- seq_len(max(P - 2L, 0L))
    box <- floor(p$u[j + 1L] * 64)
    slow <- rep(Inf, P)
    slow[which(box > 31 & p$u[j + 2L] < tb$t[pmax(box - 30, 1)])] <- 0
    slow[slow == 0] <- which(slow == 0)
    for (r in 1:3) {
      i <- seq(r, P, by = 3L)
      slow[i] <- rev(cummin(rev(slow[i])))
    }
    more <- FALSE
    while (k < n) {
      nxt <- if (pos <= P) slow[pos] else Inf
      run <- as.integer(min((P - pos + 1L) %/% 3L, (nxt - pos) %/% 3, n - k))
      if (run > 0L) {
        starts[k + seq_len(run)] <- pos + 3L * (seq_len(run) - 1L)
        k <- k + run
        pos <- pos + 3L * run
      }
      if (k == n) break
      v <- if (pos == nxt) stata_rnorm_slow(p$u, pos, tb)
      if (is.null(v)) {
        more <- TRUE
        break
      }
      k <- k + 1L
      z[k] <- v[1]
      pos <- as.integer(v[2])
    }
    if (!more) break
    stata_rng_more(p, 3L * (n - k) + 64L)
  }
  r <- which(starts > 0L)
  s <- starts[r]
  b <- floor(p$u[s + 1L] * 64)
  v <- p$u[s + 2L]
  x <- numeric(length(s))
  lo <- b < 31
  i <- b[lo] + 1
  x[lo] <- ifelse(v[lo] < tb$p[i], tb$a1[i] + v[lo] * tb$c1[i], tb$a2[i] + v[lo] * tb$c2[i])
  i <- b[!lo] - 30
  x[!lo] <- ifelse(v[!lo] >= tb$t[i], tb$a[i] + v[!lo] * tb$c[i], tb$strip[1] + v[!lo] * tb$strip[2])
  z[r] <- ifelse(p$u[s] < 0.5, 1, -1) * x
  stata_rng_done(p, pos - 1L)
  z
}
stata_rnorm_slow <- function(u, pos, tb) {
  sgn <- if (u[pos] < 0.5) 1 else -1
  b <- floor(u[pos + 1L] * 64)
  q <- pos + 3L
  repeat {
    if (q + 1L > length(u)) return(NULL)
    s <- u[q]
    w <- u[q + 1L]
    q <- q + 2L
    if (b == 63) {
      x <- sqrt(3.875 * 3.875 - 2 * log(w))
      if (x * s < 3.875) return(c(sgn * x, q))
    } else {
      x <- (b - 32 + min(s, w)) / 8
      right <- (b - 31) / 8
      if (max(s, w) - min(s, w) <= tb$wedge[b - 31] * (exp((right * right - x * x) / 2) - 1)) return(c(sgn * x, q))
    }
  }
}
stata_rgamma_std <- function(a) {
  n <- length(a)
  x <- numeric(n)
  if (n == 0) return(x)
  p <- stata_rng_pool(2L * n + 16L)
  at <- 0L
  u <- function() {
    if (at >= length(p$u)) stata_rng_more(p, 2L * n + 64L)
    at <<- at + 1L
    p$u[at]
  }
  for (i in seq_len(n)) x[i] <- stata_rgamma_one(a[i], u)
  stata_rng_done(p, at)
  x
}
stata_rng_pow <- function(x, e) {
  if (e != trunc(e) || abs(e) >= 64) return(exp(e * log(x)))
  r <- 1
  for (i in seq_len(abs(e))) r <- r * x
  if (e < 0) 1 / r else r
}
stata_rgamma_one <- function(a, u) {
  if (a == 1) return(-log(u()))
  if (a < 1) {
    t <- 0.07 + 0.75 * sqrt(1 - a)
    b <- 1 + exp(-t) * a / t
    repeat {
      u1 <- u()
      u2 <- u()
      v <- b * u1
      if (v <= 1) {
        x <- t * stata_rng_pow(v, 1 / a)
        if (u2 <= (2 - x) / (2 + x) || u2 <= exp(-x)) return(x)
      } else {
        x <- -log(t * (b - v) / a)
        y <- x / t
        if (u2 * (a + y * (1 - a)) <= 1 || u2 <= stata_rng_pow(y, a - 1)) return(x)
      }
    }
  }
  x3 <- a - 1
  d <- sqrt(x3)
  f <- function(x) exp(x3 * log(x / x3) - (x - x3))
  x1 <- 0
  x2 <- 0
  f1 <- 0
  f2 <- 0
  l1 <- 0
  if (d < x3) {
    x2 <- x3 - d
    x1 <- x2 * (1 - 1 / d)
    f1 <- f(x1)
    f2 <- f(x2)
    l1 <- (x1 - x3) / x1
  }
  x4 <- x3 + d
  x5 <- x4 * (1 + 1 / d)
  f4 <- f(x4)
  f5 <- f(x5)
  l5 <- (x5 - x3) / x5
  area <- c(f1 * (x2 - x1), f2 * (x3 - x2), f4 * (x4 - x3), f5 * (x5 - x4),
    (1 - f2) * (x3 - x2), (1 - f4) * (x4 - x3), (f2 - f1) * (x2 - x1) / 2,
    (f4 - f5) * (x5 - x4) / 2, if (l1 == 0) 0 else -f1 / l1, f5 / l5)
  p <- numeric(10)
  total <- 0
  for (i in 1:10) {
    total <- total + area[i]
    p[i] <- total
  }
  repeat {
    v0 <- u() * p[10]
    if (v0 <= p[4]) {
      if (v0 <= p[1]) return(x1 + v0 / f1)
      if (v0 <= p[2]) return(x2 + (v0 - p[1]) / f2)
      if (v0 <= p[3]) return(x3 + (v0 - p[2]) / f4)
      return(x4 + (v0 - p[3]) / f5)
    }
    if (v0 > p[8] && v0 <= p[9]) {
      x <- x1 - log((p[9] - v0) / (p[9] - p[8])) / l1
      if (x <= 0) next
    }
    w <- u()
    if (v0 <= p[5]) {
      x <- x2 + (x3 - x2) * w
      v <- f2 + (v0 - p[4]) / (x3 - x2)
    } else if (v0 <= p[6]) {
      x <- x3 + (x4 - x3) * w
      v <- f4 + (v0 - p[5]) / (x4 - x3)
    } else if (v0 <= p[8]) {
      hi <- max(w, u())
      if (v0 <= p[7]) {
        x <- x1 + (x2 - x1) * hi
        v <- f1 + (f2 - f1) * ((v0 - p[6]) / (p[7] - p[6])) * hi
      } else {
        x <- x5 - (x5 - x4) * hi
        v <- f5 + (f4 - f5) * ((v0 - p[7]) / (p[8] - p[7])) * hi
      }
    } else if (v0 <= p[9]) {
      v <- w * f1 * exp(-l1 * (x - x1))
    } else {
      x <- x5 - log((p[10] - v0) / (p[10] - p[9])) / l5
      v <- w * f5 * exp(-l5 * (x - x5))
    }
    if (x > 0 && v <= f(x)) return(x)
  }
}
stata_rnorm_tables <- list(
    p = c(0x1p+1, 0x1.b07fae0e23834p-1, 0x1.3e4f977c1a97ep-1, 0x1.830166540b04ap-1, 0x1.e5507039f0e84p-3, 0x1.e165c5dc507fcp-1, 0x1.307d70a133e62p-2, 0x1.afaaa3be5180fp-1, 0x1.b0b3d5b312143p-1, 0x1.373f38cb86e42p-2, 0x1.f34efc33b56f9p-3, 0x1.7744773f6183p-1, 0x1.9f29ff517e78ep-1, 0x1.09b8128541db3p-1, 0x1.cc80b95d24a0bp-1, 0x1p+1, 0x1.ba4b436e83ad4p-1, 0x1.55c73f6773b1p-1, 0x1.0402dfd3dc1a1p-1, 0x1.857a942835009p-2, 0x1.1f2f0557f5256p-2, 0x1.a0f22be9d3247p-3, 0x1.29fa54c6341e4p-3, 0x1.a34ea57d8ce38p-4, 0x1.227213fd77688p-4, 0x1.8c2226d7ae53p-5, 0x1.09f38e18a282p-5, 0x1.5f90f6ce87b4p-6, 0x1.c9897d147e6p-7, 0x1.251bf7a2b0fcp-7, 0x1.71b92ecaaa78p-8),
    a1 = c(0, 0, 0x1p-3, 0x1p-2, 0x1.8p-2, 0x1p-1, 0x1.4p-1, 0x1.8p-1, 0x1.cp-1, 0x1p+0, 0x1.2p+0, 0x1.4p+0, 0x1.6p+0, 0x1.8p+0, 0x1.ap+0, 0x1.cp+0, 0x1.ep+0, 0x1p+1, 0x1.1p+1, 0x1.2p+1, 0x1.3p+1, 0x1.4p+1, 0x1.5p+1, 0x1.6p+1, 0x1.7p+1, 0x1.8p+1, 0x1.9p+1, 0x1.ap+1, 0x1.bp+1, 0x1.cp+1, 0x1.dp+1),
    c1 = c(0x1p-3, 0x1.2f0eb9dd10af6p-3, 0x1.9bc605040cf63p-3, 0x1.52aebbd3524eep-3, 0x1.0e139a61264f9p-1, 0x1.10461fe0ac69ep-3, 0x1.ae76cc4cce6dep-2, 0x1.2fa44b3ec6438p-3, 0x1.2eea3298d0891p-3, 0x1.a51e74f3739e5p-2, 0x1.0681cc9d6da38p-1, 0x1.5d46b74f29dc2p-3, 0x1.3bb61a80d6e6dp-3, 0x1.ed45dd5163429p-3, 0x1.1ca0c4be44c39p-3, 0x1p-3, 0x1.28588b0141e5dp-3, 0x1.7f800362438a6p-3, 0x1.f819ef4f1ea76p-3, 0x1.5088227527af1p-2, 0x1.c8679f2c8b8bbp-2, 0x1.3a5cb047ca645p-1, 0x1.b7df21fd703dcp-1, 0x1.389780c5d77fep+0, 0x1.c3476ac6eb2f5p+0, 0x1.4ae0e0ec97edbp+1, 0x1.ecd789ff0fbb6p+1, 0x1.74d2b1ff00fadp+2, 0x1.1e79144c3b0d3p+3, 0x1.bf2d8fd6a8459p+3, 0x1.628381d9c1e2bp+4),
    a2 = c(0, -0x1.1c2b6c1613187p-1, 0x1.6d258269c00dp-5, -0x1.89f32be3a04fcp-7, 0x1.d83f6eee388bbp-2, -0x1.5761c94570dbp+0, 0x1.64ea71358200bp-1, 0x1.a066863d717efp-3, 0x1.458afa7efce0cp-2, 0x1.1206b1e38647ap+0, 0x1.35aed81a03b7ap+0, 0x1.082cc279146eap+0, 0x1.ed9cff191eae9p-1, 0x1.7d796f626d65ep+0, 0x1.43b1762173cd4p-1, 0, -0x1.5616b144c97c3p-1, -0x1.0201f45db9476p-3, -0x1.04ce1b06b713cp-8, 0x1.62dcffafa9f06p-3, 0x1.9c387f75d846p-3, 0x1.5f4785f7a9e86p-2, 0x1.6a34901d88bdcp-2, 0x1.38b37dfcdfe15p-1, 0x1.3b1d8c6dbd6f5p-1, 0x1.bcbf79bc497e3p-1, 0x1.fdda3fecfd93p-1, 0x1.fe98b97484455p-1, 0x1.1f8bfef87377p+0, 0x1.3fb60fb1dcbaep+0, 0x1.5fd185ba49534p+0),
    c2 = c(0, 0x1.9c2b6c1613187p-1, 0x1.525b4fb2c7fe6p-2, 0x1.0627ccaf8e814p-1, 0x1.4f8122238ee8ap-3, 0x1.0bb0e4a2b86d8p+1, 0x1.6c563b29f7fd4p-3, 0x1.97e65e70a3a04p-1, 0x1.9d3a82c0818fap-1, 0x1.6fca70e3cdc33p-3, 0x1.52893f2fe2432p-3, 0x1.df4cf61bae457p-2, 0x1.526300e6e1517p-1, 0x1.0a1a42764a688p-2, 0x1.3e2744ef46196p+0, 0, 0x1.d616b144c97c3p-1, 0x1.8100fa2edca3bp-2, 0x1.0413386c1adc5p-2, 0x1.9d230050560fap-3, 0x1.63c7808a27bap-3, 0x1.4170f410ac2f5p-3, 0x1.2b96dfc4ee849p-3, 0x1.1d32080c807abp-3, 0x1.1389ce490a42cp-3, 0x1.0d02190eda073p-3, 0x1.0897004c09b4p-3, 0x1.059d1a2deeeabp-3, 0x1.03a0083c64481p-3, 0x1.024f82711a294p-3, 0x1.0173d22db5661p-3),
    t = c(0x1.cb22072d20ap-9, 0x1.0f125032ebdfp-5, 0x1.4cb4b99b348b8p-4, 0x1.014efdb7683ep-3, 0x1.50bb43fca5c64p-3, 0x1.91934de5f78dcp-3, 0x1.c1d6a7308f76p-3, 0x1.e0a5c2f4c774p-3, 0x1.ee3c41ec7e48p-3, 0x1.ebd2b3e5c32ap-3, 0x1.db6d28b12458p-3, 0x1.bf9d9a2882f2p-3, 0x1.9b41d64709db4p-3, 0x1.71441e5cd6588p-3, 0x1.4464615fb7894p-3, 0x1.170e193349d18p-3, 0x1.d6797c901ed58p-4, 0x1.84dd9a7112f78p-4, 0x1.3b4b7f2ae96e8p-4, 0x1.f5c109c270f9p-5, 0x1.87edb6920cd3p-5, 0x1.2c9ee7bf5eeep-5, 0x1.c4f6a62b7a0ep-6, 0x1.4f40902bb872p-6, 0x1.e79f838846cp-7, 0x1.5c81b8b68eb8p-7, 0x1.e9a3024b93f8p-8, 0x1.521e0a915928p-8, 0x1.cb17a65bbe9p-9, 0x1.32726cb86c1p-9, 0x1.92488ead064p-10, 0x1.03a90b15fecp-10, 0x1.bf37663a4c08p-8),
    a = c(0x1.7fe3340cccb5p+0, -0x1.1859238899a87p-8, 0x1.d2bc29ea12871p-4, 0x1.db36fb5da5ab4p-3, 0x1.66d024174699bp-2, -0x1.f385cebb0d54cp-6, 0x1.6fe2d6b6d5dacp-4, 0x1.d8bf4c656f3b5p-2, 0x1.ae91b8b64adfep-3, 0x1.2bc5e82f64cbfp-1, 0x1.594d74477d11dp-2, -0x1.1e67bb5b920b8p-5, 0x1.7f5a1d58337d6p-4, 0x1.e3d81eef370e4p-2, 0x1.73f471d3feaefp-1, 0x1.d79d8c25b90aep-3, 0x1.37b1e49985e7ep-1, 0x1.7292bfe31ab42p-2, 0x1.baa9a4ae43533p-1, -0x1.0b3ee1807dc94p-7, 0x1.e6460d6f10c81p-4, 0x1.fb1fb56f34a36p-2, 0x1.7e2e28535278dp-1, 0x1.faa6fa60d2583p-3, 0x1.ff08816620d76p-1, 0x1.3f4fdf990e054p-1, 0x1.7f0956b870864p-2, 0x1.bfab08444d4dep-1, 0x1.1fe334b40529cp+0, -0x1.332a45a1acc77p-12, 0x1.ff368c9493a1dp-4, 0x1.ffdf82a18a911p-2, 0x1.7f8f6d7e4478bp-1),
    c = c(0x1.00e65f999a58p-3, 0x1.08c2c91c44cd4p-3, 0x1.16a1eb0af6bc7p-3, 0x1.24c904a25a54cp-3, 0x1.325fb7d172ccap-3, 0x1.3e70b9d761aa9p-3, 0x1.480e94a49512ap-3, 0x1.4e81673521896p-3, 0x1.516e4749b5202p-3, 0x1.50e85f426cd03p-3, 0x1.4d65177105dc6p-3, 0x1.4799eed6e482ep-3, 0x1.4052f153e6415p-3, 0x1.384fc22191e38p-3, 0x1.302e38b005445p-3, 0x1.286273da46f52p-3, 0x1.21386d99e860ap-3, 0x1.1ada8039ca97cp-3, 0x1.15596d46f2b34p-3, 0x1.10b3ee1807dc9p-3, 0x1.0cdcf948779bfp-3, 0x1.09c0952196b94p-3, 0x1.07475eb2b61ccp-3, 0x1.0559059f2da7dp-3, 0x1.03ddfa677ca28p-3, 0x1.02c0819bc7eb1p-3, 0x1.01ed528f1ef38p-3, 0x1.0153deeecac89p-3, 0x1.00e65a5fd6b22p-3, 0x1.00999522d0d66p-3, 0x1.0064b9b5b62f2p-3, 0x1.0040fabceaddep-3, 0x1.01c24a06ee1d5p-3),
    strip = c(0x1.ep+1, 0x1.1d7a2c7dba69p+5),
    wedge = c(0x1p+6, 0x1p+5, 0x1.5555555555555p+4, 0x1p+4, 0x1.999999999999ap+3, 0x1.5555555555555p+3, 0x1.2492492492492p+3, 0x1p+3, 0x1.c2972992fe516p+2, 0x1.8ff30369d1767p+2, 0x1.66f847d0b6814p+2, 0x1.4521b92096071p+2, 0x1.28b893c8848a3p+2, 0x1.1088153c15f3fp+2, 0x1.f7614f79bcc15p+1, 0x1.d318d11740d41p+1, 0x1.b33b57b21bbf3p+1, 0x1.9706feb95a519p+1, 0x1.7de3cdc8a3f68p+1, 0x1.6758f8d158984p+1, 0x1.530545ae48aacp+1, 0x1.409990e781d5ap+1, 0x1.2fd4c8a84bf16p+1, 0x1.2080ee70e5e92p+1, 0x1.1270d472f03c1p+1, 0x1.057e63145578fp+1, 0x1.f3128543afb47p+0, 0x1.dceb9f618c6d5p+0, 0x1.c8588fafd3152p+0, 0x1.b53042c26b18p+0, 0x1.a34f082ef0dbfp+0)
)"#;

use super::{in_range, no, Out, Session};
use crate::expr_r::{self, Kind, Untranslated, Use};
use crate::lang::expr::Expr;
use crate::reader::Command;

/// Whether an expression has one value for the whole data (`_N > 5`,
/// `x[1] == 2`): no variable, no `_n`, no draw.
fn same_for_every_row(e: &Expr) -> bool {
    match e {
        Expr::Number(_) | Expr::Text(_) => true,
        Expr::Name(n) => n == "_N",
        Expr::Subscript(_, i) => same_for_every_row(i),
        Expr::Call(name, args) => {
            !RANDOM_FUNCTIONS.contains(&name.as_str()) && args.iter().all(same_for_every_row)
        }
        Expr::Unary(_, a) => same_for_every_row(a),
        Expr::Binary(_, a, b) => same_for_every_row(a) && same_for_every_row(b),
    }
}

/// Stata's random-number functions: each one moves the generator on.
const RANDOM_FUNCTIONS: &[&str] = &[
    "rbeta",
    "rbinomial",
    "rcauchy",
    "rchi2",
    "rexponential",
    "rgamma",
    "rgeometric",
    "rhypergeometric",
    "rigaussian",
    "rlaplace",
    "rlogistic",
    "rnbinomial",
    "rnormal",
    "rpareto",
    "rpoisson",
    "rt",
    "runiform",
    "runiformint",
    "rweibull",
    "rweibullph",
    "uniform",
];

/// Commands that draw from the generator, or run code that may.
const DRAWING_COMMANDS: &[&str] = &[
    "bayes",
    "bayesmh",
    "bootstrap",
    "bsample",
    "bsqreg",
    "cluster",
    "dsregress",
    "elasticnet",
    "lasso",
    "mata",
    "mi",
    "permute",
    "poregress",
    "sample",
    "simulate",
    "splitsample",
    "sqrtlasso",
    "xporegress",
];

/// Whether a line Stata runs, which was not translated, may have drawn from
/// the generator: then the draws after it are not Stata's until the next
/// `set seed`.
pub(super) fn moves_generator(stata: &str, command: &str, reason: &str) -> bool {
    let lower = stata.to_ascii_lowercase();
    let calls = RANDOM_FUNCTIONS.iter().any(|f| {
        lower.match_indices(&format!("{f}(")).any(|(i, _)| {
            !lower
                .get(..i)
                .and_then(|s| s.chars().next_back())
                .is_some_and(|c| c.is_ascii_alphanumeric() || c == '_')
        })
    });
    let set = command == "set"
        && lower
            .split_whitespace()
            .nth(1)
            .is_some_and(|w| w.starts_with("seed") || w.starts_with("rng"));
    calls
        || set
        || DRAWING_COMMANDS.contains(&command)
        || lower.contains("bootstrap")
        || reason.contains("a program the do-file defines")
}

impl Session {
    /// An expression that may take Stata's random draws, one for each of
    /// `rows` ([`expr_r::translate_drawing`]); also how many it takes.
    pub(super) fn drawing(
        &self,
        text: &str,
        use_: Use,
        rows: &str,
        out: &mut Out,
    ) -> Result<(String, Kind, usize), Untranslated> {
        self.esample_ok(text)?;
        self.rc_ok(text)?;
        let kinds = |n: &str| self.kind_of(n);
        let ret = |w: &str, n: &str| self.returned_value(w, n);
        let r = expr_r::translate_drawing(text, &kinds, &ret, use_, Some(rows))?;
        if r.draws > 0 {
            if let Some(why) = self.rng_old.as_ref().or(self.rng_lost.as_ref()) {
                return Err(no(why.clone()));
            }
            self.drew.set(true);
        }
        for c in r.caveats {
            out.caveat(c);
        }
        out.helpers.extend(r.helpers);
        Ok((r.code, r.kind, r.draws))
    }

    /// The `if` and `in` of a command as one condition, where the `if` may
    /// draw (for every row, as Stata's does); also how many draws it takes.
    pub(super) fn drawing_condition(
        &self,
        c: &Command,
        out: &mut Out,
    ) -> Result<Option<(String, usize)>, Untranslated> {
        let mut parts = Vec::new();
        let mut draws = 0;
        if let Some(e) = &c.parts.if_exp {
            let (code, _, d) = self.drawing(e, Use::Condition, "dplyr::n()", out)?;
            parts.push(code);
            draws = d;
        }
        if let Some(r) = &c.parts.in_range {
            if draws > 0 {
                return Err(no(
                    "a random draw in an if together with in: not translated yet",
                ));
            }
            parts.push(in_range(r)?);
        }
        Ok(match parts.len() {
            0 => None,
            1 => parts.pop().map(|p| (p, draws)),
            _ => Some((format!("({})", parts.join(" & ")), draws)),
        })
    }

    /// The condition and value of `generate`, `replace` and `expand`: the
    /// value draws for the rows the condition selects (every row without
    /// one), as Stata does. Draws in both are refused.
    pub(super) fn drawing_value(
        &self,
        c: &Command,
        exp: &str,
        out: &mut Out,
    ) -> Result<(Option<String>, String, Kind), Untranslated> {
        let cond = self.drawing_condition(c, out)?;
        // a condition that is the same for every row (`if _N > 5`) is one
        // value in R, and Stata still draws for each row
        let rows = cond
            .as_ref()
            .map(|(c, _)| format!("rep_len({c}, dplyr::n())"));
        let (value, kind, draws) = self.drawing(
            exp.trim(),
            Use::Value,
            rows.as_deref().unwrap_or("dplyr::n()"),
            out,
        )?;
        if draws > 0 && cond.as_ref().is_some_and(|(_, d)| *d > 0) {
            return Err(no(
                "random draws in both the value and the if: Stata takes them in turn for each \
                 row; not translated yet",
            ));
        }
        // where the value draws, the condition picks the same rows; so it
        // does where the condition is one value, which dplyr::if_else would
        // not take
        let one_value = c
            .parts
            .if_exp
            .as_deref()
            .and_then(|e| crate::lang::expr::parse(e).ok())
            .is_some_and(|e| same_for_every_row(&e));
        let cond = if draws > 0 || one_value {
            rows
        } else {
            cond.map(|(c, _)| c)
        };
        Ok((cond, value, kind))
    }

    /// `version #`: before 14, Stata draws from its older generator, KISS32
    /// (Stata 19 gives other numbers for `uniform()` under `version 13`).
    pub(super) fn version(&mut self, c: &Command) {
        let v = c.parts.main.split_whitespace().next().unwrap_or("");
        self.rng_old = v.parse::<f64>().ok().filter(|v| *v < 14.0).map(|_| {
            format!(
                "under version {v} Stata draws from its older generator (KISS32), which is not \
                 translated; Stata 14 and later use the 64-bit Mersenne Twister"
            )
        });
    }

    /// `set seed #`: Stata's generator started again from # (its absolute
    /// value, as Stata 19 does with a negative seed).
    pub(super) fn set_seed(&mut self, rest: &str, out: &mut Out) -> Result<(), Untranslated> {
        let rest = rest.trim();
        let n: i64 = rest.parse().map_err(|_| {
            no(format!(
                "set seed {rest}: only a number is translated (not a state from c(rngstate))"
            ))
        })?;
        if n >= 1 << 31 {
            return Err(no(format!(
                "set seed {n}: Stata takes seeds below 2^31 (Stata: '{n}' found where integer < 2^31 expected)"
            )));
        }
        if n <= -(1 << 31) {
            return Err(no(format!("set seed {n} is not translated")));
        }
        self.helpers.insert("stata_rng");
        self.rng_lost = None;
        out.line(format!("stata_set_seed({n})"));
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lines_that_may_draw_are_known() {
        for (stata, command) in [
            ("generate x = runiform() + rnormal()", "generate"),
            ("replace y = 1 if rbinomial(1, .5)", "replace"),
            ("bsample 10", "bsample"),
            ("regress y x, vce(bootstrap)", "regress"),
            ("set rngstate XAA0", "set"),
            ("set seed abc", "set"),
            ("mata: x = runiform(1, 1)", "mata"),
        ] {
            assert!(moves_generator(stata, command, ""), "{stata}");
        }
        assert!(moves_generator(
            "myprog 1",
            "myprog",
            "this runs myprog, a program the do-file defines"
        ));
        for (stata, command) in [
            ("generate x = myrunif(1)", "generate"),
            ("tabstat x, by(g)", "tabstat"),
            ("set obs 10", "set"),
        ] {
            assert!(!moves_generator(stata, command, ""), "{stata}");
        }
    }
}
