//! Statistics menu, multivariate analysis (P6.7): principal components with
//! `prcomp`, maximum-likelihood factor analysis with `factanal` (unrotated or
//! with a varimax rotation), hierarchical (single, complete, average, Ward) or
//! k-means cluster analysis with a chosen number of groups, and linear
//! discriminant analysis with `MASS::lda` and its classification table. Each
//! fits its model, keeps it in the workspace under the name the dialog asks
//! for and prints what Stata's `pca`, `factor, ml`, `cluster` and `discrim lda`
//! print: the numbers, not the layout. None changes the data.

use rcraft_rcodegen::dialog::{Args, Opt, Problem, Spec, TextDefault, TextRule};
use rcraft_rcodegen::dialog::{VarKind, Widget};
use rcraft_rcodegen::{r_name, r_string};

use crate::data::BASE;

const NUMBERS: &[VarKind] = &[VarKind::Number];
/// What can name a group.
const GROUPS: &[VarKind] = &[
    VarKind::Number,
    VarKind::Factor,
    VarKind::Text,
    VarKind::Logical,
];

/// The chosen variables, joined for the right side of a formula.
fn rhs(a: &Args) -> String {
    a.vars("x")
        .iter()
        .map(|n| r_name(n))
        .collect::<Vec<_>>()
        .join(" + ")
}

/// The chosen variables as `c("a", "b")`.
fn names_vector(a: &Args) -> String {
    let names: Vec<String> = a.vars("x").iter().map(|n| r_string(n)).collect();
    format!("c({})", names.join(", "))
}

/// A whole number of at least `least`, or a problem on the widget.
fn whole(a: &Args, id: &'static str, what: &str, least: u32, out: &mut Vec<Problem>) -> u32 {
    match a.number(id).trim().parse::<u32>() {
        Ok(n) if n >= least => n,
        _ => {
            out.push(Problem::new(
                id,
                format!("{what} must be a whole number, {least} or more."),
            ));
            least
        }
    }
}

/// The model is not saved over the data frame.
fn not_the_data(a: &Args, out: &mut Vec<Problem>) {
    if a.name_text("save") == a.ctx.dataset {
        out.push(Problem::new(
            "save",
            format!("{} is the data frame; choose another name.", a.ctx.dataset),
        ));
    }
}

// ---- Principal components ----

pub const PRINCIPAL_COMPONENTS: Spec = Spec {
    entry: "Principal components",
    menu: "Statistics > Multivariate > Principal components",
    about: "Finds the weighted sums of the chosen variables that carry the most variation, one \
            after another and uncorrelated with each other. It fits with stats::prcomp, on the \
            correlation matrix (each variable standardised) or on the covariance matrix, and \
            prints each component's eigenvalue with its share of the variation, then the \
            loadings (the eigenvectors). Each component is signed so that its largest loading \
            is positive, or, as Stata's pca does, so that its loadings sum to more than 0. Rows \
            with a missing value are left out.",
    widgets: &[
        Widget::Variables {
            id: "x",
            label: "Variables",
            kinds: NUMBERS,
            min: 2,
            hint: "Two or more numeric variables.",
        },
        Widget::Check {
            id: "scale",
            label: "Standardise the variables (use the correlation matrix)",
            default: true,
            hint: "Untick to use the covariance matrix, which lets a variable with a large spread \
                   dominate.",
        },
        Widget::Check {
            id: "stata_sign",
            label: "Sign as Stata (each component's loadings sum to more than 0)",
            default: false,
            hint: "Otherwise each component's largest loading is made positive.",
        },
        Widget::Check {
            id: "scores",
            label: "Show the first rows of the component scores",
            default: false,
            hint: "",
        },
        Widget::Text {
            id: "save",
            label: "Save the model as",
            default: TextDefault::Fixed("pca"),
            rule: TextRule::Name,
            hint: "The fitted model stays in R's workspace under this name; a name already in use \
                   is replaced. Its scores are `name$x`.",
        },
    ],
    needs: &[],
    styles: BASE,
    check: |a| {
        let mut out = Vec::new();
        not_the_data(a, &mut out);
        out
    },
    code: |a, _| {
        let m = a.name("save");
        let flip = if a.flag("stata_sign") {
            "flip <- ifelse(colSums(fit$rotation) < 0, -1, 1)"
        } else {
            "flip <- apply(fit$rotation, 2, function(v) if (v[which.max(abs(v))] < 0) -1 else 1)"
        };
        let mut code = format!(
            "{m} <- local({{\n  fit <- stats::prcomp(~ {}, data = {}, scale. = {})\n  \
             {flip}\n  \
             fit$rotation <- sweep(fit$rotation, 2, flip, \"*\")\n  \
             fit$x <- sweep(fit$x, 2, flip, \"*\")\n  fit\n}})\n\
             print(local({{\n  eig <- {m}$sdev^2\n  \
             data.frame(Eigenvalue = eig, Proportion = eig / sum(eig), \
             Cumulative = cumsum(eig) / sum(eig), row.names = colnames({m}$rotation))\n}}))\n\
             cat(\"\\nLoadings (eigenvectors)\\n\")\nprint({m}$rotation)",
            rhs(a),
            a.data(),
            if a.flag("scale") { "TRUE" } else { "FALSE" },
        );
        if a.flag("scores") {
            code.push_str(&format!(
                "\ncat(\"\\nComponent scores (first rows)\\n\")\nprint(utils::head({m}$x))"
            ));
        }
        code
    },
};

// ---- Factor analysis ----

const FACTOR_METHODS: &[Opt] = &[
    Opt {
        value: "ml",
        label: "Maximum likelihood",
    },
    Opt {
        value: "pf",
        label: "Principal factors",
    },
    Opt {
        value: "ipf",
        label: "Iterated principal factors",
    },
    Opt {
        value: "pcf",
        label: "Principal-component factors",
    },
];

const ROTATIONS: &[Opt] = &[
    Opt {
        value: "none",
        label: "None",
    },
    Opt {
        value: "varimax",
        label: "Varimax",
    },
    Opt {
        value: "promax",
        label: "Promax (oblique)",
    },
    Opt {
        value: "oblimin",
        label: "Oblimin (oblique)",
    },
];

