//! Judges score a whole set of choices, not one rule. This is why the search
//! is a beam and not a greedy pick: the value of a rib cuff depends on
//! whether the hem is ribbed too.
//!
//! - `consistent_finish`: rib cuffs with a plain hem (or the reverse) look unplanned
//! - `rib_fabric`: the first rib part means buying a second fabric; more are free
//! - `fabric_use`: square metres the cutting layout needs (scored in `search.rs`,
//!   after drafting, because only a finished layout knows it)

use super::{by_name, Finish};
use crate::config::{Axes, Config};
use crate::draft::Choices;

pub const JUDGES: &[&str] = &["consistent_finish", "rib_fabric", "fabric_use"];

/// Penalties (negative scores) for a set of choices, with a note each.
pub fn judge(choices: &Choices, profile: &Axes, cfg: &Config) -> Vec<(&'static str, f64, String)> {
    let mut out = Vec::new();
    let finishes: Vec<(&str, Finish)> = choices
        .iter()
        .filter_map(|(r, v)| by_name(r).finish(v).map(|f| (*r, f)))
        .collect();
    let rib = finishes.iter().any(|(_, f)| *f == Finish::Rib);
    let plain = finishes.iter().any(|(_, f)| *f == Finish::Plain);
    if rib && plain {
        let which: Vec<String> = finishes.iter().map(|(r, f)| format!("{r}={}", if *f == Finish::Rib { "rib" } else { "plain" })).collect();
        out.push(("consistent_finish", -cfg.judge_penalty("consistent_finish", 1.0, profile), format!("mixed edge finishes ({})", which.join(", "))));
    }
    let ribbed: Vec<&str> = choices.iter().filter(|(r, v)| by_name(r).uses_rib(v)).map(|(r, _)| *r).collect();
    if !ribbed.is_empty() {
        out.push(("rib_fabric", -cfg.judge_penalty("rib_fabric", 1.0, profile), format!("second fabric (rib) for {}", ribbed.join(", "))));
    }
    out
}

pub fn total(choices: &Choices, profile: &Axes, cfg: &Config) -> f64 {
    judge(choices, profile, cfg).iter().map(|(_, s, _)| s).sum()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ch(pairs: &[(&'static str, &'static str)]) -> Choices {
        pairs.iter().copied().collect()
    }

    #[test]
    fn mixed_finish_is_penalised() {
        let cfg = Config::builtin();
        let mixed = total(&ch(&[("sleeve_finish", "rib_cuff"), ("body_hem", "hemmed")]), &cfg.profile, &cfg);
        let matched = total(&ch(&[("sleeve_finish", "rib_cuff"), ("body_hem", "rib_band")]), &cfg.profile, &cfg);
        assert!(mixed < matched);
    }

    #[test]
    fn rib_fabric_is_charged_once() {
        let cfg = Config::builtin();
        let one = total(&ch(&[("neck_finish", "rib_band")]), &cfg.profile, &cfg);
        let two = total(&ch(&[("neck_finish", "rib_band"), ("sleeve_finish", "rib_cuff"), ("body_hem", "rib_band")]), &cfg.profile, &cfg);
        assert!(one < 0.0);
        assert_eq!(one, two);
    }
}
