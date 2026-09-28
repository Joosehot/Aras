//! `single` hoods have a turned casing at the face; `lined` hoods are cut
//! twice, sewn together at the face, and need no casing.

use super::Rule;
use crate::config::Config;
use crate::draft::Draft;
use crate::model::Spec;

pub struct HoodLining;

impl Rule for HoodLining {
    fn name(&self) -> &'static str {
        "hood_lining"
    }
    fn all_variants(&self) -> &'static [&'static str] {
        &["single", "lined"]
    }
    fn asked(&self, spec: &Spec, cfg: &Config) -> Option<String> {
        spec.has_hood(cfg).then(|| {
            spec.pin(self.name()).map(|p| p.words.clone()).unwrap_or_else(|| spec.feature_words(&spec.hood))
        })
    }
    fn pieces(&self, variant: &str, d: &mut Draft) -> Result<(), String> {
        if variant != "lined" {
            return Ok(());
        }
        let words = d.spec.pin(self.name()).map(|p| p.words.clone()).unwrap_or_else(|| d.spec.feature_words(&d.spec.hood));
        let linings: Vec<_> = d
            .pieces
            .iter()
            .filter(|p| p.name.starts_with("hood "))
            .map(|p| {
                let mut l = p.clone();
                l.name = p.name.replace("hood ", "hood lining ");
                l.by("hood_lining/lined", &words)
            })
            .collect();
        for l in linings {
            d.add(l);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::pattern::EdgeKind;
    use crate::rules::test_util::*;

    #[test]
    fn lined_hood_doubles_the_pieces_and_drops_the_casing() {
        let p = ok("make a hoodie with a lined hood", &[("hood", "three_piece")]);
        assert!(p.pieces.iter().any(|x| x.name == "hood lining side"));
        assert!(p.pieces.iter().any(|x| x.name == "hood lining centre"));
        assert_eq!(p.piece("hood side").edge("face").kind, EdgeKind::Seam);
    }

    #[test]
    fn single_hood_has_a_casing() {
        let p = ok("make a hoodie", &[("hood_lining", "single")]);
        assert_eq!(p.piece("hood side").edge("face").kind, EdgeKind::Hem);
    }
}
