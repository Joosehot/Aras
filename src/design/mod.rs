//! Design: colours, prints and trims, resolved into a fill for every piece.
//!
//! Like the construction rules, these are hand-written and deterministic.
//! The sentence names some colours ("a navy hoodie with white drawstrings",
//! "black and white striped"); the rules fill in the rest the way a product
//! designer would:
//!
//! - a part without its own colour takes the body's (sleeves, hood, pocket,
//!   collar, waistband)
//! - rib is knitted solid: it takes the body colour, or the print's ground
//! - a hood lining takes the hood's colour, or the print's second colour
//! - drawstrings contrast with the hood; buttons and thread are tonal
//! - a print is anchored per piece so it matches across seams: horizontal
//!   bars line up at the underarm and side seams, sleeves at the underarm,
//!   legs at the crotch line, and every print is centred on centre front
//!
//! The result also carries the tech-pack facts: fabric per colourway and
//! notions (drawstring, buttons, zip, elastic, interfacing, thread).

pub mod color;
pub mod print;
pub mod sketch;

use crate::config::Config;
use crate::model::{Garment, Spec, Worded};
use crate::pattern::{Mark, Pattern, Piece};
use crate::search::Outcome;
use color::Color;
use print::{Print, PrintKind};
use std::collections::BTreeMap;

/// Turns fills into SVG paint, collecting `<pattern>` defs for prints.
pub struct Painter {
    pub defs: String,
    prefix: String,
    n: usize,
}

impl Painter {
    pub fn new(prefix: &str) -> Painter {
        Painter { defs: String::new(), prefix: prefix.to_string(), n: 0 }
    }
    /// A colour, or a `url(#..)` to a print anchored at `anchor`, turned `angle` degrees.
    pub fn paint(&mut self, fill: &Fill, anchor: crate::geom::Pt, angle: f64) -> String {
        match fill {
            Fill::Solid(c) => c.rgb.css(),
            Fill::Print(p) => {
                let id = format!("{}{}", self.prefix, self.n);
                self.n += 1;
                self.defs.push_str(&print::pattern_def(&id, p, anchor, angle));
                self.defs.push('\n');
                format!("url(#{id})")
            }
        }
    }
}

/// A part of the garment that can carry its own colour.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Zone {
    Body,
    Sleeves,
    Hood,
    HoodLining,
    Pocket,
    Rib,
    Collar,
    Cuffs,
    Waistband,
    Drawstring,
    Buttons,
    Stitching,
}

impl Zone {
    pub fn name(self) -> &'static str {
        match self {
            Zone::Body => "body",
            Zone::Sleeves => "sleeves",
            Zone::Hood => "hood",
            Zone::HoodLining => "hood lining",
            Zone::Pocket => "pocket",
            Zone::Rib => "rib trims",
            Zone::Collar => "collar",
            Zone::Cuffs => "cuffs",
            Zone::Waistband => "waistband",
            Zone::Drawstring => "drawstrings",
            Zone::Buttons => "buttons",
            Zone::Stitching => "thread",
        }
    }
}

/// A print the sentence asked for.
#[derive(Clone, Debug, PartialEq)]
pub struct PrintReq {
    pub kind: PrintKind,
    pub colors: Vec<Color>,
    pub scale: f64,
    pub words: String,
}

/// What the sentence said about colour.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct DesignSpec {
    pub base: Option<Worded<Color>>,
    pub zones: Vec<(crate::design::Zone, Worded<Color>)>,
    pub print: Option<PrintReq>,
}

