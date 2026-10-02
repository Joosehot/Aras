//! A bow at centre back: two loops from one folded strip, a knot band
//! around their middle and two tails. It sits at the bottom of an open back,
//! or at the waist of a closed one (marked on the back piece).

use super::{garment_words, Rule};
use crate::blocks::parts;
use crate::config::Config;
use crate::draft::Draft;
use crate::geom::pt;
use crate::model::{Garment, Spec};
use crate::pattern::{Cut, Mark};

pub struct Bow;

impl Rule for Bow {
    fn name(&self) -> &'static str {
        "bow"
    }
    fn all_variants(&self) -> &'static [&'static str] {
        &["at_back"]
    }
    fn asked(&self, spec: &Spec, _cfg: &Config) -> Option<String> {
        (spec.g() == Garment::Dress && spec.pin(self.name()).is_some()).then(|| garment_words(spec, self.name()))
    }
    fn pieces(&self, _variant: &str, d: &mut Draft) -> Result<(), String> {
        let c = d.cfg;
        let words = garment_words(d.spec, self.name());
        let (w, h) = (c.dress("bow_width"), c.dress("bow_height"));
        let dp = d.top.dress.expect("a dress");
        let at = dp.open_back.unwrap_or(dp.waist_y);
        d.add(parts::folded_band("bow", Cut::ONE, 4.0 * w + 20.0, h).by("bow/at_back", &words));
        d.add(parts::folded_band("bow knot", Cut::ONE, c.dress("bow_knot"), h * 0.4).by("bow/at_back", &words));
        d.add(parts::folded_band("bow tail", Cut::PAIR, c.dress("bow_tail"), h * 0.6).by("bow/at_back", &words));
        // where it is sewn on: a short line at centre back
        d.piece_mut("back").marks.push(Mark::Line { pts: vec![pt(0.0, at), pt(30.0, at)], dashed: false });
        Ok(())
    }
}
