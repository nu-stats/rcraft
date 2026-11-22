//! `cluster`: hierarchical linkage, kmeans and kmedians, generate, list, dir and drop.

use super::super::*;

/// The names in `text` when each is a plain variable name.
fn plain(text: &str) -> Option<Vec<String>> {
    let v: Vec<String> = text.split_whitespace().map(str::to_string).collect();
    (!v.is_empty() && v.iter().all(|w| is_name(w))).then_some(v)
}

fn r_vec(v: &[String]) -> String {
    let q: Vec<String> = v.iter().map(|s| r_text(s)).collect();
    format!("c({})", q.join(", "))
}

impl Session {
    /// `cluster subcommand ...`.
    pub(in crate::data_r) fn cluster(
        &mut self,
        c: &Command,
        opts: &Opts,
        out: &mut Out,
    ) -> Result<(), Untranslated> {
        self.no_by(c, "cluster")?;
        if c.parts.weight.is_some() {
            return Err(no("cluster with weights is not translated yet"));
        }
        let (sub, rest) = first_word(&c.parts.main);
        let linkage = match sub {
            "singlelinkage" | "single" | "sing" => Some("single"),
            "completelinkage" | "complete" | "comp" => Some("complete"),
            "averagelinkage" | "average" | "a" | "av" | "ave" => Some("average"),
            "wardslinkage" | "wards" | "ward" => Some("wards"),
            _ => None,
        };
        let cmdline = super::super::returned::cmdline(c);
        if let Some(l) = linkage {
            return self.cluster_fit(c, opts, out, rest, ClFit::Linkage(l), &cmdline);
        }
        match sub {
            "kmeans" | "km" => self.cluster_fit(c, opts, out, rest, ClFit::K(false), &cmdline),
            "kmedians" | "kmed" => self.cluster_fit(c, opts, out, rest, ClFit::K(true), &cmdline),
            "generate" | "gen" => self.cluster_generate(c, opts, out, rest),
            "list" | "dir" | "drop" => {
                if opts.other(&[]).is_some() || c.parts.if_exp.is_some() || c.parts.in_range.is_some()
                {
                    return Err(no(format!(
                        "cluster {sub} with options or a condition is not translated yet"
                    )));
                }
                let base = self.data()?;
                self.helpers.insert("stata_cluster");
                self.helpers.insert("stata_char_var");
                let names: Vec<String> = rest.split_whitespace().map(str::to_string).collect();
                if sub != "dir" && !names.iter().all(|n| is_name(n)) {
                    return Err(no("cluster: object names written out are translated"));
                }
                match sub {
                    "dir" => {
                        if !names.is_empty() {
                            return Err(no("cluster dir takes no names"));
                        }
                        if !self.quiet(c) {
                            out.line(format!("stata_cl_dir({base})"));
                        }
                    }
                    "list" => {
                        if !self.quiet(c) {
                            let arg = if names.is_empty() {
                                String::new()
                            } else {
                                format!(", {}", r_vec(&names))
                            };
                            out.line(format!("stata_cl_list({base}{arg})"));
                        }
                    }
                    _ => {
                        if names.is_empty() {
                            return Err(no("cluster drop needs an object name or _all"));
                        }
                        out.line(format!(
                            "{base} <- stata_cl_drop({base}, {})",
                            r_vec(&names)
                        ));
                        // the variables an object made go with it
                        for n in &names {
                            for sfx in ["_id", "_ord", "_hgt"] {
                                self.kinds.remove(&format!("{n}{sfx}"));
                            }
                        }
                    }
                }
                Ok(())
            }
            _ => Err(no(format!(
                "cluster {sub} is not translated yet (linkage methods, kmeans, kmedians, generate, list, dir and drop are)"
            ))),
        }
    }

