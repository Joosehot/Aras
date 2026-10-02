//! How a sleeveless armhole is finished: a shaped `facing` (front and back,
//! cut to the armhole's curve) or a bias `binding` stretched a little shorter
//! than the armhole.

use super::{garment_words, Rule};
use crate::blocks::parts;
use crate::config::Config;
use crate::draft::Draft;
use crate::model::{SleeveKind, Spec};
use crate::pattern::Cut;

pub struct ArmholeFinish;

impl Rule for ArmholeFinish {
    fn name(&self) -> &'static str {
        "armhole_finish"
    }
    fn all_variants(&self) -> &'static [&'static str] {
        &["facing", "binding"]
    }
    fn asked(&self, spec: &Spec, cfg: &Config) -> Option<String> {
        (spec.g().is_top() && spec.sleeves(cfg) == SleeveKind::None).then(|| spec.sleeves.as_ref().map_or_else(|| spec.garment.words.clone(), |s| s.words.clone()))
    }
    fn pieces(&self, variant: &str, d: &mut Draft) -> Result<(), String> {
        let cfg = d.cfg;
        let words = garment_words(d.spec, self.name());
        let rule = format!("armhole_finish/{variant}");
        if variant == "binding" {
            let girth = d.piece("front").len("armhole") + d.piece("back").len("armhole");
            let band = parts::folded_band("armhole binding", Cut::PAIR, girth * cfg.band("binding"), cfg.part("binding_width") * 2.0);
            d.add(band.by(&rule, &words));
            return Ok(());
        }
        let w = cfg.dress("facing_width");
        for (body, name) in [("front", "front armhole facing"), ("back", "back armhole facing")] {
            let p = d.piece(body);
            let facing = parts::armhole_facing(name, p, w)?;
            d.add(facing.by(&rule, &words));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::rules::test_util::*;

    #[test]
    fn sleeveless_dress_has_no_sleeve_and_finished_armholes() {
        let p = ok("make a sleeveless dress", &[("armhole_finish", "facing"), ("closure", "back_zip")]);
        assert!(!p.pieces.iter().any(|x| x.name == "sleeve"));
        assert!(p.pieces.iter().any(|x| x.name == "front armhole facing"));
        let b = ok("make a sleeveless dress", &[("armhole_finish", "binding"), ("closure", "back_zip")]);
        assert!(b.pieces.iter().any(|x| x.name == "armhole binding"));
    }
}
