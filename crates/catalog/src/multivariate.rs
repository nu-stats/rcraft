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
            is positive. Rows with a missing value are left out.",
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
        let mut code = format!(
            "{m} <- local({{\n  fit <- stats::prcomp(~ {}, data = {}, scale. = {})\n  \
             flip <- apply(fit$rotation, 2, function(v) if (v[which.max(abs(v))] < 0) -1 else 1)\n  \
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

const ROTATIONS: &[Opt] = &[
    Opt {
        value: "none",
        label: "None",
    },
    Opt {
        value: "varimax",
        label: "Varimax",
    },
];

fn factor_check(a: &Args) -> Vec<Problem> {
    let mut out = Vec::new();
    let f = whole(a, "factors", "The number of factors", 1, &mut out);
    let p = a.vars("x").len() as i64;
    let f = i64::from(f);
    if p >= 3 && (p - f) * (p - f) < p + f {
        out.push(Problem::new(
            "factors",
            format!("{f} factors is too many for {p} variables: the model would have more unknowns than the variables give."),
        ));
    }
    not_the_data(a, &mut out);
    out
}

pub const FACTOR_ANALYSIS: Spec = Spec {
    entry: "Factor analysis",
    menu: "Statistics > Multivariate > Factor analysis",
    about: "Explains the correlations among the chosen variables by a few common factors. It fits \
            by maximum likelihood with stats::factanal and prints each variable's uniqueness (the \
            part no factor explains), the loadings, the variation each factor carries and the \
            likelihood-ratio test of the number of factors. A varimax rotation (without Kaiser \
            normalisation) turns the factors to simplify the loadings; factors are then ordered \
            by the variation they carry and signed so their loadings sum to more than zero. \
            Rows with a missing value are left out.",
    widgets: &[
        Widget::Variables {
            id: "x",
            label: "Variables",
            kinds: NUMBERS,
            min: 3,
            hint: "Three or more numeric variables.",
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
        let fit = format!(
            "stats::factanal(~ {}, factors = {}, data = {}, rotation = \"none\", \
             control = list(opt = list(factr = 10)))",
            rhs(a),
            a.number("factors"),
            a.data()
        );
        if a.choice("rotation") == "varimax" {
            format!(
                "{m} <- local({{\n  fit <- {fit}\n  \
                 rot <- stats::varimax(fit$loadings, normalize = FALSE, eps = 1e-15)\n  \
                 load <- unclass(rot$loadings)\n  \
                 load <- load[, order(colSums(load^2), decreasing = TRUE), drop = FALSE]\n  \
                 load <- sweep(load, 2, ifelse(colSums(load) < 0, -1, 1), \"*\")\n  \
                 fit$loadings <- structure(load, class = \"loadings\")\n  \
                 fit$rotmat <- NULL\n  fit\n}})\nprint({m})"
            )
        } else {
            format!("{m} <- {fit}\nprint({m})")
        }
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

fn cluster_check(a: &Args) -> Vec<Problem> {
    let mut out = Vec::new();
    whole(a, "groups", "The number of groups", 2, &mut out);
    if a.choice("method") == "kmeans" && a.choice("start") == "random" {
        whole(a, "seed", "The seed", 0, &mut out);
    }
    not_the_data(a, &mut out);
    out
}

pub const CLUSTER_ANALYSIS: Spec = Spec {
    entry: "Cluster analysis",
    menu: "Statistics > Multivariate > Cluster analysis",
    about: "Sorts the rows into groups of similar rows. Hierarchical clustering (stats::hclust on \
            Euclidean distances; Ward's linkage uses the squared distances) joins the closest \
            rows and groups step by step and is cut into the number of groups asked for with \
            stats::cutree. K-means (stats::kmeans, Lloyd's algorithm) starts from the first rows \
            or from random rows with a seed and moves each row to its nearest group centre until \
            nothing moves. The group sizes and means are printed; the group of each row is \
            `name$groups`. Rows with a missing value are left out.",
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
        let take = format!("x <- stats::na.omit({}[, {}])", a.data(), names_vector(a));
        if method == "kmeans" {
            let (seed, centers) = if a.choice("start") == "random" {
                (format!("  set.seed({})\n", a.number("seed")), k.to_string())
            } else {
                (String::new(), format!("x[seq_len({k}), , drop = FALSE]"))
            };
            format!(
                "{m} <- local({{\n  {take}\n{seed}  \
                 fit <- stats::kmeans(x, centers = {centers}, algorithm = \"Lloyd\", \
                 iter.max = 1000)\n  fit$groups <- fit$cluster\n  fit$data <- x\n  fit\n}})\n\
                 cat(\"Group sizes\\n\")\nprint(table(Group = {m}$groups))\n\
                 cat(\"\\nGroup means\\n\")\n\
                 print(stats::aggregate({m}$data, list(Group = {m}$groups), mean))"
            )
        } else {
            let distance = if method == "ward" {
                "stats::dist(x)^2"
            } else {
                "stats::dist(x)"
            };
            let linkage = if method == "ward" { "ward.D" } else { method };
            format!(
                "{m} <- local({{\n  {take}\n  \
                 fit <- stats::hclust({distance}, method = {})\n  \
                 fit$groups <- stats::cutree(fit, k = {k})\n  fit$data <- x\n  fit\n}})\n\
                 cat(\"Group sizes\\n\")\nprint(table(Group = {m}$groups))\n\
                 cat(\"\\nGroup means\\n\")\n\
                 print(stats::aggregate({m}$data, list(Group = {m}$groups), mean))\n\
                 cat(\"\\nHeights of the last joins\\n\")\nprint(utils::tail({m}$height, 5))",
                r_string(linkage)
            )
        }
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
            share of the separation, then the table of true against assigned groups for the \
            rows used. Rows with a missing value are left out.",
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
        let prior = if a.choice("priors") == "equal" {
            let n = format!("nlevels(factor({}${g}))", a.data());
            format!(", prior = rep(1 / {n}, {n})")
        } else {
            String::new()
        };
        format!(
            "{m} <- MASS::lda({formula}, data = {}{prior})\nprint({m})\n\
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
        assert_eq!(
            code(&FACTOR_ANALYSIS, &v),
            "fa <- stats::factanal(~ height + weight + age, factors = 1, data = d, rotation = \"none\", \
             control = list(opt = list(factr = 10)))\nprint(fa)"
        );
        let v = values(
            &FACTOR_ANALYSIS,
            &[("factors", "2"), ("rotation", "varimax"), ("save", "f2")],
            &[("x", &["height", "weight", "age", "arm span", "height"])],
        );
        let c = code(&FACTOR_ANALYSIS, &v);
        assert!(
            c.starts_with("f2 <- local({\n  fit <- stats::factanal("),
            "{c}"
        );
        assert!(
            c.contains("stats::varimax(fit$loadings, normalize = FALSE"),
            "{c}"
        );
        assert!(c.ends_with("print(f2)"), "{c}");
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
                 prior = rep(1 / nlevels(factor(d$species)), nlevels(factor(d$species))))\nprint(lda)\n"
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
            .starts_with("lda <- MASS::lda(species ~ height, data = d)\n"));
        // the group is not also a predictor
        let v = values(
            &DISCRIMINANT_ANALYSIS,
            &[("group", "age")],
            &[("x", &["age", "height"])],
        );
        assert_eq!(widgets(&DISCRIMINANT_ANALYSIS, &v), vec!["x"]);
    }
}