impl DesignSpec {
    fn zone(&self, z: Zone) -> Option<&Worded<Color>> {
        self.zones.iter().rev().find(|(k, _)| *k == z).map(|(_, c)| c)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum Fill {
    Solid(Color),
    Print(Print),
}

impl Fill {
    /// The colour the eye reads first: the solid, or the print's ground.
    pub fn dominant(&self) -> &Color {
        match self {
            Fill::Solid(c) => c,
            Fill::Print(p) => &p.colors[0],
        }
    }
    pub fn label(&self) -> String {
        match self {
            Fill::Solid(c) => c.title(),
            Fill::Print(p) => {
                let names: Vec<String> = p.colors.iter().map(|c| c.title()).collect();
                format!("{} {}", names.join(" / "), p.kind.name())
            }
        }
    }
    pub fn solid(&self) -> Fill {
        Fill::Solid(self.dominant().clone())
    }
}

/// One resolved zone: its fill and why it has it.
#[derive(Clone, Debug, PartialEq)]
pub struct ZoneLook {
    pub zone: Zone,
    pub fill: Fill,
    pub why: String,
}

/// The whole colourway.
#[derive(Clone, Debug, PartialEq)]
pub struct Look {
    /// Zones this garment has, for the swatch list.
    pub zones: Vec<ZoneLook>,
    /// Every zone's fill, present or not (the inside of an unlined hood is
    /// still the hood fabric).
    pub all: BTreeMap<Zone, Fill>,
    /// One fill per pattern piece, same order as `Pattern::pieces`.
    pub pieces: Vec<Fill>,
    pub notes: Vec<String>,
    pub name: String,
}

impl Look {
    pub fn get(&self, z: Zone) -> &Fill {
        &self.all[&z]
    }
}

fn named(n: &str) -> Color {
    Color::named(n).expect("palette colour")
}

/// A colour that reads against `on`: off white on dark, charcoal on light.
pub fn contrast_neutral(on: &Color) -> Color {
    if on.rgb.luminance() < 0.3 {
        named("off white")
    } else {
        named("charcoal")
    }
}

/// A darker (or, on very dark cloth, lighter) shade of the same colour.
pub fn tonal(on: &Color, amount: f64) -> Color {
    if on.rgb.luminance() < 0.05 {
        Color::new(format!("light {}", on.name), on.rgb.lighter(amount))
    } else {
        Color::new(format!("dark {}", on.name), on.rgb.darker(amount))
    }
}

/// Zones whose colour a piece takes, first match wins.
fn chain(p: &Piece) -> &'static [Zone] {
    match p.name.as_str() {
        "sleeve" | "sleeve placket" => &[Zone::Sleeves],
        "hood side" | "hood centre" => &[Zone::Hood],
        n if n.starts_with("hood lining") => &[Zone::HoodLining],
        "pocket" => &[Zone::Pocket],
        "neckband" if p.rib => &[Zone::Collar, Zone::Rib],
        "neckband" | "neck binding" => &[Zone::Collar, Zone::Body],
        "cuff" if p.rib => &[Zone::Cuffs, Zone::Rib],
        "cuff" => &[Zone::Cuffs, Zone::Sleeves],
        "hem band" => &[Zone::Rib],
        "collar" | "collar stand" => &[Zone::Collar],
        "waistband" => &[Zone::Waistband],
        _ => &[Zone::Body],
    }
}

pub fn resolve(spec: &Spec, pattern: &Pattern, cfg: &Config) -> Look {
    let d = &spec.design;
    let g = spec.g();
    let mut notes = Vec::new();
    let default = named(&cfg.design_defaults[g.key()]);
    let base = d
        .base
        .as_ref()
        .map(|c| (c.value.clone(), format!("\"{}\"", c.words)))
        .or_else(|| d.print.as_ref().and_then(|p| p.colors.first().map(|c| (c.clone(), format!("\"{}\"", p.words)))))
        .unwrap_or_else(|| (default.clone(), format!("default for a {}", g.name())));
    let body = match &d.print {
        Some(p) => (Fill::Print(print::resolve(p.kind, &p.colors, p.scale, &base.0, cfg)), format!("\"{}\"", p.words)),
        None => (Fill::Solid(base.0.clone()), base.1.clone()),
    };
    if let Fill::Print(p) = &body.0 {
        let c = p.colors[0].rgb.contrast(p.colors[1].rgb);
        if c < cfg.design["min_print_contrast"] {
            notes.push(format!("{} on {} barely reads (contrast {c:.2})", p.colors[1].title(), p.colors[0].title()));
        }
    }

    let mut zones: BTreeMap<Zone, (Fill, String)> = BTreeMap::new();
    let explicit = |z: Zone| d.zone(z).map(|c| (Fill::Solid(c.value.clone()), format!("\"{}\"", c.words)));
    zones.insert(Zone::Body, body.clone());
    for z in [Zone::Sleeves, Zone::Hood, Zone::Pocket, Zone::Collar, Zone::Waistband] {
        zones.insert(z, explicit(z).unwrap_or_else(|| (body.0.clone(), "same as the body".into())));
    }
    let rib = explicit(Zone::Rib).unwrap_or_else(|| match &body.0 {
        Fill::Solid(_) => (body.0.clone(), "tonal: same as the body".into()),
        Fill::Print(_) => (body.0.solid(), "rib is knitted solid: the print's ground colour".into()),
    });
    zones.insert(Zone::Rib, rib);
    let cuffs = explicit(Zone::Cuffs).unwrap_or_else(|| {
        let from = if g.pulls_over_head() { Zone::Rib } else { Zone::Sleeves };
        (zones[&from].0.clone(), format!("same as the {}", from.name()))
    });
    zones.insert(Zone::Cuffs, cuffs);
    let hood = zones[&Zone::Hood].0.clone();
    let lining = explicit(Zone::HoodLining).unwrap_or_else(|| match &hood {
        Fill::Print(p) => (Fill::Solid(p.colors[1].clone()), "the print's second colour".into()),
        f => (f.clone(), "same as the hood".into()),
    });
    zones.insert(Zone::HoodLining, lining);
    let drawstring = explicit(Zone::Drawstring)
        .unwrap_or_else(|| (Fill::Solid(contrast_neutral(hood.dominant())), "contrasts with the hood".into()));
    zones.insert(Zone::Drawstring, drawstring);
    let body_c = body.0.dominant().clone();
    let buttons = explicit(Zone::Buttons).unwrap_or_else(|| {
        let c = if body_c.rgb.luminance() < 0.3 { named("off white") } else { tonal(&body_c, cfg.design["button_shade"]) };
        (Fill::Solid(c), "tonal buttons".into())
    });
    zones.insert(Zone::Buttons, buttons);
    let thread = explicit(Zone::Stitching)
        .unwrap_or_else(|| (Fill::Solid(tonal(&body_c, cfg.design["thread_shade"])), "tonal thread".into()));
    zones.insert(Zone::Stitching, thread);

    let pieces: Vec<Fill> = pattern
        .pieces
        .iter()
        .map(|p| {
            let z = chain(p).iter().find(|z| d.zone(**z).is_some()).copied().unwrap_or(*chain(p).last().expect("chain"));
            let f = zones[&z].0.clone();
            if p.rib {
                f.solid()
            } else {
                f
            }
        })
        .collect();

    // Only zones this garment has.
    let has = |n: &str| pattern.pieces.iter().any(|p| p.name.starts_with(n));
    let present = |z: Zone| match z {
        Zone::Body | Zone::Stitching => true,
        Zone::Sleeves => g.is_top(),
        Zone::Hood | Zone::Drawstring => has("hood side"),
        Zone::HoodLining => has("hood lining"),
        Zone::Pocket => has("pocket"),
        Zone::Rib => pattern.pieces.iter().any(|p| p.rib),
        Zone::Collar => has("collar") || pattern.pieces.iter().any(|p| !p.rib && (p.name == "neckband" || p.name == "neck binding")),
        Zone::Cuffs => has("cuff"),
        Zone::Waistband => has("waistband"),
        Zone::Buttons => g == Garment::Shirt || has("waistband"),
    };
    for (z, _) in &d.zones {
        if !present(*z) {
            notes.push(format!("this {} has no {}; its colour is unused", g.name(), z.name()));
        }
    }
    let all: BTreeMap<Zone, Fill> = zones.iter().map(|(z, (f, _))| (*z, f.clone())).collect();
    let zones: Vec<ZoneLook> =
        zones.into_iter().filter(|(z, _)| present(*z)).map(|(zone, (fill, why))| ZoneLook { zone, fill, why }).collect();
    let name = match &zones[0].fill {
        Fill::Solid(c) => format!("{} {}", c.title(), title_case(&pattern.title)),
        // Camo and plaid: the derived shades would only make the name longer.
        Fill::Print(p) if matches!(p.kind, PrintKind::Camo | PrintKind::Plaid) && d.print.as_ref().is_some_and(|r| r.colors.len() < 2) => {
            format!("{} {} {}", p.colors[0].title(), p.kind.adjective(), title_case(&pattern.title))
        }
        Fill::Print(p) => format!("{} & {} {} {}", p.colors[0].title(), p.colors[1].title(), p.kind.adjective(), title_case(&pattern.title)),
    };
    Look { zones, all, pieces, notes, name }
}

fn title_case(s: &str) -> String {
    Color::new(s, color::Rgb(0, 0, 0)).title()
}

/// Where a piece's print starts, in its own coordinates: the underarm or
/// crotch line and centre front, so bars meet at the seams.
pub fn anchor(p: &Piece) -> crate::geom::Pt {
    p.anchor.unwrap_or(crate::geom::pt(0.0, 0.0))
}

/// Notions and materials for the tech pack: (quantity, item).
pub fn notions(spec: &Spec, o: &Outcome, look: &Look, cfg: &Config) -> Vec<(String, String)> {
    let p = &o.pattern;
    let win = &o.finalists[o.best].choices;
    let mut out = Vec::new();
    let has = |n: &str| p.pieces.iter().any(|x| x.name == n);
    let col = |z: Zone| look.zones.iter().find(|l| l.zone == z).map(|l| l.fill.dominant().title()).unwrap_or_default();
    if has("hood side") {
        let face = p.piece("hood side").len("face") * 2.0 + p.pieces.iter().find(|x| x.name == "hood centre").map_or(0.0, |c| c.len("face") * 2.0);
        let len = ((face + 2.0 * cfg.design["drawstring_tail"]) / 100.0).ceil() * 10.0;
        out.push((format!("{len:.0} cm"), format!("flat drawstring, {}", col(Zone::Drawstring))));
        out.push(("2".into(), "metal aglets".into()));
        out.push(("2".into(), "eyelets, 10 mm".into()));
    }
    if spec.g() == Garment::Shirt {
        let front = p.piece("front")
            .marks
            .iter()
            .filter(|m| matches!(m, Mark::Line { pts, dashed: false } if pts.len() == 2 && (pts[0].x - pts[1].x).abs() < 1e-9))
            .count();
        let cuffs = if win.get("sleeve_finish") == Some(&"barrel_cuff") { 2 } else { 0 };
        let stand = usize::from(win.get("collar") == Some(&"with_stand"));
        out.push((format!("{}", front + cuffs + stand + 2), format!("buttons, 11 mm, {} (incl. 2 spare)", col(Zone::Buttons))));
    }
    if has("fly shield") {
        let need = p.pieces.iter().find(|x| x.name == "fly shield").map_or(0.0, |x| x.len("end_a")) / 10.0 - 2.0;
        let zip = [12.0, 15.0, 18.0, 20.0, 23.0].into_iter().find(|z| *z >= need).unwrap_or(23.0);
        out.push(("1".into(), format!("trouser zip, {zip:.0} cm, {}", col(Zone::Body))));
        out.push(("1".into(), format!("waistband button, 17 mm, {}", col(Zone::Buttons))));
    }
    if win.get("waist") == Some(&"elastic") {
        let m = spec.measurements(cfg);
        out.push((format!("{:.0} cm", (m.waist * cfg.band("elastic") / 10.0).ceil()), format!("elastic, {:.0} mm", cfg.part("elastic_width"))));
    }
    let interfaced: Vec<&str> = p
        .pieces
        .iter()
        .map(|x| x.name.as_str())
        .filter(|n| matches!(*n, "collar" | "collar stand" | "waistband" | "fly shield") || (*n == "cuff" && spec.g() == Garment::Shirt))
        .collect();
    if !interfaced.is_empty() {
        let mut v = interfaced.clone();
        v.dedup();
        out.push(("—".into(), format!("fusible interfacing for {}", v.join(", "))));
    }
    out.push(("1 spool".into(), format!("polyester thread, {}", col(Zone::Stitching))));
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::test_util::ok;

    fn look(s: &str) -> Look {
        let cfg = Config::builtin();
        let spec = crate::rules::test_util::spec(s);
        let p = ok(s, &[]);
        resolve(&spec, &p, &cfg)
    }

    #[test]
    fn parts_inherit_the_body_colour() {
        let l = look("make a navy hoodie");
        assert_eq!(l.get(Zone::Sleeves).dominant().name, "navy");
        assert_eq!(l.get(Zone::Hood).dominant().name, "navy");
    }

    #[test]
    fn explicit_part_colour_wins() {
        let l = look("make a black hoodie with white sleeves");
        assert_eq!(l.get(Zone::Body).dominant().name, "black");
        assert_eq!(l.get(Zone::Sleeves).dominant().name, "white");
    }

    #[test]
    fn drawstrings_contrast_with_the_hood() {
        assert_eq!(look("make a navy hoodie").get(Zone::Drawstring).dominant().name, "off white");
        assert_eq!(look("make a sand hoodie").get(Zone::Drawstring).dominant().name, "charcoal");
    }

    #[test]
    fn rib_is_solid_under_a_print() {
        let l = look("make a black and white striped hoodie with rib cuffs");
        assert!(matches!(l.get(Zone::Body), Fill::Print(_)));
        assert!(matches!(l.get(Zone::Rib), Fill::Solid(_)));
    }

    #[test]
    fn default_colour_when_none_given() {
        let l = look("make pants");
        assert_eq!(l.get(Zone::Body).dominant().name, "khaki");
    }
}
