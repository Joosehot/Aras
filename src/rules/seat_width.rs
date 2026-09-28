//! "widen/narrow the hips" on pants (amount = change in seat girth).

use super::{edit_mm, edit_words, Rule};
use crate::config::Config;
use crate::draft::Draft;
use crate::model::{EditKind, Spec};

pub struct SeatWidth;

impl Rule for SeatWidth {
    fn name(&self) -> &'static str {
        "seat_width"
    }
    fn all_variants(&self) -> &'static [&'static str] {
        &["at_side"]
    }
    fn asked(&self, spec: &Spec, _cfg: &Config) -> Option<String> {
        edit_words(spec, EditKind::SeatWidth)
    }
    fn params(&self, _variant: &str, d: &mut Draft) {
        d.pants.seat_ease += edit_mm(d, EditKind::SeatWidth);
    }
}

#[cfg(test)]
mod tests {
    use crate::rules::test_util::*;

    #[test]
    fn tighter_seat_fails_without_stretch() {
        let p = pattern("make pants and narrow the hips by 10 cm", &[]).unwrap();
        assert!(failed(&p).contains(&"seat ease".to_string()), "{:?}", p.checks);
    }
}
