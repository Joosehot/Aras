//! "widen/narrow the sleeves" (amount = change in girth). `parallel` adds
//! the same at bicep and hem; `bicep_only` keeps the hem and changes the taper.

use super::{edit_mm, edit_words, Rule};
use crate::config::Config;
use crate::draft::Draft;
use crate::model::{EditKind, Spec};

pub struct SleeveWidth;

impl Rule for SleeveWidth {
    fn name(&self) -> &'static str {
        "sleeve_width"
    }
    fn all_variants(&self) -> &'static [&'static str] {
        &["parallel", "bicep_only"]
    }
    fn asked(&self, spec: &Spec, _cfg: &Config) -> Option<String> {
        edit_words(spec, EditKind::SleeveWidth)
    }
    fn params(&self, variant: &str, d: &mut Draft) {
        let mm = edit_mm(d, EditKind::SleeveWidth);
        d.top.bicep += mm;
        if variant == "parallel" {
            d.top.sleeve_hem += mm;
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::rules::test_util::*;

    #[test]
    fn parallel_widens_the_hem_bicep_only_keeps_it() {
        let s = "make a t-shirt and widen the sleeves by 4 cm";
        let base = ok("make a t-shirt", &[]).piece("sleeve").len("hem");
        let wide = ok(s, &[("sleeve_width", "parallel")]).piece("sleeve").len("hem");
        let bicep = ok(s, &[("sleeve_width", "bicep_only")]).piece("sleeve").len("hem");
        assert!((wide - base - 40.0).abs() < 1e-6);
        assert!((bicep - base).abs() < 1e-6);
    }
}
