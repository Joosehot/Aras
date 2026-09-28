//! "lower/raise the neckline": the front neck gets deeper or shallower.

use super::{edit_mm, edit_words, Rule};
use crate::config::Config;
use crate::draft::Draft;
use crate::model::{EditKind, Spec};

pub struct NeckDepth;

impl Rule for NeckDepth {
    fn name(&self) -> &'static str {
        "neck_depth"
    }
    fn all_variants(&self) -> &'static [&'static str] {
        &["front_only"]
    }
    fn asked(&self, spec: &Spec, _cfg: &Config) -> Option<String> {
        edit_words(spec, EditKind::NeckDepth)
    }
    fn params(&self, _variant: &str, d: &mut Draft) {
        d.top.front_neck_depth += edit_mm(d, EditKind::NeckDepth);
    }
}

#[cfg(test)]
mod tests {
    use crate::rules::test_util::*;

    #[test]
    fn lower_neckline_is_longer() {
        let base = ok("make a t-shirt", &[]).piece("front").len("neck");
        let low = ok("make a t-shirt and lower the neckline by 2 cm", &[]).piece("front").len("neck");
        assert!(low > base + 10.0);
    }
}
