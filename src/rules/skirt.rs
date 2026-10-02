//! A dress's silhouette below the waist. `shift`: the side seam goes
//! straight down from the hip. `a_line`: it steps out from the hip
//! (`[dress] a_line_flare` per mm of length). `gathered`: the bodice stops at
//! a waist seam and a skirt `gather` times the bodice waist is gathered onto it.

use super::{garment_words, Rule};
use crate::config::Config;
use crate::draft::{Draft, Silhouette};
use crate::geom::pt;
use crate::model::{Garment, Spec};
use crate::pattern::{Cut, Edge, EdgeKind, Mark, Piece};

pub struct Skirt;

impl Rule for Skirt {
    fn name(&self) -> &'static str {
        "skirt"
    }
    fn all_variants(&self) -> &'static [&'static str] {
        &["shift", "a_line", "gathered"]
    }
    fn asked(&self, spec: &Spec, _cfg: &Config) -> Option<String> {
        (spec.g() == Garment::Dress).then(|| garment_words(spec, self.name()))
    }
    fn params(&self, variant: &str, d: &mut Draft) {
        let cfg = d.cfg;
        if let Some(dp) = d.top.dress.as_mut() {
            match variant {
                "a_line" => {
                    dp.silhouette = Silhouette::ALine;
                    dp.flare = cfg.dress("a_line_flare");
                }
                "gathered" => dp.silhouette = Silhouette::Gathered,
                _ => dp.silhouette = Silhouette::Shift,
            }
        }
    }
    fn pieces(&self, variant: &str, d: &mut Draft) -> Result<(), String> {
        let words = garment_words(d.spec, self.name());
        let dp = d.top.dress.expect("a dress");
        if variant != "gathered" {
            let hem = 2.0 * (d.piece("front").len("hem") + d.piece("back").len("hem"));
            let seat = d.m.seat;
            d.check("hem lets the legs walk", hem >= seat, format!("hem {:.1} cm, seat {:.1} cm", hem / 10.0, seat / 10.0));
            return Ok(());
        }
        // The skirt: a rectangle gathered onto the bodice waist, front on the
        // fold, back on the fold or in two when the zip runs into it.
        let len = d.top.length - dp.waist_y;
        let (fw, bw) = (d.piece("front").len("waist"), d.piece("back").len("waist"));
        let rule = format!("skirt/{variant}");
        for (name, bodice, zip) in [("skirt front", fw, false), ("skirt back", bw, dp.zip)] {
            let w = bodice * dp.gather;
            let mut edges = vec![
                Edge::new("waist", EdgeKind::Seam, vec![pt(0.0, 0.0), pt(w, 0.0)]),
                Edge::new("side", EdgeKind::Seam, vec![pt(w, 0.0), pt(w, len)]),
                Edge::new("hem", EdgeKind::Hem, vec![pt(w, len), pt(0.0, len)]),
            ];
            let cut = if zip {
                edges.push(Edge::new("cb", EdgeKind::Seam, vec![pt(0.0, len), pt(0.0, 0.0)]));
                Cut::PAIR
            } else {
                edges.push(Edge::new(if name == "skirt front" { "cf" } else { "cb" }, EdgeKind::Fold, vec![pt(0.0, len), pt(0.0, 0.0)]));
                Cut::FOLD
            };
            let mut p = Piece::new(name, cut, edges).anchored(pt(0.0, 0.0));
            // gathering stitches along the waist
            p = p.mark(Mark::Line { pts: vec![pt(0.0, 8.0), pt(w, 8.0)], dashed: true });
            if zip {
                p = p.mark(Mark::Line { pts: vec![pt(0.0, 0.0), pt(0.0, (dp.hip_y - dp.waist_y).min(len))], dashed: true });
            }
            d.add(p.by(&rule, &words));
        }
        let (lo, hi) = (d.cfg.dress("gather_min"), d.cfg.dress("gather_max"));
        let ratio = d.piece("skirt front").len("waist") / fw;
        d.check("skirt gathers onto the bodice", ratio >= lo - 1e-9 && ratio <= hi + 1e-9, format!("skirt waist {:.2} x the bodice waist (gathers take {lo} to {hi})", ratio));
        d.seam("skirt side seam: front / back", d.piece("skirt front").len("side"), d.piece("skirt back").len("side"), 0.0, 0.0);
        let hem = 2.0 * (d.piece("skirt front").len("hem") + d.piece("skirt back").len("hem"));
        d.check("hem lets the legs walk", hem >= d.m.seat, format!("hem {:.1} cm, seat {:.1} cm", hem / 10.0, d.m.seat / 10.0));
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::rules::test_util::*;

    #[test]
    fn a_line_hem_is_wider_than_shift() {
        let s = ok("make a dress", &[("skirt", "shift"), ("closure", "back_zip")]);
        let a = ok("make a dress", &[("skirt", "a_line"), ("closure", "back_zip")]);
        assert!(a.piece("front").len("hem") > s.piece("front").len("hem") + 30.0);
    }

    #[test]
    fn gathered_skirt_is_gathered_onto_the_bodice() {
        let p = ok("make a dress", &[("skirt", "gathered"), ("closure", "back_zip")]);
        let r = p.piece("skirt front").len("waist") / p.piece("front").len("waist");
        assert!((r - 1.6).abs() < 1e-9);
        assert!(p.piece("front").has_edge("waist") && !p.piece("front").has_edge("hem"));
    }
}
