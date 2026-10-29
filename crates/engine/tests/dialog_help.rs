//! The function links on every dialog's help page, checked against a real R
//! (skipped with a note when none is found): each function the page names
//! must have a help page in R, or the link would lead to "no help page".
//! Functions of a package that is not installed here are skipped, and said so.

use std::time::{Duration, Instant};

use rcraft_catalog::all_specs;
use rcraft_engine::Engine;
use rcraft_rcodegen::dialog_help::{for_spec, packages_window, use_my_file};
use rcraft_rcodegen::import::r_string;
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
    let limit = Instant::now() + Duration::from_secs(120);
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
fn every_function_a_dialog_page_links_has_a_help_page() {
    let Some(mut e) = engine() else { return };
    let mut names: Vec<String> = Vec::new();
    for spec in all_specs() {
        for f in for_spec(spec).functions {
            if !names.contains(&f) {
                names.push(f);
            }
        }
    }
    for page in [use_my_file(), packages_window()] {
        for f in page.functions {
            if !names.contains(&f) {
                names.push(f);
            }
        }
    }
    assert!(names.len() > 40, "only {} functions", names.len());
    let list = names
        .iter()
        .map(|n| r_string(n))
        .collect::<Vec<_>>()
        .join(", ");
    // `pkg::name` is looked up in that package; a bare name anywhere
    let code = format!(
        "local({{\n\
         for (n in c({list})) {{\n\
           parts <- strsplit(n, \"::\", fixed = TRUE)[[1L]]\n\
           pkg <- if (length(parts) == 2L) parts[1L] else NULL\n\
           name <- parts[length(parts)]\n\
           if (!is.null(pkg) && !requireNamespace(pkg, quietly = TRUE)) {{ cat(\"SKIP\\t\", n, \"\\n\", sep = \"\"); next }}\n\
           h <- tryCatch(do.call(utils::help, if (is.null(pkg)) list(name) else list(name, package = pkg)), error = function(e) character())\n\
           if (length(h) == 0L) cat(\"MISSING\\t\", n, \"\\n\", sep = \"\")\n\
         }}\n\
         }})"
    );
    let answer = ask(&mut e, &code);
    let missing: Vec<&str> = answer
        .lines()
        .filter_map(|l| l.strip_prefix("MISSING\t"))
        .collect();
    let skipped: Vec<&str> = answer
        .lines()
        .filter_map(|l| l.strip_prefix("SKIP\t"))
        .collect();
    if !skipped.is_empty() {
        eprintln!("not checked, package not installed here: {skipped:?}");
    }
    assert!(
        missing.is_empty(),
        "functions with no help page in R: {missing:?}"
    );
}
