//! "lengthen/shorten the legs". `extend_hem` continues the knee-to-hem line
//! (a tapered leg gets narrower); `keep_hem_width` keeps the hem girth.

use super::{edit_mm, edit_words, Rule};
use crate::config::Config;
use crate::draft::Draft;
use crate::model::{EditKind, Spec};

pub struct LegLength;

impl Rule for LegLength {
    fn name(&self) -> &'static str {
        "leg_length"
    }
    fn all_variants(&self) -> &'static [&'static str] {
        &["extend_hem", "keep_hem_width"]
    }
    fn asked(&self, spec: &Spec, _cfg: &Config) -> Option<String> {
        edit_words(spec, EditKind::LegLength)
    }
    fn params(&self, variant: &str, d: &mut Draft) {
        let mm = edit_mm(d, EditKind::LegLength);
        match variant {
            "extend_hem" => d.pants.leg_extend += mm,
            _ => d.pants.inseam += mm,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::rules::test_util::*;

    #[test]
    fn both_variants_lengthen_the_inseam() {
        let s = "make pants and lengthen the legs by 2 inches";
        let base = ok("make pants", &[]).piece("front").len("inseam");
        for v in ["extend_hem", "keep_hem_width"] {
            let l = ok(s, &[("leg_length", v)]).piece("front").len("inseam");
            assert!((l - base - 50.8).abs() < 3.0, "{v}: {l} vs {base}");
        }
    }
}
