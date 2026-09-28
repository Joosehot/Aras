//! "widen/narrow the body" (amount = change in chest girth): a quarter goes
//! to each side seam.

use super::{edit_mm, edit_words, Rule};
use crate::config::Config;
use crate::draft::Draft;
use crate::model::{EditKind, Spec};

pub struct ChestWidth;

impl Rule for ChestWidth {
    fn name(&self) -> &'static str {
        "chest_width"
    }
    fn all_variants(&self) -> &'static [&'static str] {
        &["at_side"]
    }
    fn asked(&self, spec: &Spec, _cfg: &Config) -> Option<String> {
        edit_words(spec, EditKind::ChestWidth)
    }
    fn params(&self, _variant: &str, d: &mut Draft) {
        d.top.chest_ease += edit_mm(d, EditKind::ChestWidth);
    }
}

#[cfg(test)]
mod tests {
    use crate::rules::test_util::*;

    #[test]
    fn wider_body_has_a_longer_hem() {
        let pick = [("body_hem", "hemmed")];
        let base = ok("make a sweatshirt", &pick).piece("front").len("hem");
        let wide = ok("make a sweatshirt and widen the body by 8 cm", &pick).piece("front").len("hem");
        assert!((wide - base - 20.0).abs() < 1e-6, "{wide} {base}");
    }
}