fn factor_check(a: &Args) -> Vec<Problem> {
    let mut out = Vec::new();
    let f = whole(a, "factors", "The number of factors", 1, &mut out);
    let p = a.vars("x").len() as i64;
    let f = i64::from(f);
    if a.choice("method") == "ml" {
        if p >= 3 && (p - f) * (p - f) < p + f {
            out.push(Problem::new(
                "factors",
                format!("{f} factors is too many for {p} variables: the model would have more unknowns than the variables give."),
            ));
        }
    } else if p >= 1 && f > p {
        out.push(Problem::new(
            "factors",
            format!("{f} factors is more than the {p} variables."),
        ));
    }
    match a.choice("rotation") {
        "promax" => match a.number("power").parse::<f64>() {
            Ok(m) if (1.0..=100.0).contains(&m) => {}
            _ => out.push(Problem::new(
                "power",
                "The promax power must be a number from 1 to 100.",
            )),
        },
        "oblimin" => match a.number("gamma").parse::<f64>() {
            Ok(g) if g.is_finite() => {}
            _ => out.push(Problem::new("gamma", "Type the oblimin gamma as a number.")),
        },
        _ => {}
    }
    not_the_data(a, &mut out);
    out
}

/// The R of a principal-factor fit (principal factors, iterated, or
/// principal-component factors), a list shaped like factanal's.
fn principal_factor_fit(a: &Args) -> String {
    let method = a.choice("method");
    let (start, floor) = if method == "pcf" {
        ("rep(1, ncol(R))", "1")
    } else {
        ("1 - 1 / diag(solve(R))", "0")
    };
    let solve = if method == "ipf" {
        "for (it in 1:20000) {\n      r <- pf(h)\n      hn <- rowSums(r$L^2)\n      \
         if (sum((hn - h)^2) < 1e-8) break\n      h <- hn\n    }"
    } else {
        "r <- pf(h)"
    };
    format!(
        "local({{\n    x <- stats::na.omit({}[, {}])\n    R <- stats::cor(x)\n    k <- {}\n    \
         pf <- function(h) {{\n      Rr <- R\n      diag(Rr) <- h\n      \
         e <- eigen(Rr, symmetric = TRUE)\n      kk <- min(k, sum(e$values > {floor}))\n      \
         if (kk < 1) stop(\"No eigenvalue is above {floor}, so no factor is kept.\", call. = FALSE)\n      \
         L <- e$vectors[, seq_len(kk), drop = FALSE] %*% diag(sqrt(e$values[seq_len(kk)]), kk)\n      \
         list(L = sweep(L, 2, ifelse(colSums(L) < 0, -1, 1), \"*\"), values = e$values)\n    }}\n    \
         h <- {start}\n    {solve}\n    L <- r$L\n    \
         dimnames(L) <- list(colnames(R), paste0(\"Factor\", seq_len(ncol(L))))\n    \
         list(loadings = structure(L, class = \"loadings\"), uniquenesses = 1 - rowSums(L^2), \
         eigenvalues = r$values, factors = ncol(L), n.obs = nrow(x), method = {})\n  }})",
        a.data(),
        names_vector(a),
        a.number("factors"),
        r_string(method),
    )
}

/// The R that rotates `fit$loadings` in place (nothing without a rotation).
/// Factors are then ordered by the variation they carry (for oblique ones,
/// the column sums of squares of the structure matrix) and signed so their
/// loadings sum to more than 0, as Stata's rotate does.
fn rotation_code(a: &Args) -> String {
    let rotation = a.choice("rotation");
    if rotation == "none" {
        return String::new();
    }
    let mut s = String::from(
        "  A <- unclass(fit$loadings)\n  \
         tidy <- function(L, Phi = NULL) {\n    \
         S <- if (is.null(Phi)) L else L %*% Phi\n    \
         o <- order(colSums(S^2), decreasing = TRUE)\n    \
         s <- ifelse(colSums(L[, o, drop = FALSE]) < 0, -1, 1)\n    \
         L <- sweep(L[, o, drop = FALSE], 2, s, \"*\")\n    \
         if (is.null(Phi)) list(L = L) else list(L = L, Phi = Phi[o, o, drop = FALSE] * outer(s, s))\n  }\n  \
         kaiser <- function(A) {\n    k <- ncol(A)\n    p <- nrow(A)\n    \
         if (k < 2) return(A)\n    \
         for (sweep in 1:1000) {\n      moved <- 0\n      \
         for (i in 1:(k - 1)) for (j in (i + 1):k) {\n        \
         x <- A[, i]\n        y <- A[, j]\n        u <- x^2 - y^2\n        v <- 2 * x * y\n        \
         phi <- atan2(2 * sum(u * v) - 2 * sum(u) * sum(v) / p, \
         sum(u^2 - v^2) - (sum(u)^2 - sum(v)^2) / p) / 4\n        \
         moved <- max(moved, abs(phi))\n        \
         A[, c(i, j)] <- cbind(cos(phi) * x + sin(phi) * y, cos(phi) * y - sin(phi) * x)\n      }\n      \
         if (moved < 1e-15) break\n    }\n    A\n  }\n",
    );
    if a.flag("exact") {
        s.push_str("  vmx <- kaiser\n");
    } else {
        // Stata's varimax: the exact angle for two factors, the SVD iteration
        // stopped at a relative change of 1e-8 for more
        s.push_str(
            "  vmx <- function(A) if (ncol(A) < 3) kaiser(A) else \
             unclass(stats::varimax(A, normalize = FALSE, eps = 1e-8)$loadings)\n",
        );
    }
    match rotation {
        "promax" => s.push_str(&format!(
            "  V <- tidy(vmx(A))$L\n  \
             r <- if (ncol(V) < 2) list(L = V) else {{\n    \
             Q <- V * abs(V)^({} - 1)\n    U <- solve(crossprod(V), crossprod(V, Q))\n    \
             U <- U %*% diag(sqrt(diag(solve(crossprod(U)))), ncol(U))\n    \
             tidy(V %*% U, solve(crossprod(U)))\n  }}\n",
            a.number("power")
        )),
        "oblimin" => s.push_str(&format!(
            "  oblimin <- function(A, gam) {{\n    p <- nrow(A)\n    k <- ncol(A)\n    \
             N <- matrix(1, k, k) - diag(k)\n    C <- diag(p) - gam / p\n    \
             crit <- function(L) {{\n      L2 <- L^2\n      M <- C %*% L2 %*% N\n      \
             list(f = sum(L2 * M) / 4, G = L * M)\n    }}\n    \
             Tm <- diag(k)\n    al <- 1\n    L <- A\n    q <- crit(L)\n    \
             G <- -t(t(L) %*% q$G %*% solve(Tm))\n    \
             for (it in 1:10000) {{\n      \
             Gp <- G - Tm %*% diag(colSums(Tm * G), k)\n      s <- sqrt(sum(Gp^2))\n      \
             if (s < 1e-8) break\n      al <- 2 * al\n      ok <- FALSE\n      \
             for (i in 1:60) {{\n        X <- Tm - al * Gp\n        \
             Tt <- sweep(X, 2, sqrt(colSums(X^2)), \"/\")\n        \
             Lt <- A %*% t(solve(Tt))\n        qt <- crit(Lt)\n        \
             if (qt$f < q$f - 0.5 * s^2 * al) {{\n          ok <- TRUE\n          break\n        }}\n        \
             al <- al / 2\n      }}\n      if (!ok) break\n      \
             Tm <- Tt\n      L <- Lt\n      q <- qt\n      G <- -t(t(L) %*% q$G %*% solve(Tm))\n    }}\n    \
             tidy(L, crossprod(Tm))\n  }}\n  \
             r <- if (ncol(A) < 2) list(L = A) else oblimin(A, {})\n",
            a.number("gamma")
        )),
        _ => s.push_str("  r <- tidy(vmx(A))\n"),
    }
    s.push_str(
        "  L <- r$L\n  dimnames(L) <- dimnames(A)\n  \
         fit$loadings <- structure(L, class = \"loadings\")\n  \
         fit$rotmat <- if (is.null(r$Phi)) NULL else solve(t(chol(r$Phi)))\n  \
         fit$Phi <- r$Phi\n",
    );
    s
}

