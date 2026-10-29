//! P2B.3: Stata's data commands translated to R and run in a real R.
//!
//! Each case is a short do-file over made-up data. Its translation must run in
//! R with every line translated. When `RCRAFT_STATCRAFT` names a StatCraft
//! executable (our Stata-compatible program, whose data commands are checked
//! line by line against Stata), the same do-file also runs there, and the two
//! saved datasets must hold the same variables, rows and values, in the same
//! order. Without R the test is skipped with a note.

#![allow(clippy::panic)]

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use rcraft_engine::{Engine, Item, Outcome, Source};
use rcraft_rhost::{find_installs, Env, SessionConfig};
use rcraft_statatr::data_r::translate;
use rcraft_statatr::reader::read;

fn engine() -> Option<Engine> {
    let install = find_installs(&Env::from_process())
        .into_iter()
        .find(|i| i.is_supported());
    let Some(install) = install else {
        eprintln!("skipped: no supported R on this machine");
        return None;
    };
    Engine::start(SessionConfig::new(install)).ok()
}

fn settle(e: &mut Engine) {
    let limit = Instant::now() + Duration::from_secs(120);
    loop {
        e.poll(Duration::from_millis(50));
        if !e.is_working() {
            break;
        }
        assert!(Instant::now() < limit, "the engine did not settle");
    }
}

fn run(e: &mut Engine, code: &str) -> (Outcome, String) {
    let id = e.submit(Source::Typed, code);
    assert!(id.is_ok(), "{id:?}");
    let id = id.unwrap_or_default();
    settle(e);
    let outcome = e
        .history()
        .entries()
        .last()
        .and_then(|h| h.outcome)
        .unwrap_or(Outcome::Error);
    let text = e
        .results()
        .blocks()
        .iter()
        .find(|b| b.command == id)
        .map(|b| {
            b.items
                .iter()
                .map(|i| match i {
                    Item::Text { text, .. } => text.clone(),
                    Item::Warning(w) => format!("warning: {w}"),
                    Item::Error { message, .. } => format!("error: {message}"),
                    Item::Notice(n) => n.clone(),
                    Item::Plot { .. } => String::new(),
                })
                .collect::<Vec<_>>()
                .join("")
        })
        .unwrap_or_default();
    (outcome, text)
}

fn slash(p: &Path) -> String {
    p.display().to_string().replace('\\', "/")
}

/// The made-up datasets, written by R with haven.
fn fixtures(dir: &str) -> String {
    format!(
        "local({{
  d <- '{dir}'
  haven::write_dta(data.frame(
    id = 1:12,
    g = c('a', 'b', 'a', '', 'b', 'a', 'c', 'b', 'a', 'c', '', 'b'),
    x = c(3, 1, NA, 7, 2, 5, 5, NA, 9, 1, 4, 6),
    y = c(10, NA, 30, 40, NA, 60, 70, 80, NA, 100, 110, 120),
    s = c('1', '2.5', '', '.', ' 7 ', '1e3', '8', '9', '10', '11', '12', '13'),
    w = c('north', 'south', 'east', '', 'west', 'north', 'east', 'south', 'b', 'B', 'a', 'west')
  ), file.path(d, 'start.dta'))
  haven::write_dta(data.frame(id = c(2, 4, 6, 13, 14), x = c(100, 200, 300, 400, 500), extra = c(1.5, NA, 3.5, 4.5, 5.5)), file.path(d, 'using.dta'))
  haven::write_dta(data.frame(g = c('a', 'b', 'c', 'd'), gv = c(10, 20, 30, 40)), file.path(d, 'groups.dta'))
  haven::write_dta(data.frame(id = c(13, 14), g = c('d', 'a'), x = c(11, NA), more = c(1, 2)), file.path(d, 'more.dta'))
  haven::write_dta(data.frame(id = c(1, 2, 3), sex = c(0, 1, 0), inc2020 = c(10, 20, NA), inc2021 = c(11, 21, 31), ue2020 = c(0, 1, 0), ue2021 = c(1, NA, 0)), file.path(d, 'wide.dta'))
  haven::write_dta(data.frame(a = c(1, 1, 2, 2, 2, 3, NA, NA), b = c('x', 'x', 'y', 'y', 'z', 'x', '', '')), file.path(d, 'dups.dta'))
  writeLines(c('Name,Score,Note,Count', 'ann,1.5,NA,3', 'bob,,x,4', ',3,,5'), file.path(d, 'scores.csv'))
  writexl::write_xlsx(data.frame(Site = c('A', 'B', 'C'), Value = c(1.25, NA, 3)), file.path(d, 'sites.xlsx'))
}})"
    )
}

