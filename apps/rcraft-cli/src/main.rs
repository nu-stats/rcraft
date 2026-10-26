//! RCraft command-line app. For now: `--version` and `--find-r`.

use rcraft_rhost::{find_installs, Env, MIN_VERSION};

fn main() {
    let arg = std::env::args().nth(1);
    match arg.as_deref() {
        Some("--version") | Some("-V") => {
            println!("{} {}", rcraft_core::PRODUCT, rcraft_core::version());
        }
        Some("--find-r") => find_r(),
        _ => {
            eprintln!("usage: rcraft --version | --find-r");
            std::process::exit(2);
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
