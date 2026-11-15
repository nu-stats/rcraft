//! The R and package versions a check ran with (AGENTS.md section 5): a
//! menu entry moves to Verified only on a check that records them, run in the
//! R that `ENVIRONMENT` pins. Shared by the test files that verify entries
//! (`mod pinned;`).

use std::time::{Duration, Instant};

use rcraft_engine::Engine;

/// Prints the R version and those of `packages`, and says when this R is not
/// the pinned one (the run is then not a Verified one).
pub fn record(e: &mut Engine, packages: &[&str]) {
    let names = packages
        .iter()
        .map(|p| format!("\"{p}\""))
        .collect::<Vec<_>>()
        .join(", ");
    let code = format!(
        "cat(paste(R.version$major, R.version$minor, sep = '.'), \
         vapply(c({names}), function(p) paste(p, tryCatch(as.character(utils::packageVersion(p)), \
         error = function(e) 'not installed')), ''), sep = '\\n')"
    );
    let id = e.query(&code);
    let limit = Instant::now() + Duration::from_secs(60);
    while e.is_working() || e.query_result(id).is_none() {
        e.poll(Duration::from_millis(20));
        if Instant::now() > limit {
            eprintln!("note: R did not say its version");
            return;
        }
    }
    let text = e
        .query_result(id)
        .map(|r| r.text.clone())
        .unwrap_or_default();
    let mut lines = text.lines();
    let ran = lines.next().unwrap_or("").trim().to_string();
    let pinned = include_str!("../../../../ENVIRONMENT")
        .lines()
        .find_map(|l| l.strip_prefix("R_VERSION="))
        .unwrap_or("")
        .trim();
    let with: Vec<&str> = lines.map(str::trim).filter(|l| !l.is_empty()).collect();
    eprintln!(
        "ran with R {ran}{}{}; ENVIRONMENT pins R {pinned}",
        if with.is_empty() { "" } else { " and " },
        with.join(", ")
    );
    if ran != pinned {
        eprintln!("note: this R is not the pinned one, so the run is not a Verified one");
    }
}

/// R that runs a dialog's code as it is and keeps the value of its last
/// expression as `name`, so a check can read the numbers it gave.
#[allow(dead_code)]
pub fn keep_value(code: &str, name: &str) -> String {
    format!("{name} <- eval(parse(text = r\"---({code})---\"), envir = globalenv())")
}

/// R that is TRUE when every named check holds, and prints the ones that do
/// not.
#[allow(dead_code)]
pub fn all_of(setup: &str, checks: &[(&str, &str)]) -> String {
    let list = checks
        .iter()
        .map(|(name, check)| format!("{name} = isTRUE({check})"))
        .collect::<Vec<_>>()
        .join(", ");
    format!("local({{ {setup}; ok <- c({list}); if (!all(ok)) print(ok); all(ok) }})")
}
