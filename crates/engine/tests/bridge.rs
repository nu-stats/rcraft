//! The data bridge against a real R (skipped with a note when none is found):
//! R writes a table with base R only, the Rust reader reads it.

use std::path::PathBuf;
use std::time::Duration;

use rcraft_rdata::{is_na, read_file, write_code, AttrValue, Data, INT_NA};
use rcraft_rhost::{find_installs, Env, Session, SessionConfig};

fn session() -> Option<Session> {
    let install = find_installs(&Env::from_process())
        .into_iter()
        .find(|i| i.is_supported());
    let Some(install) = install else {
        eprintln!("skipped: no supported R on this machine");
        return None;
    };
    Session::start(SessionConfig::new(install)).ok()
}

fn temp_file(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("rcraft-bridge-{}", std::process::id()));
    let _ = std::fs::create_dir_all(&dir);
    dir.join(name)
}

fn run_ok(s: &mut Session, code: &str) {
    let out = s.run_to_end(code, Duration::from_secs(120));
    assert!(out.is_ok(), "{out:?}");
    if let Ok(o) = out {
        assert!(o.error.is_none(), "{code}\n{:?}", o.error);
    }
}

fn path_for_r(p: &std::path::Path) -> String {
    p.to_string_lossy().replace('\\', "/")
}

const MAKE: &str = r#"
df <- data.frame(
  i = c(1L, NA, 3L),
  d = c(1.5, NA, NaN),
  l = c(TRUE, NA, FALSE),
  f = factor(c("b", "a", NA), levels = c("a", "b", "c")),
  o = factor(c("lo", "hi", "lo"), levels = c("lo", "hi"), ordered = TRUE),
  day = as.Date(c("2020-02-29", NA, "1970-01-02")),
  t = c("plain", NA, "comma, \"quote\"\nline é中"),
  stringsAsFactors = FALSE
)
df$when <- as.POSIXct(c("2021-03-04 05:06:07", NA, "2000-01-01 00:00:00"), tz = "UTC")
df$gap <- as.difftime(c(1, NA, 3), units = "mins")
attr(df$d, "label") <- "A labelled variable"
attr(df$i, "labels") <- c(Low = 1, High = 3)
attr(df$i, "junk") <- list(1, 2)
df$lst <- list(1:3, "a", NULL)
df$m <- matrix(1:6, 3)
"#;

#[test]
fn every_kind_reads_back_exactly() {
    let Some(mut s) = session() else { return };
    let path = temp_file("kinds.rcb");
    run_ok(&mut s, MAKE);
    run_ok(&mut s, &write_code("df", &path_for_r(&path)));
    let table = read_file(&path);
    let _ = std::fs::remove_file(&path);
    assert!(table.is_ok(), "{table:?}");
    let Ok(t) = table else { return };
    assert_eq!(t.nrow, 3);
    let names: Vec<&str> = t.columns.iter().map(|c| c.name.as_str()).collect();
    assert_eq!(
        names,
        ["i", "d", "l", "f", "o", "day", "t", "when", "gap", "lst", "m"]
    );

    let col = |n: &str| t.columns.iter().find(|c| c.name == n);
    let Some(i) = col("i") else { return };
    assert_eq!(i.data, Data::Int(vec![1, INT_NA, 3]));
    assert_eq!(
        i.value_labels(),
        vec![(1.0, "Low".into()), (3.0, "High".into())]
    );
    assert_eq!(
        i.dropped_attrs, 1,
        "the list attribute is reported as dropped"
    );

    let Some(d) = col("d") else { return };
    assert_eq!(d.label(), Some("A labelled variable"));
    match &d.data {
        Data::Double(v) => {
            assert_eq!(v.first(), Some(&1.5));
            assert!(v.get(1).is_some_and(|x| is_na(*x)), "NA stays NA");
            assert!(
                v.get(2).is_some_and(|x| x.is_nan() && !is_na(*x)),
                "NaN stays NaN"
            );
        }
        other => panic!("{other:?}"),
    }

    let Some(l) = col("l") else { return };
    assert_eq!(l.data, Data::Logical(vec![1, -1, 0]));

    let Some(f) = col("f") else { return };
    assert_eq!(
        f.data,
        Data::Factor {
            levels: vec![Some("a".into()), Some("b".into()), Some("c".into())],
            codes: vec![1, 0, -1],
            ordered: false
        }
    );
    let Some(o) = col("o") else { return };
    assert!(matches!(&o.data, Data::Factor { ordered: true, .. }));

    let Some(day) = col("day") else { return };
    match &day.data {
        Data::Date(v) => {
            assert_eq!(v.first(), Some(&18321.0)); // 2020-02-29
            assert!(v.get(1).is_some_and(|x| x.is_nan()));
            assert_eq!(v.get(2), Some(&1.0));
        }
        other => panic!("{other:?}"),
    }

    let Some(tx) = col("t") else { return };
    assert_eq!(
        tx.data,
        Data::Text(vec![
            Some("plain".into()),
            None,
            Some("comma, \"quote\"\nline é中".into())
        ])
    );

    let Some(w) = col("when") else { return };
    match &w.data {
        Data::DateTime { secs, tz } => {
            assert_eq!(tz, "UTC");
            assert_eq!(secs.first(), Some(&1_614_834_367.0)); // 2021-03-04 05:06:07 UTC
            assert_eq!(secs.get(2), Some(&946_684_800.0));
        }
        other => panic!("{other:?}"),
    }
    let Some(g) = col("gap") else { return };
    match &g.data {
        Data::Duration { values, units } => {
            assert_eq!(units, "mins");
            assert_eq!(values.first(), Some(&1.0));
        }
        other => panic!("{other:?}"),
    }
    let Some(lst) = col("lst") else { return };
    assert_eq!(
        lst.data,
        Data::Other(vec![
            Some("1, 2, 3".into()),
            Some("a".into()),
            Some("NULL".into())
        ])
    );
    let Some(m) = col("m") else { return };
    assert_eq!(
        m.data,
        Data::Other(vec![
            Some("1, 4".into()),
            Some("2, 5".into()),
            Some("3, 6".into())
        ])
    );
    // value labels are numbers with names, not text
    assert!(i
        .attrs
        .iter()
        .any(|a| a.key == "labels" && matches!(a.value, AttrValue::Number(_))));
}

