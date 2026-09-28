//! Beam search over construction decisions, one rule at a time.
//!
//! Each asked rule is a slot; its variants are scored `weight * (axes .
//! profile)`, where the profile comes from `rules.toml` and the sentence's
//! modifiers. Judges see the whole set of choices so far (rib at the cuffs
//! is worth more when the hem is ribbed too), which is why this is a beam
//! and not a greedy pick. The finalists are then drafted for real: a
//! candidate that fails a construction check is dropped, and the rest pay
//! for the fabric their cutting layout needs. Ties break on candidate
//! order, so the result is deterministic.

use crate::config::{Axes, Config};
use crate::draft::{self, Choices};
use crate::layout::{self, Marker};
use crate::model::Spec;
use crate::parser::Diag;
use crate::pattern::Pattern;
use crate::rules::{self, judges};
use std::cmp::Ordering;

#[derive(Clone, Debug)]
pub struct Slot {
    pub rule: &'static str,
    pub words: String,
    /// (variant, local score), in rule order.
    pub options: Vec<(&'static str, f64)>,
}

#[derive(Clone, Debug)]
struct State {
    choices: Choices,
    local: f64,
}

#[derive(Clone, Debug)]
pub struct Finalist {
    pub choices: Choices,
    pub local: f64,
    pub judges: Vec<(&'static str, f64, String)>,
    /// Fabric area and its (negative) score, when the pattern drafted cleanly.
    pub fabric: Option<(f64, f64)>,
    pub total: f64,
    /// Why it was rejected.
    pub rejected: Option<String>,
}

pub struct Outcome {
    pub profile: Axes,
    pub slots: Vec<Slot>,
    pub finalists: Vec<Finalist>,
    /// Index of the winner in `finalists`.
    pub best: usize,
    pub exhaustive: bool,
    pub pattern: Pattern,
    pub marker: Marker,
}

pub fn profile(spec: &Spec, cfg: &Config) -> Axes {
    spec.mods.iter().fold(cfg.profile, |p, m| p.add(&cfg.modifiers[m.key()]))
}

/// The decisions this garment needs, with the variants the sentence allows.
pub fn slots(spec: &Spec, cfg: &Config, prof: &Axes) -> Result<Vec<Slot>, Vec<Diag>> {
    let mut out = Vec::new();
    let mut diags = Vec::new();
    for r in rules::registry() {
        let Some(words) = r.asked(spec, cfg) else {
            if let Some(p) = spec.pin(r.name()) {
                diags.push(Diag::new(format!("\"{}\" doesn't apply to this {}", p.words, spec.g().name())));
            }
            continue;
        };
        let mut variants = r.variants(spec, cfg);
        if let Some(p) = spec.pin(r.name()) {
            let fits: Vec<_> = variants.iter().copied().filter(|v| p.variants.contains(v)).collect();
            if fits.is_empty() {
                diags.push(Diag::new(format!("\"{}\" doesn't fit a {}", p.words, spec.g().name())).hint(format!("{} options here: {}", r.name(), variants.join(", "))));
                continue;
            }
            variants = fits;
        }
        let options = variants.into_iter().map(|v| (v, cfg.variant_score(r.name(), v, prof))).collect();
        out.push(Slot { rule: r.name(), words, options });
    }
    if diags.is_empty() {
        Ok(out)
    } else {
        Err(diags)
    }
}

fn rank(states: &mut [State], prof: &Axes, cfg: &Config) {
    let key = |s: &State| s.local + judges::total(&s.choices, prof, cfg);
    states.sort_by(|a, b| key(b).partial_cmp(&key(a)).unwrap_or(Ordering::Equal));
}

fn expand(states: Vec<State>, slot: &Slot) -> Vec<State> {
    let mut out = Vec::new();
    for s in states {
        for (v, score) in &slot.options {
            let mut choices = s.choices.clone();
            choices.insert(slot.rule, v);
            out.push(State { choices, local: s.local + score });
        }
    }
    out
}

fn finalize(spec: &Spec, cfg: &Config, prof: &Axes, s: &State) -> (Finalist, Option<(Pattern, Marker)>) {
    let js = judges::judge(&s.choices, prof, cfg);
    let base = s.local + js.iter().map(|(_, v, _)| v).sum::<f64>();
    let mut f = Finalist { choices: s.choices.clone(), local: s.local, judges: js, fabric: None, total: base, rejected: None };
    match draft::draft(spec, cfg, &s.choices) {
        Err(e) => {
            f.rejected = Some(e);
            (f, None)
        }
        Ok(p) => {
            if let Some(c) = p.checks.iter().find(|c| !c.ok) {
                f.rejected = Some(format!("{}: {}", c.name, c.detail));
                return (f, None);
            }
            let m = layout::marker(&p, cfg);
            let area = m.area_m2();
            let cost = -cfg.judge_penalty("fabric_use", area, prof);
            f.fabric = Some((area, cost));
            f.total = base + cost;
            (f, Some((p, m)))
        }
    }
}

pub fn search(spec: &Spec, cfg: &Config) -> Result<Outcome, Vec<Diag>> {
    let prof = profile(spec, cfg);
    let slots = slots(spec, cfg, &prof)?;
    let mut beam = vec![State { choices: Choices::new(), local: 0.0 }];
    let mut all = beam.clone();
    for slot in &slots {
        beam = expand(beam, slot);
        rank(&mut beam, &prof, cfg);
        beam.truncate(cfg.search.beam);
        all = expand(all, slot);
    }
    let mut exhaustive = false;
    let mut finalists = Vec::new();
    let mut best: Option<(usize, Pattern, Marker)> = None;
    for pass in [beam, all] {
        for s in &pass {
            if finalists.iter().any(|f: &Finalist| f.choices == s.choices) {
                continue;
            }
            let (f, drafted) = finalize(spec, cfg, &prof, s);
            if let Some((p, m)) = drafted {
                let better = best.as_ref().is_none_or(|(i, _, _)| f.total > finalists[*i].total);
                if better {
                    best = Some((finalists.len(), p, m));
                }
            }
            finalists.push(f);
        }
        if best.is_some() {
            break;
        }
        // Nothing in the beam drafted cleanly: try every combination.
        exhaustive = true;
    }
    let Some((best, pattern, marker)) = best else {
        let mut reasons: Vec<String> = finalists.iter().filter_map(|f| f.rejected.clone()).collect();
        reasons.sort();
        reasons.dedup();
        let mut d = Diag::new(format!("no way to build this {} passes the construction checks", spec.g().name()));
        d = d.hint(reasons.into_iter().take(4).collect::<Vec<_>>().join("\n        "));
        return Err(vec![d]);
    };
    Ok(Outcome { profile: prof, slots, finalists, best, exhaustive, pattern, marker })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::test_util::spec;

    fn winner(s: &str) -> Choices {
        let cfg = Config::builtin();
        let o = search(&spec(s), &cfg).unwrap_or_else(|d| panic!("{s}: {d:?}"));
        o.finalists[o.best].choices.clone()
    }

    #[test]
    fn simply_prefers_simple_construction() {
        assert_eq!(winner("make a shirt")["collar"], "with_stand");
        assert_eq!(winner("simply make a shirt")["collar"], "one_piece");
    }

    #[test]
    fn ribbed_hoodie_keeps_its_finishes_consistent() {
        let w = winner("make a hoodie");
        assert_eq!(w["sleeve_finish"] == "rib_cuff", w["body_hem"] == "rib_band", "{w:?}");
    }

    #[test]
    fn search_avoids_variants_that_fail_checks() {
        // A much narrower neckline only passes the head with a rib band.
        let w = winner("cheaply make a t-shirt and narrow the collar by 1.5 inches");
        assert_eq!(w["neck_finish"], "rib_band", "{w:?}");
    }

    #[test]
    fn impossible_requests_explain_why() {
        let cfg = Config::builtin();
        let err = search(&spec("make a t-shirt and narrow the collar by 12 inches"), &cfg).err().unwrap();
        assert!(err[0].message.contains("construction checks"));
    }
}
