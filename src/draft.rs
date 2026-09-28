//! Drafting: spec + chosen rule variants -> pattern pieces.
//!
//! 1. numbers: the garment's `rules.toml` entry, plus its fit, into params
//! 2. every chosen rule adjusts the params (edits, cuffs shortening a sleeve, ...)
//! 3. the blocks are drafted from the body measurements (torso + sleeve, or trousers)
//! 4. every chosen rule adds or reshapes pieces (bands, hood, collar, waistband, ...)
//! 5. construction checks: seams match, the garment goes on over head, hand, foot

use crate::blocks::{torso, trousers};
use crate::config::{Config, FabricConfig, Measurements};
use crate::geom;
use crate::model::{Spec, SleeveKind};
use crate::parser::fmt_mm;
use crate::pattern::{Check, Pattern, Piece};
use crate::rules;
use std::collections::BTreeMap;

/// Chosen variant per rule name.
pub type Choices = BTreeMap<&'static str, &'static str>;

/// Numbers for a top, after fit, edits and rules. All mm.
#[derive(Clone, Debug, Default)]
pub struct TopParams {
    pub chest_ease: f64,
    /// Neck point to hem at centre back.
    pub length: f64,
    /// Neck width from centre to neck point.
    pub nw: f64,
    pub back_neck_depth: f64,
    pub front_neck_depth: f64,
    pub shoulder_len: f64,
    pub shoulder_drop: f64,
    pub armhole_drop: f64,
    /// Full bicep girth including ease.
    pub bicep: f64,
    /// Cap top to hem.
    pub sleeve_len: f64,
    /// Full hem girth.
    pub sleeve_hem: f64,
    /// Length added (or removed) by continuing the sleeve's taper.
    pub sleeve_extend: f64,
    pub hem_flare: f64,
    /// Button stand beyond centre front (0 = cut on the fold).
    pub button_ext: f64,
    /// How far a curved hem rises at the side seam (0 = straight).
    pub shirttail: f64,
    pub long: bool,
}

/// Numbers for pants. Knee and hem are full leg girths.
#[derive(Clone, Debug, Default)]
pub struct PantsParams {
    pub waist_ease: f64,
    pub seat_ease: f64,
    pub rise_ease: f64,
    pub knee: f64,
    pub hem: f64,
    pub inseam: f64,
    pub leg_extend: f64,
    pub elastic: bool,
    /// Set by the block: front pleat and back dart widths at the waist.
    pub pleat: f64,
    pub dart: f64,
}

pub struct Draft<'a> {
    pub spec: &'a Spec,
    pub cfg: &'a Config,
    pub m: &'a Measurements,
    pub fabric: &'a FabricConfig,
    pub rib: &'a FabricConfig,
    pub fabric_name: &'a str,
    pub choices: &'a Choices,
    pub top: TopParams,
    pub pants: PantsParams,
    pub pieces: Vec<Piece>,
    pub checks: Vec<Check>,
    pub notes: Vec<String>,
}

