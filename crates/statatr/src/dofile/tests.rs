use super::*;

fn run(text: &str) -> DoFile {
    dry_run(text, &NoFiles, Options::default())
}

/// Each line as `stata | what | info or reason`.
fn shown(d: &DoFile) -> Vec<String> {
    d.translation
        .lines
        .iter()
        .zip(&d.origins)
        .map(|(l, o)| {
            let tail = match &l.result {
                Err(e) => format!("not: {}", e.reason),
                Ok(_) => o.info.clone().unwrap_or_default(),
            };
            format!("{} | {:?} | {tail}", l.stata, o.what)
        })
        .collect()
}

fn stata_lines(d: &DoFile, start: &str) -> Vec<String> {
    d.translation
        .lines
        .iter()
        .map(|l| l.stata.clone())
        .filter(|s| s.starts_with(start))
        .collect()
}

#[test]
fn macros_are_set_and_replaced() {
    let d = run(
        "local controls mpg weight\nuse auto\nregress price `controls'\nglobal y price\nsummarize $y",
    );
    let s = shown(&d);
    assert_eq!(
        s[0],
        "local controls mpg weight | Macro | controls is \"mpg weight\""
    );
    assert!(
        s[2].starts_with("regress price mpg weight | Command"),
        "{s:?}"
    );
    assert!(s[4].starts_with("summarize price | Command"), "{s:?}");
    assert!(d.translation.lines[2].result.is_ok());
    assert!(d.translation.lines[4].result.is_ok());
}

#[test]
fn loops_are_run_round_by_round() {
    let d = run(
        "use auto\nforeach v of varlist price mpg {\n  summarize `v'\n}\nforvalues i = 1/2 {\n  display `i'*10\n}",
    );
    let s = shown(&d);
    assert!(s[1].contains("Control | runs 2 times: price, mpg"), "{s:?}");
    assert!(s[2].starts_with("summarize price | Command"), "{s:?}");
    assert!(s[3].starts_with("summarize mpg | Command"), "{s:?}");
    assert!(s[4].contains("runs 2 times: 1, 2"), "{s:?}");
    assert!(s[5].starts_with("display 1*10"), "{s:?}");
    assert!(s[6].starts_with("display 2*10"), "{s:?}");
    assert_eq!(d.origins[3].loops, ["v = mpg"]);
    assert_eq!(d.origins[6].loops, ["i = 2"]);
}

#[test]
fn while_and_if_with_values_known_here() {
    let d = run(
        "local i = 1\nwhile `i' <= 3 {\n  display `i'\n  local ++i\n}\nif `i' == 4 {\n  display \"four\"\n}\nelse {\n  display \"not four\"\n}",
    );
    assert_eq!(
        stata_lines(&d, "display"),
        ["display 1", "display 2", "display 3", "display \"four\""]
    );
}

#[test]
fn a_value_only_r_knows_is_asked_for_and_used() {
    let text = "use auto\nsummarize price\nlocal m = r(mean)\ngenerate d = price - `m'\nif r(N) > 10 {\n  display \"many\"\n}";
    let mut r = Run::new(text, &NoFiles, Options::default());
    let mut asks = Vec::new();
    loop {
        match r.next_event() {
            Event::Line => {}
            Event::Ask(code) => {
                asks.push(code.clone());
                // what R would print: the mean in %18.0g, then r(N) > 10
                let printed = if asks.len() == 1 {
                    "  6165.256756756757\n"
                } else {
                    "                 1\n"
                };
                r.give(Ok(printed.into()));
            }
            Event::Done => break,
        }
    }
    let d = r.finish();
    assert_eq!(asks.len(), 2);
    assert!(asks[0].code.contains("%18.0g"), "{}", asks[0].code);
    assert!(!asks[0].text);
    let s = shown(&d);
    assert!(s[2].contains("m is \"6165.256756756757\""), "{s:?}");
    assert!(
        s[3].starts_with("generate d = price - 6165.256756756757"),
        "{s:?}"
    );
    assert!(s.iter().any(|l| l.starts_with("display \"many\"")), "{s:?}");
}

#[test]
fn a_dry_run_says_what_is_known_only_when_it_runs() {
    let d =
        run("use auto\nsummarize price\nlocal m = r(mean)\ngenerate d = price - `m'\nsummarize d");
    let s = shown(&d);
    assert!(s[2].contains("Later"), "{s:?}");
    assert!(s[3].contains("Later") && s[3].contains("`m'"), "{s:?}");
    assert!(s[4].contains("Command"), "{s:?}");
    assert_eq!(scan(&d).later, 2);
}

