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
    let mut names_asked = 0;
    loop {
        match r.next_event() {
            Event::Line => {}
            // the data's names, for a name in an expression (price)
            Event::Ask(code) if code.code.starts_with("cat(names(") => {
                names_asked += 1;
                r.give(Ok("make\nprice\nmpg\n".into()));
            }
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
    assert_eq!(names_asked, 1);
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
    // a variable made by a line known only when run is known only when run too
    assert!(
        s[4].contains("Later") && s[4].contains("the line that makes it"),
        "{s:?}"
    );
    assert_eq!(scan(&d).later, 3);
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
        "include setup\ndisplay \"`x' $g\"\ndo \"sub/part.do\" arg1\nuse data\nuse missing\ndo nothing\ndo \"sub/part.do\" arg2, nostop\ndo sub/part, other",
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
    // nostop is how the run goes already; other options are refused
    assert!(
        s.iter().any(|l| l.starts_with("display \"arg2 \"")),
        "{s:?}"
    );
    assert!(
        s.iter()
            .any(|l| l.contains("do with the option other is not translated yet")),
        "{s:?}"
    );
    assert_eq!(
        d.files,
        ["", "proj/setup.do", "proj/sub/part.do", "proj/sub/part.do"]
    );
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
        "program define hello\n  display \"hi\"\nend\nhello\nmata:\n  x = 1\nend\nuse auto\nforeach v of varlist x* {\n  summarize `v'\n}\nlocal t : type price",
    );
    let s = shown(&d);
    // a program runs where it is called, its lines translated as any
    assert!(s[0].contains("defines the program hello"), "{s:?}");
    assert!(s[1].contains("runs the program hello"), "{s:?}");
    assert!(s[2].starts_with("display \"hi\" | Command"), "{s:?}");
    assert!(s[3].contains("the program hello returns"), "{s:?}");
    assert!(s[4].contains("Mata"), "{s:?}");
    // a dry run has no names to ask for
    assert!(
        s[6].contains("Later") && s[6].contains("variable names are known only when"),
        "{s:?}"
    );
    assert!(s[7].contains("inside a block that is not run"), "{s:?}");
    assert!(
        s[8].contains(": type is not translated: R does not keep"),
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

/// A run whose R is made up: the data's names, r(levels) and a label.
fn run_with(text: &str, names_asked: &mut usize) -> DoFile {
    let mut r = Run::new(text, &NoFiles, Options::default());
    loop {
        match r.next_event() {
            Event::Line => {}
            Event::Ask(v) => {
                let answer = if v.code.contains("cat(names(") {
                    *names_asked += 1;
                    "y\nx1\nx2\nx3\nprice\ng".to_string()
                } else if v.code.contains("\"levels\"") {
                    "1 2".to_string()
                } else if v.code.contains("\"label\"") {
                    "First x".to_string()
                } else {
                    return r.finish();
                };
                r.give(Ok(answer));
            }
            Event::Done => return r.finish(),
        }
    }
}

#[test]
fn the_datas_names_come_from_r_when_a_varlist_needs_them() {
    let mut asked = 0;
    let d = run_with(
        "use d\nforeach v of varlist x* {\n  summarize `v'\n}\nregress y x1-x3\nlevelsof g, local(gs)\n\
         foreach k of local gs {\n  display `k'\n}\nunab u : x* pri\ndisplay \"`u'\"\n\
         local lab : variable label x1\nlabel define yn 0 \"no\" 1 \"yes\"\nlocal t : label yn 1\n\
         local t2 : label yn 7\ndisplay \"`lab' `t' `t2'\"\nkeep x~\ngenerate z = 1\nsummarize x*",
        &mut asked,
    );
    let s = shown(&d);
    assert_eq!(
        stata_lines(&d, "summarize"),
        [
            "summarize x1",
            "summarize x2",
            "summarize x3",
            "summarize x*"
        ],
        "{s:?}"
    );
    let reg = s
        .iter()
        .find(|l| l.starts_with("regress"))
        .cloned()
        .unwrap_or_default();
    assert!(reg.contains("the variables: y x1 x2 x3"), "{reg}");
    assert!(d
        .translation
        .lines
        .iter()
        .any(|l| l.stata == "regress y x1-x3" && l.result.is_ok()));
    assert_eq!(
        stata_lines(&d, "display"),
        [
            "display 1",
            "display 2",
            "display \"x1 x2 x3 price\"",
            "display \"First x yes 7\""
        ],
        "{s:?}"
    );
    assert!(
        s.iter()
            .any(|l| l.starts_with("keep x~") && l.contains("x~ ambiguous abbreviation")),
        "{s:?}"
    );
    // asked once, kept while the lines leave the names alone, asked again after generate
    assert_eq!(asked, 2, "{s:?}");
}

/// What Stata 19 printed for each (made-up text): every numbered local is
/// cleared, quotes hold words together, each parse() character is a token.
#[test]
fn tokenize_sets_the_positional_locals_as_stata_does() {
    let d = run(concat!(
        "local 5 \"old\"\nlocal 9 \"nine\"\n",
        "tokenize \"a b c\"\ndisplay \"A[`1'][`2'][`3'][`4'][`5'][`9']\"\n",
        "tokenize `\"one \"two three\" four\"'\ndisplay `\"B[`1'][`2'][`3'][`4']\"'\n",
        "tokenize \"a,b,,c\", parse(\",\")\ndisplay \"C[`1'][`2'][`3'][`4'][`5'][`6'][`7']\"\n",
        "tokenize\ndisplay \"D[`1'][`2']\"\n",
        "local s \"x  y\"\ntokenize `s'\ndisplay \"E[`1'][`2'][`3']\"\n",
        "tokenize \"p=q r\", parse(\"= \")\ndisplay \"F[`1'][`2'][`3'][`4'][`5'][`6']\"\n",
    ));
    let got: Vec<String> = stata_lines(&d, "display")
        .iter()
        .map(|s| {
            let s = s.trim_start_matches("display ").trim_start_matches('`');
            s.trim_matches(|c| c == '"' || c == '\'').to_string()
        })
        .collect();
    assert_eq!(
        got,
        [
            "A[a][b][c][][][]",
            "B[one][two three][four][]",
            "C[a][,][b][,][,][c][]",
            "D[][]",
            "E[x][y][]",
            "F[p][=][q][r][][]",
        ],
        "{:?}",
        shown(&d)
    );
    // neighbouring parse() characters: Stata 19 keeps ==, >=, <=, != and ~=
    // together (even when = is not one of them), and nothing else
    let cases = [
        ("a==b", "=", "[a][==][b][]"),
        ("a>=b", "=>", "[a][>=][b][]"),
        ("a=>b", "=>", "[a][=][>][b]"),
        ("a,;b", ",;", "[a][,][;][b]"),
        ("a<>b", "<>", "[a][<][>][b]"),
        ("a!=b", "!=", "[a][!=][b][]"),
        ("a~=b", "~=", "[a][~=][b][]"),
        ("a+=b", "+=", "[a][+][=][b]"),
        ("a>>b", ">", "[a][>][>][b]"),
        ("a<=b", "<", "[a][<=][b][]"),
        ("a<=b", "=", "[a<][=][b][]"),
    ];
    for (text, parse, want) in cases {
        let d = run(&format!(
            "tokenize \"{text}\", parse(\"{parse}\")\ndisplay \"[`1'][`2'][`3'][`4']\"\n"
        ));
        assert_eq!(
            stata_lines(&d, "display"),
            [format!("display \"{want}\"")],
            "{text} with {parse}"
        );
    }
}

/// `if exp command` and `else command` on one line each.
#[test]
fn one_line_if_and_else_run_the_command_the_condition_picks() {
    let d = run(concat!(
        "local k 2\n",
        "if `k' == 2 display \"two\"\nelse display \"not two\"\n",
        "if `k' == 3 display \"three\"\nelse display \"not three\"\n",
        "if \"`k'\" == \"2\" & `k' > 1 local m yes\ndisplay \"m=`m'\"\n",
    ));
    assert_eq!(
        stata_lines(&d, "display"),
        [
            "display \"two\"",
            "display \"not three\"",
            "display \"m=yes\""
        ],
        "{:?}",
        shown(&d)
    );
    let s = shown(&d);
    assert!(
        s.iter()
            .any(|l| l.contains("false: the command after it is skipped")),
        "{s:?}"
    );
    assert!(
        s.iter()
            .any(|l| l.contains("the if before it was true: skipped")),
        "{s:?}"
    );
    // a condition only R knows waits for the run
    let later = run("if _rc display \"failed\"\n");
    assert_eq!(later.origins[0].what, What::Later, "{:?}", shown(&later));
}

/// A macro a line that was not run would have set, or a form of macro
/// RCraft does not have, stops the lines that use it instead of being empty.
#[test]
fn macros_not_known_are_said_so_not_left_empty() {
    let d = run(concat!(
        "local s \"a b\"\ngettoken first s : s, quotes\ndisplay \"[`first'] [`s']\"\n",
        "local one \"x\"\ndisplay \"[`macval(one)']\"\ndisplay \"[$S_level]\"\n",
        "global S_mine 3\ndisplay \"$S_mine\"\n",
    ));
    let s = shown(&d);
    let not = |start: &str| {
        s.iter()
            .find(|l| l.starts_with(start))
            .is_some_and(|l| l.contains("| not: "))
    };
    assert!(not("gettoken"), "{s:?}");
    assert!(
        s.iter()
            .any(|l| l
                .contains("`first' is not known: gettoken first s : s, quotes was not translated")),
        "{s:?}"
    );
    assert!(
        s.iter()
            .any(|l| l.contains("macval(one)") && l.contains("not translated yet")),
        "{s:?}"
    );
    assert!(
        s.iter()
            .any(|l| l.contains("$S_level is one of the globals Stata sets itself")),
        "{s:?}"
    );
    // an S_ global the do-file sets itself is its own
    assert!(s.iter().any(|l| l.starts_with("display \"3\"")), "{s:?}");
}

/// A local whose new value cannot be worked out does not keep its old one.
#[test]
fn a_local_that_cannot_be_set_does_not_keep_its_old_value() {
    let d = run(
        "local s 2\nlocal s `=r(N)' more\ndisplay \"`s'\"\nlocal r 1\nlocal r = `s' + 1\ndisplay \"`r'\"\n",
    );
    let s = shown(&d);
    assert!(!s.iter().any(|l| l.starts_with("display \"1\"")), "{s:?}");
    assert!(!s.iter().any(|l| l.starts_with("display \"2\"")), "{s:?}");
}

#[test]
fn a_draw_that_waits_for_r_in_a_dry_run_leaves_the_draws_after_it() {
    let d = run("set obs 2\nset seed 1\nlocal r = runiform()\ngenerate x = runiform()\n");
    let s = shown(&d);
    assert!(
        s.iter()
            .any(|l| l.starts_with("local r = runiform() | Later")),
        "{s:?}"
    );
    assert!(
        s.iter()
            .any(|l| l.starts_with("generate x = runiform() | Command | ")),
        "{s:?}"
    );
    assert!(
        d.translation
            .lines
            .iter()
            .all(|l| l.stata.starts_with("local") || l.result.is_ok()),
        "{s:?}"
    );
}

#[test]
fn an_add_on_that_is_not_translated_may_set_any_global() {
    let d = run("global mine a\nsysdir set PLUS \"p\"\nset obs 2\nmtable, at(x=1)\ndisplay \"[$mine]\"\ndisplay \"[$mtable_cspec]\"\n");
    let s = shown(&d);
    assert!(
        s.iter()
            .any(|l| l.starts_with("display \"[a]\" | Command | ")),
        "{s:?}"
    );
    assert!(
        s.iter()
            .any(|l| l.contains("$mtable_cspec is not known: mtable, at(x=1), which was not translated, may have set it")),
        "{s:?}"
    );
    // a Stata command that is refused sets no globals
    let d = run("set obs 2\nbsample\ndisplay \"[$nothing]\"\n");
    let s = shown(&d);
    assert!(
        s.iter()
            .any(|l| l.starts_with("display \"[]\" | Command | ")),
        "{s:?}"
    );
}

#[test]
fn natscale_leaves_its_labels_in_r() {
    let d = run("clear\n_natscale 10 20.1 5\ndisplay \"s \" %21x r(min) \" \" %21x r(max) \" \" r(n)\nreturn list\n");
    let s = shown(&d);
    assert!(
        d.translation.lines.iter().all(|l| l.result.is_ok()),
        "{s:?}"
    );
    let code: Vec<String> = d.translation.lines.iter().map(|l| l.script()).collect();
    assert!(
        code.iter().any(|c| c.contains(
            "stata_r <- list(n = 0x1.8000000000000p2, min = 0x1.4000000000000p3, max = 0x1.4000000000000p4, delta = 0x1.0000000000000p1)"
        )),
        "{code:?}"
    );
    // a label at 0 with a step below 1: Stata's r(min) can be 2^-55, so declined
    let d = run("clear\n_natscale 0 1.01 5\n");
    let s = shown(&d);
    assert!(
        s.iter()
            .any(|l| l.contains("_natscale with a label at 0 and a step below 1")),
        "{s:?}"
    );
}

/// What Stata 19 printed for each (made-up text, `plan/stata/gettoken.md`):
/// leading blanks skipped, the rest keeps its delimiter, a parse() character
/// is a token of its own, quotes come off, match() takes a parenthesis.
#[test]
fn gettoken_takes_the_first_token_as_stata_does() {
    let d = run(concat!(
        "local list \"a b c\"\ngettoken f r : list\ndisplay \"A[`f'][`r']\"\n",
        "local list `\"\"a b\" c\"'\ngettoken f r : list\ndisplay `\"B[`f'][`r']\"'\n",
        "local s \"x y|z w\"\ngettoken a b : s, parse(\"|\")\ndisplay \"C[`a'][`b']\"\n",
        "gettoken p b : b, parse(\"|\")\ndisplay \"D[`p'][`b']\"\n",
        "local s \"  x|y\"\ngettoken a b : s, parse(\"|\")\ndisplay \"E[`a'][`b']\"\n",
        "local e \"\"\ngettoken a b : e\ndisplay \"F[`a'][`b']\"\n",
        "local s \"x<=5\"\ngettoken a b : s, parse(\"<= \")\ngettoken c d : b, parse(\"<= \")\ndisplay \"G[`a'][`b'][`c'][`d']\"\n",
        "local s \"a   b  \"\ngettoken a b : s\ndisplay \"H[`a'][`b']\"\n",
        "global G \"one two\"\ngettoken (global) H (local) K : (global) G\ndisplay \"I[$H][`K']\"\n",
        "local varlist \"y x1 x2\"\ngettoken depvar explvar: varlist\ndisplay \"J[`depvar'][`explvar']\"\n",
        "local s \"| x | y\"\ngettoken a b : s, parse(\"|\")\ngettoken a b : b, parse(\"|\")\ndisplay \"K[`a'][`b']\"\n",
        "local s \"a\"\ngettoken a s : s\ngettoken a s : s\ndisplay \"L[`a'][`s']\"\n",
        "local s \"(b c) d\"\ngettoken a b : s, match(m)\ndisplay \"M[`a'][`b'][`m']\"\n",
        "local s \"a(b c)d e\"\ngettoken a b : s, match(m)\ndisplay \"N[`a'][`b'][`m']\"\n",
        "local s \"a b\"\ngettoken a : s\ndisplay \"O[`a'][`s']\"\n",
    ));
    let got: Vec<String> = stata_lines(&d, "display")
        .iter()
        .map(|s| {
            let s = s.trim_start_matches("display ").trim_start_matches('`');
            s.trim_matches(|c| c == '"' || c == '\'').to_string()
        })
        .collect();
    assert_eq!(
        got,
        [
            "A[a][ b c]",
            "B[a b][ c]",
            "C[x y][|z w]",
            "D[|][z w]",
            "E[x][|y]",
            "F[][]",
            "G[x][<=5][<=][5]",
            "H[a][   b  ]",
            "I[one][ two]",
            "J[y][ x1 x2]",
            "K[x ][| y]",
            "L[][]",
            "M[b c][ d][(]",
            "N[a(b][ c)d e][]",
            "O[a][a b]",
        ],
        "{:?}",
        shown(&d)
    );
}

#[test]
fn an_error_ends_the_loops_and_programs_around_it() {
    // Stata 19: `if $qtr == 0 {` with $qtr not set stops the program at its
    // second round (==0 invalid name, r(198)); the do-file goes on after the
    // call, and a loop at the top ends at its first round
    let d = run(
        "program lp\n  forvalues i = 1/3 {\n    display \"round `i'\"\n    if `i' > 1 {\n      if $qtr == 0 {\n        display \"zero\"\n      }\n    }\n  }\n  display \"after loop\"\nend\nlp\ndisplay \"after lp\"\nforvalues i = 1/2 {\n  if $qtr == 1 {\n    display \"one\"\n  }\n  display \"top loop `i'\"\n}\ndisplay \"end\"",
    );
    let s = shown(&d);
    let rounds = stata_lines(&d, "display \"round");
    assert_eq!(rounds.len(), 2, "{s:?}");
    assert!(stata_lines(&d, "display \"after loop").is_empty(), "{s:?}");
    assert_eq!(stata_lines(&d, "display \"after lp").len(), 1, "{s:?}");
    assert!(stata_lines(&d, "display \"top loop").is_empty(), "{s:?}");
    assert_eq!(stata_lines(&d, "display \"end").len(), 1, "{s:?}");
    // the program still returns (its temporary variables go)
    assert!(
        s.iter().any(|l| l.contains("the program lp returns")),
        "{s:?}"
    );
}

#[test]
fn an_error_under_capture_ends_only_what_capture_runs() {
    let d = run(
        "program lp\n  forvalues i = 1/3 {\n    if $qtr == 0 {\n      display \"zero\"\n    }\n    display \"round `i'\"\n  }\nend\ncapture lp\ndisplay _rc\nforvalues i = 1/2 {\n  capture local x = 1 +\n  display \"go on `i'\"\n}",
    );
    let s = shown(&d);
    assert!(stata_lines(&d, "display \"round").is_empty(), "{s:?}");
    assert!(
        s.iter()
            .any(|l| l.starts_with("display _rc") && l.contains("_rc is not known")),
        "{s:?}"
    );
    // capture on the line itself: the loop goes on
    assert_eq!(stata_lines(&d, "display \"go on").len(), 2, "{s:?}");
}

#[test]
fn a_tempfile_left_by_an_earlier_run_goes_before_it_is_used() {
    // the numbering starts again each run, so the name may be an earlier
    // run's file, and Stata's tempfile names one that does not exist yet
    let opts = Options {
        folder: "proj".into(),
        temp_folder: "proj/temp".into(),
        ..Options::default()
    };
    let d = dry_run("tempfile a b", &NoFiles, opts);
    let code = d.translation.lines[0]
        .result
        .as_ref()
        .map(|r| r.code.clone())
        .unwrap_or_default();
    assert_eq!(
        code,
        "dir.create(\"proj/temp\", showWarnings = FALSE, recursive = TRUE)\nunlink(c(\"proj/temp/ST_000000.tmp\", \"proj/temp/ST_000001.tmp\"))"
    );
}

#[test]
fn egen_varlist_functions_are_not_read_as_expressions() {
    // rowmean(v*) takes a varlist; mean(v) takes an expression, where v
    // abbreviates both v1 and v2
    let text = "use auto\negen vmean = rowmean(v*)\negen m = mean(v)";
    let mut r = Run::new(text, &NoFiles, Options::default());
    loop {
        match r.next_event() {
            Event::Line => {}
            Event::Ask(code) if code.code.starts_with("cat(names(") => {
                r.give(Ok("v1\nv2\n".into()));
            }
            Event::Ask(_) => r.give(Err("not asked".into())),
            Event::Done => break,
        }
    }
    let d = r.finish();
    let lines = &d.translation.lines;
    let row = lines.iter().find(|l| l.stata.contains("rowmean")).unwrap();
    assert!(row.result.is_ok(), "{:?}", row.result);
    let mean = lines.iter().find(|l| l.stata.contains("mean(v)")).unwrap();
    let why = mean
        .result
        .as_ref()
        .err()
        .map(|e| e.reason.clone())
        .unwrap_or_default();
    assert!(why.contains("v ambiguous abbreviation"), "{why}");
}

#[test]
fn putexcel_leaves_the_model_and_the_data_alone() {
    // not translated itself, but a model's results after it are still known
    let text = "use auto\nregress price mpg\nputexcel A1 = \"x\"\ndisplay e(r2)\ndisplay _se[mpg]";
    let d = dry_run(text, &NoFiles, Options::default());
    let lines = &d.translation.lines;
    let reg = lines
        .iter()
        .find(|l| l.stata.starts_with("regress"))
        .unwrap();
    assert!(reg.result.is_ok(), "{:?}", reg.result);
    let put = lines
        .iter()
        .find(|l| l.stata.starts_with("putexcel"))
        .unwrap();
    assert!(put.result.is_err());
    for l in lines.iter().filter(|l| l.stata.starts_with("display")) {
        assert!(l.result.is_ok(), "{}: {:?}", l.stata, l.result);
    }
}

#[test]
fn a_varlist_takes_abbreviations_as_stata_does() {
    // drop _m for _merge; an ambiguous one is Stata's error
    let text = "use auto\ndrop _m\nkeep pri m";
    let mut r = Run::new(text, &NoFiles, Options::default());
    loop {
        match r.next_event() {
            Event::Line => {}
            Event::Ask(code) if code.code.starts_with("cat(names(") => {
                r.give(Ok("make\nprice\nmpg\n_merge\n".into()));
            }
            Event::Ask(_) => r.give(Err("not asked".into())),
            Event::Done => break,
        }
    }
    let d = r.finish();
    let lines = &d.translation.lines;
    let drop = lines.iter().find(|l| l.stata.starts_with("drop")).unwrap();
    assert!(drop.result.is_ok(), "{:?}", drop.result);
    let code = format!("{:?}", drop.result);
    assert!(code.contains("_merge"), "{code}");
    let keep = lines.iter().find(|l| l.stata.starts_with("keep")).unwrap();
    let why = keep
        .result
        .as_ref()
        .err()
        .map(|e| e.reason.clone())
        .unwrap_or_default();
    assert!(why.contains("m ambiguous abbreviation"), "{why}");
}

#[test]
fn a_refused_command_that_changes_variables_in_place_loses_them() {
    // winsor2 (SSC) is not translated; it changes price, so what reads price
    // after it is refused rather than run on the old values
    let text = "use auto\nwinsor2 price, replace\nsummarize price\nsave t, replace\nsummarize mpg";
    let mut r = Run::new(text, &NoFiles, Options::default());
    loop {
        match r.next_event() {
            Event::Line => {}
            Event::Ask(code) if code.code.starts_with("cat(names(") => {
                r.give(Ok("make\nprice\nmpg\n".into()));
            }
            Event::Ask(_) => r.give(Err("not asked".into())),
            Event::Done => break,
        }
    }
    let d = r.finish();
    let sums: Vec<_> = d
        .translation
        .lines
        .iter()
        .filter(|l| l.stata.starts_with("summarize"))
        .collect();
    let why = sums[0]
        .result
        .as_ref()
        .err()
        .map(|e| e.reason.clone())
        .unwrap_or_default();
    assert!(why.contains("price is not known here"), "{why}");
    // save's replace overwrites a file and loses nothing
    assert!(sums[1].result.is_ok(), "{:?}", sums[1].result);
}

#[test]
fn robregs_estimator_is_not_a_variable() {
    // robreg m y x: m names the M estimator, though it abbreviates mpg and make
    let text = "use auto\nrobreg m price mp";
    let mut r = Run::new(text, &NoFiles, Options::default());
    loop {
        match r.next_event() {
            Event::Line => {}
            Event::Ask(code) if code.code.starts_with("cat(names(") => {
                r.give(Ok("make\nprice\nmpg\n".into()));
            }
            Event::Ask(_) => r.give(Err("not asked".into())),
            Event::Done => break,
        }
    }
    let d = r.finish();
    let rob = d
        .translation
        .lines
        .iter()
        .find(|l| l.stata.starts_with("robreg"))
        .unwrap();
    let code = format!("{:?}", rob.result);
    assert!(rob.result.is_ok() && code.contains("mpg"), "{code}");
}
