//! A package's page, vignettes and NEWS against a real R (skipped with a note
//! when none is found). `grid` ships with R and has thirteen vignettes with
//! PDFs, code and source; `MASS` ships a NEWS file.

use std::time::{Duration, Instant};

use rcraft_engine::Engine;
use rcraft_rcodegen::help_docs::{
    extension, file_text_code, file_url, kind_of, news_code, package_code, parse_package,
    parse_text, parse_vignettes, vignettes_code, FileKind, PackageAnswer,
};
use rcraft_rhost::{find_installs, Env, SessionConfig};

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

fn ask(e: &mut Engine, code: &str) -> String {
    let id = e.query(code);
    let limit = Instant::now() + Duration::from_secs(300);
    loop {
        e.poll(Duration::from_millis(50));
        if !e.is_working() {
            break;
        }
        assert!(Instant::now() < limit, "the engine did not settle");
    }
    match e.query_result(id) {
        Some(r) => {
            assert!(r.ok, "R said: {}", r.text);
            r.text.clone()
        }
        None => String::new(),
    }
}

#[test]
fn a_package_page_has_its_description_topics_and_vignettes() {
    let Some(mut e) = engine() else { return };
    let answer = ask(&mut e, &package_code("grid"));
    let found = parse_package(&answer);
    assert!(
        matches!(found, Some(PackageAnswer::Found(_))),
        "{answer:.400}"
    );
    let Some(PackageAnswer::Found(p)) = found else {
        return;
    };
    assert_eq!(p.package, "grid");
    assert!(p.description.contains("Package:"), "{}", p.description);
    assert!(
        p.topics.iter().any(|t| t.name == "viewport"),
        "{:?}",
        p.topics.iter().take(5).collect::<Vec<_>>()
    );
    assert!(p.topics.iter().all(|t| !t.name.is_empty()));
    assert!(p.vignettes.len() >= 10, "{} vignettes", p.vignettes.len());
    let intro = p.vignettes.iter().find(|v| v.item == "grid");
    assert!(intro.is_some());
    let Some(intro) = intro else { return };
    assert_eq!(intro.title, "Introduction to grid");
    assert!(intro
        .files
        .iter()
        .any(|f| kind_of(f) == Some(FileKind::Document) && extension(f) == "pdf"));
    assert!(intro
        .files
        .iter()
        .any(|f| kind_of(f) == Some(FileKind::Code)));
    assert!(intro
        .files
        .iter()
        .any(|f| kind_of(f) == Some(FileKind::Source)));
    // the system opens a file by an address
    assert!(intro
        .files
        .first()
        .is_some_and(|f| file_url(f).starts_with("file://")));
}

#[test]
fn a_package_that_is_not_installed_is_said_so_and_odd_names_are_harmless() {
    let Some(mut e) = engine() else { return };
    let answer = ask(&mut e, &package_code("zzznotinstalled"));
    assert_eq!(parse_package(&answer), Some(PackageAnswer::NotInstalled));
    for odd in ["a\"b", "x\\y", "", "é", "a b"] {
        let _ = parse_package(&ask(&mut e, &package_code(odd)));
        let _ = parse_vignettes(&ask(&mut e, &vignettes_code(Some(odd))));
        let _ = parse_text(&ask(&mut e, &news_code(odd)));
    }
    assert_eq!(ask(&mut e, "cat(1 + 1)").trim(), "2");
}

#[test]
fn vignettes_of_one_package_and_of_all() {
    let Some(mut e) = engine() else { return };
    let one = parse_vignettes(&ask(&mut e, &vignettes_code(Some("grid")))).unwrap_or_default();
    assert!(one.len() >= 10);
    assert!(one.iter().all(|v| v.package == "grid"));
    let all = parse_vignettes(&ask(&mut e, &vignettes_code(None))).unwrap_or_default();
    assert!(all.len() >= one.len());
    assert!(all.iter().any(|v| v.package == "grid"));
    // a package that is not installed has none: an empty list, not an error
    let none = parse_vignettes(&ask(&mut e, &vignettes_code(Some("zzznotinstalled"))));
    assert_eq!(none, Some(Vec::new()));
}

#[test]
fn news_and_vignette_text_come_from_inside_the_packages_only() {
    let Some(mut e) = engine() else { return };
    // a NEWS file, when this machine has MASS (it ships with R)
    let news = parse_text(&ask(&mut e, &news_code("MASS")));
    if let Ok(Some(doc)) = &news {
        assert_eq!(doc.title, "News for MASS");
        assert!(doc.text.lines().count() > 3);
    } else {
        assert_eq!(news, Ok(None), "MASS has no NEWS here");
    }
    // a package that is not installed has no NEWS
    assert_eq!(parse_text(&ask(&mut e, &news_code("zzznone"))), Ok(None));

    // a vignette's R code, by its path from the list
    let list = parse_vignettes(&ask(&mut e, &vignettes_code(Some("grid")))).unwrap_or_default();
    let code_file = list
        .iter()
        .flat_map(|v| v.files.iter())
        .find(|f| kind_of(f) == Some(FileKind::Code))
        .cloned()
        .unwrap_or_default();
    assert!(!code_file.is_empty());
    let doc = parse_text(&ask(&mut e, &file_text_code(&code_file)));
    assert!(matches!(doc, Ok(Some(_))), "{doc:?}");

    // a file that is not in an installed package is refused, not read
    let outside = std::env::temp_dir().join("rcraft-help-outside.R");
    assert!(std::fs::write(&outside, "secret <- 1\n").is_ok());
    let refused = parse_text(&ask(
        &mut e,
        &file_text_code(&outside.to_string_lossy().replace('\\', "/")),
    ));
    assert_eq!(refused, Ok(None));
    assert_eq!(
        parse_text(&ask(&mut e, &file_text_code("C:/no/such/file.R"))),
        Ok(None)
    );
    let _ = std::fs::remove_file(&outside);
}
