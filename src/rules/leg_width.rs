//! "widen/narrow the legs" (amount = change in hem girth). `straight` moves
//! knee and hem together; `flare` changes only the hem.

use super::{edit_mm, edit_words, Rule};
use crate::config::Config;
use crate::draft::Draft;
use crate::model::{EditKind, Spec};

pub struct LegWidth;

impl Rule for LegWidth {
    fn name(&self) -> &'static str {
        "leg_width"
    }
    fn all_variants(&self) -> &'static [&'static str] {
        &["straight", "flare"]
    }
    fn asked(&self, spec: &Spec, _cfg: &Config) -> Option<String> {
        edit_words(spec, EditKind::LegWidth)
    }
    fn params(&self, variant: &str, d: &mut Draft) {
        let mm = edit_mm(d, EditKind::LegWidth);
        d.pants.hem += mm;
        if variant == "straight" {
            d.pants.knee += mm;
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::rules::test_util::*;

    #[test]
    fn tapering_tight_pants_too_far_blocks_the_foot() {
        let p = pattern("make tight pants and taper the legs by 4 inches", &[]).unwrap();
        assert!(failed(&p).iter().any(|c| c.starts_with("foot")), "{:?}", p.checks);
    }
}
