//! The v0 proof:
//! 1. determinism: every example drafts to byte-identical SVG, run after run
//! 2. stability: the output matches the checked-in golden file
//! 3. validity: every seam check passes, every outline is simple, no two
//!    pieces overlap on the fabric, and the SVG is well formed
//!
//! Regenerate goldens after an intended change with `ARAS_BLESS=1 cargo test`.

use aras::config::Config;
use aras::geom;
use aras::{compile, Options};
use std::fs;
use std::path::{Path, PathBuf};

fn examples() -> Vec<(String, String)> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("examples");
    let mut out: Vec<(String, String)> = fs::read_dir(&dir)
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().is_some_and(|x| x == "txt"))
        .map(|p| (p.file_stem().unwrap().to_string_lossy().into_owned(), fs::read_to_string(&p).unwrap()))
        .collect();
    out.sort();
    assert!(!out.is_empty(), "no examples found");
    out
}

fn golden_path(stem: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("examples/out").join(format!("{stem}.svg"))
}

#[test]
fn examples_are_deterministic_and_match_goldens() {
    let cfg = Config::builtin();
    let bless = std::env::var_os("ARAS_BLESS").is_some();
    let mut failures = Vec::new();
    for (stem, sentence) in examples() {
        let first = compile(&sentence, &cfg, &Options::default()).unwrap_or_else(|d| panic!("{stem}: {d:?}")).svg;
        for _ in 0..3 {
            let again = compile(&sentence, &cfg, &Options::default()).unwrap().svg;
            assert_eq!(first, again, "{stem}: output changed between runs");
        }
        let path = golden_path(&stem);
        if bless {
            fs::write(&path, &first).unwrap();
            continue;
        }
        match fs::read_to_string(&path) {
            Ok(golden) if golden.replace("\r\n", "\n") == first => {}
            Ok(_) => failures.push(format!("{stem}: differs from {}", path.display())),
            Err(_) => failures.push(format!("{stem}: missing {}", path.display())),
        }
    }
    assert!(failures.is_empty(), "{}\n(run with ARAS_BLESS=1 to accept)", failures.join("\n"));
}

#[test]
fn examples_are_valid_patterns() {
    let cfg = Config::builtin();
    for (stem, sentence) in examples() {
        let c = compile(&sentence, &cfg, &Options::default()).unwrap_or_else(|d| panic!("{stem}: {d:?}"));
        let p = &c.outcome.pattern;
        for check in &p.checks {
            assert!(check.ok, "{stem}: {} failed: {}", check.name, check.detail);
        }
        for piece in &p.pieces {
            let (seam, hem) = p.allowances(piece);
            for unfold in [false, true] {
                let (stitch, cut) = piece.outlines(seam, hem, unfold);
                assert!(!geom::self_intersects(&stitch) && !geom::self_intersects(&cut), "{stem}: {} crosses itself", piece.name);
            }
        }
        for sheet in &c.outcome.marker.sheets {
            for (i, a) in sheet.placed.iter().enumerate() {
                assert!(a.rect.max.x <= sheet.width + 1e-6, "{stem}: {} is off the fabric", p.pieces[a.piece].name);
                for b in &sheet.placed[i + 1..] {
                    let apart = a.rect.max.x <= b.rect.min.x + 1e-6
                        || b.rect.max.x <= a.rect.min.x + 1e-6
                        || a.rect.max.y <= b.rect.min.y + 1e-6
                        || b.rect.max.y <= a.rect.min.y + 1e-6;
                    assert!(apart, "{stem}: {} overlaps {}", p.pieces[a.piece].name, p.pieces[b.piece].name);
                }
            }
        }
        let svg = &c.svg;
        assert!(svg.starts_with("<?xml") && svg.ends_with("</svg>\n"), "{stem}: SVG not closed");
        assert_eq!(svg.matches("<g>").count() + svg.matches("<g ").count(), svg.matches("</g>").count(), "{stem}: unbalanced groups");
        assert!(!svg.contains("NaN") && !svg.contains("inf"), "{stem}: non-finite coordinate");
    }
}

