//! Excel, Stata, SPSS, SAS and R data files end to end against a real R and
//! the `readxl` and `haven` packages (each test is skipped with a note when R
//! or the packages are missing).

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

use rcraft_engine::{Engine, Outcome, Source};
use rcraft_rcodegen::{
    package_check_code, sheets_code, ArrowFormat, ArrowImport, ExcelImport, HavenImport, HavenKind,
    Import, RObjectImport,
};
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
    let limit = Instant::now() + Duration::from_secs(90);
    loop {
        e.poll(Duration::from_millis(50));
        if !e.is_working() {
            break;
        }
        assert!(Instant::now() < limit, "the engine did not settle");
    }
}

/// Asks R something and returns what it printed.
fn ask(e: &mut Engine, code: &str) -> (bool, String) {
    let id = e.query(code);
    settle(e);
    e.query_result(id)
        .map(|r| (r.ok, r.text.clone()))
        .unwrap_or((false, String::new()))
}

/// An engine, only if `package` can be loaded in its R.
fn engine_with(package: &str) -> Option<Engine> {
    let mut e = engine()?;
    let (_, answer) = ask(&mut e, &package_check_code(package));
    if answer.trim() != "yes" {
        eprintln!("skipped: the R package {package} is not installed");
        return None;
    }
    Some(e)
}

fn dir() -> PathBuf {
    static N: AtomicU64 = AtomicU64::new(0);
    let d = std::env::temp_dir().join(format!(
        "rcraft-formats-{}-{}",
        std::process::id(),
        N.fetch_add(1, Ordering::SeqCst)
    ));
    let _ = std::fs::create_dir_all(&d);
    d
}

fn rpath(p: &Path) -> String {
    p.to_string_lossy().replace('\\', "/")
}

/// Runs an import as a menu command; returns the table and the outcome.
fn run_import(e: &mut Engine, import: &Import) -> (Option<Table>, Outcome) {
    let r = e.submit(Source::Menu("File > Use my file".into()), &import.code());
    assert!(r.is_ok(), "{r:?}");
    settle(e);
    let outcome = e
        .history()
        .entries()
        .last()
        .and_then(|h| h.outcome)
        .unwrap_or(Outcome::Error);
    if outcome != Outcome::Ok {
        // say what R said, so a failure on another machine can be read
        let id = r.unwrap_or_default();
        let said: Vec<String> = e
            .results()
            .blocks()
            .iter()
            .filter(|b| b.command == id)
            .flat_map(|b| b.items.iter())
            .map(|i| format!("{i:?}"))
            .collect();
        println!(
            "the import failed.\ncode: {}\nR said: {}",
            import.code(),
            said.join(" | ")
        );
    }
    let Some(name) = import.result_name().map(str::to_string) else {
        return (None, outcome);
    };
    e.watch(&name);
    settle(e);
    let table = e.table(&name).cloned();
    e.unwatch(&name);
    (table, outcome)
}

fn col<'a>(t: &'a Table, name: &str) -> Option<&'a rcraft_rdata::Column> {
    t.columns.iter().find(|c| c.name == name)
}

// ---- a minimal .xlsx, written by hand (a zip of XML, stored uncompressed)

fn crc32(data: &[u8]) -> u32 {
    let mut crc = 0xFFFF_FFFFu32;
    for &b in data {
        crc ^= u32::from(b);
        for _ in 0..8 {
            crc = if crc & 1 != 0 {
                (crc >> 1) ^ 0xEDB8_8320
            } else {
                crc >> 1
            };
        }
    }
    !crc
}

