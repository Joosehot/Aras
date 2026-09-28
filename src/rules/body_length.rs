//! "lengthen/shorten/crop the body": the hem moves, the side seams follow.

use super::{edit_mm, edit_words, Rule};
use crate::config::Config;
use crate::draft::Draft;
use crate::model::{EditKind, Spec};

pub struct BodyLength;

impl Rule for BodyLength {
    fn name(&self) -> &'static str {
        "body_length"
    }
    fn all_variants(&self) -> &'static [&'static str] {
        &["at_hem"]
    }
    fn asked(&self, spec: &Spec, _cfg: &Config) -> Option<String> {
        edit_words(spec, EditKind::BodyLength)
    }
    fn params(&self, _variant: &str, d: &mut Draft) {
        d.top.length += edit_mm(d, EditKind::BodyLength);
    }
}

#[cfg(test)]
mod tests {
    use crate::rules::test_util::*;

    #[test]
    fn crop_shortens_both_sides_equally() {
        let base = ok("make a t-shirt", &[]);
        let crop = ok("crop the t-shirt", &[]);
        let d = base.piece("back").len("side") - crop.piece("back").len("side");
        assert!((d - 80.0).abs() < 1.0, "{d}");
        assert!((crop.piece("front").len("side") - crop.piece("back").len("side")).abs() < 1e-6);
    }
}
