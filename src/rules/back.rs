//! A dress's back: `closed`, or `open` to `[dress] open_back` above the
//! waist. An open back must leave a strap between it and the armhole at
//! least `[dress] strap_min` wide, checked along the whole shoulder.

use super::{garment_words, Rule};
use crate::config::Config;
use crate::draft::Draft;
use crate::geom::Pt;
use crate::model::{Garment, Spec};

pub struct Back;

impl Rule for Back {
    fn name(&self) -> &'static str {
        "back"
    }
    fn all_variants(&self) -> &'static [&'static str] {
        &["closed", "open"]
    }
    fn asked(&self, spec: &Spec, _cfg: &Config) -> Option<String> {
        (spec.g() == Garment::Dress).then(|| garment_words(spec, self.name()))
    }
    /// Open only when the sentence asks for it.
    fn variants(&self, spec: &Spec, _cfg: &Config) -> Vec<&'static str> {
        if spec.pin(self.name()).is_some() {
            vec!["closed", "open"]
        } else {
            vec!["closed"]
        }
    }
    fn params(&self, variant: &str, d: &mut Draft) {
        let drop = d.cfg.dress("open_back");
        if let Some(dp) = d.top.dress.as_mut() {
            dp.open_back = (variant == "open").then_some(dp.waist_y - drop);
        }
    }
    fn pieces(&self, variant: &str, d: &mut Draft) -> Result<(), String> {
        if variant != "open" {
            return Ok(());
        }
        let back = d.piece("back");
        let neck = back.edge("neck").pts.clone();
        let arm = back.edge("armhole").pts.clone();
        // the strap: across from each point of the back neckline to the armhole
        let x_at = |pts: &[Pt], y: f64| -> Option<f64> {
            pts.windows(2).find_map(|w| {
                let (a, b) = (w[0], w[1]);
                let (lo, hi) = (a.y.min(b.y), a.y.max(b.y));
                (y >= lo && y <= hi && hi > lo).then(|| a.x + (b.x - a.x) * (y - a.y) / (b.y - a.y))
            })
        };
        let strap = neck.iter().filter_map(|p| x_at(&arm, p.y).map(|ax| ax - p.x)).fold(f64::INFINITY, f64::min);
        let min = d.cfg.dress("strap_min");
        d.check("strap beside the open back", strap >= min, format!("narrowest {:.1} cm (at least {:.1} cm)", strap / 10.0, min / 10.0));
        let _ = garment_words(d.spec, self.name());
        Ok(())
    }
}