#[test]
fn include_and_do_run_other_files() {
    struct Two;
    impl Files for Two {
        fn read(&self, path: &str) -> Option<String> {
            match path {
                "proj/setup.do" => Some("local x hello\nglobal g there".into()),
                "proj/sub/part.do" => Some("display \"`1' `x'\"".into()),
                _ => None,
            }
        }
        fn exists(&self, path: &str) -> bool {
            path == "proj/data.dta"
        }
    }
    let opts = Options {
        folder: "proj".into(),
        ..Options::default()
    };
    let d = dry_run(
        "include setup\ndisplay \"`x' $g\"\ndo \"sub/part.do\" arg1\nuse data\nuse missing\ndo nothing",
        &Two,
        opts,
    );
    let s = shown(&d);
    // include shares the locals; do has its own and gets its arguments
    assert!(
        s.iter().any(|l| l.starts_with("display \"hello there\"")),
        "{s:?}"
    );
    assert!(
        s.iter().any(|l| l.starts_with("display \"arg1 \"")),
        "{s:?}"
    );
    assert!(
        s.iter()
            .any(|l| l.contains("not: the file is not there: proj/nothing.do")),
        "{s:?}"
    );
    assert_eq!(d.files, ["", "proj/setup.do", "proj/sub/part.do"]);
    let sc = scan(&d);
    let status = |p: &str| sc.files.iter().find(|f| f.path == p).map(|f| f.status);
    assert_eq!(status("proj/data.dta"), Some(FileStatus::There));
    assert_eq!(status("proj/missing.dta"), Some(FileStatus::Missing));
}

#[test]
fn cd_moves_the_folder_and_paths_can_be_mapped() {
    let opts = Options {
        folder: "/home/me/proj".into(),
        map: vec![("C:\\Users\\Ann\\Study".into(), "/home/me/study".into())],
        ..Options::default()
    };
    let d = dry_run(
        "global root \"C:\\Users\\Ann\\Study\"\ncd \"$root/data\"\nuse wave1\nsave wave1b, replace\nuse wave1b",
        &NoFiles,
        opts,
    );
    let s = shown(&d);
    assert!(s[1].starts_with("cd \"/home/me/study/data\""), "{s:?}");
    let code = d.translation.lines[1]
        .result
        .as_ref()
        .map(|r| r.code.clone())
        .unwrap_or_default();
    assert_eq!(code, "setwd(\"/home/me/study/data\")");
    let sc = scan(&d);
    let paths: Vec<(&str, FileStatus)> = sc
        .files
        .iter()
        .map(|f| (f.path.as_str(), f.status))
        .collect();
    assert!(
        paths.contains(&("/home/me/study/data/wave1.dta", FileStatus::Missing)),
        "{paths:?}"
    );
    assert!(
        paths.contains(&("/home/me/study/data/wave1b.dta", FileStatus::Written)),
        "{paths:?}"
    );
}

#[test]
fn what_is_not_run_says_why() {
    let d = run(
        "program define hello\n  display \"hi\"\nend\nhello\nmata:\n  x = 1\nend\nforeach v of varlist x* {\n  summarize `v'\n}\nlocal t : variable label price",
    );
    let s = shown(&d);
    assert!(
        s[0].contains("not: programs (program define hello"),
        "{s:?}"
    );
    assert!(s[1].contains("not: this runs hello"), "{s:?}");
    assert!(s[2].contains("Mata"), "{s:?}");
    assert!(
        s[3].contains("needs the names of the data's variables"),
        "{s:?}"
    );
    assert!(s[4].contains("inside a block that is not run"), "{s:?}");
    assert!(
        s[5].contains("variable label is not translated yet"),
        "{s:?}"
    );
    let sc = scan(&d);
    assert!(sc.limits.iter().any(|l| l.starts_with("Mata")));
    assert!(sc.limits.iter().any(|l| l.starts_with("Programs")));
}

