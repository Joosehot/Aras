//! "lengthen/shorten the sleeves". `extend_hem` continues the sleeve's taper
//! past the old hem (the wrist gets narrower as it gets longer);
//! `keep_hem_width` spreads the sleeve evenly and keeps the hem girth.

use super::{edit_mm, edit_words, Rule};
use crate::config::Config;
use crate::draft::Draft;
use crate::model::{EditKind, Spec};

pub struct SleeveLength;

impl Rule for SleeveLength {
    fn name(&self) -> &'static str {
        "sleeve_length"
    }
    fn all_variants(&self) -> &'static [&'static str] {
        &["extend_hem", "keep_hem_width"]
    }
    fn asked(&self, spec: &Spec, _cfg: &Config) -> Option<String> {
        edit_words(spec, EditKind::SleeveLength)
    }
    fn params(&self, variant: &str, d: &mut Draft) {
        let mm = edit_mm(d, EditKind::SleeveLength);
        match variant {
            "extend_hem" => d.top.sleeve_extend += mm,
            _ => d.top.sleeve_len += mm,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::rules::test_util::*;

    const S: &str = "make a t-shirt with long sleeves and lengthen the sleeves by 3 inches";

    #[test]
    fn extending_the_taper_narrows_the_hem() {
        let base = ok("make a t-shirt with long sleeves", &[]).piece("sleeve").len("hem");
        let ext = ok(S, &[("sleeve_length", "extend_hem")]).piece("sleeve").len("hem");
        let keep = ok(S, &[("sleeve_length", "keep_hem_width")]).piece("sleeve").len("hem");
        assert!(ext < base - 5.0, "{ext} vs {base}");
        assert!((keep - base).abs() < 1e-6);
    }

    #[test]
    fn not_asked_without_an_edit() {
        let sp = spec("make a t-shirt");
        assert!(crate::rules::by_name("sleeve_length").asked(&sp, &crate::config::Config::builtin()).is_none());
    }
}