fn zip_stored(files: &[(&str, String)]) -> Vec<u8> {
    let mut out = Vec::new();
    let mut central = Vec::new();
    for (name, text) in files {
        let data = text.as_bytes();
        let offset = out.len() as u32;
        let crc = crc32(data);
        let head = |v: &mut Vec<u8>, central: bool| {
            if central {
                v.extend_from_slice(&0x0201_4b50u32.to_le_bytes());
                v.extend_from_slice(&20u16.to_le_bytes()); // made by
            } else {
                v.extend_from_slice(&0x0403_4b50u32.to_le_bytes());
            }
            v.extend_from_slice(&20u16.to_le_bytes()); // needed
            v.extend_from_slice(&0u16.to_le_bytes()); // flags
            v.extend_from_slice(&0u16.to_le_bytes()); // stored
            v.extend_from_slice(&0u16.to_le_bytes()); // time
            v.extend_from_slice(&0x21u16.to_le_bytes()); // date
            v.extend_from_slice(&crc.to_le_bytes());
            v.extend_from_slice(&(data.len() as u32).to_le_bytes());
            v.extend_from_slice(&(data.len() as u32).to_le_bytes());
            v.extend_from_slice(&(name.len() as u16).to_le_bytes());
            v.extend_from_slice(&0u16.to_le_bytes()); // extra
            if central {
                v.extend_from_slice(&0u16.to_le_bytes()); // comment
                v.extend_from_slice(&0u16.to_le_bytes()); // disk
                v.extend_from_slice(&0u16.to_le_bytes()); // internal attrs
                v.extend_from_slice(&0u32.to_le_bytes()); // external attrs
                v.extend_from_slice(&offset.to_le_bytes());
            }
            v.extend_from_slice(name.as_bytes());
        };
        head(&mut out, false);
        out.extend_from_slice(data);
        head(&mut central, true);
    }
    let cd_offset = out.len() as u32;
    out.extend_from_slice(&central);
    out.extend_from_slice(&0x0605_4b50u32.to_le_bytes());
    out.extend_from_slice(&0u16.to_le_bytes());
    out.extend_from_slice(&0u16.to_le_bytes());
    out.extend_from_slice(&(files.len() as u16).to_le_bytes());
    out.extend_from_slice(&(files.len() as u16).to_le_bytes());
    out.extend_from_slice(&(central.len() as u32).to_le_bytes());
    out.extend_from_slice(&cd_offset.to_le_bytes());
    out.extend_from_slice(&0u16.to_le_bytes());
    out
}

enum Cell<'a> {
    Text(&'a str),
    Num(f64),
    Blank,
}

fn sheet_xml(rows: &[Vec<Cell>]) -> String {
    let mut x = String::from(
        "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\
<worksheet xmlns=\"http://schemas.openxmlformats.org/spreadsheetml/2006/main\"><sheetData>",
    );
    for (r, row) in rows.iter().enumerate() {
        x.push_str(&format!("<row r=\"{}\">", r + 1));
        for (c, cell) in row.iter().enumerate() {
            let at = format!("{}{}", (b'A' + c as u8) as char, r + 1);
            match cell {
                Cell::Text(t) => x.push_str(&format!(
                    "<c r=\"{at}\" t=\"inlineStr\"><is><t>{t}</t></is></c>"
                )),
                Cell::Num(n) => x.push_str(&format!("<c r=\"{at}\"><v>{n}</v></c>")),
                Cell::Blank => {}
            }
        }
        x.push_str("</row>");
    }
    x.push_str("</sheetData></worksheet>");
    x
}

fn workbook() -> Vec<u8> {
    let ns = "http://schemas.openxmlformats.org/";
    let people = sheet_xml(&[
        vec![Cell::Text("id"), Cell::Text("name"), Cell::Text("score")],
        vec![Cell::Num(1.0), Cell::Text("Ann"), Cell::Num(3.5)],
        vec![Cell::Num(2.0), Cell::Text("Bo"), Cell::Blank],
        vec![Cell::Num(3.0), Cell::Text("Cy"), Cell::Num(7.0)],
    ]);
    let other = sheet_xml(&[vec![Cell::Text("note")], vec![Cell::Text("second sheet")]]);
    zip_stored(&[
        (
            "[Content_Types].xml",
            format!(
                "<?xml version=\"1.0\" encoding=\"UTF-8\"?><Types xmlns=\"{ns}package/2006/content-types\">\
<Default Extension=\"rels\" ContentType=\"application/vnd.openxmlformats-package.relationships+xml\"/>\
<Default Extension=\"xml\" ContentType=\"application/xml\"/>\
<Override PartName=\"/xl/workbook.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.spreadsheetml.sheet.main+xml\"/>\
<Override PartName=\"/xl/worksheets/sheet1.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.spreadsheetml.worksheet+xml\"/>\
<Override PartName=\"/xl/worksheets/sheet2.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.spreadsheetml.worksheet+xml\"/></Types>"
            ),
        ),
        (
            "_rels/.rels",
            format!(
                "<?xml version=\"1.0\" encoding=\"UTF-8\"?><Relationships xmlns=\"{ns}package/2006/relationships\">\
<Relationship Id=\"rId1\" Type=\"{ns}officeDocument/2006/relationships/officeDocument\" Target=\"xl/workbook.xml\"/></Relationships>"
            ),
        ),
        (
            "xl/workbook.xml",
            format!(
                "<?xml version=\"1.0\" encoding=\"UTF-8\"?><workbook xmlns=\"{ns}spreadsheetml/2006/main\" \
xmlns:r=\"{ns}officeDocument/2006/relationships\"><sheets>\
<sheet name=\"People\" sheetId=\"1\" r:id=\"rId1\"/><sheet name=\"Other data\" sheetId=\"2\" r:id=\"rId2\"/></sheets></workbook>"
            ),
        ),
        (
            "xl/_rels/workbook.xml.rels",
            format!(
                "<?xml version=\"1.0\" encoding=\"UTF-8\"?><Relationships xmlns=\"{ns}package/2006/relationships\">\
<Relationship Id=\"rId1\" Type=\"{ns}officeDocument/2006/relationships/worksheet\" Target=\"worksheets/sheet1.xml\"/>\
<Relationship Id=\"rId2\" Type=\"{ns}officeDocument/2006/relationships/worksheet\" Target=\"worksheets/sheet2.xml\"/></Relationships>"
            ),
        ),
        ("xl/worksheets/sheet1.xml", people),
        ("xl/worksheets/sheet2.xml", other),
    ])
}

