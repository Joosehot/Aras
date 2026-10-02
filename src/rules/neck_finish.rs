//! How a neckline without a hood or collar is finished. Knit pullovers: a
//! `rib_band`, a `self_band` of the main fabric, or a narrow `binding`, each
//! cut shorter than the neckline and stretched on; the head must still get
//! through. Woven dresses: a shaped `facing` turned inside, or a bias
//! `binding` (the dress's closure rule checks the head).

use super::{garment_words, Rule};
use crate::blocks::parts;
use crate::config::Config;
use crate::draft::Draft;
use crate::model::{Garment, Spec};
use crate::pattern::Cut;

pub struct NeckFinish;

impl Rule for NeckFinish {
    fn name(&self) -> &'static str {
        "neck_finish"
    }
    fn all_variants(&self) -> &'static [&'static str] {
        &["rib_band", "self_band", "binding", "facing"]
    }
    fn asked(&self, spec: &Spec, cfg: &Config) -> Option<String> {
        ((spec.g().pulls_over_head() || spec.g() == Garment::Dress) && !spec.has_hood(cfg)).then(|| garment_words(spec, self.name()))
    }
    fn variants(&self, spec: &Spec, _cfg: &Config) -> Vec<&'static str> {
        if spec.g() == Garment::Dress {
            vec!["facing", "binding"]
        } else {
            vec!["rib_band", "self_band", "binding"]
        }
    }
    fn uses_rib(&self, variant: &str) -> bool {
        variant == "rib_band"
    }
    fn pieces(&self, variant: &str, d: &mut Draft) -> Result<(), String> {
        let cfg = d.cfg;
        let words = garment_words(d.spec, self.name());
        let dress = d.spec.g() == Garment::Dress;
        // a cowl front has its own facing: only the back neckline is finished here
        let cowl = d.top.dress.is_some_and(|dp| dp.cowl);
        if variant == "facing" {
            let w = cfg.dress("facing_width");
            if !cowl {
                let front = parts::neck_facing("front neck facing", d.piece("front"), w)?;
                d.add(front.by("neck_finish/facing", &words));
            }
            let back = parts::neck_facing("back neck facing", d.piece("back"), w)?;
            d.add(back.by("neck_finish/facing", &words));
            return Ok(());
        }
        let front_neck = if cowl { 0.0 } else { d.piece("front").len("neck") };
        let girth = 2.0 * (front_neck + d.piece("back").len("neck"));
        let (ratio, finished, rib, name) = match variant {
            "rib_band" => (cfg.band("neck_rib"), cfg.part("neckband_width"), true, "neckband"),
            "self_band" => (cfg.band("neck_self"), cfg.part("neckband_width"), false, "neckband"),
            _ => (cfg.band("binding"), cfg.part("binding_width") * 2.0, false, "neck binding"),
        };
        let len = girth * ratio;
        let stretch = if rib { d.rib.stretch } else { d.fabric.stretch };
        if !dress {
            d.clearance("head goes through the neckline", girth, d.fabric.stretch, d.m.head, "head");
            d.clearance("head goes through the neckband", len, stretch, d.m.head, "head");
        }
        // a dress's centre-back zip opens the binding too: it is cut in two
        let cut = if dress && !d.piece("back").cut.fold { Cut::PAIR } else { Cut::ONE };
        let len = if cut == Cut::PAIR { len / 2.0 } else { len };
        let mut band = parts::folded_band(name, cut, len, finished);
        if rib {
            band = band.rib();
        }
        d.add(band.by(&format!("neck_finish/{variant}"), &words));
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::rules::test_util::*;

    #[test]
    fn band_is_shorter_than_the_neckline() {
        let p = ok("make a t-shirt", &[("neck_finish", "rib_band")]);
        let girth = 2.0 * (p.piece("front").len("neck") + p.piece("back").len("neck"));
        let band = p.piece("neckband").len("top");
        assert!((band / girth - 0.8).abs() < 1e-9);
        assert!(p.piece("neckband").rib);
    }

    #[test]
    fn hood_replaces_the_neck_finish() {
        let p = ok("make a t-shirt with a hood", &[]);
        assert!(!p.pieces.iter().any(|x| x.name == "neckband"));
    }

    #[test]
    fn a_dress_neckline_gets_facings() {
        let p = ok("make a dress", &[("neck_finish", "facing"), ("closure", "back_zip")]);
        assert!(p.piece("front neck facing").cut.fold);
        assert!(!p.piece("back neck facing").cut.fold, "the zip splits the back facing");
    }
}