pub const FACTOR_ANALYSIS: Spec = Spec {
    entry: "Factor analysis",
    menu: "Statistics > Multivariate > Factor analysis",
    about: "Explains the correlations among the chosen variables by a few common factors. By \
            maximum likelihood it fits with stats::factanal and prints each variable's uniqueness \
            (the part no factor explains), the loadings, the variation each factor carries and \
            the likelihood-ratio test of the number of factors (with N, not N-1, in its \
            multiplier, as Stata has it). Principal factors take the eigenvectors of the \
            correlation matrix with the squared multiple correlations on its diagonal; iterated \
            principal factors put the communalities back on the diagonal until they move by less \
            than 1e-4 (as Stata's ipf stops); principal-component factors keep the ones on the \
            diagonal. As in Stata, those keep no more factors than eigenvalues above 0 (above 1 \
            for principal-component factors), and a uniqueness may come out below 0. A varimax \
            rotation (without Kaiser normalisation) turns the factors to simplify the loadings: \
            as Stata does, the exact rotation for two factors and, for more, the iteration \
            stopped when the criterion grows by less than 1e-8 (or, if asked, the exact \
            maximum). Promax raises the varimax loadings to a power to make a target and fits \
            oblique factors to it; oblimin (gamma 0 is quartimin) is fitted by gradient \
            projection from the unrotated factors. Oblique factors print their correlations, in \
            `name$Phi`. Factors are ordered by the variation they carry (for oblique ones, of \
            the structure matrix) and signed so their loadings sum to more than zero. Rows with \
            a missing value are left out.",
    widgets: &[
        Widget::Variables {
            id: "x",
            label: "Variables",
            kinds: NUMBERS,
            min: 3,
            hint: "Three or more numeric variables.",
        },
        Widget::Choice {
            id: "method",
            label: "Method",
            options: FACTOR_METHODS,
            default: "ml",
            hint: "",
        },
        Widget::Text {
            id: "factors",
            label: "Number of factors",
            default: TextDefault::Fixed("1"),
            rule: TextRule::Number,
            hint: "More factors than the variables can support are refused.",
        },
        Widget::Choice {
            id: "rotation",
            label: "Rotation",
            options: ROTATIONS,
            default: "none",
            hint: "",
        },
        Widget::Text {
            id: "power",
            label: "Promax power",
            default: TextDefault::Fixed("3"),
            rule: TextRule::Number,
            hint: "Used by promax only; 3 is Stata's default.",
        },
        Widget::Text {
            id: "gamma",
            label: "Oblimin gamma",
            default: TextDefault::Fixed("0"),
            rule: TextRule::Number,
            hint: "Used by oblimin only; 0 is quartimin, Stata's default.",
        },
        Widget::Check {
            id: "exact",
            label: "Varimax to the exact maximum",
            default: false,
            hint: "With three or more factors Stata stops sooner; tick to rotate until the \
                   angles are below 1e-15.",
        },
        Widget::Text {
            id: "save",
            label: "Save the model as",
            default: TextDefault::Fixed("fa"),
            rule: TextRule::Name,
            hint: "The fitted model stays in R's workspace under this name; a name already in use \n                   is replaced.",
        },
    ],
    needs: &[],
    styles: BASE,
    check: factor_check,
    code: |a, _| {
        let m = a.name("save");
        let rotation = rotation_code(a);
        if a.choice("method") != "ml" {
            let title = match a.choice("method") {
                "ipf" => "Iterated principal factors",
                "pcf" => "Principal-component factors",
                _ => "Principal factors",
            };
            let fit = principal_factor_fit(a);
            return format!(
                "{m} <- local({{\n  fit <- {fit}\n{rotation}  fit\n}})\n\
                 cat(sprintf(\"{title}, %d rows, %d factors\\n\\nUniquenesses:\\n\", \
                 {m}$n.obs, {m}$factors))\n\
                 print(round({m}$uniquenesses, 3))\nprint({m}$loadings, digits = 3)\n\
                 if (!is.null({m}$Phi)) {{\n  cat(\"\\nFactor correlations:\\n\")\n  \
                 print(round({m}$Phi, 3))\n}}\n\
                 cat(\"\\nEigenvalues of the correlation matrix with communalities on the diagonal:\\n\")\n\
                 print(round({m}$eigenvalues, 5))"
            );
        }
        // The optimiser is asked for the tightest stop first (factr = 10), which is what
        // matches Stata; on some platforms its line search gives up on an easy problem
        // ("unable to optimize from this starting value"), so it is asked again with a
        // looser stop, then looser still, and only then is the failure reported.
        let fit = format!(
            "local({{\n    for (fr in c(10, 1e3, 1e5, 1e7)) {{\n      \
             r <- tryCatch(stats::factanal(~ {}, factors = {}, data = {}, rotation = \"none\", \
             control = list(opt = list(factr = fr))), error = function(e) e)\n      \
             if (!inherits(r, \"error\")) return(r)\n    }}\n    \
             stop(conditionMessage(r), call. = FALSE)\n  }})",
            rhs(a),
            a.number("factors"),
            a.data()
        );
        // factanal's likelihood-ratio statistic multiplies the criterion by
        // N - 1 - (2p+5)/6 - 2k/3; Stata's multiplies by N - (2p+5)/6 - 2k/3
        let chi2 = "  if (!is.null(fit$STATISTIC)) {\n    \
             p <- length(fit$uniquenesses); k <- ncol(fit$loadings); n <- fit$n.obs\n    \
             fit$STATISTIC <- fit$STATISTIC * (n - (2 * p + 5) / 6 - 2 * k / 3) / \
             (n - 1 - (2 * p + 5) / 6 - 2 * k / 3)\n    \
             fit$PVAL <- stats::pchisq(fit$STATISTIC, fit$dof, lower.tail = FALSE)\n  }\n";
        format!("{m} <- local({{\n  fit <- {fit}\n{chi2}{rotation}  fit\n}})\nprint({m})")
    },
};

