//! The files in `fixtures/unpack/` come from Husky Explorer's tests. They were
//! made with XZ Utils 5.8.3, the Zstandard CLI 1.5.7 and bzip2 1.0.8 from two
//! made-up texts, [`SMALL`] and [`gen`]:
//!
//! - `s64.xz`, `s32.xz`, `s256.xz`: `xz -C crc64 / crc32 / sha256 small.csv`
//! - `gen.xz`: `xz --block-size=40000 gen.csv` (two blocks)
//! - `s.zst`, `two.zst`: `zstd small.csv`; `two.zst` is `s.zst` twice (two frames)
//! - `gen1.zst`, `gen19.zst`, `gen22.zst`: `zstd -1 / -19 --no-check / --ultra -22 gen.csv`
//! - `both.tar.xz`, `both.tar.zst`: `tar --format=ustar` of small.csv and gen.csv, then xz / zstd
//! - `small.csv.bz2`: `bzip2 small.csv`; `two-blocks.bz2`: 30,000 lines over two 900 KB blocks

use std::io::Write;

use super::*;

const SMALL: &str = "GEOID,value\n25025,1\n25017,2\n";

fn gen() -> String {
    (0..4000)
        .map(|i| {
            format!(
                "{},{},county {}\n",
                25000 + (i * 37) % 211,
                (i * 7919) % 1009,
                i % 13
            )
        })
        .collect()
}