#[test]
fn a_large_table_matches_r_and_a_truncated_file_is_refused() {
    let Some(mut s) = session() else { return };
    let path = temp_file("big.rcb");
    run_ok(
        &mut s,
        "set.seed(7); n <- 300000L; big <- data.frame(x = rnorm(n), \
         y = ifelse(runif(n) < .1, NA, runif(n)), s = paste0('r', sample.int(n)))",
    );
    run_ok(&mut s, &write_code("big", &path_for_r(&path)));
    let out = s.run_to_end(
        "cat(sprintf('%.9f %d', sum(big$x), sum(is.na(big$y))))",
        Duration::from_secs(60),
    );
    let Ok(out) = out else { panic!("{out:?}") };
    let text = out.stdout.trim().to_string();
    let t = read_file(&path);
    assert!(t.is_ok(), "{t:?}");
    let Ok(t) = t else { return };
    assert_eq!(t.nrow, 300_000);
    let (Some(x), Some(y)) = (t.columns.first(), t.columns.get(1)) else {
        panic!("columns")
    };
    let Data::Double(xs) = &x.data else {
        panic!("x")
    };
    let sum: f64 = xs.iter().sum();
    let expected = format!("{sum:.9} {}", y.data.missing());
    assert_eq!(text, expected);

    // a file cut short is an error, not a half-read table
    let bytes = std::fs::read(&path).unwrap_or_default();
    let cut = temp_file("cut.rcb");
    let _ = std::fs::write(&cut, bytes.get(..bytes.len() / 2).unwrap_or_default());
    assert!(read_file(&cut).is_err());
    let _ = std::fs::remove_file(&path);
    let _ = std::fs::remove_file(&cut);
}

#[test]
fn too_many_cells_is_an_r_error_not_a_crash() {
    let Some(mut s) = session() else { return };
    let path = temp_file("never.rcb");
    // ask for the table with a tiny cell limit by calling the writer directly
    let code = format!(
        "tryCatch(({})(data.frame(a = 1:10), {}, 5), error = function(e) cat(conditionMessage(e)))",
        rcraft_rdata::WRITER_R.trim_end(),
        rcraft_rdata::r_string(&path_for_r(&path))
    );
    let out = s.run_to_end(&code, Duration::from_secs(60));
    assert!(out.is_ok());
    if let Ok(o) = out {
        assert!(o.stdout.contains("too large"), "{}", o.stdout);
    }
    let _ = std::fs::remove_file(&path);
}
