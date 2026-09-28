//! Shirt collars. `one_piece`: a convertible collar sewn straight to the
//! neckline. `with_stand`: a curved stand sewn to the neckline and a collar
//! sewn to the stand. Both are cut twice on the fold (upper and under).

use super::{garment_words, Rule};
use crate::config::Config;
use crate::draft::Draft;
use crate::geom::{self, cubic, pt};
use crate::model::{Garment, Spec};
use crate::pattern::{Cut, Edge, EdgeKind, Piece};

pub struct Collar;

const TWICE_ON_FOLD: Cut = Cut { count: 2, fold: true };

/// Half a collar: neck edge `len` long along the bottom, point at the front.
fn collar(name: &str, len: f64, height: f64, point: f64) -> Piece {
    Piece::new(
        name,
        TWICE_ON_FOLD,
        vec![
            Edge::new("outer", EdgeKind::Seam, vec![pt(0.0, 0.0), pt(len + point, 0.0)]),
            Edge::new("end", EdgeKind::Seam, vec![pt(len + point, 0.0), pt(len, height)]),
            Edge::new("neck", EdgeKind::Seam, vec![pt(len, height), pt(0.0, height)]),
            Edge::new("cb", EdgeKind::Fold, vec![pt(0.0, height), pt(0.0, 0.0)]),
        ],
    )
}

/// Half a stand whose neck edge is `len` long and rises toward the front.
fn stand(len: f64, height: f64, rise: f64) -> Piece {
    let neck = |l: f64| cubic(pt(0.0, height), pt(l * 0.5, height), pt(l * 0.85, height - rise * 0.3), pt(l, height - rise));
    let l = geom::solve(len * 0.8, len * 1.2, |l| geom::length(&neck(l)) - len).unwrap_or(len);
    let mut n = neck(l);
    n.reverse();
    let top = cubic(pt(0.0, 0.0), pt(l * 0.5, 0.0), pt(l * 0.85, -rise * 0.3), pt(l, -rise));
    Piece::new(
        "collar stand",
        TWICE_ON_FOLD,
        vec![
            Edge::new("top", EdgeKind::Seam, top),
            Edge::new("end", EdgeKind::Seam, vec![pt(l, -rise), pt(l, height - rise)]),
            Edge::new("neck", EdgeKind::Seam, n),
            Edge::new("cb", EdgeKind::Fold, vec![pt(0.0, height), pt(0.0, 0.0)]),
        ],
    )
}

impl Rule for Collar {
    fn name(&self) -> &'static str {
        "collar"
    }
    fn all_variants(&self) -> &'static [&'static str] {
        &["one_piece", "with_stand"]
    }
    fn asked(&self, spec: &Spec, _cfg: &Config) -> Option<String> {
        (spec.g() == Garment::Shirt).then(|| garment_words(spec, self.name()))
    }
    fn pieces(&self, variant: &str, d: &mut Draft) -> Result<(), String> {
        let cfg = d.cfg;
        let half = d.piece("front").len("neck") + d.piece("back").len("neck");
        let words = garment_words(d.spec, self.name());
        let rule = format!("collar/{variant}");
        let (h, point) = (cfg.part("collar_height"), cfg.part("collar_point"));
        if variant == "one_piece" {
            let c = collar("collar", half, h, point);
            d.seam("collar / neckline", 2.0 * half, 2.0 * c.len("neck"), 0.0, 0.0);
            d.add(c.by(&rule, &words));
        } else {
            let s = stand(half, cfg.part("stand_height"), cfg.part("stand_rise"));
            let c = collar("collar", s.len("top"), h - cfg.part("stand_height"), point);
            d.seam("collar stand / neckline", 2.0 * half, 2.0 * s.len("neck"), 0.0, 0.0);
            d.seam("collar / stand", 2.0 * s.len("top"), 2.0 * c.len("neck"), 0.0, 0.0);
            d.add(s.by(&rule, &words));
            d.add(c.by(&rule, &words));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::rules::test_util::*;

    #[test]
    fn stand_matches_the_neckline() {
        let p = ok("make a shirt", &[("collar", "with_stand")]);
        assert!(p.pieces.iter().any(|x| x.name == "collar stand"));
        assert!(p.checks.iter().any(|c| c.name == "collar stand / neckline" && c.ok));
    }

    #[test]
    fn one_piece_collar_only() {
        let p = ok("make a shirt", &[("collar", "one_piece")]);
        assert!(!p.pieces.iter().any(|x| x.name == "collar stand"));
    }
}
