//! The rule system. Every construction decision is one rule in one file:
//!
//! - it says whether the garment needs the decision (`asked`) and which
//!   words asked for it
//! - it offers the variants that fit this garment (`variants`)
//! - for the chosen variant it adjusts the numbers before the blocks are
//!   drafted (`params`) and adds or reshapes pieces afterwards (`pieces`)
//!
//! Each variant's finish/simplicity/economy lives in `rules.toml`. Judges
//! (`judges.rs`) score combinations across the whole garment.

use crate::config::Config;
use crate::draft::Draft;
use crate::model::{EditKind, Spec};

pub mod armhole_finish;
pub mod back;
pub mod body_hem;
pub mod bow;
pub mod neckline;
pub mod closure;
pub mod skirt;
pub mod body_length;
pub mod chest_width;
pub mod collar;
pub mod hood;
pub mod hood_lining;
pub mod judges;
pub mod leg_length;
pub mod leg_width;
pub mod neck_depth;
pub mod neck_finish;
pub mod neck_width;
pub mod pocket;
pub mod rise;
pub mod seat_width;
pub mod sleeve_finish;
pub mod sleeve_length;
pub mod sleeve_width;
pub mod waist;

/// How an edge is finished, for the consistent-finish judge.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Finish {
    Rib,
    Plain,
}

pub trait Rule: Sync {
    fn name(&self) -> &'static str;
    fn all_variants(&self) -> &'static [&'static str];
    /// The words that ask for this decision, or `None` when the garment
    /// doesn't need it.
    fn asked(&self, spec: &Spec, cfg: &Config) -> Option<String>;
    /// Variants that fit this garment (pins in the sentence narrow them further).
    fn variants(&self, _spec: &Spec, _cfg: &Config) -> Vec<&'static str> {
        self.all_variants().to_vec()
    }
    fn finish(&self, _variant: &str) -> Option<Finish> {
        None
    }
    fn uses_rib(&self, _variant: &str) -> bool {
        false
    }
    /// Adjust the numbers before the blocks are drafted.
    fn params(&self, _variant: &str, _d: &mut Draft) {}
    /// Add or reshape pieces after the blocks are drafted.
    fn pieces(&self, _variant: &str, _d: &mut Draft) -> Result<(), String> {
        Ok(())
    }
}

/// Every rule, in the order it is applied.
pub fn registry() -> Vec<&'static dyn Rule> {
    vec![
        &sleeve_length::SleeveLength,
        &sleeve_width::SleeveWidth,
        &neck_width::NeckWidth,
        &neck_depth::NeckDepth,
        &body_length::BodyLength,
        &chest_width::ChestWidth,
        &seat_width::SeatWidth,
        &leg_length::LegLength,
        &leg_width::LegWidth,
        &rise::Rise,
        &body_hem::BodyHem,
        &sleeve_finish::SleeveFinish,
        &waist::Waist,
        &skirt::Skirt,
        &neckline::Neckline,
        &back::Back,
        &closure::Closure,
        &bow::Bow,
        &armhole_finish::ArmholeFinish,
        &neck_finish::NeckFinish,
        &collar::Collar,
        &hood::Hood,
        &hood_lining::HoodLining,
        &pocket::Pocket,
    ]
}

pub fn by_name(name: &str) -> &'static dyn Rule {
    registry().into_iter().find(|r| r.name() == name).unwrap_or_else(|| panic!("no rule {name}"))
}

/// Words and amount of an edit, for the edit rules.
pub fn edit_words(spec: &Spec, kind: EditKind) -> Option<String> {
    let valid = spec.g().is_top() == kind.for_tops();
    spec.edit(kind).filter(|_| valid).map(|e| e.words)
}

pub fn edit_mm(d: &Draft, kind: EditKind) -> f64 {
    d.spec.edit(kind).map_or(0.0, |e| e.mm)
}

/// Words for a garment-level decision: pinned words if any, else the garment.
pub fn garment_words(spec: &Spec, rule: &str) -> String {
    spec.pin(rule).map_or_else(|| spec.garment.words.clone(), |p| p.words.clone())
}

#[cfg(test)]
pub mod test_util {
    use crate::config::Config;
    use crate::draft::{self, Choices};
    use crate::lexicon::tokenize;
    use crate::model::Spec;
    use crate::parser::{parse, ParseOptions};
    use crate::pattern::Pattern;

    pub fn spec(s: &str) -> Spec {
        parse(s, tokenize(s), &Config::builtin(), &ParseOptions::default()).unwrap_or_else(|d| panic!("{s}: {d:?}"))
    }

    /// Draft with the given choices; every other asked rule takes its first variant.
    pub fn pattern(s: &str, pick: &[(&'static str, &'static str)]) -> Result<Pattern, String> {
        let cfg = Config::builtin();
        let sp = spec(s);
        let mut choices = Choices::new();
        for r in super::registry() {
            if r.asked(&sp, &cfg).is_some() {
                let mut vs = r.variants(&sp, &cfg);
                if let Some(p) = sp.pin(r.name()) {
                    vs.retain(|v| p.variants.contains(v));
                }
                choices.insert(r.name(), vs[0]);
            }
        }
        for (k, v) in pick {
            choices.insert(k, v);
        }
        draft::draft(&sp, &cfg, &choices)
    }

    pub fn ok(s: &str, pick: &[(&'static str, &'static str)]) -> Pattern {
        let p = pattern(s, pick).unwrap_or_else(|e| panic!("{s}: {e}"));
        let bad: Vec<_> = p.checks.iter().filter(|c| !c.ok).map(|c| format!("{}: {}", c.name, c.detail)).collect();
        assert!(bad.is_empty(), "{s}: failed checks\n{}", bad.join("\n"));
        p
    }

    pub fn failed(p: &Pattern) -> Vec<String> {
        p.checks.iter().filter(|c| !c.ok).map(|c| c.name.clone()).collect()
    }
}