#[test]
fn every_sheet_kind_is_deterministic_and_well_formed() {
    let cfg = Config::builtin();
    for (stem, sentence) in examples() {
        for sheet in [aras::SheetKind::Product, aras::SheetKind::Marker, aras::SheetKind::Pattern] {
            let opts = Options { sheet, ..Options::default() };
            let a = compile(&sentence, &cfg, &opts).unwrap().svg;
            let b = compile(&sentence, &cfg, &opts).unwrap().svg;
            assert_eq!(a, b, "{stem} {sheet:?}: not deterministic");
            assert!(a.starts_with("<?xml") && a.ends_with("</svg>
"), "{stem} {sheet:?}");
            assert!(!a.contains("NaN") && !a.contains("inf"), "{stem} {sheet:?}: non-finite");
            for id in a.match_indices("url(#").map(|(i, _)| &a[i + 5..i + 5 + a[i + 5..].find(')').unwrap()]) {
                assert!(a.contains(&format!("id=\"{id}\"")), "{stem} {sheet:?}: missing pattern {id}");
            }
        }
    }
}

#[test]
fn colour_blocking_splits_the_fabric() {
    let cfg = Config::builtin();
    let c = compile("make a black hoodie with white sleeves", &cfg, &Options::default()).unwrap();
    let labels: Vec<&str> = c.marker.sheets.iter().filter(|s| !s.rib).map(|s| s.label.as_str()).collect();
    assert_eq!(labels, vec!["Black", "White"]);
}

#[test]
fn stripes_match_at_the_side_seams() {
    // Front and back anchor their print on the same underarm line; the
    // sleeve anchors on its own underarm, which is sewn to that point.
    let cfg = Config::builtin();
    let c = compile("make a navy and white striped t-shirt", &cfg, &Options::default()).unwrap();
    let p = &c.outcome.pattern;
    let (f, b, s) = (p.piece("front"), p.piece("back"), p.piece("sleeve"));
    assert_eq!(f.anchor.unwrap().y, b.anchor.unwrap().y);
    assert_eq!(f.anchor.unwrap().y, f.edge("side").pts[0].y);
    assert_eq!(s.anchor.unwrap().y, s.edge("underarm_front").pts[0].y);
}

#[test]
fn every_size_drafts_every_garment() {
    let cfg = Config::builtin();
    for g in ["t-shirt", "hoodie", "sweatshirt", "long sleeve shirt", "tight pants", "pants", "baggy pants"] {
        for size in ["small", "medium", "large", "extra large"] {
            let s = format!("make {size} {g}");
            let c = compile(&s, &cfg, &Options::default()).unwrap_or_else(|d| panic!("{s}: {d:?}"));
            assert!(c.outcome.pattern.ok(), "{s}");
        }
    }
}

#[test]
fn grading_is_monotonic() {
    let cfg = Config::builtin();
    let chest = |size: &str| {
        let c = compile(&format!("make a t-shirt in size {size}"), &cfg, &Options::default()).unwrap();
        c.outcome.pattern.piece("front").len("hem")
    };
    let sizes: Vec<f64> = ["S", "M", "L", "XL"].iter().map(|s| chest(s)).collect();
    assert!(sizes.windows(2).all(|w| w[1] > w[0]), "{sizes:?}");
}

#[test]
fn unknown_words_fail_with_a_hint() {
    let cfg = Config::builtin();
    let err = compile("make a hodie", &cfg, &Options::default()).err().unwrap();
    assert!(err[0].message.contains("hodie"));
    assert!(err[0].hint.as_deref().unwrap().contains("hoodie"));
}

#[test]
fn lenient_mode_ignores_unknown_words() {
    let cfg = Config::builtin();
    let opts = Options { lenient: true, ..Options::default() };
    assert!(compile("kindly make a t-shirt", &cfg, &opts).is_ok());
}

#[test]
fn base_option_edits_without_naming_a_garment() {
    let cfg = Config::builtin();
    let opts = Options { base: Some(aras::model::Garment::Hoodie), ..Options::default() };
    let c = compile("widen the collar and lengthen the sleeves by two inches", &cfg, &opts).unwrap();
    assert_eq!(c.spec.edits.len(), 2);
}

#[test]
fn modifiers_change_the_tradeoff() {
    let cfg = Config::builtin();
    let pick = |s: &str| {
        let c = compile(s, &cfg, &Options::default()).unwrap();
        c.outcome.finalists[c.outcome.best].choices["collar"]
    };
    assert_eq!(pick("make a shirt"), "with_stand");
    assert_eq!(pick("simply make a shirt"), "one_piece");
}
