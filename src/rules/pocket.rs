//! The kangaroo pocket: one piece on the fold with slanted hand openings,
//! sewn to the front just above the hem. Its placement is marked on the front.

use super::Rule;
use crate::config::Config;
use crate::draft::Draft;
use crate::geom::pt;
use crate::model::Spec;
use crate::pattern::{Cut, Edge, EdgeKind, Mark, Piece};

pub struct Pocket;

impl Rule for Pocket {
    fn name(&self) -> &'static str {
        "pocket"
    }
    fn all_variants(&self) -> &'static [&'static str] {
        &["kangaroo"]
    }
    fn asked(&self, spec: &Spec, cfg: &Config) -> Option<String> {
        spec.has_pocket(cfg).then(|| spec.feature_words(&spec.pocket))
    }
    fn pieces(&self, _variant: &str, d: &mut Draft) -> Result<(), String> {
        let cfg = d.cfg;
        let front = d.piece("front");
        let hem = front.edge("hem").pts.clone();
        let (side_x, hem_y) = (hem[0].x, hem[hem.len() - 1].y);
        let w = side_x * cfg.part("pocket_width");
        let (h, op) = (cfg.part("pocket_height"), cfg.part("pocket_opening"));
        if op >= w || op >= h {
            return Err("the kangaroo pocket is too small for its openings".into());
        }
        let pocket = Piece::new(
            "pocket",
            Cut::FOLD,
            vec![
                Edge::new("top", EdgeKind::Hem, vec![pt(0.0, 0.0), pt(w - op, 0.0)]),
                Edge::new("opening", EdgeKind::Hem, vec![pt(w - op, 0.0), pt(w, op)]),
                Edge::new("side", EdgeKind::Seam, vec![pt(w, op), pt(w, h)]),
                Edge::new("bottom", EdgeKind::Seam, vec![pt(w, h), pt(0.0, h)]),
                Edge::new("cf", EdgeKind::Fold, vec![pt(0.0, h), pt(0.0, 0.0)]),
            ],
        );
        let y0 = hem_y - cfg.part("pocket_above_hem") - h;
        let outline = vec![pt(0.0, y0), pt(w - op, y0), pt(w, y0 + op), pt(w, y0 + h), pt(0.0, y0 + h)];
        d.piece_mut("front").marks.push(Mark::Line { pts: outline, dashed: true });
        let words = d.spec.feature_words(&d.spec.pocket);
        d.add(pocket.by("pocket/kangaroo", &words));
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::rules::test_util::*;

    #[test]
    fn hoodie_has_a_pocket_unless_removed() {
        assert!(ok("make a hoodie", &[]).pieces.iter().any(|p| p.name == "pocket"));
        assert!(!ok("make a hoodie without a pocket", &[]).pieces.iter().any(|p| p.name == "pocket"));
    }
}