// ---- Cluster analysis ----

const METHODS: &[Opt] = &[
    Opt {
        value: "ward",
        label: "Hierarchical, Ward's linkage",
    },
    Opt {
        value: "average",
        label: "Hierarchical, average linkage",
    },
    Opt {
        value: "complete",
        label: "Hierarchical, complete linkage",
    },
    Opt {
        value: "single",
        label: "Hierarchical, single linkage",
    },
    Opt {
        value: "kmeans",
        label: "K-means",
    },
];

const STARTS: &[Opt] = &[
    Opt {
        value: "random",
        label: "Random rows (with the seed below)",
    },
    Opt {
        value: "first",
        label: "The first rows of the data",
    },
];

const DISTANCES: &[Opt] = &[
    Opt {
        value: "default",
        label: "As Stata: squared Euclidean for Ward's, Euclidean otherwise",
    },
    Opt {
        value: "L2",
        label: "Euclidean (L2)",
    },
    Opt {
        value: "L2squared",
        label: "Squared Euclidean",
    },
    Opt {
        value: "L1",
        label: "Absolute value, city block (L1)",
    },
    Opt {
        value: "Linf",
        label: "Maximum difference (L-infinity)",
    },
    Opt {
        value: "canberra",
        label: "Canberra",
    },
];

fn cluster_check(a: &Args) -> Vec<Problem> {
    let mut out = Vec::new();
    whole(a, "groups", "The number of groups", 2, &mut out);
    if a.choice("method") == "kmeans" && a.choice("start") == "random" {
        whole(a, "seed", "The seed", 0, &mut out);
    }
    if a.flag("rules") {
        whole(
            a,
            "most",
            "The most groups for the stopping rules",
            2,
            &mut out,
        );
    }
    not_the_data(a, &mut out);
    out
}

/// The R dissimilarities between the rows of `x` for a hierarchical method.
fn dissimilarity(method: &str, distance: &str) -> &'static str {
    match distance {
        "L2" => "stats::dist(x)",
        "L2squared" => "stats::dist(x)^2",
        "L1" => "stats::dist(x, method = \"manhattan\")",
        "Linf" => "stats::dist(x, method = \"maximum\")",
        // Stata's Canberra: |a - b| / (|a| + |b|), a term with both 0 counting 0
        "canberra" => {
            "local({\n    m <- as.matrix(x)\n    a <- abs(m)\n    \
             stats::as.dist(t(vapply(seq_len(nrow(m)), function(i) {\n      \
             b <- sweep(a, 2, a[i, ], \"+\")\n      \
             rowSums(ifelse(b > 0, abs(sweep(m, 2, m[i, ])) / b, 0))\n    }, numeric(nrow(m)))))\n  })"
        }
        _ if method == "ward" => "stats::dist(x)^2",
        _ => "stats::dist(x)",
    }
}

/// The R distance from every row of `m` to the centre `c`, for k-means under
/// a measure other than Euclidean.
fn centre_distance(distance: &str) -> &'static str {
    match distance {
        "L1" => "rowSums(abs(sweep(m, 2, c)))",
        "Linf" => "apply(abs(sweep(m, 2, c)), 1, max)",
        _ => {
            "{\n        b <- sweep(abs(m), 2, abs(c), \"+\")\n        \
              rowSums(ifelse(b > 0, abs(sweep(m, 2, c)) / b, 0))\n      }"
        }
    }
}

/// The R of the stopping rules for the fitted `m`: Calinski and Harabasz's
/// pseudo-F (and, for a hierarchy, Duda and Hart's Je(2)/Je(1) and pseudo-T²
/// for splitting the group that splits next), as Stata's cluster stop.
fn stopping_rules(m: &str, hierarchical: bool, most: &str) -> String {
    let start = format!(
        "{m}$rules <- local({{\n  x <- as.matrix({m}$data)\n  n <- nrow(x)\n  \
         sse <- function(y) sum(sweep(y, 2, colMeans(y))^2)\n  \
         ch <- function(g) {{\n    k <- length(unique(g))\n    \
         W <- sum(vapply(split(seq_len(n), g), function(i) sse(x[i, , drop = FALSE]), 0))\n    \
         ((sse(x) - W) / (k - 1)) / (W / (n - k))\n  }}\n  "
    );
    let shown = format!("cat(\"\\nStopping rules\\n\")\nprint({m}$rules, row.names = FALSE)");
    if hierarchical {
        format!(
            "{start}ks <- seq_len(max(1, min({most}, n - 1)))\n  \
             out <- data.frame(Groups = ks, Calinski.Harabasz = NA_real_, Duda.Hart = NA_real_, \
             pseudo.T2 = NA_real_)\n  \
             for (k in ks) {{\n    g1 <- stats::cutree({m}, k)\n    g2 <- stats::cutree({m}, k + 1)\n    \
             if (k >= 2) out$Calinski.Harabasz[k] <- ch(g1)\n    \
             tab <- table(g1, g2)\n    parent <- which(rowSums(tab > 0) == 2)\n    \
             if (length(parent) == 1) {{\n      rows <- which(g1 == as.integer(rownames(tab)[parent]))\n      \
             j1 <- sse(x[rows, , drop = FALSE])\n      \
             j2 <- sum(vapply(split(rows, g2[rows]), function(i) sse(x[i, , drop = FALSE]), 0))\n      \
             out$Duda.Hart[k] <- j2 / j1\n      \
             if (length(rows) > 2) out$pseudo.T2[k] <- (j1 - j2) / (j2 / (length(rows) - 2))\n    }}\n  }}\n  \
             out\n}})\n{shown}"
        )
    } else {
        format!(
            "{start}data.frame(Groups = length(unique({m}$groups)), \
             Calinski.Harabasz = ch({m}$groups))\n}})\n{shown}"
        )
    }
}