/// The cases: a name, the dataset to start from and the commands.
const CASES: &[(&str, &str, &str)] = &[
    (
        "generate",
        "start",
        "gen z = x * 2 if g == \"a\"\nreplace y = 0 if y == .\ngen double w2 = 0.1\ngen f = 0.1\ngen big = x > 4\ngen str3 t = w + \"!\" if x < 6\nbysort g: gen n = _n\nby g: gen N = _N\ngen lagx = x[_n-1]",
    ),
    ("drop_keep", "start", "drop if x > 5\nkeep id x g\nkeep in 2/6"),
    ("sort_gsort", "start", "gsort -x g\ngen order1 = _n\nsort g x\ngen order2 = _n"),
    (
        "recode",
        "start",
        "recode x (1 2 = 10) (3/5 = 20) (missing = 99) (else = 0), gen(xr)\nrecode y (min/50 = 1) (60/max = 2) if id > 3",
    ),
    (
        "egen",
        "start",
        "egen m = mean(x), by(g)\negen sd = sd(x), by(g)\negen mn = min(y)\negen mx = max(y), by(g)\negen tot = total(x), by(g)\negen cnt = count(y), by(g)\negen md = median(x)\negen p25 = pctile(x), p(25)\negen r = rank(x)\negen rf = rank(x), field\negen grp = group(g)\negen gm = group(g), missing\negen tg = tag(g)\negen rt = rowtotal(x y)\negen rm = rowmean(x y)\negen rx = rowmax(x y)\negen nm = rownonmiss(x y)\negen st = std(x)",
    ),
    (
        "collapse",
        "start",
        "collapse (mean) mx=x (sd) sx=x (sum) tot=x (count) n=x (median) md=x (max) top=x (min) low=y (first) f=x (lastnm) l=y (p75) q3=x, by(g)",
    ),
    ("collapse_all", "start", "collapse x y"),
    ("merge_11", "start", "merge 1:1 id using \"DIR/using.dta\""),
    ("merge_m1", "start", "merge m:1 g using \"DIR/groups.dta\", keep(master match)"),
    ("merge_gen", "start", "merge 1:1 id using \"DIR/using.dta\", gen(how) keepusing(extra)"),
    ("append", "start", "append using \"DIR/more.dta\", gen(src)"),
    ("reshape_long", "wide", "reshape long inc ue, i(id) j(year)"),
    ("reshape_wide", "wide", "reshape long inc ue, i(id) j(year)\nreshape wide inc ue, i(id) j(year)"),
    ("duplicates_drop", "dups", "duplicates drop"),
    ("duplicates_tag", "dups", "duplicates tag a, gen(dup)"),
    (
        "convert",
        "start",
        "encode w, gen(wn)\ndestring s, gen(sn)\ndestring s, replace force\ntostring id, gen(ids)",
    ),
    (
        "names_labels",
        "start",
        "rename (x y) (xx yy)\norder w g, last\nlabel define big 0 \"small\" 1 \"large\"\ngen b = xx > 4\nlabel values b big\nlabel variable b \"Is big\"",
    ),
    // no dataset to start from: the file is imported
    ("import_csv", "", "import delimited using \"DIR/scores.csv\", clear"),
    ("import_excel", "", "import excel using \"DIR/sites.xlsx\", firstrow clear"),
    ("import_excel_letters", "", "import excel using \"DIR/sites.xlsx\", clear"),
];

