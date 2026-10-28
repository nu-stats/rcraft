//! The reader over a folder of real do-files, when `RCRAFT_DOFILES_DIR` names
//! one (skipped otherwise). StatCraft's probe do-files (`observe/dofiles`, our
//! own, on made-up data) are the first such folder: every file must read with
//! no panic, and the problems found are printed for a person to judge.

#![allow(clippy::panic)]

use rcraft_statatr::reader::read;

#[test]
fn every_do_file_in_a_folder_reads() {
    let Ok(dir) = std::env::var("RCRAFT_DOFILES_DIR") else {
        eprintln!("skipped: set RCRAFT_DOFILES_DIR to a folder of do-files");
        return;
    };
    let mut files: Vec<_> = std::fs::read_dir(&dir)
        .unwrap_or_else(|e| panic!("{dir}: {e}"))
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == "do"))
        .collect();
    files.sort();
    let (mut commands, mut problems, mut unknown) =
        (0usize, 0usize, std::collections::BTreeMap::new());
    for f in &files {
        let text = std::fs::read_to_string(f).unwrap_or_default();
        let p = read(&text);
        commands += p.commands().len();
        for c in p.commands() {
            if !c.known {
                *unknown.entry(c.name.clone()).or_insert(0usize) += 1;
            }
        }
        for pr in p.problems() {
            problems += 1;
            eprintln!(
                "{}:{}:{}: {}",
                f.file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_default(),
                pr.span.line,
                pr.span.column,
                pr.message
            );
        }
    }
    let mut top: Vec<(String, usize)> = unknown.into_iter().collect();
    top.sort_by_key(|t| std::cmp::Reverse(t.1));
    eprintln!(
        "{} files, {commands} commands, {problems} problems; names not in the table (most used first): {:?}",
        files.len(),
        top.iter().take(40).collect::<Vec<_>>()
    );
}
