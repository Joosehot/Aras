//! How pants close. `zip_fly`: a shaped waist (front pleat if the side seam
//! can't take the suppression, back darts), a fly and a straight waistband.
//! `elastic`: no shaping, a casing turned down at the waist; the hips must
//! pass through it.

use super::{garment_words, Rule};
use crate::blocks::parts;
use crate::config::Config;
use crate::draft::Draft;
use crate::model::Spec;
use crate::geom::pt;
use crate::pattern::{Cut, Mark};

pub struct Waist;

impl Rule for Waist {
    fn name(&self) -> &'static str {
        "waist"
    }
    fn all_variants(&self) -> &'static [&'static str] {
        &["zip_fly", "elastic"]
    }
    fn asked(&self, spec: &Spec, _cfg: &Config) -> Option<String> {
        (!spec.g().is_top()).then(|| garment_words(spec, self.name()))
    }
    fn params(&self, variant: &str, d: &mut Draft) {
        d.pants.elastic = variant == "elastic";
    }
    fn pieces(&self, variant: &str, d: &mut Draft) -> Result<(), String> {
        let (cfg, m) = (d.cfg, d.m);
        let words = garment_words(d.spec, self.name());
        let rule = format!("waist/{variant}");
        let (fw, bw) = (d.piece("front").len("waist"), d.piece("back").len("waist"));
        if variant == "elastic" {
            let girth = 2.0 * (fw + bw);
            d.clearance("hips go through the elastic waist", girth, d.fabric.stretch, m.seat, "seat");
            d.notes.push(format!(
                "cut {:.1} cm of {:.0} mm elastic for the casing",
                m.waist * cfg.band("elastic") / 10.0,
                cfg.part("elastic_width")
            ));
            return Ok(());
        }
        let half = fw - d.pants.pleat + bw - d.pants.dart;
        d.seam("waistband / body waist + ease", m.waist + d.pants.waist_ease, 2.0 * half, 0.0, 0.0);
        let band = parts::folded_band("waistband", Cut::PAIR, half + cfg.part("waistband_ext"), cfg.part("waistband_height"));
        d.add(band.by(&rule, &words));
        let (fw, fd) = (cfg.part("fly_width"), cfg.part("fly_depth"));
        let fold = Mark::Line { pts: vec![pt(fw, 0.0), pt(fw, fd)], dashed: true };
        let shield = parts::rect("fly shield", Cut::ONE, fw * 2.0, fd).mark(fold);
        d.add(shield.by(&rule, &words));
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::rules::test_util::*;

    #[test]
    fn zip_fly_has_waistband_and_fly() {
        let p = ok("make pants", &[("waist", "zip_fly")]);
        assert!(p.piece("front").has_edge("fly"));
        assert!(p.pieces.iter().any(|x| x.name == "waistband"));
    }

    #[test]
    fn elastic_waist_passes_the_hips() {
        let p = ok("make baggy pants", &[("waist", "elastic")]);
        assert!(!p.piece("front").has_edge("fly"));
        assert!(p.checks.iter().any(|c| c.name.starts_with("hips") && c.ok));
    }

    #[test]
    fn baggy_pants_get_a_front_pleat() {
        let p = ok("make baggy pants", &[("waist", "zip_fly")]);
        assert!(p.notes.iter().any(|n| n.contains("pleat")));
    }
}