pub const CLUSTER_ANALYSIS: Spec = Spec {
    entry: "Cluster analysis",
    menu: "Statistics > Multivariate > Cluster analysis",
    about: "Sorts the rows into groups of similar rows. Hierarchical clustering (stats::hclust) \
            joins the closest rows and groups step by step and is cut into the number of groups \
            asked for with stats::cutree; the dissimilarity is, as in Stata, the squared \
            Euclidean distance for Ward's linkage and the Euclidean distance otherwise, or one \
            chosen: Euclidean, squared Euclidean, absolute value (L1), maximum difference \
            (L-infinity) or Canberra (|a - b| / (|a| + |b|) summed over the variables, as \
            Stata defines it). K-means (Lloyd's algorithm) starts from the first rows or from \
            random rows with a seed and moves each row to its nearest group centre (the first \
            on a tie) until nothing moves; the centres are the group means; a measure other \
            than Euclidean is used for the nearest centre, as Stata's measure() does. The \
            stopping rules print Calinski and Harabasz's pseudo-F for each number of groups \
            and, for a hierarchy, Duda and Hart's Je(2)/Je(1) and pseudo-T-squared for the split \
            from that number of groups to the next, as Stata's cluster stop does (on the \
            variables' sums of squares, whatever the dissimilarity), kept in `name$rules`. The \
            group sizes and means \
            are printed; the group of each row is `name$groups`. Rows with a missing value are \
            left out.",
    widgets: &[
        Widget::Variables {
            id: "x",
            label: "Variables",
            kinds: NUMBERS,
            min: 1,
            hint: "Numeric variables; the clustering uses them as they are, so standardise first \
                   if their scales differ.",
        },
        Widget::Choice {
            id: "method",
            label: "Method",
            options: METHODS,
            default: "ward",
            hint: "",
        },
        Widget::Choice {
            id: "distance",
            label: "Dissimilarity",
            options: DISTANCES,
            default: "default",
            hint: "",
        },
        Widget::Text {
            id: "groups",
            label: "Number of groups",
            default: TextDefault::Fixed("3"),
            rule: TextRule::Number,
            hint: "",
        },
        Widget::Choice {
            id: "start",
            label: "K-means starts from",
            options: STARTS,
            default: "first",
            hint: "Used by K-means only.",
        },
        Widget::Text {
            id: "seed",
            label: "Seed for random starts",
            default: TextDefault::Fixed("1"),
            rule: TextRule::Number,
            hint: "Used by K-means with random starting rows, so the same seed gives the same \
                   groups.",
        },
        Widget::Check {
            id: "rules",
            label: "Stopping rules (Calinski-Harabasz; Duda-Hart for a hierarchy)",
            default: false,
            hint: "",
        },
        Widget::Text {
            id: "most",
            label: "Stopping rules up to this many groups",
            default: TextDefault::Fixed("10"),
            rule: TextRule::Number,
            hint: "For hierarchical clustering; k-means gives the rule for its own groups.",
        },
        Widget::Text {
            id: "save",
            label: "Save the model as",
            default: TextDefault::Fixed("clusters"),
            rule: TextRule::Name,
            hint: "The fitted model stays in R's workspace under this name; a name already in use \
                   is replaced.",
        },
    ],
    needs: &[],
    styles: BASE,
    check: cluster_check,
    code: |a, _| {
        let m = a.name("save");
        let k = a.number("groups");
        let method = a.choice("method");
        let distance = a.choice("distance");
        let take = format!("x <- stats::na.omit({}[, {}])", a.data(), names_vector(a));
        let mut code = if method == "kmeans" {
            let random = a.choice("start") == "random";
            let fit = if matches!(distance, "L1" | "Linf" | "canberra") {
                let (seed, centres) = if random {
                    (
                        format!("    set.seed({})\n", a.number("seed")),
                        format!("m[sample.int(nrow(m), {k}), , drop = FALSE]"),
                    )
                } else {
                    (String::new(), format!("m[seq_len({k}), , drop = FALSE]"))
                };
                format!(
                    "  fit <- local({{\n    m <- as.matrix(x)\n{seed}    ctr <- {centres}\n    \
                     far <- function(c) {}\n    g <- integer(0)\n    \
                     for (it in 1:1000) {{\n      \
                     gn <- max.col(-vapply(seq_len(nrow(ctr)), function(j) far(ctr[j, ]), \
                     numeric(nrow(m))), ties.method = \"first\")\n      \
                     if (identical(gn, g)) break\n      g <- gn\n      \
                     if (any(tabulate(g, nrow(ctr)) == 0)) \
                     stop(\"A group lost all its rows; try other starting rows.\", call. = FALSE)\n      \
                     ctr <- do.call(rbind, lapply(seq_len(nrow(ctr)), function(j) \
                     colMeans(m[g == j, , drop = FALSE])))\n    }}\n    \
                     list(cluster = g, centers = ctr, iter = it)\n  }})",
                    centre_distance(distance)
                )
            } else {
                let (seed, centers) = if random {
                    (format!("  set.seed({})\n", a.number("seed")), k.to_string())
                } else {
                    (String::new(), format!("x[seq_len({k}), , drop = FALSE]"))
                };
                format!(
                    "{seed}  fit <- stats::kmeans(x, centers = {centers}, algorithm = \"Lloyd\", \
                     iter.max = 1000)"
                )
            };
            format!(
                "{m} <- local({{\n  {take}\n{fit}\n  fit$groups <- fit$cluster\n  fit$data <- x\n  fit\n}})\n\
                 cat(\"Group sizes\\n\")\nprint(table(Group = {m}$groups))\n\
                 cat(\"\\nGroup means\\n\")\n\
                 print(stats::aggregate({m}$data, list(Group = {m}$groups), mean))"
            )
        } else {
            let linkage = if method == "ward" { "ward.D" } else { method };
            format!(
                "{m} <- local({{\n  {take}\n  \
                 fit <- stats::hclust({}, method = {})\n  \
                 fit$groups <- stats::cutree(fit, k = {k})\n  fit$data <- x\n  fit\n}})\n\
                 cat(\"Group sizes\\n\")\nprint(table(Group = {m}$groups))\n\
                 cat(\"\\nGroup means\\n\")\n\
                 print(stats::aggregate({m}$data, list(Group = {m}$groups), mean))\n\
                 cat(\"\\nHeights of the last joins\\n\")\nprint(utils::tail({m}$height, 5))",
                dissimilarity(method, distance),
                r_string(linkage)
            )
        };
        if a.flag("rules") {
            code.push('\n');
            code.push_str(&stopping_rules(&m, method != "kmeans", a.number("most")));
        }
        code
    },
};

// ---- Discriminant analysis ----

const PRIORS: &[Opt] = &[
    Opt {
        value: "equal",
        label: "Equal for every group",
    },
    Opt {
        value: "proportional",
        label: "In proportion to the group sizes",
    },
];