#[test]
fn extended_functions_and_tempnames() {
    let d = run(
        "local A a b c b a\nlocal n : word count `A'\nlocal u : list uniq A\nlocal w : word 2 of `A'\nlocal r : subinstr local A \"b\" \"z\", all count(local k)\ndisplay \"`n' `u' `w' `r' `k'\"\ntempvar t1 t2\ndisplay \"`t1' `t2'\"\nlocal s : display %9.2f 3.14159",
    );
    let s = shown(&d);
    assert!(
        s.iter()
            .any(|l| l.starts_with("display \"5 a b c b a z c z a 2\"")),
        "{s:?}"
    );
    assert!(
        s.iter()
            .any(|l| l.starts_with("display \"__000000 __000001\"")),
        "{s:?}"
    );
    // : display is R's to print
    assert!(s.last().is_some_and(|l| l.contains("Later")), "{s:?}");
}

#[test]
fn loops_stop_and_continue() {
    let d = run(
        "forvalues i = 1/5 {\n  if `i' == 2 {\n    continue\n  }\n  if `i' == 4 {\n    continue, break\n  }\n  display `i'\n}\ndisplay \"[`i']\"\nexit\ndisplay \"never\"",
    );
    assert_eq!(
        stata_lines(&d, "display"),
        ["display 1", "display 3", "display \"[]\""]
    );
}

#[test]
fn the_scripts_and_the_report() {
    let d = run("use auto\nlocal v price\nsummarize `v'\nbogus thing");
    let r = r_script(&d, "a.do", "/home/me");
    assert!(r.contains("# Stata: summarize price\n"), "{r}");
    assert!(r.contains("#   v is \"price\"\n"), "{r}");
    assert!(
        r.contains("#   not translated: bogus is not translated yet"),
        "{r}"
    );
    assert!(r.contains("# setwd(\"/home/me\")"), "{r}");
    let q = quarto(&d, "a.do");
    assert!(q.starts_with("---\ntitle: \"a.do\""), "{q}");
    assert!(q.contains("```{r}\n# Stata: summarize price\n"), "{q}");
    assert!(q.contains("**Not run:** `bogus thing`"), "{q}");
    let ran = vec![Ran::Ok, Ran::Ok, Ran::Ok, Ran::Skipped];
    let printed = vec![
        String::new(),
        String::new(),
        "  output\n".into(),
        String::new(),
    ];
    let rep = report(&d, "a.do", &ran, &printed);
    assert!(rep.contains("4 lines: 3 ran"), "{rep}");
    assert!(rep.contains(". summarize price\n  output\n"), "{rep}");
    assert!(
        rep.contains("  not run: bogus is not translated yet"),
        "{rep}"
    );
}

#[test]
fn numlists_and_forvalues_step_as_stata_does() {
    let f = |r: &str| forvalues(r).unwrap_or_default();
    let n = |r: &str| numlist(r).unwrap_or_default();
    assert_eq!(
        f("0(.1)1").last().map(String::as_str),
        Some(".9999999999999999")
    );
    assert_eq!(f("0(.1)1").len(), 11);
    assert_eq!(f("10(-3)1"), ["10", "7", "4", "1"]);
    assert_eq!(f("1 3 to 9"), ["1", "3", "5", "7", "9"]);
    assert_eq!(f("1 3 : 9"), ["1", "3", "5", "7", "9"]);
    assert!(f("5/1").is_empty());
    assert_eq!(n("0(.1)1").len(), 11);
    assert_eq!(n("0.7(0.1)1.0"), [".7", ".8", ".9", "1"]);
    assert_eq!(n("1/3 3/1"), ["1", "2", "3", "3", "2", "1"]);
    assert_eq!(
        n("1/3 10(5)20 2.5"),
        ["1", "2", "3", "10", "15", "20", "2.5"]
    );
    assert_eq!(n("1 3 to 9"), ["1", "3", "5", "7", "9"]);
    assert_eq!(n("1 2: 6"), ["1", "2", "3", "4", "5", "6"]);
    assert_eq!(n("1[2]5"), ["1", "3", "5"]);
    assert_eq!(n("1e3 -2 .5"), ["1000", "-2", ".5"]);
    assert_eq!(n("1(.123456789012345)1.5")[1], "1.123456789012");
}

#[test]
fn paths_are_joined_and_mapped() {
    assert_eq!(join("/a/b", "../c/d.dta"), "/a/c/d.dta");
    assert_eq!(join("C:/x", "y"), "C:/x/y");
    assert_eq!(join("C:/x", "D:/y"), "D:/y");
    assert_eq!(
        replace_path(
            "use \"c:\\users\\ann\\study/x\"",
            "C:/Users/Ann/Study",
            "/s"
        ),
        "use \"/s/x\""
    );
}