    fn cluster_fit(
        &mut self,
        c: &Command,
        opts: &Opts,
        out: &mut Out,
        rest: &str,
        fit: ClFit,
        cmdline: &str,
    ) -> Result<(), Untranslated> {
        if c.parts.if_exp.is_some() || c.parts.in_range.is_some() {
            return Err(no("cluster with if or in is not translated yet"));
        }
        let known: &[(&str, usize)] = match fit {
            ClFit::Linkage(_) => &[("name", 1), ("measure", 1)],
            ClFit::K(_) => &[("k", 1), ("start", 5), ("name", 1), ("measure", 1)],
        };
        if let Some(o) = opts.other(known) {
            return Err(no(format!(
                "cluster with the option {o} is not translated yet"
            )));
        }
        let Some(vars) = plain(rest) else {
            return Err(no(
                "cluster with variables not written out is not translated yet",
            ));
        };
        let name = opts.arg("name", 1).map(|n| n.trim().to_string());
        if let Some(n) = &name {
            if !is_name(n) {
                return Err(no("cluster: name() takes one name"));
            }
        }
        let measure = opts.arg("measure", 1).map(|m| m.trim().to_string());
        let base = self.data()?;
        for h in ["stata_cluster", "stata_char_var"] {
            self.helpers.insert(h);
        }
        let name_arg = name
            .as_ref()
            .map(|n| format!(", name = {}", r_text(n)))
            .unwrap_or_default();
        let measure_arg = measure
            .as_ref()
            .map(|m| format!(", measure = {}", r_text(m)))
            .unwrap_or_default();
        let call = match fit {
            ClFit::Linkage(l) => format!(
                "stata_cl_hier({base}, {}, {}{measure_arg}{name_arg}, cmdline = {})",
                r_vec(&vars),
                r_text(l),
                r_text(cmdline)
            ),
            ClFit::K(medians) => {
                let Some(k) = opts.arg("k", 1).and_then(|k| k.trim().parse::<u32>().ok()) else {
                    out.line("stop(\"option k() required (Stata: r(198))\", call. = FALSE)");
                    return Ok(());
                };
                let start = opts
                    .arg("start", 5)
                    .map(|s| s.trim().to_lowercase())
                    .unwrap_or_else(|| "krandom".to_string());
                if !["firstk", "lastk", "krandom"].contains(&start.as_str()) {
                    return Err(no(format!("cluster start({start}) is not translated yet")));
                }
                format!(
                    "stata_cl_k({base}, {}, {k}, medians = {}, start = {}{measure_arg}{name_arg}, cmdline = {})",
                    r_vec(&vars),
                    if medians { "TRUE" } else { "FALSE" },
                    r_text(&start),
                    r_text(cmdline)
                )
            }
        };
        out.line(format!("{base} <- {call}"));
        // the variables an object adds (those with a name() are known here)
        if let Some(n) = &name {
            match fit {
                ClFit::Linkage(_) => {
                    for sfx in ["_id", "_ord", "_hgt"] {
                        self.kinds.insert(format!("{n}{sfx}"), Kind::Number);
                    }
                }
                ClFit::K(_) => {
                    self.kinds.insert(n.clone(), Kind::Number);
                }
            }
        }
        Ok(())
    }

    fn cluster_generate(
        &mut self,
        c: &Command,
        opts: &Opts,
        out: &mut Out,
        rest: &str,
    ) -> Result<(), Untranslated> {
        if let Some(o) = opts.other(&[("name", 1)]) {
            return Err(no(format!(
                "cluster generate with the option {o} is not translated yet"
            )));
        }
        let Some((newvar, spec)) = rest.split_once('=') else {
            return Err(no("cluster generate needs newvar = groups(#)"));
        };
        let newvar = newvar.trim();
        let spec = spec.trim();
        let Some(inner) = spec
            .strip_prefix("groups(")
            .or_else(|| spec.strip_prefix("group("))
            .and_then(|s| s.strip_suffix(')'))
        else {
            return Err(no("cluster generate: only groups() is translated"));
        };
        if !is_name(newvar) {
            return Err(no(
                "cluster generate: a plain new variable name is translated",
            ));
        }
        let ks: Vec<u32> = if let Some((a, b)) = inner.split_once('/') {
            match (a.trim().parse::<u32>(), b.trim().parse::<u32>()) {
                (Ok(a), Ok(b)) if a >= 1 && b >= a && b - a < 200 => (a..=b).collect(),
                _ => return Err(no("cluster generate: groups(# / #) needs whole numbers")),
            }
        } else {
            let v: Result<Vec<u32>, _> = inner.split_whitespace().map(str::parse::<u32>).collect();
            match v {
                Ok(v) if !v.is_empty() && v.iter().all(|k| *k >= 1) => v,
                _ => return Err(no("cluster generate: groups() needs whole numbers")),
            }
        };
        let name = opts.arg("name", 1).map(|n| n.trim().to_string());
        if let Some(n) = &name {
            if !is_name(n) {
                return Err(no("cluster: name() takes one name"));
            }
        }
        let base = self.data()?;
        for h in ["stata_cluster", "stata_char_var"] {
            self.helpers.insert(h);
        }
        let ks_r: Vec<String> = ks.iter().map(|k| format!("{k}L")).collect();
        out.line(format!(
            "{base} <- stata_cl_generate({base}, {}, c({}){})",
            r_text(newvar),
            ks_r.join(", "),
            name.as_ref()
                .map(|n| format!(", name = {}", r_text(n)))
                .unwrap_or_default()
        ));
        let _ = c;
        for k in &ks {
            let v = if ks.len() > 1 {
                format!("{newvar}{k}")
            } else {
                newvar.to_string()
            };
            self.kinds.insert(v, Kind::Number);
        }
        Ok(())
    }
}

/// What `cluster` fits.
#[derive(Clone, Copy)]
enum ClFit {
    Linkage(&'static str),
    /// kmeans (false) or kmedians (true)
    K(bool),
}
