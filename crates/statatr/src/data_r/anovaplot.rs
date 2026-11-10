//! `anovaplot x g [, scatter(...) titles]` (Nick Cox, Stata Journal gr0009_2),
//! as Stata 19 draws it after `anova` (observed with made-up data exported as
//! SVG; its code and help were not read, see `plan/stata/anovaplot.md`): over
//! the model's own rows, its fitted values joined along x for each value of g
//! (one line each, labelled with g's value label), the data as points (none
//! with `scatter(ms(i))`), x's values as its ticks with their labels, y titled
//! with the dependent variable's label, x with x's, and "Profiles by" g's label
//! (else its name) above.

use super::*;

impl Session {
    pub(super) fn anovaplot(
        &mut self,
        c: &Command,
        opts: &Opts,
        out: &mut Out,
    ) -> Result<(), Untranslated> {
        if c.parts.if_exp.is_some() || c.parts.in_range.is_some() {
            return Err(no("anovaplot with if or in is not translated yet"));
        }
        let words: Vec<&str> = c.parts.main.split_whitespace().collect();
        let [x, g] = words.as_slice() else {
            return Err(no(
                "anovaplot is translated with two variables, the x one and the one for the lines",
            ));
        };
        let (x, g) = (x.to_string(), g.to_string());
        let fitted_by_anova =
            matches!(&self.e_ret, super::returned::Returned::Kept { cmd, .. } if cmd == "anova");
        let Some(est) = self.estimates.as_ref().filter(|_| fitted_by_anova) else {
            return Err(no(
                "anovaplot is translated after anova fitted earlier in this do-file",
            ));
        };
        if self.changed_since_fit {
            return Err(no(
                "anovaplot: the data changed since anova was fitted (Stata draws the data as it is now)",
            ));
        }
        for v in [&x, &g] {
            if !est.factors.contains_key(v.as_str()) {
                return Err(no(format!(
                    "anovaplot {v}: a variable that is not one of the model's categorical terms is not translated yet"
                )));
            }
        }
        let dep = est.dep.clone();
        // scatter(...): how the points look; ms(i) or msymbol(none) hides them
        let mut points = true;
        if let Some(s) = opts.arg("scatter", 7) {
            for o in crate::lang::syntax::options(&s) {
                let a = o.raw.clone().unwrap_or_default();
                if o.is("msymbol", 2) {
                    if matches!(a.trim(), "i" | "none") {
                        points = false;
                    }
                } else if !super::graph::LOOK.iter().any(|(n, m)| o.is(n, *m)) {
                    return Err(no(format!(
                        "anovaplot, scatter({}) is not translated yet",
                        o.name
                    )));
                }
            }
        }
        let d = self.data()?;
        self.helpers.insert("stata_graph");
        self.helpers.insert("stata_tab_values");
        out.package("ggplot2");
        out.line(format!(
            "stata_ap <- local({{ e <- stata_data; gl <- stata_tab_values(e[[{g}]]); data.frame(x = stata_num(e[[{x}]]), g = factor(as.character(gl), levels = unique(as.character(if (is.factor(gl)) levels(gl) else sort(unique(gl))))), y = stata_num(e[[{y}]]), fit = as.numeric(stats::fitted(stata_estimates))) }})",
            g = r_text(&g),
            x = r_text(&x),
            y = r_text(&dep)
        ));
        // x's ticks: its values in the model's rows, with their labels
        out.line(format!(
            "stata_ap_x <- local({{ v <- sort(unique(stata_ap$x)); l <- attr(stata_data[[{x}]], \"labels\", exact = TRUE); s <- if (is.null(l)) format(v) else names(l)[match(v, as.numeric(l))]; s[is.na(s)] <- format(v[is.na(s)]); list(at = v, labels = s) }})",
            x = r_text(&x)
        ));
        // each line marked at its cells' fitted values; the data's points, hidden by
        // ms(i), still set the y axis's range as Stata's do
        let mut layers = vec![
            "ggplot2::geom_line(data = stata_ap[order(stata_ap$x), ], ggplot2::aes(x = x, y = fit, colour = g))".to_string(),
            "ggplot2::geom_point(data = unique(stata_ap[, c(\"x\", \"g\", \"fit\")]), ggplot2::aes(x = x, y = fit, colour = g))".to_string(),
        ];
        layers.push(if points {
            "ggplot2::geom_point(data = stata_ap, ggplot2::aes(x = x, y = y))".to_string()
        } else {
            "ggplot2::geom_blank(data = stata_ap, ggplot2::aes(x = x, y = y))".to_string()
        });
        layers.push(
            "ggplot2::scale_x_continuous(breaks = stata_ap_x$at, labels = stata_ap_x$labels)"
                .to_string(),
        );
        layers.push(format!(
            "ggplot2::labs(colour = NULL, subtitle = paste(\"Profiles by\", stata_varlab({d}, {})))",
            r_text(&g)
        ));
        self.draw(
            c,
            opts,
            out,
            &[("scatter", 7)],
            &layers,
            &format!("stata_varlab({d}, {})", r_text(&x)),
            &format!("stata_varlab({d}, {})", r_text(&dep)),
        )?;
        out.caveat("anovaplot as the Stata Journal's gr0009_2 draws it, matched to its output (not its code)");
        Ok(())
    }
}