#[test]
fn an_excel_workbook_lists_its_sheets_and_imports_a_sheet_and_a_range() {
    let Some(mut e) = engine_with("readxl") else {
        return;
    };
    let path = dir().join("book.xlsx");
    assert!(std::fs::write(&path, workbook()).is_ok());
    let p = rpath(&path);

    let (ok, sheets) = ask(&mut e, &sheets_code(&p));
    assert!(ok, "{sheets}");
    assert_eq!(sheets.lines().collect::<Vec<_>>(), ["People", "Other data"]);

    // the first sheet by default
    let mut x = ExcelImport::new(&p);
    x.name = "imported".into();
    let (t, outcome) = run_import(&mut e, &Import::Excel(x.clone()));
    assert_eq!(outcome, Outcome::Ok);
    let Some(t) = t else { panic!("no table") };
    assert_eq!((t.nrow, t.columns.len()), (3, 3));
    assert_eq!(
        col(&t, "id").map(|c| &c.data),
        Some(&Data::Double(vec![1.0, 2.0, 3.0]))
    );
    let Some(Data::Double(score)) = col(&t, "score").map(|c| &c.data) else {
        panic!("score")
    };
    assert_eq!(score.first(), Some(&3.5));
    assert!(
        score.get(1).is_some_and(|v| v.is_nan()),
        "a blank cell is missing"
    );

    // another sheet, by name
    x.sheet = "Other data".into();
    let (t, outcome) = run_import(&mut e, &Import::Excel(x.clone()));
    assert_eq!(outcome, Outcome::Ok);
    let Some(t) = t else { panic!("no table") };
    assert_eq!(
        col(&t, "note").map(|c| &c.data),
        Some(&Data::Text(vec![Some("second sheet".into())]))
    );

    // a range, with the first row of the range as names
    x.sheet = "People".into();
    x.range = "B1:C3".into();
    let (t, outcome) = run_import(&mut e, &Import::Excel(x.clone()));
    assert_eq!(outcome, Outcome::Ok);
    let Some(t) = t else { panic!("no table") };
    let names: Vec<&str> = t.columns.iter().map(|c| c.name.as_str()).collect();
    assert_eq!((names, t.nrow), (vec!["name", "score"], 2));

    // no header: names are made up
    x.range = "A2:C4".into();
    x.header = false;
    let (t, _) = run_import(&mut e, &Import::Excel(x.clone()));
    let Some(t) = t else { panic!("no table") };
    assert_eq!(t.nrow, 3);
    assert_eq!(t.columns.first().map(|c| c.name.as_str()), Some("...1"));

    // the preview prints rows without creating anything
    let (ok, preview) = ask(&mut e, &Import::Excel(ExcelImport::new(&p)).preview_code(5));
    assert!(ok, "{preview}");
    assert!(
        preview.contains("3 columns; first 3 rows") && preview.contains("Ann"),
        "{preview}"
    );

    // an unreadable workbook is an error with a reason, not a crash
    let bad = dir().join("bad.xlsx");
    assert!(std::fs::write(&bad, b"not really a workbook").is_ok());
    let (ok, why) = ask(&mut e, &sheets_code(&rpath(&bad)));
    assert!(ok && why.starts_with("ERROR"), "{why}");
}

