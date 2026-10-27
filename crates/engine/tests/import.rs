//! "Use my file" for delimited text, end to end against a real R (skipped with
//! a note when none is found): write a file, let the sniffer and the code
//! generator build the import, run it, bring the table over, compare.

use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

use rcraft_engine::{Engine, Outcome, Source};
use rcraft_rcodegen::CsvImport;
use rcraft_rdata::sniff::{sniff_file, Encoding};
use rcraft_rdata::{Data, Table};
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
    let limit = Instant::now() + Duration::from_secs(60);
    loop {
        e.poll(Duration::from_millis(50));
        if !e.is_working() {
            break;
        }
        assert!(Instant::now() < limit, "the engine did not settle");
    }
}

fn file(name: &str, bytes: &[u8]) -> PathBuf {
    static N: AtomicU64 = AtomicU64::new(0);
    let dir = std::env::temp_dir().join(format!(
        "rcraft-import-{}-{}",
        std::process::id(),
        N.fetch_add(1, Ordering::SeqCst)
    ));
    let _ = std::fs::create_dir_all(&dir);
    let path = dir.join(name);
    let _ = std::fs::write(&path, bytes);
    path
}

/// Imports `bytes` as a file called `name`, after `tweak` changes the
/// dialog's choices. Returns the table and the outcome of the command.
fn import(
    e: &mut Engine,
    name: &str,
    bytes: &[u8],
    tweak: impl FnOnce(&mut CsvImport),
) -> (Option<Table>, Outcome) {
    let path = file(name, bytes);
    let sniffed = sniff_file(&path);
    assert!(sniffed.is_ok(), "{sniffed:?}");
    let Ok(sniffed) = sniffed else {
        return (None, Outcome::Error);
    };
    let mut spec = CsvImport::from_sniffed(&path.to_string_lossy(), &sniffed);
    spec.name = "imported".into();
    tweak(&mut spec);
    assert!(spec.problem().is_none(), "{:?}", spec.problem());
    let id = e.submit(Source::Menu("File > Use my file".into()), &spec.code());
    assert!(id.is_ok(), "{id:?}");
    settle(e);
    let outcome = e
        .history()
        .entries()
        .last()
        .and_then(|h| h.outcome)
        .unwrap_or(Outcome::Error);
    e.watch("imported");
    settle(e);
    let table = e.table("imported").cloned();
    e.unwatch("imported");
    let _ = std::fs::remove_dir_all(path.parent().unwrap_or(&path));
    (table, outcome)
}

fn col<'a>(t: &'a Table, name: &str) -> Option<&'a Data> {
    t.columns.iter().find(|c| c.name == name).map(|c| &c.data)
}

fn text(items: &[Option<&str>]) -> Data {
    Data::Text(items.iter().map(|s| s.map(str::to_string)).collect())
}

#[test]
fn comma_file_with_quotes_missing_values_and_types() {
    let Some(mut e) = engine() else { return };
    let csv =
        "id,name,score\n1,\"Ann, Jr.\",3.5\n2,\"two\nlines\",\n3,Cy,NA\n4,\"say \"\"hi\"\"\",7\n";
    let (t, outcome) = import(&mut e, "p.csv", csv.as_bytes(), |_| {});
    assert_eq!(outcome, Outcome::Ok);
    let Some(t) = t else { panic!("no table") };
    assert_eq!((t.nrow, t.columns.len()), (4, 3));
    assert_eq!(col(&t, "id"), Some(&Data::Int(vec![1, 2, 3, 4])));
    assert_eq!(
        col(&t, "name"),
        Some(&text(&[
            Some("Ann, Jr."),
            Some("two\nlines"),
            Some("Cy"),
            Some("say \"hi\"")
        ]))
    );
    let Some(Data::Double(score)) = col(&t, "score") else {
        panic!("score is not a number: {:?}", col(&t, "score"))
    };
    assert_eq!(score.first(), Some(&3.5));
    assert!(score.get(1).is_some_and(|x| x.is_nan()));
    assert!(score.get(2).is_some_and(|x| x.is_nan()));
    assert_eq!(score.get(3), Some(&7.0));
}

#[test]
fn semicolons_decimal_commas_and_latin1() {
    let Some(mut e) = engine() else { return };
    let bytes = b"id;score;city\n1;3,5;K\xf6ln\n2;4,25;Bonn\n";
    let (t, outcome) = import(&mut e, "de.csv", bytes, |s| {
        assert_eq!(s.delimiter, Some(';'));
        assert_eq!(s.decimal, ',');
        assert_eq!(s.encoding, Encoding::Latin1);
    });
    assert_eq!(outcome, Outcome::Ok);
    let Some(t) = t else { panic!("no table") };
    assert_eq!(col(&t, "score"), Some(&Data::Double(vec![3.5, 4.25])));
    assert_eq!(
        col(&t, "city"),
        Some(&text(&[Some("K\u{f6}ln"), Some("Bonn")]))
    );
}

