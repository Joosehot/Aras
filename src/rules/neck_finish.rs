//! How a pullover neckline without a hood is finished: a `rib_band`, a
//! `self_band` of the main fabric, or a narrow `binding`. Each is cut shorter
//! than the neckline and stretched on; the head must still get through.

use super::{garment_words, Rule};
use crate::blocks::parts;
use crate::config::Config;
use crate::draft::Draft;
use crate::model::Spec;
use crate::pattern::Cut;

pub struct NeckFinish;

impl Rule for NeckFinish {
    fn name(&self) -> &'static str {
        "neck_finish"
    }
    fn all_variants(&self) -> &'static [&'static str] {
        &["rib_band", "self_band", "binding"]
    }
    fn asked(&self, spec: &Spec, cfg: &Config) -> Option<String> {
        (spec.g().pulls_over_head() && !spec.has_hood(cfg)).then(|| garment_words(spec, self.name()))
    }
    fn uses_rib(&self, variant: &str) -> bool {
        variant == "rib_band"
    }
    fn pieces(&self, variant: &str, d: &mut Draft) -> Result<(), String> {
        let cfg = d.cfg;
        let girth = 2.0 * (d.piece("front").len("neck") + d.piece("back").len("neck"));
        let (ratio, finished, rib, name) = match variant {
            "rib_band" => (cfg.band("neck_rib"), cfg.part("neckband_width"), true, "neckband"),
            "self_band" => (cfg.band("neck_self"), cfg.part("neckband_width"), false, "neckband"),
            _ => (cfg.band("binding"), cfg.part("binding_width") * 2.0, false, "neck binding"),
        };
        let len = girth * ratio;
        let stretch = if rib { d.rib.stretch } else { d.fabric.stretch };
        d.clearance("head goes through the neckline", girth, d.fabric.stretch, d.m.head, "head");
        d.clearance("head goes through the neckband", len, stretch, d.m.head, "head");
        let mut band = parts::folded_band(name, Cut::ONE, len, finished);
        if rib {
            band = band.rib();
        }
        let words = garment_words(d.spec, self.name());
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
}
