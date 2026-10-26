//! The terminal console: a Command window in a terminal, on the engine.
//! One command per line; lines starting with `:` are RCraft's own commands.

use std::io::{self, BufRead, Write};
use std::path::Path;
use std::time::Duration;

use rcraft_engine::{Engine, Item, Outcome, Source};
use rcraft_rhost::{find_installs, Env, SessionConfig, MIN_VERSION};

const HELP: &str = "\
Type R code and press Enter. Commands for RCraft itself:
  :history          list the commands run so far
  :log FILE         also write the session log to FILE
  :script FILE      save the code that ran without error as an R script
  :restart          start a fresh R (objects are lost)
  :help             this text
  :quit             leave (also :q)";

pub fn run() -> i32 {
    let Some(install) = find_installs(&Env::from_process())
        .into_iter()
        .find(|i| i.is_supported())
    else {
        eprintln!("No R found. RCraft needs R {MIN_VERSION} or later.");
        return 1;
    };
    let mut engine = match Engine::start(SessionConfig::new(install)) {
        Ok(e) => e,
        Err(e) => {
            eprintln!("{e}");
            return 1;
        }
    };
    println!(
        "{} {} on {}\n:help for commands, :quit to leave.\n",
        rcraft_core::PRODUCT,
        rcraft_core::version(),
        engine.r_version()
    );

    let stdin = io::stdin();
    let mut line = String::new();
    loop {
        print!("> ");
        let _ = io::stdout().flush();
        line.clear();
        match stdin.lock().read_line(&mut line) {
            Ok(0) | Err(_) => break,
            Ok(_) => {}
        }
        let input = line.trim_end_matches(['\r', '\n']);
        if input.trim().is_empty() {
            continue;
        }
        if let Some(meta) = input.trim().strip_prefix(':') {
            if !meta_command(&mut engine, meta) {
                break;
            }
            continue;
        }
        run_and_print(&mut engine, input);
    }
    0
}

/// Returns false to leave the console.
fn meta_command(engine: &mut Engine, text: &str) -> bool {
    let (word, rest) = text.split_once(' ').unwrap_or((text, ""));
    let rest = rest.trim();
    match word {
        "q" | "quit" => return false,
        "help" => println!("{HELP}"),
        "history" => {
            for (i, entry) in engine.history().entries().iter().enumerate() {
                let mark = match entry.outcome {
                    Some(Outcome::Ok) => " ",
                    Some(_) => "!",
                    None => "?",
                };
                let first = entry.code.lines().next().unwrap_or("");
                println!("{:>4}{mark} {first}", i + 1);
            }
        }
        "log" if !rest.is_empty() => match engine.log_to(Path::new(rest)) {
            Ok(()) => println!("(logging to {rest})"),
            Err(e) => eprintln!("could not open {rest}: {e}"),
        },
        "script" if !rest.is_empty() => match std::fs::write(rest, engine.script_text()) {
            Ok(()) => println!("(script saved to {rest})"),
            Err(e) => eprintln!("could not write {rest}: {e}"),
        },
        "restart" => match engine.restart() {
            Ok(()) => println!("(R restarted)"),
            Err(e) => eprintln!("{e}"),
        },
        _ => eprintln!("unknown command :{word}; try :help"),
    }
    true
}

/// Runs one command and prints what it produces as it arrives.
fn run_and_print(engine: &mut Engine, code: &str) {
    let id = match engine.submit(Source::Typed, code) {
        Ok(id) => id,
        Err(e) => {
            eprintln!("{e}");
            return;
        }
    };
    let mut shown = Shown::default();
    while engine.is_busy() {
        engine.poll(Duration::from_millis(100));
        shown.print_new(engine, id);
    }
    // notes that arrive just after the command ends (R stopped, and so on)
    engine.poll(Duration::from_millis(50));
    shown.print_new(engine, id);
}

/// How much of a command's result block has been printed.
#[derive(Default)]
struct Shown {
    items: usize,
    /// Bytes already printed of the item at index `items` (a growing text).
    partial: usize,
}

impl Shown {
    fn print_new(&mut self, engine: &Engine, id: u64) {
        let Some(block) = engine.results().blocks().iter().find(|b| b.command == id) else {
            return;
        };
        while let Some(item) = block.items.get(self.items) {
            match item {
                Item::Text { text, .. } => {
                    print!("{}", text.get(self.partial..).unwrap_or(""));
                    self.partial = text.len();
                    // a text item may still grow; move on only when another
                    // item follows it
                    if block.items.get(self.items + 1).is_none() {
                        break;
                    }
                }
                Item::Warning(message) => println!("Warning: {message}"),
                Item::Error { message, call } if call.is_empty() => println!("Error: {message}"),
                Item::Error { message, call } => println!("Error in {call} : {message}"),
                Item::Notice(note) => println!("({note})"),
            }
            self.items += 1;
            self.partial = 0;
        }
        let _ = io::stdout().flush();
    }
}