const MAKE_LABELLED: &str = r#"
df <- data.frame(id = 1:4, income = c(10.5, NA, 30, 41.25), name = c("a", "b", NA, "d"),
                 stringsAsFactors = FALSE)
df$sex <- haven::labelled(c(1, 2, 1, 2), labels = c(Male = 1, Female = 2), label = "Sex of person")
attr(df$income, "label") <- "Yearly income"
"#;

/// Checks a table made from `MAKE_LABELLED`. Stata and SPSS have no missing
/// text, so a missing string comes back empty.
fn check_labelled(t: &Table, factors: bool) {
    assert_eq!((t.nrow, t.columns.len()), (4, 4));
    let income = col(t, "income");
    let sex = col(t, "sex");
    assert!(income.is_some() && sex.is_some(), "{:?}", t.columns);
    let (Some(income), Some(sex)) = (income, sex) else {
        return;
    };
    assert_eq!(income.label(), Some("Yearly income"));
    if factors {
        let expected = Data::Factor {
            levels: vec![Some("Male".to_string()), Some("Female".to_string())],
            codes: vec![0, 1, 0, 1],
            ordered: false,
        };
        assert_eq!(sex.data, expected);
    } else {
        assert_eq!(sex.label(), Some("Sex of person"));
        assert_eq!(
            sex.value_labels(),
            vec![(1.0, "Male".to_string()), (2.0, "Female".to_string())]
        );
    }
    assert_eq!(
        col(t, "name").map(|c| &c.data),
        Some(&Data::Text(vec![
            Some("a".into()),
            Some("b".into()),
            Some(String::new()),
            Some("d".into())
        ]))
    );
}

#[test]
fn stata_files_keep_variable_and_value_labels_or_become_factors() {
    let Some(mut e) = engine_with("haven") else {
        return;
    };
    let path = dir().join("survey.dta");
    let make = format!(
        "{MAKE_LABELLED}\nhaven::write_dta(df, \"{}\")",
        rpath(&path)
    );
    let r = e.submit(Source::Typed, &make);
    assert!(r.is_ok());
    settle(&mut e);

    let mut h = HavenImport::new(&rpath(&path), HavenKind::Stata);
    h.name = "imported".into();
    let (t, outcome) = run_import(&mut e, &Import::Haven(h.clone()));
    assert_eq!(outcome, Outcome::Ok);
    check_labelled(&t.unwrap_or_else(|| panic!("no table")), false);

    h.labels_as_factors = true;
    let (t, outcome) = run_import(&mut e, &Import::Haven(h.clone()));
    assert_eq!(outcome, Outcome::Ok);
    check_labelled(&t.unwrap_or_else(|| panic!("no table")), true);

    let (ok, preview) = ask(&mut e, &Import::Haven(h).preview_code(3));
    assert!(
        ok && preview.contains("4 columns; first 3 rows"),
        "{preview}"
    );
}

#[test]
fn spss_and_sas_transport_files_import() {
    let Some(mut e) = engine_with("haven") else {
        return;
    };
    let d = dir();
    let (sav, xpt) = (d.join("s.sav"), d.join("s.xpt"));
    let make = format!(
        "{MAKE_LABELLED}\nhaven::write_sav(df, \"{}\"); haven::write_xpt(df[c('id','income','name')], \"{}\")",
        rpath(&sav),
        rpath(&xpt)
    );
    let r = e.submit(Source::Typed, &make);
    assert!(r.is_ok());
    settle(&mut e);

    let mut h = HavenImport::new(&rpath(&sav), HavenKind::Spss);
    h.name = "imported".into();
    let (t, outcome) = run_import(&mut e, &Import::Haven(h));
    assert_eq!(outcome, Outcome::Ok);
    check_labelled(&t.unwrap_or_else(|| panic!("no table")), false);

    let mut x = HavenImport::new(&rpath(&xpt), HavenKind::SasTransport);
    x.name = "imported".into();
    let (t, outcome) = run_import(&mut e, &Import::Haven(x));
    assert_eq!(outcome, Outcome::Ok);
    let Some(t) = t else { panic!("no table") };
    assert_eq!((t.nrow, t.columns.len()), (4, 3));
}