impl<'a> Draft<'a> {
    fn new(spec: &'a Spec, cfg: &'a Config, choices: &'a Choices) -> Draft<'a> {
        let g = spec.g();
        let gc = &cfg.garments[g.key()];
        let fit = &cfg.fits[g.class()][spec.fit().key(g)];
        let num = |k: &str| gc.nums[k] + fit.nums.get(k).copied().unwrap_or(0.0);
        let m = spec.measurements(cfg);
        let mut top = TopParams::default();
        let mut pants = PantsParams::default();
        if g.is_top() {
            let long = spec.sleeves(cfg) == SleeveKind::Long;
            let shoulder_add = num("shoulder_add");
            let bicep = m.upper_arm + num("bicep_ease");
            top = TopParams {
                chest_ease: num("chest_ease"),
                length: m.nape_to_waist + num("below_waist"),
                nw: m.neck / 5.0 + num("neck_width_add"),
                back_neck_depth: num("back_neck_depth"),
                front_neck_depth: num("front_neck_depth"),
                shoulder_len: m.shoulder + shoulder_add,
                shoulder_drop: num("shoulder_drop"),
                armhole_drop: num("armhole_drop"),
                bicep,
                sleeve_len: if long { m.arm + num("long_sleeve_add") - shoulder_add } else { num("short_sleeve") },
                sleeve_hem: if long { m.wrist + num("wrist_ease") } else { bicep * cfg.draft("short_hem_ratio") },
                sleeve_extend: 0.0,
                hem_flare: num("hem_flare"),
                button_ext: num("button_ext"),
                shirttail: 0.0,
                long,
            };
        } else {
            let knee = m.knee + num("knee_ease");
            pants = PantsParams {
                waist_ease: num("waist_ease"),
                seat_ease: num("seat_ease"),
                rise_ease: num("rise_ease"),
                knee,
                hem: knee + num("hem_vs_knee"),
                inseam: m.inside_leg,
                leg_extend: 0.0,
                elastic: false,
                pleat: 0.0,
                dart: 0.0,
            };
        }
        let fabric_name = spec.fabric_name(cfg);
        Draft {
            spec,
            cfg,
            m,
            fabric: cfg.fabric(fabric_name),
            rib: cfg.fabric("rib"),
            fabric_name,
            choices,
            top,
            pants,
            pieces: Vec::new(),
            checks: Vec::new(),
            notes: Vec::new(),
        }
    }

    pub fn chose(&self, rule: &str, variant: &str) -> bool {
        self.choices.get(rule).is_some_and(|v| *v == variant)
    }
    pub fn piece(&self, name: &str) -> &Piece {
        self.pieces.iter().find(|p| p.name == name).unwrap_or_else(|| panic!("no piece {name}"))
    }
    pub fn piece_mut(&mut self, name: &str) -> &mut Piece {
        self.pieces.iter_mut().find(|p| p.name == name).unwrap_or_else(|| panic!("no piece {name}"))
    }
    pub fn has(&self, name: &str) -> bool {
        self.pieces.iter().any(|p| p.name == name)
    }
    pub fn add(&mut self, p: Piece) {
        self.pieces.push(p);
    }
    pub fn check(&mut self, name: impl Into<String>, ok: bool, detail: impl Into<String>) {
        self.checks.push(Check { name: name.into(), detail: detail.into(), ok });
    }
    /// Two seam lengths that are sewn together: `b - a` must fall in `[lo, hi]`.
    pub fn seam(&mut self, name: &str, a: f64, b: f64, lo: f64, hi: f64) {
        let diff = b - a;
        let tol = self.cfg.check("seam_tolerance");
        let ok = diff >= lo - tol && diff <= hi + tol;
        let want = if lo == hi { format!("want {}", fmt_mm(lo)) } else { format!("want {} to {}", fmt_mm(lo), fmt_mm(hi)) };
        self.check(name, ok, format!("{} vs {}: difference {} ({want})", fmt_mm(a), fmt_mm(b), fmt_mm(diff)));
    }
    /// `girth * stretch` must reach `body`.
    pub fn clearance(&mut self, name: &str, girth: f64, stretch: f64, body: f64, body_name: &str) {
        let reach = girth * stretch;
        self.check(
            name,
            reach >= body,
            format!("opening {} stretches to {}, {body_name} is {}", fmt_mm(girth), fmt_mm(reach), fmt_mm(body)),
        );
    }
}

/// Every piece's cutting outline must be a simple polygon.
fn outline_checks(d: &mut Draft) {
    let mut bad = Vec::new();
    for p in &d.pieces {
        let (seam, hem) = if p.rib { (d.rib.seam, d.rib.hem) } else { (d.fabric.seam, d.fabric.hem) };
        for unfold in [false, true] {
            let (stitch, cut) = p.outlines(seam, hem, unfold);
            if geom::self_intersects(&stitch) || geom::self_intersects(&cut) {
                bad.push(p.name.clone());
                break;
            }
        }
    }
    let ok = bad.is_empty();
    let detail = if ok { "every stitching and cutting line is a simple outline".to_string() } else { format!("crossed outline in {}", bad.join(", ")) };
    d.check("outlines", ok, detail);
}

pub fn draft(spec: &Spec, cfg: &Config, choices: &Choices) -> Result<Pattern, String> {
    let mut d = Draft::new(spec, cfg, choices);
    let reg = rules::registry();
    for r in &reg {
        if let Some(v) = choices.get(r.name()) {
            r.params(v, &mut d);
        }
    }
    if spec.g().is_top() {
        torso::draft(&mut d)?;
    } else {
        trousers::draft(&mut d)?;
    }
    for r in &reg {
        if let Some(v) = choices.get(r.name()) {
            r.pieces(v, &mut d)?;
        }
    }
    outline_checks(&mut d);
    let fit = spec.fit.as_ref().map(|f| format!("{} ", f.words)).unwrap_or_default();
    Ok(Pattern {
        title: format!("{fit}{}", spec.g().name()),
        size: spec.size.key().to_string(),
        fabric: d.fabric_name.to_string(),
        seam: d.fabric.seam,
        hem: d.fabric.hem,
        rib_seam: d.rib.seam,
        rib_hem: d.rib.hem,
        pieces: d.pieces,
        checks: d.checks,
        notes: d.notes,
    })
}
