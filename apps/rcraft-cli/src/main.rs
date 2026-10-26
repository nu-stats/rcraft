//! RCraft command-line app: with no arguments, the terminal console;
//! also `--version`, `--find-r` and `--eval`.

mod console;

use std::time::Duration;

use rcraft_rhost::{find_installs, Env, RunStatus, Session, SessionConfig, MIN_VERSION};

fn main() {
    let arg = std::env::args().nth(1);
    match arg.as_deref() {
        None => std::process::exit(console::run()),
        Some("--version") | Some("-V") => {
            println!("{} {}", rcraft_core::PRODUCT, rcraft_core::version());
        }
        Some("--find-r") => find_r(),
        Some("--eval") => match std::env::args().nth(2) {
            Some(code) => eval(&code),
            None => {
                eprintln!("usage: rcraft --eval \"R code\"");
                std::process::exit(2);
            }
        },
        _ => {
            eprintln!("usage: rcraft | rcraft --version | --find-r | --eval \"R code\"");
            std::process::exit(2);
        }
    }
}

/// Runs one piece of R code in a fresh supervised session and prints what
/// came back. A check that the session works; the real interface comes later.
fn eval(code: &str) {
    let Some(install) = find_installs(&Env::from_process())
        .into_iter()
        .find(|i| i.is_supported())
    else {
        eprintln!("No R found. RCraft needs R {MIN_VERSION} or later.");
        std::process::exit(1);
    };
    let mut session = match Session::start(SessionConfig::new(install)) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("{e}");
            std::process::exit(1);
        }
    };
    match session.run_to_end(code, Duration::from_secs(300)) {
        Ok(out) => {
            print!("{}", out.stdout);
            eprint!("{}", out.stderr);
            for warning in &out.warnings {
                eprintln!("Warning: {warning}");
            }
            if let Some(error) = &out.error {
                eprintln!("Error in {}: {}", error.call, error.message);
            }
            if out.truncated {
                eprintln!("(output cut off)");
            }
            if out.status != RunStatus::Ok {
                std::process::exit(1);
            }
        }
        Err(e) => {
            eprintln!("{e}");
            std::process::exit(1);
        }
    }
}

fn find_r() {
    let installs = find_installs(&Env::from_process());
    if installs.is_empty() {
        println!("No R found. RCraft needs R {MIN_VERSION} or later.");
        return;
    }
    for install in installs {
        let note = if install.is_supported() {
            ""
        } else {
            " (too old)"
        };
        println!(
            "R {}{}  {}",
            install.version,
            note,
            install.rscript.display()
        );
    }
}