#[test]
fn data_commands_translate_and_run_like_stata() {
    let Some(mut e) = engine() else { return };
    settle(&mut e);
    let (have, _) = run(&mut e, "stopifnot(requireNamespace(\"dplyr\", quietly = TRUE), requireNamespace(\"haven\", quietly = TRUE))");
    if have != Outcome::Ok {
        eprintln!("skipped: the R packages dplyr and haven are needed");
        return;
    }
    let (tidyr, _) = run(
        &mut e,
        "stopifnot(requireNamespace(\"tidyr\", quietly = TRUE))",
    );
    let dir: PathBuf = std::env::temp_dir().join(format!("rcraft-p2b3-{}", std::process::id()));
    assert!(std::fs::create_dir_all(&dir).is_ok());
    let d = slash(&dir);
    let (made, text) = run(&mut e, &fixtures(&d));
    assert_eq!(made, Outcome::Ok, "{text}");
    let statcraft = std::env::var("RCRAFT_STATCRAFT").ok().map(PathBuf::from);
    if statcraft.is_none() {
        eprintln!("note: set RCRAFT_STATCRAFT to a statcraft executable to compare with it");
    }

    let mut wrong = Vec::new();
    for (name, start, body) in CASES {
        if name.starts_with("reshape") && tidyr != Outcome::Ok {
            eprintln!("skipped {name}: the R package tidyr is not installed");
            continue;
        }
        let body = body.replace("DIR", &d);
        let open = if start.is_empty() {
            String::new()
        } else {
            format!("use \"{d}/{start}.dta\", clear\n")
        };
        let dofile = |who: &str| format!("{open}{body}\nsave \"{d}/{name}_{who}.dta\", replace\n");
        // R
        let t = translate(&read(&dofile("r")));
        let not: Vec<String> = t
            .lines
            .iter()
            .filter_map(|l| {
                l.result
                    .as_ref()
                    .err()
                    .map(|e| format!("{} -> {}", l.stata, e.reason))
            })
            .collect();
        if !not.is_empty() {
            wrong.push(format!("{name}: not translated: {}", not.join("; ")));
            continue;
        }
        let script = t.script();
        let (ran, text) = run(&mut e, &script);
        if ran != Outcome::Ok {
            wrong.push(format!("{name}: R stopped: {text}\n{script}"));
            continue;
        }
        // StatCraft, when asked
        let Some(sc) = &statcraft else { continue };
        let path = dir.join(format!("{name}.do"));
        assert!(std::fs::write(&path, dofile("s")).is_ok());
        let status = std::process::Command::new(sc)
            .arg("run")
            .arg(&path)
            .current_dir(&dir)
            .status();
        if !status.is_ok_and(|s| s.success()) {
            let log = std::fs::read_to_string(dir.join(format!("{name}.log"))).unwrap_or_default();
            wrong.push(format!("{name}: StatCraft stopped:\n{log}"));
            continue;
        }
        let compare = format!(
            "local({{
  norm <- function(f) {{
    x <- haven::zap_formats(haven::zap_labels(haven::zap_label(haven::read_dta(f))))
    as.data.frame(lapply(x, function(v) {{ attributes(v) <- NULL; v }}))
  }}
  a <- norm('{d}/{name}_s.dta'); b <- norm('{d}/{name}_r.dta')
  if (!identical(names(a), names(b))) stop('variables: StatCraft ', paste(names(a), collapse = ' '), ' | R ', paste(names(b), collapse = ' '))
  if (nrow(a) != nrow(b)) stop('rows: StatCraft ', nrow(a), ' | R ', nrow(b))
  for (n in names(a)) {{
    x <- a[[n]]; y <- b[[n]]
    if (is.character(x) || is.character(y)) ok <- as.character(x) == as.character(y)
    else ok <- (is.na(x) & is.na(y)) | (!is.na(x) & !is.na(y) & abs(x - y) <= 1e-9 * pmax(1, abs(x)))
    ok[is.na(ok)] <- FALSE
    if (!all(ok)) stop(n, ' differs at rows ', paste(head(which(!ok)), collapse = ','), ': StatCraft ', paste(head(x[!ok]), collapse = ','), ' | R ', paste(head(y[!ok]), collapse = ','))
  }}
}})"
        );
        let (same, text) = run(&mut e, &compare);
        if same != Outcome::Ok {
            wrong.push(format!("{name}: {text}"));
        }
    }
    let _ = std::fs::remove_dir_all(&dir);
    assert!(wrong.is_empty(), "\n{}", wrong.join("\n\n"));
}
