//! `rules.toml`: every number the engine uses. Rule code only names keys.

use anyhow::{bail, Context as _, Result};
use serde::Deserialize;
use std::collections::BTreeMap;
use std::path::Path;

/// The built-in configuration, so the binary works without a rules file.
pub const DEFAULT_RULES: &str = include_str!("../rules.toml");

/// A point in tradeoff space: what a choice offers, or what the sentence
/// asks for (the profile).
#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Axes {
    #[serde(default)]
    pub finish: f64,
    #[serde(default)]
    pub simplicity: f64,
    #[serde(default)]
    pub economy: f64,
}

impl Axes {
    pub fn dot(&self, o: &Axes) -> f64 {
        self.finish * o.finish + self.simplicity * o.simplicity + self.economy * o.economy
    }
    pub fn add(&self, o: &Axes) -> Axes {
        Axes {
            finish: self.finish + o.finish,
            simplicity: self.simplicity + o.simplicity,
            economy: self.economy + o.economy,
        }
    }
}

#[derive(Clone, Debug, Deserialize)]
pub struct SearchConfig {
    pub beam: usize,
}

/// One size's body measurements, mm.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Measurements {
    pub chest: f64,
    pub waist: f64,
    pub seat: f64,
    pub neck: f64,
    pub nape_to_waist: f64,
    pub shoulder: f64,
    pub back_width: f64,
    pub scye_depth: f64,
    pub arm: f64,
    pub upper_arm: f64,
    pub wrist: f64,
    pub hand: f64,
    pub head: f64,
    pub hip_depth: f64,
    pub body_rise: f64,
    pub inside_leg: f64,
    pub knee: f64,
    pub heel_instep: f64,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FabricConfig {
    pub stretch: f64,
    pub seam: f64,
    pub hem: f64,
    pub cap_ease_min: f64,
    pub cap_ease_max: f64,
    pub width: f64,
}

#[derive(Clone, Debug, Deserialize)]
pub struct GarmentConfig {
    pub fabric: String,
    #[serde(default)]
    pub sleeves: Option<String>,
    #[serde(default)]
    pub hood: bool,
    #[serde(default)]
    pub pocket: bool,
    #[serde(flatten)]
    pub nums: BTreeMap<String, f64>,
}