#[test]
fn r_data_files_import_one_object_or_a_whole_workspace() {
    let Some(mut e) = engine() else { return };
    let d = dir();
    let (rds, rda) = (d.join("o.rds"), d.join("w.RData"));
    let make = format!(
        "tmp1 <- data.frame(a = 1:3); tmp2 <- data.frame(b = c('x','y')); \
         saveRDS(tmp1, \"{}\"); save(tmp1, tmp2, file = \"{}\"); rm(tmp1, tmp2)",
        rpath(&rds),
        rpath(&rda)
    );
    let r = e.submit(Source::Typed, &make);
    assert!(r.is_ok());
    settle(&mut e);

    let mut one = RObjectImport::new(&rpath(&rds), false);
    one.name = "imported".into();
    let (t, outcome) = run_import(&mut e, &Import::RObject(one.clone()));
    assert_eq!(outcome, Outcome::Ok);
    assert_eq!(
        t.and_then(|t| t.columns.first().map(|c| c.data.clone())),
        Some(Data::Int(vec![1, 2, 3]))
    );
    let (ok, preview) = ask(&mut e, &Import::RObject(one).preview_code(5));
    assert!(ok && preview.contains("data.frame"), "{preview}");

    let all = Import::RObject(RObjectImport::new(&rpath(&rda), true));
    let (_, outcome) = run_import(&mut e, &all);
    assert_eq!(outcome, Outcome::Ok);
    let names: Vec<String> = e.workspace().iter().map(|w| w.name.clone()).collect();
    assert!(
        names.contains(&"tmp1".into()) && names.contains(&"tmp2".into()),
        "{names:?}"
    );
    let (ok, preview) = ask(&mut e, &all.preview_code(5));
    assert!(ok && preview.contains("tmp1, tmp2"), "{preview}");
}

#[test]
fn a_missing_package_is_reported_as_no_not_an_error() {
    let Some(mut e) = engine() else { return };
    let (ok, answer) = ask(&mut e, &package_check_code("definitelyNotAPackage"));
    assert!(ok);
    assert_eq!(answer.trim(), "no");
}

#[test]
fn parquet_and_feather_files_import_with_their_types() {
    let Some(mut e) = engine_with("arrow") else {
        return;
    };
    let d = dir();
    let (pq, ft) = (d.join("t.parquet"), d.join("t.feather"));
    let make = format!(
        "df <- data.frame(id = 1:4, x = c(1.5, NA, 3, 4), g = factor(c('a', 'b', 'a', NA)), \
         when = as.Date('2024-01-01') + 0:3, name = c('p', 'q', NA, 's')); \
         arrow::write_parquet(df, \"{}\"); arrow::write_feather(df, \"{}\")",
        rpath(&pq),
        rpath(&ft)
    );
    assert!(e.submit(Source::Typed, &make).is_ok());
    settle(&mut e);

    for (path, format, label) in [
        (&pq, ArrowFormat::Parquet, "parquet"),
        (&ft, ArrowFormat::Feather, "feather"),
    ] {
        let mut a = ArrowImport::new(&rpath(path), format);
        a.name = "imported".into();
        let (t, outcome) = run_import(&mut e, &Import::Arrow(a.clone()));
        assert_eq!(outcome, Outcome::Ok, "{label}");
        let Some(t) = t else {
            panic!("{label}: no table")
        };
        assert_eq!((t.nrow, t.columns.len()), (4, 5), "{label}");
        assert_eq!(
            col(&t, "id").map(|c| c.data.clone()),
            Some(Data::Int(vec![1, 2, 3, 4])),
            "{label}"
        );
        assert!(
            matches!(col(&t, "x").map(|c| &c.data), Some(Data::Double(v)) if v.len() == 4 && rcraft_rdata::is_na(v[1])),
            "{label}: x"
        );
        assert!(
            matches!(col(&t, "g").map(|c| &c.data), Some(Data::Factor { levels, .. }) if levels.len() == 2),
            "{label}: g"
        );
        assert!(
            matches!(col(&t, "when").map(|c| &c.data), Some(Data::Date(v)) if v.len() == 4),
            "{label}: when"
        );
        assert!(
            matches!(col(&t, "name").map(|c| &c.data), Some(Data::Text(v)) if v.get(2) == Some(&None)),
            "{label}: name"
        );
        let (ok, preview) = ask(&mut e, &Import::Arrow(a).preview_code(2));
        assert!(
            ok && preview.contains("5 columns; 4 rows") && preview.contains("name"),
            "{label}: {preview}"
        );
    }
}
