//! Hoods. `two_piece`: two sides joined by a seam over the head.
//! `three_piece`: narrower sides and a centre panel from forehead to nape,
//! which gives a rounder shape. The hood's neck edge is solved to match the
//! body's neckline exactly; the hood sizes come from head girth.

use super::Rule;
use crate::config::Config;
use crate::draft::Draft;
use crate::geom::{self, cubic, pt, Pt};
use crate::model::Spec;
use crate::pattern::{Cut, Edge, EdgeKind, Piece};

pub struct Hood;

/// Neck edge from the front (face) point to the back neck point at `bx`.
fn neck(h: f64, bx: f64, rise: f64) -> Vec<Pt> {
    cubic(pt(0.0, h), pt(bx * 0.4, h + 15.0), pt(bx * 0.7, h - rise * 0.6), pt(bx, h - rise))
}

/// Hood side for a given top point `top_y` (0 for two-piece) and depth.
fn side(h: f64, depth: f64, neck_len: f64, rise: f64, top_y: f64, face: Edge) -> Result<Piece, String> {
    let bx = geom::solve(40.0, 800.0, |bx| geom::length(&neck(h, bx, rise)) - neck_len)
        .ok_or_else(|| format!("no hood neck edge is {:.1} cm long", neck_len / 10.0))?;
    let bk = pt(bx, h - rise);
    let q = pt(depth, h * 0.4);
    let top = pt(0.0, top_y);
    let back = geom::chain(&[
        cubic(bk, bk + pt((depth - bx) * 0.3, -(bk.y - q.y) * 0.4), q + pt(0.0, (bk.y - q.y) * 0.4), q),
        cubic(q, q + pt(0.0, -(q.y - top_y) * 0.55), top + pt(depth * 0.55, 0.0), top),
    ]);
    Ok(Piece::new(
        "hood side",
        Cut::PAIR,
        vec![face, Edge::new("neck", EdgeKind::Seam, neck(h, bx, rise)), Edge::new("back", EdgeKind::Seam, back)],
    ))
}

/// Face edge: a casing when unlined, a plain seam when lined.
pub fn face(pts: Vec<Pt>, lined: bool, casing: f64) -> Edge {
    if lined {
        Edge::new("face", EdgeKind::Seam, pts)
    } else {
        Edge::new("face", EdgeKind::Hem, pts).sa(casing)
    }
}

impl Rule for Hood {
    fn name(&self) -> &'static str {
        "hood"
    }
    fn all_variants(&self) -> &'static [&'static str] {
        &["two_piece", "three_piece"]
    }
    fn asked(&self, spec: &Spec, cfg: &Config) -> Option<String> {
        spec.has_hood(cfg).then(|| {
            spec.pin(self.name()).map(|p| p.words.clone()).unwrap_or_else(|| spec.feature_words(&spec.hood))
        })
    }
    fn pieces(&self, variant: &str, d: &mut Draft) -> Result<(), String> {
        let (cfg, m) = (d.cfg, d.m);
        let lined = d.chose("hood_lining", "lined");
        let casing = cfg.part("hood_face_hem");
        let half_neck = d.piece("front").len("neck") + d.piece("back").len("neck");
        let h = m.head * cfg.part("hood_height") + cfg.part("hood_ease");
        let depth = m.head * cfg.part("hood_depth") + cfg.part("hood_ease");
        let rise = cfg.part("hood_neck_rise");
        let words = d.spec.pin(self.name()).map(|p| p.words.clone()).unwrap_or_else(|| d.spec.feature_words(&d.spec.hood));
        let rule = format!("hood/{variant}");
        d.clearance("head goes through the neckline", 2.0 * half_neck, d.fabric.stretch, m.head, "head");
        if variant == "two_piece" {
            let s = side(h, depth, half_neck, rise, 0.0, face(vec![pt(0.0, 0.0), pt(0.0, h)], lined, casing))?;
            d.seam("hood neck / neckline", 2.0 * half_neck, 2.0 * s.len("neck"), 0.0, 0.0);
            d.add(s.by(&rule, &words));
            return Ok(());
        }
        let (cw, nape) = (cfg.part("hood_center_width"), cfg.part("hood_center_nape"));
        let s = side(h, depth - cw / 2.0, half_neck - nape / 2.0, rise, cw / 2.0, face(vec![pt(0.0, cw / 2.0), pt(0.0, h)], lined, casing))?;
        let len = s.len("back");
        let centre = Piece::new(
            "hood centre",
            Cut::FOLD,
            vec![
                face(vec![pt(0.0, 0.0), pt(cw / 2.0, 0.0)], lined, casing),
                Edge::new("side", EdgeKind::Seam, vec![pt(cw / 2.0, 0.0), pt(nape / 2.0, len)]),
                Edge::new("neck", EdgeKind::Seam, vec![pt(nape / 2.0, len), pt(0.0, len)]),
                Edge::new("cb", EdgeKind::Fold, vec![pt(0.0, len), pt(0.0, 0.0)]),
            ],
        );
        d.seam("hood centre / hood side", s.len("back"), centre.len("side"), -1.0, 1.0);
        d.seam("hood neck / neckline", 2.0 * half_neck, 2.0 * s.len("neck") + 2.0 * centre.len("neck"), 0.0, 0.0);
        d.add(s.by(&rule, &words));
        d.add(centre.by(&rule, &words));
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::rules::test_util::*;

    #[test]
    fn two_piece_neck_matches() {
        let p = ok("make a hoodie", &[("hood", "two_piece")]);
        assert!(p.checks.iter().any(|c| c.name == "hood neck / neckline" && c.ok));
        assert!(!p.pieces.iter().any(|x| x.name == "hood centre"));
    }

    #[test]
    fn three_piece_has_a_centre_panel_that_fits() {
        let p = ok("make a hoodie", &[("hood", "three_piece")]);
        assert!(p.pieces.iter().any(|x| x.name == "hood centre"));
    }

    #[test]
    fn widening_the_collar_grows_the_hood_neck() {
        let a = ok("make a hoodie", &[("hood", "two_piece")]).piece("hood side").len("neck");
        let b = ok("make a hoodie and widen the collar by 2 inches", &[("hood", "two_piece")]).piece("hood side").len("neck");
        assert!(b > a + 20.0);
    }
}