#[derive(Clone, Debug, Default, Deserialize)]
pub struct FitConfig {
    #[serde(default)]
    pub fabric: Option<String>,
    #[serde(flatten)]
    pub nums: BTreeMap<String, f64>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct RuleConfig {
    pub weight: f64,
    pub variants: BTreeMap<String, Axes>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct JudgeConfig {
    pub weight: f64,
    pub axes: Axes,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Config {
    pub search: SearchConfig,
    pub profile: Axes,
    pub modifiers: BTreeMap<String, Axes>,
    pub sizes: BTreeMap<String, Measurements>,
    pub fabrics: BTreeMap<String, FabricConfig>,
    pub garments: BTreeMap<String, GarmentConfig>,
    /// class ("top", "pants") -> fit name -> adjustments
    pub fits: BTreeMap<String, BTreeMap<String, FitConfig>>,
    pub drafting: BTreeMap<String, f64>,
    pub parts: BTreeMap<String, f64>,
    pub bands: BTreeMap<String, f64>,
    pub edits: BTreeMap<String, f64>,
    pub checks: BTreeMap<String, f64>,
    pub layout: BTreeMap<String, f64>,
    pub rules: BTreeMap<String, RuleConfig>,
    pub judges: BTreeMap<String, JudgeConfig>,
    /// Print repeats, shades and design thresholds.
    pub design: BTreeMap<String, f64>,
    /// Colour of each garment when the sentence names none.
    pub design_defaults: BTreeMap<String, String>,
}

/// Keys the code reads from the flat tables. A missing one fails at load.
const DRAFTING_KEYS: &[&str] = &[
    "back_width_ease", "front_width_less", "width_point", "short_hem_ratio", "cap_min", "cap_max",
    "armhole_step", "armhole_tries", "front_fork", "back_fork", "leg_balance", "cf_in", "cb_in",
    "cb_rise", "max_side_shape", "back_dart", "back_dart_length",
];
const PART_KEYS: &[&str] = &[
    "neckband_width", "binding_width", "rib_cuff_height", "hem_band_height", "barrel_cuff_height",
    "barrel_cuff_ease", "cuff_overlap", "cuff_pleats_max", "placket_length", "placket_strip_width",
    "hemmed_wide_ease", "collar_height", "stand_height", "stand_rise", "collar_point", "hood_height",
    "hood_depth", "hood_ease", "hood_neck_rise", "hood_center_width", "hood_center_nape",
    "hood_face_hem", "pocket_width", "pocket_height", "pocket_opening", "pocket_above_hem",
    "waistband_height", "waistband_ext", "fly_depth", "fly_width", "elastic_width", "placket_facing",
    "shirttail_rise",
];
const BAND_KEYS: &[&str] = &["neck_rib", "neck_self", "binding", "cuff_rib", "hem_rib", "elastic"];
const CHECK_KEYS: &[&str] = &["seam_tolerance", "min_shoulder", "inseam_ease", "max_extend"];
const LAYOUT_KEYS: &[&str] = &["gap", "sheet_width", "margin"];
const DESIGN_KEYS: &[&str] = &[
    "stripe_repeat", "pinstripe_repeat", "gingham_repeat", "plaid_repeat", "dot_repeat", "camo_repeat",
    "thin", "thick", "shade", "button_shade", "thread_shade", "min_print_contrast", "drawstring_tail",
    "flat_long_sleeve", "flat_short_sleeve",
];
pub const TOP_KEYS: &[&str] = &[
    "chest_ease", "below_waist", "neck_width_add", "back_neck_depth", "front_neck_depth",
    "shoulder_add", "shoulder_drop", "armhole_drop", "bicep_ease", "short_sleeve",
    "long_sleeve_add", "wrist_ease", "hem_flare", "button_ext",
];
pub const PANTS_KEYS: &[&str] = &["waist_ease", "seat_ease", "rise_ease", "knee_ease", "hem_vs_knee"];

impl Config {
    pub fn parse(text: &str) -> Result<Config> {
        let cfg: Config = toml::from_str(text).context("parsing rules.toml")?;
        cfg.validate()?;
        Ok(cfg)
    }

    pub fn load(path: &Path) -> Result<Config> {
        let text = std::fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
        Config::parse(&text)
    }

    pub fn builtin() -> Config {
        Config::parse(DEFAULT_RULES).expect("built-in rules.toml is valid")
    }

    /// Everything the code names must have an entry, so a typo fails at
    /// load time instead of mid-draft.
    fn validate(&self) -> Result<()> {
        for r in crate::rules::registry() {
            let Some(rc) = self.rules.get(r.name()) else {
                bail!("rules.toml: missing [rules.{}]", r.name());
            };
            for v in r.all_variants() {
                if !rc.variants.contains_key(*v) {
                    bail!("rules.toml: missing rules.{}.variants.{v}", r.name());
                }
            }
        }
        for j in crate::rules::judges::JUDGES {
            if !self.judges.contains_key(*j) {
                bail!("rules.toml: missing [judges.{j}]");
            }
        }
        for m in crate::lexicon::MODIFIER_KEYS {
            if !self.modifiers.contains_key(*m) {
                bail!("rules.toml: missing [modifiers.{m}]");
            }
        }
        for s in crate::model::Size::ALL {
            if !self.sizes.contains_key(s.key()) {
                bail!("rules.toml: missing [sizes.{}]", s.key());
            }
        }
        for (table, map, keys) in [
            ("drafting", &self.drafting, DRAFTING_KEYS),
            ("parts", &self.parts, PART_KEYS),
            ("bands", &self.bands, BAND_KEYS),
            ("checks", &self.checks, CHECK_KEYS),
            ("layout", &self.layout, LAYOUT_KEYS),
            ("design", &self.design, DESIGN_KEYS),
            ("edits", &self.edits, crate::model::EditKind::KEYS),
        ] {
            for k in keys {
                if !map.contains_key(*k) {
                    bail!("rules.toml: missing {table}.{k}");
                }
            }
        }
        for g in crate::model::Garment::ALL {
            let Some(gc) = self.garments.get(g.key()) else {
                bail!("rules.toml: missing [garments.{}]", g.key());
            };
            if !self.fabrics.contains_key(&gc.fabric) {
                bail!("rules.toml: garments.{} uses unknown fabric {:?}", g.key(), gc.fabric);
            }
            let keys = if g.is_top() { TOP_KEYS } else { PANTS_KEYS };
            for k in keys {
                if !gc.nums.contains_key(*k) {
                    bail!("rules.toml: missing garments.{}.{k}", g.key());
                }
            }
            for f in crate::model::Fit::ALL {
                let class = g.class();
                let Some(fc) = self.fits.get(class).and_then(|c| c.get(f.key(g))) else {
                    bail!("rules.toml: missing [fits.{class}.{}]", f.key(g));
                };
                for k in fc.nums.keys() {
                    if !keys.contains(&k.as_str()) {
                        bail!("rules.toml: fits.{class}.{}.{k} is not a {class} number", f.key(g));
                    }
                }
                if let Some(fab) = &fc.fabric {
                    if !self.fabrics.contains_key(fab) {
                        bail!("rules.toml: fits.{class}.{} uses unknown fabric {fab:?}", f.key(g));
                    }
                }
            }
        }
        for g in crate::model::Garment::ALL {
            match self.design_defaults.get(g.key()) {
                Some(c) if crate::design::color::Color::named(c).is_some() => {}
                Some(c) => bail!("rules.toml: design_defaults.{} is not a known colour: {c:?}", g.key()),
                None => bail!("rules.toml: missing design_defaults.{}", g.key()),
            }
        }
        if !self.fabrics.contains_key("rib") {
            bail!("rules.toml: missing [fabrics.rib]");
        }
        Ok(())
    }

    pub fn draft(&self, key: &str) -> f64 {
        self.drafting[key]
    }
    pub fn part(&self, key: &str) -> f64 {
        self.parts[key]
    }
    pub fn band(&self, key: &str) -> f64 {
        self.bands[key]
    }
    pub fn check(&self, key: &str) -> f64 {
        self.checks[key]
    }
    pub fn layout(&self, key: &str) -> f64 {
        self.layout[key]
    }
    pub fn fabric(&self, name: &str) -> &FabricConfig {
        &self.fabrics[name]
    }

    /// Score of one rule variant under a profile.
    pub fn variant_score(&self, rule: &str, variant: &str, profile: &Axes) -> f64 {
        let rc = &self.rules[rule];
        rc.weight * rc.variants[variant].dot(profile)
    }

    /// Penalty a judge charges for `amount` units under a profile.
    pub fn judge_penalty(&self, judge: &str, amount: f64, profile: &Axes) -> f64 {
        let jc = &self.judges[judge];
        jc.weight * jc.axes.dot(profile) * amount
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builtin_rules_load() {
        let cfg = Config::builtin();
        assert!(cfg.search.beam > 0);
        assert_eq!(cfg.sizes["M"].chest, 1000.0);
    }

    #[test]
    fn missing_variant_is_rejected() {
        let broken = DEFAULT_RULES.replace("variants.three_piece", "variants.three_peice");
        assert!(Config::parse(&broken).is_err());
    }

    #[test]
    fn fit_with_unknown_number_is_rejected() {
        let broken = DEFAULT_RULES.replace("[fits.top.fitted]\n", "[fits.top.fitted]\nseat_ease = 1\n");
        assert!(Config::parse(&broken).is_err());
    }
}
