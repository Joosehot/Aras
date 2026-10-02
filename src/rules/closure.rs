//! How a dress goes on. `back_zip`: an invisible zip down centre back to the
//! hip, so the back is cut in two. `pull_on`: no opening at all; it passes
//! only if the head goes through the neckline and the narrowest girth (the
//! waist) goes over the chest and the seat. The checks decide, not a guess.

use super::{garment_words, Rule};
use crate::config::Config;
use crate::draft::Draft;
use crate::model::{Garment, Spec};

pub struct Closure;

impl Rule for Closure {
    fn name(&self) -> &'static str {
        "closure"
    }
    fn all_variants(&self) -> &'static [&'static str] {
        &["back_zip", "pull_on"]
    }
    fn asked(&self, spec: &Spec, _cfg: &Config) -> Option<String> {
        (spec.g() == Garment::Dress).then(|| garment_words(spec, self.name()))
    }
    fn params(&self, variant: &str, d: &mut Draft) {
        if let Some(dp) = d.top.dress.as_mut() {
            dp.zip = variant == "back_zip";
        }
    }
    fn pieces(&self, variant: &str, d: &mut Draft) -> Result<(), String> {
        let (m, stretch) = (d.m, d.fabric.stretch);
        let dp = d.top.dress.expect("a dress");
        let neck = 2.0 * (d.piece("front").len("neck") + d.piece("back").len("neck"));
        // the girth at the waist line, all four quarters
        let waist = 4.0 * crate::blocks::torso::dress_side(d, d.piece("front").edge("side").pts[0].x).0;
        if variant == "back_zip" {
            let back = d.piece("back");
            let zip = back
                .marks
                .iter()
                .find_map(|mk| match mk {
                    crate::pattern::Mark::Line { pts, dashed: true } if pts.len() == 2 && pts[0].x.abs() < 1e-9 && pts[1].x.abs() < 1e-9 => Some(pts[1].y - pts[0].y),
                    _ => None,
                })
                .unwrap_or(0.0)
                + d.pieces.iter().find(|p| p.name == "skirt back").map_or(0.0, |s| {
                    s.marks.iter().find_map(|mk| match mk {
                        crate::pattern::Mark::Line { pts, dashed: true } if pts[0].x.abs() < 1e-9 && pts[1].x.abs() < 1e-9 => Some(pts[1].y - pts[0].y),
                        _ => None,
                    }).unwrap_or(0.0)
                });
            d.clearance("head goes through the opened zip", neck + 2.0 * zip, stretch, m.head, "head");
            // below the zip the dress is closed: there it must pass the seat
            let below = 4.0 * crate::blocks::torso::dress_side(d, d.piece("front").edge("side").pts[0].x).1;
            d.clearance("seat goes through below the zip", below, stretch, m.seat, "seat");
            let max = d.cfg.dress("zip_max");
            d.check("zip length", zip <= max, format!("{:.1} cm (zips go to {:.1} cm)", zip / 10.0, max / 10.0));
            let _ = dp;
        } else {
            d.clearance("head goes through the neckline", neck, stretch, m.head, "head");
            d.clearance("dress goes over the chest at its waist", waist, stretch, m.chest, "chest");
            d.clearance("dress goes over the seat at its waist", waist, stretch, m.seat, "seat");
        }
        let _ = garment_words(d.spec, self.name());
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::rules::test_util::*;

    #[test]
    fn a_fitted_woven_dress_cannot_pull_on() {
        let p = pattern("make a dress", &[("closure", "pull_on")]).unwrap();
        assert!(failed(&p).iter().any(|c| c.contains("head") || c.contains("over the")), "{:?}", failed(&p));
    }

    #[test]
    fn the_zip_cuts_the_back_in_two() {
        let p = ok("make a dress", &[("closure", "back_zip")]);
        assert!(!p.piece("back").cut.fold);
        assert!(p.piece("front").cut.fold);
    }
}