const DISCRIMINANTS: &[Opt] = &[
    Opt {
        value: "linear",
        label: "Linear (one covariance matrix)",
    },
    Opt {
        value: "quadratic",
        label: "Quadratic (a covariance matrix per group)",
    },
];

fn discriminant_check(a: &Args) -> Vec<Problem> {
    let mut out = Vec::new();
    let g = a.var_text("group");
    if a.vars("x").contains(&g) {
        out.push(Problem::new(
            "x",
            format!("{g} is the group variable; it cannot also be a predictor."),
        ));
    }
    not_the_data(a, &mut out);
    out
}

pub const DISCRIMINANT_ANALYSIS: Spec = Spec {
    entry: "Discriminant analysis",
    menu: "Statistics > Multivariate > Discriminant analysis",
    about: "Finds the combinations of the chosen variables that best tell the groups apart and \
            assigns each row to the most likely group. It fits a linear discriminant with \
            MASS::lda (one covariance matrix for all groups) and prints the priors, the group \
            means, the coefficients of the linear discriminant functions and each function's \
            share of the separation (found with the groups weighed by their sizes, as Stata's \
            discrim lda does, whatever the priors), then the table of true against assigned groups for the \
            rows used. The quadratic discriminant (MASS::qda, as Stata's discrim qda) gives each \
            group its own covariance matrix; it prints the priors and group means, then the \
            same table, and each row's posterior probabilities are \
            `predict(name)$posterior`. Each group needs more rows than predictors. Rows with a \
            missing value are left out.",
    widgets: &[
        Widget::Variable {
            id: "group",
            label: "Group variable",
            kinds: GROUPS,
            required: true,
            hint: "Two or more groups, each with at least two rows.",
        },
        Widget::Variables {
            id: "x",
            label: "Predictors",
            kinds: NUMBERS,
            min: 1,
            hint: "",
        },
        Widget::Choice {
            id: "method",
            label: "Discriminant",
            options: DISCRIMINANTS,
            default: "linear",
            hint: "",
        },
        Widget::Choice {
            id: "priors",
            label: "Prior probabilities",
            options: PRIORS,
            default: "equal",
            hint: "How likely each group is taken to be before looking at the row.",
        },
        Widget::Text {
            id: "save",
            label: "Save the model as",
            default: TextDefault::Fixed("lda"),
            rule: TextRule::Name,
            hint: "The fitted model stays in R's workspace under this name; a name already in use \
                   is replaced.",
        },
    ],
    needs: &["MASS"],
    styles: BASE,
    check: discriminant_check,
    code: |a, _| {
        let m = a.name("save");
        let g = a.var("group").unwrap_or_default();
        let formula = format!("{g} ~ {}", rhs(a));
        // lda weighs the group means by the priors when it finds the canonical
        // functions; Stata weighs them by the group sizes. With equal priors the
        // functions and their shares come from the fit with the sizes as priors
        // (the classification still uses the priors asked for)
        let quadratic = a.choice("method") == "quadratic";
        let (prior, sizes) = if a.choice("priors") == "equal" {
            let n = format!("nlevels(factor({}${g}))", a.data());
            let sizes = if quadratic {
                String::new()
            } else {
                format!(
                    "\n{m}[c(\"scaling\", \"svd\")] <- MASS::lda({formula}, data = {})[c(\"scaling\", \"svd\")]",
                    a.data()
                )
            };
            (format!(", prior = rep(1 / {n}, {n})"), sizes)
        } else {
            (String::new(), String::new())
        };
        let fit = if quadratic { "qda" } else { "lda" };
        format!(
            "{m} <- MASS::{fit}({formula}, data = {}{prior}){sizes}\nprint({m})\n\
             cat(\"\\nClassification of the rows used (true group down, assigned group across)\\n\")\n\
             print(local({{\n  rows <- stats::model.frame({formula}, data = {})\n  \
             assigned <- stats::predict({m}, rows)$class\n  \
             tab <- table(True = factor(rows[[1]], levels = levels(assigned)), Assigned = assigned)\n  \
             cat(sprintf(\"Correctly classified: %.2f%%\\n\", 100 * sum(diag(tab)) / sum(tab)))\n  \
             stats::addmargins(tab)\n}}))",
            a.data(),
            a.data(),
        )
    },
};

pub const SPECS: &[&Spec] = &[
    &PRINCIPAL_COMPONENTS,
    &FACTOR_ANALYSIS,
    &CLUSTER_ANALYSIS,
    &DISCRIMINANT_ANALYSIS,
];

#[cfg(test)]
mod tests {
    use super::*;
    use rcraft_rcodegen::dialog::{ColumnInfo, Context, Style, Values};

    fn ctx() -> Context {
        let col = |name: &str, kind| ColumnInfo {
            name: name.into(),
            kind,
        };
        Context {
            dataset: "d".into(),
            frames: Vec::new(),
            columns: vec![
                col("height", VarKind::Number),
                col("weight", VarKind::Number),
                col("arm span", VarKind::Number),
                col("age", VarKind::Number),
                col("species", VarKind::Factor),
            ],
        }
    }

    fn values(spec: &Spec, set: &[(&str, &str)], lists: &[(&str, &[&str])]) -> Values {
        let mut v = spec.defaults(&ctx());
        for (id, text) in set {
            v.set_text(id, *text);
        }
        for (id, names) in lists {
            v.set_list(id, names.iter().map(|n| n.to_string()).collect());
        }
        v
    }

    fn code(spec: &Spec, v: &Values) -> String {
        let out = spec.generate(&ctx(), v, Style::Base);
        assert!(out.is_ok(), "{out:?}");
        out.unwrap_or_default()
    }

