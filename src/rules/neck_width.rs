//! "widen/narrow the collar" (amount = across the opening). The neck point
//! moves along the shoulder, so the shoulder seam gets shorter by half the
//! amount and the shoulder point stays where it was.

use super::{edit_mm, edit_words, Rule};
use crate::config::Config;
use crate::draft::Draft;
use crate::model::{EditKind, Spec};

pub struct NeckWidth;

impl Rule for NeckWidth {
    fn name(&self) -> &'static str {
        "neck_width"
    }
    fn all_variants(&self) -> &'static [&'static str] {
        &["at_shoulder"]
    }
    fn asked(&self, spec: &Spec, _cfg: &Config) -> Option<String> {
        edit_words(spec, EditKind::NeckWidth)
    }
    fn params(&self, _variant: &str, d: &mut Draft) {
        let half = edit_mm(d, EditKind::NeckWidth) / 2.0;
        d.top.nw += half;
        d.top.shoulder_len -= half;
    }
}

#[cfg(test)]
mod tests {
    use crate::rules::test_util::*;

    #[test]
    fn wider_collar_means_longer_neckband_and_shorter_shoulder() {
        let base = ok("make a t-shirt", &[]);
        let wide = ok("make a t-shirt and widen the collar by 1 inch", &[]);
        assert!(wide.piece("neckband").len("top") > base.piece("neckband").len("top") + 10.0);
        assert!(wide.piece("front").len("shoulder") < base.piece("front").len("shoulder"));
    }

    #[test]
    fn too_wide_fails_the_shoulder_check() {
        // Either the shoulder check fails or the shoulder can't be drafted at all.
        if let Ok(p) = pattern("make a t-shirt and widen the collar by 10 inches", &[]) {
            assert!(failed(&p).contains(&"shoulder length".to_string()), "{:?}", p.checks);
        }
    }

    #[test]
    fn too_narrow_fails_the_head_check() {
        let p = pattern("make a t-shirt and narrow the collar by 4 inches", &[("neck_finish", "self_band")]).unwrap();
        assert!(failed(&p).iter().any(|c| c.starts_with("head")), "{:?}", p.checks);
    }
}
