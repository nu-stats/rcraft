//! The readers against files R itself wrote (made-up packages, see the script
//! comment in `tests/fixtures/README`).

// a test file may panic to report a failure
#![allow(clippy::panic)]

use rcraft_rpkg::index::{Detail, Index, DB_COLUMNS, DCF_FIELDS};
use rcraft_rpkg::{dcf, rds, table, Error};

const V3: &[u8] = include_bytes!("fixtures/db_matrix_v3.rds");
const V2_PLAIN: &[u8] = include_bytes!("fixtures/db_matrix_v2_plain.rds");
const FRAME: &[u8] = include_bytes!("fixtures/db_frame.rds");
const XZ: &[u8] = include_bytes!("fixtures/db_matrix_xz.rds");
const FUNCTION: &[u8] = include_bytes!("fixtures/db_function.rds");
const PACKAGES: &[u8] = include_bytes!("fixtures/PACKAGES.gz");

fn db(bytes: &[u8]) -> table::Table {
    let node = rds::read(bytes).unwrap_or_else(|e| panic!("{e}"));
    table::from_node(&node, DB_COLUMNS).unwrap_or_else(|e| panic!("{e}"))
}

#[test]
fn the_default_gzip_version_3_file_reads() {
    let t = db(V3);
    assert_eq!(t.rows, 3);
    assert_eq!(t.get("Package", 0), Some("fakeone"));
    assert_eq!(t.get("Title", 0), Some("Fake One: A Made-Up Package"));
    // a newline inside a field survives the reader (the index tidies it later)
    assert_eq!(
        t.get("Description", 0),
        Some("A made-up package used to test\nthe reader. It has a two-line description.")
    );
    // R's NA is a missing value, an empty string is an empty string
    assert_eq!(t.get("Priority", 0), None);
    assert_eq!(t.get("LinkingTo", 1), Some("fakeone"));
    assert_eq!(t.get("Suggests", 1), Some(""));
    // text that is not ASCII
    assert_eq!(t.get("Maintainer", 1), Some("Zoë Müller <zoe@example.org>"));
    // a column that is in the file but not asked for is left out
    assert_eq!(t.column("MD5sum"), None);
}

#[test]
fn version_2_uncompressed_and_a_data_frame_give_the_same_table() {
    let a = db(V3);
    assert_eq!(db(V2_PLAIN), a);
    assert_eq!(db(FRAME), a);
}

#[test]
fn a_compression_or_object_the_reader_does_not_handle_is_refused_by_name() {
    assert!(matches!(
        rds::read(XZ),
        Err(Error::Format(_) | Error::Unsupported(_))
    ));
    assert!(matches!(rds::read(FUNCTION), Err(Error::Unsupported(_))));
}

#[test]
fn every_truncation_of_the_plain_file_is_an_error_and_never_a_crash() {
    assert!(rds::read(V2_PLAIN).is_ok());
    for n in 0..V2_PLAIN.len() {
        let cut = V2_PLAIN.get(..n).unwrap_or(&[]);
        assert!(
            rds::read(cut).is_err(),
            "prefix of {n} bytes read as a value"
        );
    }
}

#[test]
fn every_truncation_of_the_gzip_file_is_an_error_and_never_a_crash() {
    for n in 0..V3.len() {
        let cut = V3.get(..n).unwrap_or(&[]);
        assert!(
            rds::read(cut).is_err(),
            "prefix of {n} bytes read as a value"
        );
    }
}

#[test]
fn single_changed_bytes_never_crash_the_reader() {
    // not every change is an error (a letter in a text field is still text);
    // the point is that none panics or allocates without bound
    for i in 0..V2_PLAIN.len() {
        for byte in [0x00u8, 0x7f, 0xff] {
            let mut copy = V2_PLAIN.to_vec();
            if let Some(b) = copy.get_mut(i) {
                *b = byte;
            }
            let _ = rds::read(&copy).and_then(|n| table::from_node(&n, DB_COLUMNS));
        }
    }
}

#[test]
fn the_plain_text_list_reads_and_merges_with_the_database() {
    let names = dcf::read(PACKAGES, DCF_FIELDS).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(names.rows, 4);
    assert_eq!(names.get("Imports", 0), Some("fakethree (>= 0.4),\nstats"));
    assert_eq!(names.get("NeedsCompilation", 1), Some("yes"));
    let ix = Index::from_tables(Some(&db(V3)), Some(&names), 10);
    assert_eq!(ix.detail, Detail::Full);
    assert_eq!(ix.len(), 4);
    // sorted by name ignoring case, so "Faketwo" comes after "fakethree"
    let order: Vec<&str> = ix.packages().iter().map(|p| p.name.as_str()).collect();
    assert_eq!(order, vec!["fakeone", "fakethree", "Faketwo", "onlyindcf"]);
    let one = ix.get("fakeone");
    assert_eq!(one.map(|p| p.published.as_str()), Some("2025-02-03"));
    assert_eq!(one.and_then(|p| p.needs_compilation), Some(false));
    let deps: Vec<String> = one
        .map(|p| p.hard_dependencies().into_iter().map(|d| d.name).collect())
        .unwrap_or_default();
    assert_eq!(deps, vec!["fakethree", "stats"]);
    // offline search works on what was read
    assert_eq!(ix.with_prefix("FAKET").len(), 2);
}
