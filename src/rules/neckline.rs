//! A dress's front neckline: `round`, or a `cowl`. The cowl raises centre
//! front (torso.rs works out how far from `[dress] cowl_drape`) so the extra
//! length drapes in soft folds; its straight top edge gets a facing. A cowl
//! hangs best cut on the bias, which the notes say.

use super::{garment_words, Rule};
use crate::config::Config;
use crate::draft::Draft;
use crate::geom::pt;
use crate::model::{Garment, Spec};
use crate::pattern::{Cut, Edge, EdgeKind, Piece};

pub struct Neckline;

impl Rule for Neckline {
    fn name(&self) -> &'static str {
        "neckline"
    }
    fn all_variants(&self) -> &'static [&'static str] {
        &["round", "cowl"]
    }
    fn asked(&self, spec: &Spec, _cfg: &Config) -> Option<String> {
        (spec.g() == Garment::Dress).then(|| garment_words(spec, self.name()))
    }
    /// A cowl only when the sentence asks for one.
    fn variants(&self, spec: &Spec, _cfg: &Config) -> Vec<&'static str> {
        if spec.pin(self.name()).is_some() {
            vec!["round", "cowl"]
        } else {
            vec!["round"]
        }
    }
    fn params(&self, variant: &str, d: &mut Draft) {
        if let Some(dp) = d.top.dress.as_mut() {
            dp.cowl = variant == "cowl";
        }
    }
    fn pieces(&self, variant: &str, d: &mut Draft) -> Result<(), String> {
        if variant != "cowl" {
            return Ok(());
        }
        let words = garment_words(d.spec, self.name());
        let top = d.piece("front").edge("neck").pts.clone();
        let (a, b) = (top[0], top[1]);
        let f = d.cfg.dress("cowl_facing");
        let dir = (b - a).unit();
        let mut n = pt(-dir.y, dir.x);
        if n.y < 0.0 {
            n = n * -1.0;
        }
        let c = b + n * f;
        let t = c.x / dir.x;
        let q = pt(0.0, c.y - dir.y * t);
        let facing = Piece::new(
            "cowl facing",
            Cut::FOLD,
            vec![
                Edge::new("top", EdgeKind::Seam, vec![a, b]),
                Edge::new("shoulder", EdgeKind::Seam, vec![b, c]),
                Edge::new("inner", EdgeKind::Hem, vec![c, q]).sa(5.0),
                Edge::new("cf", EdgeKind::Fold, vec![q, a]),
            ],
        );
        d.add(facing.by("neckline/cowl", &words));
        // worn, the top edge hangs between the back's neck points
        let w = d.piece("back").edge("neck").pts.last().expect("neck").x;
        let l = a.dist(b);
        let sag = (3.0 * w * (l - w).max(0.0) / 2.0).sqrt();
        let want = d.cfg.dress("cowl_drape");
        d.check("cowl drapes", (sag - want).abs() < 5.0, format!("top edge spread to {:.1} cm a side, so it hangs {:.1} cm deep (want {:.1} cm)", l / 10.0, sag / 10.0, want / 10.0));
        d.notes.push("cut the cowl front and its facing on the bias (fold the fabric at 45 degrees) so the cowl drapes".into());
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::rules::test_util::*;

    #[test]
    fn a_cowl_raises_centre_front_by_the_drape() {
        let p = ok("make a dress with a cowl neck", &[("closure", "back_zip")]);
        assert!(p.checks.iter().any(|c| c.name == "cowl drapes" && c.ok));
        // spread, not a spike: the top edge runs straight across and wide
        let top = &p.piece("front").edge("neck").pts;
        assert!((top[0].y - top[1].y).abs() < 1e-9 && top[1].x > 150.0, "{top:?}");
        // the swing keeps the shoulder seam's length
        let (f, b) = (p.piece("front").len("shoulder"), p.piece("back").len("shoulder"));
        assert!((f - b).abs() < 1e-6);
        assert!(p.pieces.iter().any(|x| x.name == "cowl facing"));
        assert!(!p.pieces.iter().any(|x| x.name == "front neck facing"));
    }

    #[test]
    fn an_open_back_keeps_a_strap() {
        let p = ok("make a dress with an open back", &[("closure", "back_zip")]);
        assert!(p.piece("back").edge("neck").pts[0].y > 250.0);
        assert!(p.checks.iter().any(|c| c.name.contains("strap") && c.ok));
    }
}
