//! How the bottom of a top is finished. T-shirts are `hemmed`; sweatshirts
//! and hoodies choose a `rib_band` (body shortened by the band) or a plain
//! hem; shirts choose a curved `shirttail` or a `straight` hem.

use super::{garment_words, Finish, Rule};
use crate::blocks::parts;
use crate::config::Config;
use crate::draft::Draft;
use crate::model::{Garment, Spec};
use crate::pattern::Cut;

pub struct BodyHem;

impl Rule for BodyHem {
    fn name(&self) -> &'static str {
        "body_hem"
    }
    fn all_variants(&self) -> &'static [&'static str] {
        &["hemmed", "rib_band", "shirttail", "straight"]
    }
    fn asked(&self, spec: &Spec, _cfg: &Config) -> Option<String> {
        spec.g().is_top().then(|| garment_words(spec, self.name()))
    }
    fn variants(&self, spec: &Spec, _cfg: &Config) -> Vec<&'static str> {
        match spec.g() {
            Garment::Tshirt => vec!["hemmed"],
            Garment::Hoodie | Garment::Sweatshirt => vec!["rib_band", "hemmed"],
            Garment::Shirt => vec!["shirttail", "straight"],
            Garment::Dress => vec!["hemmed"],
            Garment::Pants => vec![],
        }
    }
    fn finish(&self, variant: &str) -> Option<Finish> {
        match variant {
            "rib_band" => Some(Finish::Rib),
            "hemmed" => Some(Finish::Plain),
            _ => None,
        }
    }
    fn uses_rib(&self, variant: &str) -> bool {
        variant == "rib_band"
    }
    fn params(&self, variant: &str, d: &mut Draft) {
        match variant {
            "rib_band" => d.top.length -= d.cfg.part("hem_band_height"),
            "shirttail" => d.top.shirttail = d.cfg.part("shirttail_rise"),
            _ => {}
        }
    }
    fn pieces(&self, variant: &str, d: &mut Draft) -> Result<(), String> {
        if variant != "rib_band" {
            return Ok(());
        }
        let half_girth = d.piece("front").len("hem") + d.piece("back").len("hem");
        let len = half_girth * d.cfg.band("hem_rib");
        let words = garment_words(d.spec, self.name());
        let band = parts::folded_band("hem band", Cut::PAIR, len, d.cfg.part("hem_band_height")).rib().by("body_hem/rib_band", &words);
        d.clearance("hem band stretches to the hem", len, d.rib.stretch, half_girth, "half hem");
        d.add(band);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::rules::test_util::*;

    #[test]
    fn rib_band_shortens_the_body_by_its_height() {
        let hem = ok("make a hoodie", &[("body_hem", "hemmed")]);
        let rib = ok("make a hoodie", &[("body_hem", "rib_band")]);
        let d = hem.piece("back").len("cb") - rib.piece("back").len("cb");
        assert!((d - 60.0).abs() < 1e-6);
        assert!(rib.piece("hem band").rib);
    }

    #[test]
    fn shirttail_curves_the_hem() {
        let p = ok("make a shirt", &[("body_hem", "shirttail")]);
        assert!(p.piece("back").edge("hem").pts.len() > 2);
        assert!(p.piece("back").len("side") < ok("make a shirt", &[("body_hem", "straight")]).piece("back").len("side"));
    }
}