    fn widgets(spec: &Spec, v: &Values) -> Vec<&'static str> {
        spec.problems(&ctx(), v).iter().map(|p| p.widget).collect()
    }

    #[test]
    fn principal_components_writes_prcomp() {
        let v = values(
            &PRINCIPAL_COMPONENTS,
            &[],
            &[("x", &["height", "arm span"])],
        );
        assert_eq!(
            code(&PRINCIPAL_COMPONENTS, &v),
            "pca <- local({\n  fit <- stats::prcomp(~ height + `arm span`, data = d, scale. = TRUE)\n  \
             flip <- apply(fit$rotation, 2, function(v) if (v[which.max(abs(v))] < 0) -1 else 1)\n  \
             fit$rotation <- sweep(fit$rotation, 2, flip, \"*\")\n  \
             fit$x <- sweep(fit$x, 2, flip, \"*\")\n  fit\n})\n\
             print(local({\n  eig <- pca$sdev^2\n  \
             data.frame(Eigenvalue = eig, Proportion = eig / sum(eig), Cumulative = cumsum(eig) / sum(eig), row.names = colnames(pca$rotation))\n}))\n\
             cat(\"\\nLoadings (eigenvectors)\\n\")\nprint(pca$rotation)"
        );
        let mut v = values(
            &PRINCIPAL_COMPONENTS,
            &[("save", "comp")],
            &[("x", &["height", "weight"])],
        );
        v.set_flag("scale", false);
        v.set_flag("scores", true);
        let c = code(&PRINCIPAL_COMPONENTS, &v);
        assert!(c.contains("scale. = FALSE"), "{c}");
        assert!(c.ends_with("print(utils::head(comp$x))"), "{c}");
        v.set_flag("stata_sign", true);
        let c = code(&PRINCIPAL_COMPONENTS, &v);
        assert!(
            c.contains("flip <- ifelse(colSums(fit$rotation) < 0, -1, 1)"),
            "{c}"
        );
        let v = values(
            &PRINCIPAL_COMPONENTS,
            &[("save", "d")],
            &[("x", &["height", "weight"])],
        );
        assert_eq!(widgets(&PRINCIPAL_COMPONENTS, &v), vec!["save"]);
        // two variables at least
        let v = values(&PRINCIPAL_COMPONENTS, &[], &[("x", &["height"])]);
        assert!(PRINCIPAL_COMPONENTS
            .generate(&ctx(), &v, Style::Base)
            .is_err());
    }

    #[test]
    fn factor_analysis_writes_factanal() {
        let v = values(
            &FACTOR_ANALYSIS,
            &[("factors", "1")],
            &[("x", &["height", "weight", "age"])],
        );
        let c = code(&FACTOR_ANALYSIS, &v);
        assert!(
            c.starts_with(
                "fa <- local({\n  fit <- local({\n    for (fr in c(10, 1e3, 1e5, 1e7)) {"
            ),
            "{c}"
        );
        // the tightest stop is tried first; a failure is retried looser, then reported
        assert!(
            c.contains(
                "stats::factanal(~ height + weight + age, factors = 1, data = d, \
                 rotation = \"none\", control = list(opt = list(factr = fr)))"
            ),
            "{c}"
        );
        assert!(
            c.contains("stop(conditionMessage(r), call. = FALSE)"),
            "{c}"
        );
        // the statistic is Stata's: N, not N - 1, in the multiplier
        assert!(
            c.contains("(n - (2 * p + 5) / 6 - 2 * k / 3) / (n - 1 - (2 * p + 5) / 6 - 2 * k / 3)"),
            "{c}"
        );
        assert!(c.contains("fit$PVAL <- stats::pchisq("), "{c}");
        assert!(c.ends_with("  fit\n})\nprint(fa)"), "{c}");
        let v = values(
            &FACTOR_ANALYSIS,
            &[("factors", "2"), ("rotation", "varimax"), ("save", "f2")],
            &[("x", &["height", "weight", "age", "arm span", "height"])],
        );
        let c = code(&FACTOR_ANALYSIS, &v);
        assert!(
            c.starts_with("f2 <- local({\n  fit <- local({\n    for (fr in c(10, 1e3,"),
            "{c}"
        );
        assert!(
            c.contains("if (ncol(A) < 3) kaiser(A) else unclass(stats::varimax(A, normalize = FALSE, eps = 1e-8)"),
            "{c}"
        );
        assert!(c.ends_with("print(f2)"), "{c}");
        // principal factors, promax with its power; iterated, oblimin; exact varimax
        let v = values(
            &FACTOR_ANALYSIS,
            &[
                ("method", "pf"),
                ("factors", "2"),
                ("rotation", "promax"),
                ("power", "4"),
            ],
            &[("x", &["height", "weight", "age", "arm span"])],
        );
        let c = code(&FACTOR_ANALYSIS, &v);
        assert!(
            c.contains("h <- 1 - 1 / diag(solve(R))")
                && c.contains("kk <- min(k, sum(e$values > 0))")
                && c.contains("Q <- V * abs(V)^(4 - 1)")
                && c.contains("Principal factors, %d rows")
                && !c.contains("factanal"),
            "{c}"
        );
        let mut v = values(
            &FACTOR_ANALYSIS,
            &[
                ("method", "ipf"),
                ("factors", "2"),
                ("rotation", "oblimin"),
                ("gamma", "0.5"),
            ],
            &[("x", &["height", "weight", "age", "arm span"])],
        );
        let c = code(&FACTOR_ANALYSIS, &v);
        assert!(
            c.contains("if (sum((hn - h)^2) < 1e-8) break")
                && c.contains("oblimin(A, 0.5)")
                && c.contains("fit$rotmat <- if (is.null(r$Phi)) NULL"),
            "{c}"
        );
        v.set_text("method", "pcf");
        v.set_text("rotation", "varimax");
        v.set_flag("exact", true);
        let c = code(&FACTOR_ANALYSIS, &v);
        assert!(
            c.contains("h <- rep(1, ncol(R))")
                && c.contains("sum(e$values > 1)")
                && c.contains("vmx <- kaiser\n"),
            "{c}"
        );
    }

    #[test]
    fn factor_analysis_checks_the_number_of_factors() {
        // three variables carry one factor, not two
        let v = values(
            &FACTOR_ANALYSIS,
            &[("factors", "2")],
            &[("x", &["height", "weight", "age"])],
        );
        assert_eq!(widgets(&FACTOR_ANALYSIS, &v), vec!["factors"]);
        let v = values(
            &FACTOR_ANALYSIS,
            &[("factors", "1.5")],
            &[("x", &["height", "weight", "age"])],
        );
        assert!(widgets(&FACTOR_ANALYSIS, &v).contains(&"factors"));
        let v = values(
            &FACTOR_ANALYSIS,
            &[("factors", "0")],
            &[("x", &["height", "weight", "age", "arm span"])],
        );
        assert!(widgets(&FACTOR_ANALYSIS, &v).contains(&"factors"));
        // principal factors need no degrees of freedom, only no more factors
        // than variables; the promax power and oblimin gamma are numbers
        let v = values(
            &FACTOR_ANALYSIS,
            &[("method", "pf"), ("factors", "3")],
            &[("x", &["height", "weight", "age"])],
        );
        assert!(widgets(&FACTOR_ANALYSIS, &v).is_empty());
        let v = values(
            &FACTOR_ANALYSIS,
            &[
                ("method", "pf"),
                ("factors", "4"),
                ("rotation", "promax"),
                ("power", "0.5"),
            ],
            &[("x", &["height", "weight", "age"])],
        );
        assert_eq!(widgets(&FACTOR_ANALYSIS, &v), vec!["factors", "power"]);
    }

    #[test]
    fn cluster_analysis_writes_hclust_and_kmeans() {
        let v = values(&CLUSTER_ANALYSIS, &[], &[("x", &["height", "arm span"])]);
        assert_eq!(
            code(&CLUSTER_ANALYSIS, &v),
            "clusters <- local({\n  x <- stats::na.omit(d[, c(\"height\", \"arm span\")])\n  \
             fit <- stats::hclust(stats::dist(x)^2, method = \"ward.D\")\n  \
             fit$groups <- stats::cutree(fit, k = 3)\n  fit$data <- x\n  fit\n})\n\
             cat(\"Group sizes\\n\")\nprint(table(Group = clusters$groups))\n\
             cat(\"\\nGroup means\\n\")\n\
             print(stats::aggregate(clusters$data, list(Group = clusters$groups), mean))\n\
             cat(\"\\nHeights of the last joins\\n\")\nprint(utils::tail(clusters$height, 5))"
        );
        let v = values(
            &CLUSTER_ANALYSIS,
            &[("method", "single"), ("groups", "4")],
            &[("x", &["height"])],
        );
        let c = code(&CLUSTER_ANALYSIS, &v);
        assert!(
            c.contains("stats::hclust(stats::dist(x), method = \"single\")"),
            "{c}"
        );
        assert!(c.contains("k = 4"), "{c}");
        // k-means from the first rows, and from random rows with a seed
        let v = values(
            &CLUSTER_ANALYSIS,
            &[("method", "kmeans")],
            &[("x", &["height", "weight"])],
        );
        let c = code(&CLUSTER_ANALYSIS, &v);
        assert!(
            c.contains("stats::kmeans(x, centers = x[seq_len(3), , drop = FALSE], algorithm = \"Lloyd\", iter.max = 1000)"),
            "{c}"
        );
        assert!(!c.contains("set.seed"), "{c}");
        let v = values(
            &CLUSTER_ANALYSIS,
            &[("method", "kmeans"), ("start", "random"), ("seed", "42")],
            &[("x", &["height", "weight"])],
        );
        let c = code(&CLUSTER_ANALYSIS, &v);
        assert!(c.contains("set.seed(42)"), "{c}");
        assert!(c.contains("centers = 3, algorithm"), "{c}");
        // other dissimilarities; k-means by L1 from its own loop; the stopping rules
        let mut v = values(
            &CLUSTER_ANALYSIS,
            &[("method", "average"), ("distance", "L1")],
            &[("x", &["height", "weight"])],
        );
        v.set_flag("rules", true);
        let c = code(&CLUSTER_ANALYSIS, &v);
        assert!(
            c.contains(
                "stats::hclust(stats::dist(x, method = \"manhattan\"), method = \"average\")"
            ) && c.contains("Calinski.Harabasz")
                && c.contains("out$Duda.Hart[k] <- j2 / j1")
                && c.contains("seq_len(max(1, min(10, n - 1)))"),
            "{c}"
        );
        let v = values(
            &CLUSTER_ANALYSIS,
            &[("method", "single"), ("distance", "canberra")],
            &[("x", &["height", "weight"])],
        );
        assert!(code(&CLUSTER_ANALYSIS, &v).contains("abs(sweep(m, 2, m[i, ])) / b"));
        let mut v = values(
            &CLUSTER_ANALYSIS,
            &[("method", "kmeans"), ("distance", "Linf")],
            &[("x", &["height", "weight"])],
        );
        v.set_flag("rules", true);
        let c = code(&CLUSTER_ANALYSIS, &v);
        assert!(
            c.contains("far <- function(c) apply(abs(sweep(m, 2, c)), 1, max)")
                && c.contains("ties.method = \"first\"")
                && !c.contains("stats::kmeans")
                && !c.contains("Duda"),
            "{c}"
        );
    }

    #[test]
    fn cluster_analysis_checks_its_numbers() {
        let v = values(&CLUSTER_ANALYSIS, &[("groups", "1")], &[("x", &["height"])]);
        assert_eq!(widgets(&CLUSTER_ANALYSIS, &v), vec!["groups"]);
        let v = values(
            &CLUSTER_ANALYSIS,
            &[("method", "kmeans"), ("start", "random"), ("seed", "-1")],
            &[("x", &["height"])],
        );
        assert_eq!(widgets(&CLUSTER_ANALYSIS, &v), vec!["seed"]);
        // the seed does not matter for hierarchical clustering
        let v = values(&CLUSTER_ANALYSIS, &[("seed", "-1")], &[("x", &["height"])]);
        assert!(widgets(&CLUSTER_ANALYSIS, &v).is_empty());
    }

    #[test]
    fn discriminant_analysis_writes_lda_and_its_table() {
        let v = values(
            &DISCRIMINANT_ANALYSIS,
            &[("group", "species")],
            &[("x", &["height", "arm span"])],
        );
        let c = code(&DISCRIMINANT_ANALYSIS, &v);
        assert!(
            c.starts_with(
                "lda <- MASS::lda(species ~ height + `arm span`, data = d, \
                 prior = rep(1 / nlevels(factor(d$species)), nlevels(factor(d$species))))\n\
                 lda[c(\"scaling\", \"svd\")] <- MASS::lda(species ~ height + `arm span`, \
                 data = d)[c(\"scaling\", \"svd\")]\nprint(lda)\n"
            ),
            "{c}"
        );
        assert!(c.contains("stats::predict(lda, rows)$class"), "{c}");
        assert!(c.contains("stats::addmargins(tab)"), "{c}");
        let v = values(
            &DISCRIMINANT_ANALYSIS,
            &[("group", "species"), ("priors", "proportional")],
            &[("x", &["height"])],
        );
        assert!(code(&DISCRIMINANT_ANALYSIS, &v)
            .starts_with("lda <- MASS::lda(species ~ height, data = d)\nprint(lda)\n"));
        // quadratic: qda, with the priors, and no canonical functions
        let v = values(
            &DISCRIMINANT_ANALYSIS,
            &[("group", "species"), ("method", "quadratic")],
            &[("x", &["height"])],
        );
        let c = code(&DISCRIMINANT_ANALYSIS, &v);
        assert!(
            c.starts_with("lda <- MASS::qda(species ~ height, data = d, prior = rep(")
                && !c.contains("scaling")
                && c.contains("stats::predict(lda, rows)$class"),
            "{c}"
        );
        // the group is not also a predictor
        let v = values(
            &DISCRIMINANT_ANALYSIS,
            &[("group", "age")],
            &[("x", &["age", "height"])],
        );
        assert_eq!(widgets(&DISCRIMINANT_ANALYSIS, &v), vec!["x"]);
    }
}
