//! How a sleeve ends. Short sleeves are `hemmed`. Long knit sleeves are
//! `hemmed` (the hand must fit through the stretched hem) or get a
//! `rib_cuff`. Shirt sleeves get a `barrel_cuff` with pleats and a placket,
//! or a `hemmed_wide` open hem wide enough for the hand.

use super::{garment_words, Finish, Rule};
use crate::blocks::parts;
use crate::config::Config;
use crate::draft::Draft;
use crate::geom::pt;
use crate::model::{Garment, SleeveKind, Spec};
use crate::pattern::{Cut, Mark};

pub struct SleeveFinish;

impl Rule for SleeveFinish {
    fn name(&self) -> &'static str {
        "sleeve_finish"
    }
    fn all_variants(&self) -> &'static [&'static str] {
        &["hemmed", "rib_cuff", "barrel_cuff", "hemmed_wide"]
    }
    fn asked(&self, spec: &Spec, cfg: &Config) -> Option<String> {
        (spec.g().is_top() && spec.sleeves(cfg) != SleeveKind::None).then(|| garment_words(spec, self.name()))
    }
    fn variants(&self, spec: &Spec, cfg: &Config) -> Vec<&'static str> {
        match (spec.g(), spec.sleeves(cfg)) {
            (_, SleeveKind::Short) => vec!["hemmed"],
            (Garment::Shirt, SleeveKind::Long) => vec!["barrel_cuff", "hemmed_wide"],
            (Garment::Dress, SleeveKind::Long) => vec!["hemmed_wide"],
            (_, SleeveKind::Long) => vec!["rib_cuff", "hemmed"],
            (_, SleeveKind::None) => vec![],
        }
    }
    fn finish(&self, variant: &str) -> Option<Finish> {
        match variant {
            "rib_cuff" => Some(Finish::Rib),
            "hemmed" => Some(Finish::Plain),
            _ => None,
        }
    }
    fn uses_rib(&self, variant: &str) -> bool {
        variant == "rib_cuff"
    }
    fn params(&self, variant: &str, d: &mut Draft) {
        let (cfg, m) = (d.cfg, d.m);
        match variant {
            "rib_cuff" => d.top.sleeve_len -= cfg.part("rib_cuff_height"),
            "barrel_cuff" => {
                d.top.sleeve_len -= cfg.part("barrel_cuff_height");
                d.top.sleeve_hem = m.wrist + cfg.part("barrel_cuff_ease") + cfg.part("cuff_pleats_max") / 2.0;
            }
            "hemmed_wide" => d.top.sleeve_hem = d.top.sleeve_hem.max(m.hand + cfg.part("hemmed_wide_ease")),
            _ => {}
        }
    }
    fn pieces(&self, variant: &str, d: &mut Draft) -> Result<(), String> {
        let (cfg, m) = (d.cfg, d.m);
        let hem = d.piece("sleeve").len("hem");
        let words = garment_words(d.spec, self.name());
        let rule = format!("sleeve_finish/{variant}");
        match variant {
            "hemmed" | "hemmed_wide" => {
                d.clearance("hand goes through the sleeve hem", hem, d.fabric.stretch, m.hand, "hand");
            }
            "rib_cuff" => {
                let len = hem * cfg.band("cuff_rib");
                d.clearance("hand goes through the cuff", len, d.rib.stretch, m.hand, "hand");
                d.clearance("cuff stretches to the sleeve hem", len, d.rib.stretch, hem, "sleeve hem");
                d.add(parts::folded_band("cuff", Cut::PAIR, len, cfg.part("rib_cuff_height")).rib().by(&rule, &words));
            }
            "barrel_cuff" => {
                let len = m.wrist + cfg.part("barrel_cuff_ease");
                d.seam("sleeve hem / cuff (pleats take the difference)", len, hem, 0.0, cfg.part("cuff_pleats_max"));
                let cuff = parts::folded_band("cuff", Cut::PAIR, len + cfg.part("cuff_overlap"), cfg.part("barrel_cuff_height"));
                d.add(cuff.by(&rule, &words));
                let pl = cfg.part("placket_length");
                let strip = parts::rect("sleeve placket", Cut::PAIR, pl * 2.0, cfg.part("placket_strip_width"));
                d.add(strip.by(&rule, &words));
                // Placket slit halfway across the back half of the hem.
                let s = d.piece("sleeve");
                let h = s.edge("hem").pts.clone();
                let (hf, hb) = (h[0], h[h.len() - 1]);
                let at = hb.lerp(hf, 0.25);
                let slit = Mark::Line { pts: vec![at, at + pt(0.0, -pl)], dashed: false };
                d.piece_mut("sleeve").marks.push(slit);
            }
            _ => {}
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::rules::test_util::*;

    #[test]
    fn rib_cuff_shortens_the_sleeve_and_fits_the_hand() {
        let hem = ok("make a hoodie", &[("sleeve_finish", "hemmed")]);
        let rib = ok("make a hoodie", &[("sleeve_finish", "rib_cuff")]);
        let (a, b) = (hem.piece("sleeve"), rib.piece("sleeve"));
        assert!(a.len("underarm_front") > b.len("underarm_front") + 50.0);
        assert!(rib.piece("cuff").rib);
    }

    #[test]
    fn barrel_cuff_pleats_are_within_range() {
        let p = ok("make a shirt", &[("sleeve_finish", "barrel_cuff")]);
        assert!(p.checks.iter().any(|c| c.name.contains("pleats") && c.ok));
        assert!(p.pieces.iter().any(|x| x.name == "sleeve placket"));
    }

    #[test]
    fn only_hemmed_for_short_sleeves() {
        let cfg = crate::config::Config::builtin();
        let r = crate::rules::by_name("sleeve_finish");
        assert_eq!(r.variants(&spec("make a t-shirt"), &cfg), vec!["hemmed"]);
        assert!(r.variants(&spec("make a shirt"), &cfg).contains(&"barrel_cuff"));
    }
}