#[test]
fn no_header_gives_v1_v2_and_skip_drops_preamble_lines() {
    let Some(mut e) = engine() else { return };
    let (t, outcome) = import(&mut e, "n.tsv", b"1\tx\n2\ty\n", |s| {
        assert_eq!(s.delimiter, Some('\t'));
        assert!(!s.header);
    });
    assert_eq!(outcome, Outcome::Ok);
    let Some(t) = t else { panic!("no table") };
    let names: Vec<&str> = t.columns.iter().map(|c| c.name.as_str()).collect();
    assert_eq!(names, ["V1", "V2"]);

    let (t, outcome) = import(
        &mut e,
        "s.csv",
        b"Exported by tool\nDate: today\na,b\n1,2\n3,4\n",
        |s| s.skip = 2,
    );
    assert_eq!(outcome, Outcome::Ok);
    let Some(t) = t else { panic!("no table") };
    assert_eq!(col(&t, "a"), Some(&Data::Int(vec![1, 3])));
}

#[test]
fn byte_order_marks_and_utf16_read_cleanly() {
    let Some(mut e) = engine() else { return };
    let mut bom = vec![0xEF, 0xBB, 0xBF];
    bom.extend_from_slice("id,name\n1,Zo\u{eb}\n".as_bytes());
    let (t, outcome) = import(&mut e, "bom.csv", &bom, |s| {
        assert_eq!(s.encoding, Encoding::Utf8Bom);
    });
    assert_eq!(outcome, Outcome::Ok);
    let Some(t) = t else { panic!("no table") };
    assert_eq!(t.columns.first().map(|c| c.name.as_str()), Some("id"));
    assert_eq!(col(&t, "name"), Some(&text(&[Some("Zo\u{eb}")])));

    let mut u16 = vec![0xFF, 0xFE];
    for unit in "id,name\n1,Zo\u{eb}\n2,\u{4e2d}\n".encode_utf16() {
        u16.extend_from_slice(&unit.to_le_bytes());
    }
    let (t, outcome) = import(&mut e, "u16.csv", &u16, |s| {
        assert_eq!(s.encoding, Encoding::Utf16Le);
    });
    assert_eq!(outcome, Outcome::Ok);
    let Some(t) = t else { panic!("no table") };
    assert_eq!(
        col(&t, "name"),
        Some(&text(&[Some("Zo\u{eb}"), Some("\u{4e2d}")]))
    );
}

#[test]
fn custom_missing_text_hash_signs_and_white_space() {
    let Some(mut e) = engine() else { return };
    let csv = "a,b,c\n1, x ,#1\n-9, y ,#2\n3,,#3\n";
    let (t, outcome) = import(&mut e, "m.csv", csv.as_bytes(), |s| {
        s.na_strings = vec![String::new(), "NA".into(), "-9".into()];
        s.strip_white = true;
    });
    assert_eq!(outcome, Outcome::Ok);
    let Some(t) = t else { panic!("no table") };
    let Some(Data::Int(a)) = col(&t, "a") else {
        panic!("a: {:?}", col(&t, "a"))
    };
    assert_eq!((a.first(), a.get(2)), (Some(&1), Some(&3)));
    assert_eq!(a.get(1), Some(&rcraft_rdata::INT_NA));
    assert_eq!(col(&t, "b"), Some(&text(&[Some("x"), Some("y"), None])));
    // `#` starts a comment in R's defaults; the generated code turns that off
    assert_eq!(
        col(&t, "c"),
        Some(&text(&[Some("#1"), Some("#2"), Some("#3")]))
    );
}

#[test]
fn a_missing_file_is_an_r_error_and_the_session_carries_on() {
    let Some(mut e) = engine() else { return };
    let spec = CsvImport {
        path: "C:/no/such/folder/nothing.csv".into(),
        name: "imported".into(),
        header: true,
        delimiter: Some(','),
        decimal: '.',
        quote: rcraft_rcodegen::Quote::Double,
        skip: 0,
        na_strings: vec![],
        encoding: Encoding::Utf8,
        strip_white: false,
        strings_as_factors: false,
    };
    let r = e.submit(Source::Menu("File > Use my file".into()), &spec.code());
    assert!(r.is_ok());
    settle(&mut e);
    assert_eq!(
        e.history().entries().last().and_then(|h| h.outcome),
        Some(Outcome::Error)
    );
    assert!(!e.workspace().iter().any(|w| w.name == "imported"));
    let ok = e.submit(Source::Typed, "1 + 1");
    assert!(ok.is_ok());
    settle(&mut e);
}
