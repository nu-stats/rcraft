//! RCraft command-line app. Phase 0: prints the version.

fn main() {
    let arg = std::env::args().nth(1);
    match arg.as_deref() {
        Some("--version") | Some("-V") => {
            println!("{} {}", rcraft_core::PRODUCT, rcraft_core::version());
        }
        _ => {
            eprintln!("usage: rcraft --version");
            std::process::exit(2);
        }
    }
}
