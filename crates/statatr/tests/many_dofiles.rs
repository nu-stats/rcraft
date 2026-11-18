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

/// The share of lines translated per tier (P2C.8) over the same folder, run
/// dry: each file as the do-file window would start it, without R.
#[test]
fn the_share_translated_per_tier() {
    use rcraft_statatr::coverage::{Coverage, Tier};
    use rcraft_statatr::dofile::{dry_run, NoFiles, Options};
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
    let mut c = Coverage::default();
    // every line not translated, with its reason, when asked for
    let mut refused = String::new();
    // each source line once (a line in a loop or a program runs many times)
    let mut distinct: std::collections::BTreeMap<(String, usize, usize), u8> =
        std::collections::BTreeMap::new();
    // each source line refused in some run of it: its text and first reason
    let mut source_refused: std::collections::BTreeMap<(String, usize, usize), String> =
        std::collections::BTreeMap::new();
    // each source line's command word, for the per-command table
    let mut source_word: std::collections::BTreeMap<(String, usize, usize), String> =
        std::collections::BTreeMap::new();
    for f in &files {
        let text = std::fs::read_to_string(f).unwrap_or_default();
        let name = f
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        // as the app runs a do-file: with a folder for its tempfiles
        let opts = Options {
            name: name.clone(),
            temp_folder: "C:/RCraft/temp".into(),
            ..Options::default()
        };
        let d = dry_run(&text, &NoFiles, opts);
        for (line, origin) in d.translation.lines.iter().zip(&d.origins) {
            let st = rcraft_statatr::dofile::status(line, origin);
            // each line of the source once, at the worst it fared in any run of it
            let rank = match st {
                rcraft_statatr::dofile::Status::Not => 2,
                rcraft_statatr::dofile::Status::Later => 1,
                _ => 0,
            };
            let slot = distinct
                .entry((name.clone(), origin.file, origin.span.line))
                .or_insert(0);
            *slot = (*slot).max(rank);
            let word = rcraft_statatr::dofile::command_name(line.stata.trim());
            if !word.is_empty() {
                source_word
                    .entry((name.clone(), origin.file, origin.span.line))
                    .or_insert(word);
            }
            if let (rcraft_statatr::dofile::Status::Not, Err(e)) = (st, &line.result) {
                source_refused
                    .entry((name.clone(), origin.file, origin.span.line))
                    .or_insert_with(|| format!("{}	{}", line.stata.trim(), e.reason));
                refused.push_str(&format!("{name}\t{}\t{}\n", line.stata.trim(), e.reason));
            }
        }
        c.add(&d);
    }
    if let Ok(out) = std::env::var("RCRAFT_COVERAGE_LINES") {
        std::fs::write(&out, &refused).unwrap_or_else(|e| panic!("{out}: {e}"));
    }
    // one row per source line refused somewhere: file, line, text, reason
    if let Ok(out) = std::env::var("RCRAFT_COVERAGE_SOURCE") {
        let rows: String = source_refused
            .iter()
            .map(|((f, _, l), t)| {
                format!(
                    "{f}	{l}	{t}
"
                )
            })
            .collect();
        std::fs::write(&out, rows).unwrap_or_else(|e| panic!("{out}: {e}"));
    }
    // one row per command and file: source lines (each once), translated, not
    // translated, known only when run. Names and counts only.
    if let Ok(out) = std::env::var("RCRAFT_COVERAGE_COMMANDS") {
        let mut rows: std::collections::BTreeMap<(String, String), (usize, usize, usize, usize)> =
            std::collections::BTreeMap::new();
        for (key, word) in &source_word {
            let rank = distinct.get(key).copied().unwrap_or(0);
            let e = rows.entry((word.clone(), key.0.clone())).or_default();
            e.0 += 1;
            match rank {
                0 => e.1 += 1,
                1 => e.3 += 1,
                _ => e.2 += 1,
            }
        }
        let text: String = rows
            .iter()
            .map(|((w, f), (l, t, n, k))| format!("{w}\t{f}\t{l}\t{t}\t{n}\t{k}\n"))
            .collect();
        std::fs::write(&out, text).unwrap_or_else(|e| panic!("{out}: {e}"));
    }
    let total = c.total();
    let (dt, dl, dn) = distinct.values().fold((0, 0, 0), |(t, l, n), r| match r {
        0 => (t + 1, l, n),
        1 => (t, l + 1, n),
        _ => (t, l, n + 1),
    });
    eprintln!(
        "source lines, each once: {}: {dt} translated, {dn} not, {dl} known only when run ({:.1}% of those a dry run judges)",
        distinct.len(),
        if dt + dn > 0 { 100.0 * dt as f64 / (dt + dn) as f64 } else { 0.0 }
    );
    eprintln!(
        "{} files, {} lines: {} translated, {} not, {} known only when run ({:.1}% of those a dry run judges)",
        c.files, total.lines, total.translated, total.not, total.later, total.share()
    );
    for t in Tier::ALL {
        if let Some(n) = c.tiers.get(&t) {
            eprintln!(
                "| {} | {} | {} | {} | {} | {:.1}% |",
                t.title(),
                n.lines,
                n.translated,
                n.not,
                n.later,
                n.share()
            );
        }
    }
    let mut commands: Vec<_> = c.commands.iter().collect();
    commands.sort_by_key(|(_, n)| std::cmp::Reverse(n.not));
    for (name, n) in commands.iter().take(60) {
        eprintln!(
            "  {name} [{:?}]: {} lines, {} not, {} later",
            rcraft_statatr::coverage::tier(name),
            n.lines,
            n.not,
            n.later
        );
    }
    let mut reasons: Vec<_> = c.reasons.iter().collect();
    reasons.sort_by_key(|(_, n)| std::cmp::Reverse(**n));
    for (why, n) in reasons.iter().take(60) {
        eprintln!("  {n} x {why}");
    }
    assert_eq!(c.files, files.len());
    // the table for docs/stata-coverage.md, when asked for
    if let Ok(out) = std::env::var("RCRAFT_COVERAGE_TABLE") {
        let mut md = String::from(
            "| Tier | Lines | Translated | Not translated | Known when run | Share translated |\n|---|---:|---:|---:|---:|---:|\n",
        );
        for t in Tier::ALL {
            if let Some(n) = c.tiers.get(&t) {
                md.push_str(&format!(
                    "| {} | {} | {} | {} | {} | {:.1}% |\n",
                    t.title(),
                    n.lines,
                    n.translated,
                    n.not,
                    n.later,
                    n.share()
                ));
            }
        }
        md.push_str(&format!(
            "| **All** | {} | {} | {} | {} | {:.1}% |\n",
            total.lines,
            total.translated,
            total.not,
            total.later,
            total.share()
        ));
        // a line refused only because a line before it was is counted apart,
        // so the first causes show
        let follows = |why: &str| {
            why.contains("is not known here")
                || why.contains("is not known:")
                || why.contains("was not set:")
        };
        let after: usize = reasons
            .iter()
            .filter(|(w, _)| follows(w))
            .map(|(_, n)| **n)
            .sum();
        md.push_str(&format!(
            "\n{after} lines are not translated only because a line before them was not (the variable, \
             scalar, macro, data or stored result it would have made). The first causes, most lines first:\n\n\
             | Lines | Reason |\n|---:|---|\n"
        ));
        for (why, n) in reasons.iter().filter(|(w, _)| !follows(w)).take(20) {
            md.push_str(&format!("| {n} | {} |\n", why.replace('|', "\\|")));
        }
        std::fs::write(&out, md).unwrap_or_else(|e| panic!("{out}: {e}"));
    }
}
