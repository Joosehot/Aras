//! "raise/lower the waist": the body rise gets longer or shorter.

use super::{edit_mm, edit_words, Rule};
use crate::config::Config;
use crate::draft::Draft;
use crate::model::{EditKind, Spec};

pub struct Rise;

impl Rule for Rise {
    fn name(&self) -> &'static str {
        "rise"
    }
    fn all_variants(&self) -> &'static [&'static str] {
        &["at_waist"]
    }
    fn asked(&self, spec: &Spec, _cfg: &Config) -> Option<String> {
        edit_words(spec, EditKind::Rise)
    }
    fn params(&self, _variant: &str, d: &mut Draft) {
        d.pants.rise_ease += edit_mm(d, EditKind::Rise);
    }
}

#[cfg(test)]
mod tests {
    use crate::rules::test_util::*;

    #[test]
    fn higher_waist_lengthens_the_crotch_seam() {
        let base = ok("make pants", &[]).piece("front").len("crotch");
        let high = ok("make pants and raise the waist by 3 cm", &[]).piece("front").len("crotch");
        assert!(high > base + 20.0);
    }
}