fn fixture(name: &str) -> Vec<u8> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("fixtures")
        .join("unpack")
        .join(name);
    std::fs::read(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

fn text(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

fn names_and_texts(files: &[Inner]) -> Vec<(String, String)> {
    files
        .iter()
        .map(|f| (f.name.clone(), text(&f.bytes)))
        .collect()
}

fn open(bytes: &[u8], name: &str) -> Result<Vec<Inner>, UnpackError> {
    unpack(bytes, name, MAX_UNPACKED)
}

fn gzip(data: &[u8]) -> Vec<u8> {
    let mut e = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
    e.write_all(data).unwrap();
    e.finish().unwrap()
}

/// A zip built here: each file stored (`false`) or deflated (`true`).
fn zip(files: &[(&str, &[u8], bool)]) -> Vec<u8> {
    let mut out = Vec::new();
    let mut dir = Vec::new();
    for (name, data, deflate) in files {
        let mut crc = flate2::Crc::new();
        crc.update(data);
        let packed = if *deflate {
            let mut e =
                flate2::write::DeflateEncoder::new(Vec::new(), flate2::Compression::default());
            e.write_all(data).unwrap();
            e.finish().unwrap()
        } else {
            data.to_vec()
        };
        let method: u16 = if *deflate { 8 } else { 0 };
        let local = out.len() as u32;
        out.extend_from_slice(&0x0403_4b50u32.to_le_bytes());
        out.extend_from_slice(&[20, 0, 0, 0]);
        out.extend_from_slice(&method.to_le_bytes());
        out.extend_from_slice(&[0; 4]);
        out.extend_from_slice(&crc.sum().to_le_bytes());
        out.extend_from_slice(&(packed.len() as u32).to_le_bytes());
        out.extend_from_slice(&(data.len() as u32).to_le_bytes());
        out.extend_from_slice(&(name.len() as u16).to_le_bytes());
        out.extend_from_slice(&[0, 0]);
        out.extend_from_slice(name.as_bytes());
        out.extend_from_slice(&packed);

        dir.extend_from_slice(&0x0201_4b50u32.to_le_bytes());
        dir.extend_from_slice(&[20, 0, 20, 0, 0, 0]);
        dir.extend_from_slice(&method.to_le_bytes());
        dir.extend_from_slice(&[0; 4]);
        dir.extend_from_slice(&crc.sum().to_le_bytes());
        dir.extend_from_slice(&(packed.len() as u32).to_le_bytes());
        dir.extend_from_slice(&(data.len() as u32).to_le_bytes());
        dir.extend_from_slice(&(name.len() as u16).to_le_bytes());
        dir.extend_from_slice(&[0; 12]);
        dir.extend_from_slice(&local.to_le_bytes());
        dir.extend_from_slice(name.as_bytes());
    }
    let dir_at = out.len() as u32;
    out.extend_from_slice(&dir);
    out.extend_from_slice(&0x0605_4b50u32.to_le_bytes());
    out.extend_from_slice(&[0; 4]);
    out.extend_from_slice(&(files.len() as u16).to_le_bytes());
    out.extend_from_slice(&(files.len() as u16).to_le_bytes());
    out.extend_from_slice(&(dir.len() as u32).to_le_bytes());
    out.extend_from_slice(&dir_at.to_le_bytes());
    out.extend_from_slice(&[0, 0]);
    out
}

/// A tar built by hand: one 512-byte header per file, data padded to 512.
fn tar(files: &[(&str, &str)]) -> Vec<u8> {
    let mut out = Vec::new();
    for (name, body) in files {
        let mut h = [0u8; 512];
        h[..name.len()].copy_from_slice(name.as_bytes());
        let size = format!("{:011o}", body.len());
        h[124..135].copy_from_slice(size.as_bytes());
        h[156] = b'0';
        h[257..263].copy_from_slice(b"ustar\0");
        out.extend_from_slice(&h);
        out.extend_from_slice(body.as_bytes());
        out.resize(out.len().div_ceil(512) * 512, 0);
    }
    out.extend_from_slice(&[0; 1024]);
    out
}

#[test]
fn the_kind_comes_from_the_first_bytes_not_the_name() {
    assert_eq!(kind_of(&gzip(b"x")), Kind::Gzip);
    assert_eq!(kind_of(&zip(&[("a.csv", b"x", false)])), Kind::Zip);
    assert_eq!(kind_of(&fixture("small.csv.bz2")), Kind::Bzip2);
    assert_eq!(kind_of(&tar(&[("a.csv", "x")])), Kind::Tar);
    assert_eq!(kind_of(&fixture("s64.xz")), Kind::Xz);
    assert_eq!(kind_of(&fixture("s.zst")), Kind::Zstd);
    assert_eq!(kind_of(b"GEOID,value"), Kind::Plain);
    assert_eq!(kind_of(b""), Kind::Plain);
    assert_eq!(kind_of(b"BZh"), Kind::Plain);
}

#[test]
fn xz_files_with_each_kind_of_check_and_two_blocks_are_read() {
    for f in ["s64.xz", "s32.xz", "s256.xz"] {
        let files = open(&fixture(f), "small.csv.xz").unwrap();
        assert_eq!(
            names_and_texts(&files),
            [("small.csv".into(), SMALL.into())],
            "{f}"
        );
    }
    let files = open(&fixture("gen.xz"), "gen.csv.xz").unwrap();
    assert_eq!(text(&files[0].bytes), gen());
}

#[test]
fn a_damaged_or_cut_off_xz_file_is_refused() {
    let mut bad = fixture("gen.xz");
    bad[5000] ^= 0x08;
    assert!(matches!(open(&bad, "g.xz"), Err(UnpackError::Damaged(_))));
    let cut = fixture("gen.xz");
    assert!(matches!(
        open(&cut[..4000], "g.xz"),
        Err(UnpackError::Damaged(_))
    ));
    for f in ["s64.xz", "s32.xz", "s256.xz"] {
        let mut flipped = fixture(f);
        // inside the only block's data or check
        let at = flipped.len() - 30;
        flipped[at] ^= 0x01;
        assert!(
            matches!(open(&flipped, f), Err(UnpackError::Damaged(_))),
            "{f}"
        );
    }
    assert_eq!(
        unpack(&fixture("gen.xz"), "g.xz", 1000),
        Err(UnpackError::TooLarge(1000))
    );
}

#[test]
fn zstd_files_at_each_level_with_and_without_a_checksum_and_two_frames_are_read() {
    let files = open(&fixture("s.zst"), "small.csv.zst").unwrap();
    assert_eq!(
        names_and_texts(&files),
        [("small.csv".into(), SMALL.into())]
    );
    let two = open(&fixture("two.zst"), "two.zst").unwrap();
    assert_eq!(text(&two[0].bytes), format!("{SMALL}{SMALL}"));
    for f in ["gen1.zst", "gen19.zst", "gen22.zst"] {
        let files = open(&fixture(f), f).unwrap();
        assert_eq!(text(&files[0].bytes), gen(), "{f}");
    }
    // a skippable frame between frames is passed over
    let mut skip = fixture("s.zst");
    skip.extend([0x50, 0x2a, 0x4d, 0x18, 3, 0, 0, 0, 1, 2, 3]);
    assert_eq!(text(&open(&skip, "s.zst").unwrap()[0].bytes), SMALL);
    let mut cut_skip = fixture("s.zst");
    cut_skip.extend([0x50, 0x2a, 0x4d, 0x18, 9, 0, 0, 0, 1]);
    assert!(matches!(
        open(&cut_skip, "s.zst"),
        Err(UnpackError::Damaged(_))
    ));
}

#[test]
fn a_damaged_or_cut_off_zstd_file_is_refused() {
    let mut bad = fixture("gen1.zst");
    bad[9000] ^= 0x10;
    assert!(matches!(open(&bad, "g.zst"), Err(UnpackError::Damaged(_))));
    let cut = fixture("gen1.zst");
    assert!(matches!(
        open(&cut[..9000], "g.zst"),
        Err(UnpackError::Damaged(_))
    ));
    // the content checksum is checked: change the last byte of s.zst
    let mut sum = fixture("s.zst");
    let last = sum.len() - 1;
    sum[last] ^= 0x01;
    assert!(matches!(open(&sum, "s.zst"), Err(UnpackError::Damaged(_))));
    assert_eq!(
        unpack(&fixture("gen1.zst"), "g.zst", 1000),
        Err(UnpackError::TooLarge(1000))
    );
}

#[test]
fn bzip2_files_are_read_across_blocks_and_streams() {
    let files = open(&fixture("small.csv.bz2"), "small.csv.bz2").unwrap();
    assert_eq!(
        names_and_texts(&files),
        [("small.csv".into(), SMALL.into())]
    );
    let two = open(&fixture("two-blocks.bz2"), "two.bz2").unwrap();
    assert_eq!(
        text(&two[0].bytes),
        "GEOID,value,25025,Suffolk County\n".repeat(30000)
    );
    // two streams one after another, as pbzip2 writes, are read as one
    let mut joined = fixture("small.csv.bz2");
    joined.extend(fixture("small.csv.bz2"));
    assert_eq!(
        text(&open(&joined, "j.bz2").unwrap()[0].bytes),
        SMALL.repeat(2)
    );
}

#[test]
fn a_damaged_or_cut_off_bzip2_file_is_refused() {
    let mut bad = fixture("two-blocks.bz2");
    bad[60] ^= 0x04;
    assert!(matches!(open(&bad, "b.bz2"), Err(UnpackError::Damaged(_))));
    let small = fixture("small.csv.bz2");
    assert!(matches!(
        open(&small[..30], "b.bz2"),
        Err(UnpackError::Damaged(_))
    ));
    // the whole-stream checksum, in the last bytes (the very last holds padding)
    let mut sum = fixture("small.csv.bz2");
    let last = sum.len() - 2;
    sum[last] ^= 0x01;
    assert!(matches!(open(&sum, "b.bz2"), Err(UnpackError::Damaged(_))));
    assert_eq!(
        unpack(&fixture("two-blocks.bz2"), "b.bz2", 1000),
        Err(UnpackError::TooLarge(1000))
    );
    // every cut and every flipped bit of a small file: an error, never a panic
    for n in 0..small.len() {
        let _ = open(&small[..n], "b.bz2");
        for bit in 0..8 {
            let mut f = small.clone();
            f[n] ^= 1 << bit;
            if let Ok(files) = open(&f, "b.bz2") {
                // a flip that still opens gives the right bytes (one in the
                // padding, say), or the file is no longer bzip2 and comes back as it is
                let ok = files[0].bytes == SMALL.as_bytes() || files[0].bytes == f;
                assert!(ok, "byte {n} bit {bit}");
            }
        }
    }
}

#[test]
fn gz_zip_tar_and_tar_gz_are_opened_and_named() {
    let plain = open(SMALL.as_bytes(), "plain.csv").unwrap();
    assert_eq!(
        names_and_texts(&plain),
        [("plain.csv".into(), SMALL.into())]
    );
    let gz = open(&gzip(SMALL.as_bytes()), "data.csv.gz").unwrap();
    assert_eq!(names_and_texts(&gz), [("data.csv".into(), SMALL.into())]);
    // two gzip members one after another
    let mut two = gzip(b"a,b\n");
    two.extend(gzip(b"1,2\n"));
    assert_eq!(text(&open(&two, "t.gz").unwrap()[0].bytes), "a,b\n1,2\n");

    let big = SMALL.repeat(50);
    let z = zip(&[
        ("data/a.csv", big.as_bytes(), true),
        ("b.txt", b"hello", false),
        ("data/", b"", false),
        ("__MACOSX/data/._a.csv", b"junk", false),
    ]);
    assert_eq!(
        names_and_texts(&open(&z, "x.zip").unwrap()),
        [
            ("data/a.csv".into(), big.clone()),
            ("b.txt".into(), "hello".into())
        ]
    );
    let t = tar(&[
        ("one.csv", SMALL),
        ("two.json", "{\"a\":1}"),
        ("._one.csv", "junk"),
    ]);
    let names: Vec<String> = open(&t, "x.tar")
        .unwrap()
        .into_iter()
        .map(|f| f.name)
        .collect();
    assert_eq!(names, ["one.csv", "two.json"]);
    assert_eq!(
        names_and_texts(&open(&gzip(&t), "x.tgz").unwrap()),
        [
            ("one.csv".into(), SMALL.into()),
            ("two.json".into(), "{\"a\":1}".into())
        ]
    );
    for f in ["both.tar.xz", "both.tar.zst"] {
        let files = open(&fixture(f), f).unwrap();
        let names: Vec<&str> = files.iter().map(|x| x.name.as_str()).collect();
        assert_eq!(names, ["small.csv", "gen.csv"], "{f}");
        assert_eq!(text(&files[1].bytes), gen());
    }
}

#[test]
fn archives_inside_archives_are_opened() {
    let inner = gzip(SMALL.as_bytes());
    let nested = zip(&[("inner.csv.gz", &inner, false)]);
    assert_eq!(
        names_and_texts(&open(&nested, "outer.zip").unwrap()),
        [("inner.csv".into(), SMALL.into())]
    );
    // but not without end
    let mut deep = SMALL.as_bytes().to_vec();
    for _ in 0..10 {
        deep = gzip(&deep);
    }
    let files = open(&deep, "deep.gz").unwrap();
    assert_eq!(files.len(), 1);
    assert_eq!(kind_of(&files[0].bytes), Kind::Gzip);
}

#[test]
fn r_files_and_office_files_are_left_packed() {
    // .rds is gzip inside; R reads it as it is
    let rds = gzip(b"X\n\0\0\0\x03");
    let files = open(&rds, "model.rds").unwrap();
    assert_eq!(files[0].bytes, rds);
    // an xlsx is a zip with [Content_Types].xml
    let xlsx = zip(&[
        ("[Content_Types].xml", b"<Types/>", false),
        ("xl/workbook.xml", b"<w/>", true),
    ]);
    let files = open(&xlsx, "book.xlsx").unwrap();
    assert_eq!(files[0].bytes, xlsx);
    let renamed = open(&xlsx, "book.zip").unwrap();
    assert_eq!(renamed[0].bytes, xlsx);
    // a zip holding a workbook gives the workbook whole
    let outer = zip(&[("book.xlsx", &xlsx, false)]);
    assert_eq!(open(&outer, "o.zip").unwrap()[0].bytes, xlsx);
}

#[test]
fn damaged_locked_and_unknown_archives_say_why() {
    let z = zip(&[("a.csv", SMALL.as_bytes(), false)]);
    let mut bad = z.clone();
    let g = bad.iter().position(|&b| b == b'G').unwrap();
    bad[g] = b'H';
    assert!(
        matches!(open(&bad, "bad.zip"), Err(UnpackError::Damaged(e)) if e.contains("checksum"))
    );

    let mut locked = z.clone();
    let dir = (0..locked.len() - 4)
        .find(|&i| u32_at(&locked, i) == Some(0x0201_4b50))
        .unwrap();
    locked[dir + 8] = 1;
    let e = open(&locked, "locked.zip").unwrap_err();
    assert!(e.to_string().contains("password-protected"), "{e}");

    let rar = open(&[0x52, 0x61, 0x72, 0x21, 0x1a, 0x07, 0, 0], "a.rar").unwrap_err();
    assert!(
        rar.to_string()
            .contains("RAR file, which RCraft cannot open"),
        "{rar}"
    );
    let seven = open(&[0x37, 0x7a, 0xbc, 0xaf, 0x27, 0x1c, 0, 4], "a.7z").unwrap_err();
    assert!(seven.to_string().contains("7-Zip"), "{seven}");
    let cut = open(&z[..20], "cut.zip").unwrap_err();
    assert!(cut.to_string().contains("damaged or cut off"), "{cut}");

    // an unknown method
    let mut lzma = z.clone();
    lzma[dir + 10] = 14;
    assert!(matches!(
        open(&lzma, "l.zip"),
        Err(UnpackError::Method { method: 14, .. })
    ));

    // every cut of a zip and a tar.gz: an error or files, never a panic
    let tgz = gzip(&tar(&[("one.csv", SMALL)]));
    for n in 0..z.len() {
        let _ = open(&z[..n], "z.zip");
    }
    for n in 0..tgz.len() {
        let _ = open(&tgz[..n], "t.tgz");
    }
}

#[test]
fn the_size_cap_holds() {
    let big = "x".repeat(5000);
    assert_eq!(
        unpack(&gzip(big.as_bytes()), "big.gz", 1000),
        Err(UnpackError::TooLarge(1000))
    );
    let z = zip(&[("a.csv", big.as_bytes(), true)]);
    assert_eq!(
        unpack(&z, "big.zip", 1000),
        Err(UnpackError::TooLarge(1000))
    );
    // the cap counts all files together
    let z = zip(&[
        ("a.csv", &[b'x'; 600], false),
        ("b.csv", &[b'y'; 600], false),
    ]);
    assert_eq!(
        unpack(&z, "two.zip", 1000),
        Err(UnpackError::TooLarge(1000))
    );
}

#[test]
fn offered_keeps_the_data_files_or_all_when_none_fit() {
    let f = |n: &str| Inner {
        name: n.to_string(),
        bytes: Vec::new(),
    };
    let files = [f("readme.txt"), f("data/a.CSV"), f("b.csv")];
    let names: Vec<&str> = offered(&files, &["csv"])
        .iter()
        .map(|f| f.name.as_str())
        .collect();
    assert_eq!(names, ["data/a.CSV", "b.csv"]);
    assert_eq!(offered(&files, &["json"]).len(), 3);
}

#[test]
fn files_are_opened_from_disk_and_written_beside_the_archive() {
    let dir = std::env::temp_dir().join(format!("rcraft-unpack-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();

    let plain = dir.join("plain.csv");
    std::fs::write(&plain, SMALL).unwrap();
    assert_eq!(open_path(&plain, MAX_UNPACKED), Ok(None));
    let rds = dir.join("m.rds");
    std::fs::write(&rds, gzip(b"X\n")).unwrap();
    assert_eq!(open_path(&rds, MAX_UNPACKED), Ok(None));
    let xlsx = dir.join("book.xlsx");
    std::fs::write(&xlsx, zip(&[("[Content_Types].xml", b"<Types/>", false)])).unwrap();
    assert_eq!(open_path(&xlsx, MAX_UNPACKED), Ok(None));
    assert!(matches!(
        open_path(&dir.join("gone.gz"), MAX_UNPACKED),
        Err(UnpackError::Io(_))
    ));

    let archive = dir.join("data.csv.gz");
    std::fs::write(&archive, gzip(SMALL.as_bytes())).unwrap();
    let files = open_path(&archive, MAX_UNPACKED).unwrap().unwrap();
    assert_eq!(names_and_texts(&files), [("data.csv".into(), SMALL.into())]);
    let inner = &files[0];

    // written beside it, and the same file is used again
    let first = write_beside(&archive, inner).unwrap();
    assert_eq!(first, dir.join("data.csv"));
    assert_eq!(write_beside(&archive, inner).unwrap(), first);
    // a different file of that name is never replaced
    std::fs::write(&first, "other").unwrap();
    let second = write_beside(&archive, inner).unwrap();
    assert_eq!(second, dir.join("data (unpacked).csv"));
    assert_eq!(std::fs::read_to_string(&first).unwrap(), "other");
    std::fs::write(&second, "other too").unwrap();
    assert_eq!(
        write_beside(&archive, inner).unwrap(),
        dir.join("data (unpacked 2).csv")
    );

    // names from inside the archive cannot climb out of the folder
    let sneaky = Inner {
        name: "../../evil/..\\x:y?.csv".into(),
        bytes: b"z".to_vec(),
    };
    let path = write_beside(&archive, &sneaky).unwrap();
    assert_eq!(path, dir.join("xy.csv"));
    let dots = Inner {
        name: "a/..".into(),
        bytes: b"z".to_vec(),
    };
    assert_eq!(write_beside(&archive, &dots).unwrap(), dir.join("unpacked"));

    let _ = std::fs::remove_dir_all(&dir);
}
