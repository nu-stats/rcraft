//! Help pages and help search against a real R (skipped with a note when none
//! is found): the code the Help window sends is run as a hidden query and the
//! answer is read back, so what is tested is what the window shows.

use std::time::{Duration, Instant};

use rcraft_engine::{Engine, Source};
use rcraft_rcodegen::help::{
    page_code, parse_page, parse_search, search_code, Hit, Page, PageAnswer,
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

fn settle(e: &mut Engine) {
    let limit = Instant::now() + Duration::from_secs(300);
    loop {
        e.poll(Duration::from_millis(50));
        if !e.is_working() {
            break;
        }
        assert!(Instant::now() < limit, "the engine did not settle");
    }
}

fn ask(e: &mut Engine, code: &str) -> String {
    let id = e.query(code);
    settle(e);
    match e.query_result(id) {
        Some(r) => {
            assert!(r.ok, "R said: {}", r.text);
            r.text.clone()
        }
        None => String::new(),
    }
}

fn page(e: &mut Engine, topic: &str, package: Option<&str>) -> Option<PageAnswer> {
    let text = ask(e, &page_code(topic, package));
    parse_page(&text)
}

fn found(e: &mut Engine, topic: &str, package: Option<&str>) -> Page {
    let answer = page(e, topic, package);
    assert!(
        matches!(answer, Some(PageAnswer::Found(_))),
        "{topic}: expected a page, got {answer:?}"
    );
    match answer {
        Some(PageAnswer::Found(p)) => *p,
        _ => Page::default(),
    }
}

#[test]
fn a_help_page_comes_with_text_examples_and_links() {
    let Some(mut e) = engine() else { return };
    let p = found(&mut e, "mean", None);
    assert_eq!(p.package, "base");
    assert_eq!(p.topic, "mean");
    assert_eq!(p.title, "Arithmetic Mean");
    assert!(p.text.contains("Description:"), "{}", p.text);
    assert!(p.text.contains("Usage:"));
    assert!(p.text.contains("Arguments:"));
    assert!(p.text.contains("mean(x, ...)"));
    assert!(p.examples.contains("mean(x)"), "{}", p.examples);
    assert!(!p.examples.contains("### Name"), "the header is dropped");
    assert!(
        p.links.iter().any(|l| l == "weighted.mean"),
        "{:?}",
        p.links
    );

    // the examples run as they are
    let out = ask(
        &mut e,
        &format!("local({{ {} ; cat(\"ran\\n\") }})", p.examples),
    );
    assert!(out.contains("ran"), "{out}");
}

#[test]
fn a_page_can_be_asked_for_in_one_package() {
    let Some(mut e) = engine() else { return };
    let p = found(&mut e, "rnorm", Some("stats"));
    assert_eq!(p.package, "stats");
    assert!(p.title.contains("Normal"), "{}", p.title);
    // a name R spells with symbols
    let p = found(&mut e, "[", None);
    assert_eq!(p.package, "base");
    assert!(!p.text.is_empty());
    // a topic asked for in the wrong package is not found
    assert!(matches!(
        page(&mut e, "rnorm", Some("tools")),
        Some(PageAnswer::NotFound { .. })
    ));
}

#[test]
fn a_missing_topic_names_near_misses_and_odd_input_is_harmless() {
    let Some(mut e) = engine() else { return };
    let answer = page(&mut e, "mea", None);
    assert!(
        matches!(answer, Some(PageAnswer::NotFound { .. })),
        "{answer:?}"
    );
    if let Some(PageAnswer::NotFound { suggestions }) = answer {
        assert!(suggestions.iter().any(|s| s == "mean"), "{suggestions:?}");
    }
    for topic in ["zzzzqqq", "a\"b", "a\\b", "'", "é", "x y", "%in%", "$", ""] {
        let _ = page(&mut e, topic, None);
    }
    let _ = page(&mut e, "mean", Some("no\"pkg"));
    // the session is still there
    assert_eq!(ask(&mut e, "cat(1 + 1)").trim(), "2");
}

#[test]
fn search_finds_and_ranks_pages() {
    let Some(mut e) = engine() else { return };
    let words = "linear models";
    let text = ask(&mut e, &search_code(words));
    let hits: Vec<Hit> = parse_search(words, &text).unwrap_or_default();
    assert!(!hits.is_empty(), "{text}");
    assert!(
        hits.iter().any(|h| h.package == "stats" && h.topic == "lm"),
        "{hits:?}"
    );
    assert!(hits.len() <= 100);

    // every word must match, in the title, the name or the keywords
    let words = "linear regression";
    let text = ask(&mut e, &search_code(words));
    let hits = parse_search(words, &text).unwrap_or_default();
    assert!(
        hits.iter().any(|h| h.package == "stats" && h.topic == "lm"),
        "{hits:?}"
    );
    // words that are not valid patterns are no reason to fail
    for odd in ["c++", "(", "[a", "  ", "*"] {
        let text = ask(&mut e, &search_code(odd));
        assert!(parse_search(odd, &text).is_some(), "{odd}: {text}");
    }

    // a topic named exactly comes first
    let text = ask(&mut e, &search_code("lm"));
    let hits = parse_search("lm", &text).unwrap_or_default();
    assert_eq!(hits.first().map(|h| h.topic.as_str()), Some("lm"));

    // nothing found is an empty list, not an error
    let text = ask(&mut e, &search_code("zzzqqqxxxnothingherezzz"));
    assert_eq!(
        parse_search("zzzqqqxxxnothingherezzz", &text),
        Some(Vec::new())
    );

    // typed lines go through the same code
    e.submit(Source::Typed, "1").ok();
    settle(&mut e);
}
