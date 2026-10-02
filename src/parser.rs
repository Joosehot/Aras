//! Tokens -> garment specification.
//!
//! Grammar, briefly:
//! - Clauses are separated by "and", commas, "then" and full stops.
//! - A garment noun starts the pattern ("make a hoodie"). Only one garment.
//! - "with ..." lists features until the next verb: "with long sleeves and a hood".
//!   "without"/"no" removes them. "add"/"remove" do the same as verbs.
//! - Change verbs take parts and an amount: "lengthen the sleeves by 2 inches",
//!   "make the collar wider". A clause with no verb repeats the previous
//!   change: "lengthen the sleeves and the body by 2 inches".
//! - An amount belongs to its own clause. Clauses that share a verb and give
//!   no amount take the next one given ("the sleeves and the body by 2
//!   inches"); otherwise the `[edits]` default applies and `--explain` says so.
//! - Unknown words, numbers without units and amounts nothing uses are errors.

use crate::config::Config;
use crate::design::color::Color;
use crate::design::print::PrintKind;
use crate::design::{DesignSpec, PrintReq, Zone};
use crate::lexicon::{Modifier, Part, PinDef, Tok, Token, Verb};
use crate::model::{DressLength, Edit, EditKind, Fit, Garment, Pin, Size, SleeveKind, Spec, Worded};
use std::fmt;

#[derive(Clone, Debug, PartialEq)]
pub struct Diag {
    pub message: String,
    pub hint: Option<String>,
}

impl Diag {
    pub fn new(message: impl Into<String>) -> Diag {
        Diag { message: message.into(), hint: None }
    }
    pub fn hint(mut self, hint: impl Into<String>) -> Diag {
        self.hint = Some(hint.into());
        self
    }
}

impl fmt::Display for Diag {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "error: {}", self.message)?;
        if let Some(h) = &self.hint {
            write!(f, "\n  hint: {h}")?;
        }
        Ok(())
    }
}

/// Settings from outside the sentence.
#[derive(Clone, Debug, Default)]
pub struct ParseOptions {
    pub lenient: bool,
    /// Garment to change when the sentence names none ("widen the collar").
    pub base: Option<Garment>,
    /// Size override; wins over the sentence.
    pub size: Option<Size>,
}

/// An edit before the garment is known: "lengthen" + "sleeves".
struct RawEdit {
    verb: Verb,
    part: Option<Part>,
    mm: Option<f64>,
    words: String,
    group: usize,
}

#[derive(Default)]
struct State {
    garment: Option<Worded<Garment>>,
    size: Option<Size>,
    fit: Option<Worded<Fit>>,
    sleeves: Option<Worded<SleeveKind>>,
    hood: Option<Worded<bool>>,
    pocket: Option<Worded<bool>>,
    pins: Vec<Pin>,
    mods: Vec<Modifier>,
    edits: Vec<RawEdit>,
    /// Inside a "with"/"without" list: Some(negated).
    listing: Option<bool>,
    /// The last verb, repeated by clauses that have none.
    last: Option<Verb>,
    group: usize,
    design: DesignSpec,
    dress_length: Option<Worded<DressLength>>,
}

pub fn parse(sentence: &str, tokens: Vec<Token>, cfg: &Config, opts: &ParseOptions) -> Result<Spec, Vec<Diag>> {
    let mut diags = Vec::new();
    let mut toks = Vec::new();
    for t in tokens {
        if t.tok == Tok::Unknown {
            if opts.lenient {
                continue;
            }
            let mut d = Diag::new(format!("unknown word \"{}\"", t.text));
            d = match crate::lexicon::suggest(&t.text) {
                Some(s) => d.hint(format!("did you mean \"{s}\"? (--vocabulary lists every word, --lenient skips unknown ones)")),
                None => d.hint("--vocabulary lists every word Aras knows; --lenient skips unknown ones"),
            };
            diags.push(d);
            continue;
        }
        toks.push(t);
    }
    if !diags.is_empty() {
        return Err(diags);
    }

    // "black and white striped", "wheat, navy and green": an "and" or a
    // comma between two colours joins them.
    let is_color = |t: Option<&Token>| matches!(t.map(|t| t.tok), Some(Tok::Color(_) | Tok::Shade(_)));
    let toks: Vec<Token> = (0..toks.len())
        .filter(|&i| !(matches!(toks[i].tok, Tok::And | Tok::Comma) && i > 0 && is_color(toks.get(i - 1)) && is_color(toks.get(i + 1))))
        .map(|i| toks[i].clone())
        .collect();

    // "small yellow flowers": "small" before a colour or a print sizes the
    // print, not the garment.
    let toks: Vec<Token> = (0..toks.len())
        .map(|i| {
            let t = toks[i].clone();
            let before_print = matches!(toks.get(i + 1).map(|n| n.tok), Some(Tok::Color(_) | Tok::Shade(_) | Tok::Print(_)));
            if t.text == "small" && before_print {
                Token { tok: Tok::Scale(-1.0), text: t.text }
            } else {
                t
            }
        })
        .collect();
    let mut st = State::default();
    for clause in toks.split(|t| matches!(t.tok, Tok::And | Tok::Comma | Tok::Then | Tok::Stop)) {
        if clause.iter().all(|t| matches!(t.tok, Tok::Filler | Tok::Article)) {
            continue;
        }
        if let Err(d) = st.clause(clause, cfg) {
            diags.push(d);
        }
        if clause_ends_sentence(&toks, clause) {
            st.listing = None;
        }
    }
    if !diags.is_empty() {
        return Err(diags);
    }

    let garment = match (st.garment.take(), opts.base) {
        (Some(g), _) => g,
        (None, Some(b)) => Worded { value: b, words: format!("--base {}", b.key()) },
        (None, None) => {
            return Err(vec![Diag::new("no garment: say what to make")
                .hint("start with \"make a t-shirt\", \"a hoodie\", \"a shirt\" or \"pants\", or pass --base to change a base pattern")])
        }
    };
    let g = garment.value;
    let mut spec = Spec {
        sentence: sentence.trim().to_string(),
        garment,
        size: opts.size.or(st.size).unwrap_or(Size::M),
        fit: st.fit,
        sleeves: st.sleeves,
        hood: st.hood,
        pocket: st.pocket,
        pins: st.pins,
        edits: Vec::new(),
        mods: st.mods,
        design: st.design,
        dress_length: st.dress_length,
    };
    spec.mods.sort();
    spec.mods.dedup();

    // Dress words on other garments, and what v0 dresses don't have.
    if g == Garment::Dress {
        for (what, set) in [("a hood", spec.hood.as_ref()), ("a pocket", spec.pocket.as_ref())] {
            if let Some(w) = set.filter(|w| w.value) {
                diags.push(Diag::new(format!("\"{}\": v0 dresses have no {what}", w.words)));
            }
        }
        // "with a zipper": on a dress that is the back zip, not a fly
        for p in spec.pins.iter_mut() {
            if p.rule == "waist" {
                if p.variants.contains(&"zip_fly") {
                    *p = Pin { rule: "closure", variants: &["back_zip"], words: p.words.clone() };
                } else {
                    diags.push(Diag::new(format!("\"{}\": a dress has no waistband in v0", p.words)));
                }
            }
        }
        if spec.design.print.as_ref().is_some_and(|p| p.kind == PrintKind::Sweep) {
            diags.push(Diag::new("a sweep is drawn for tops in v0, not dresses").hint("try an all-over print: stripes, gingham, dots or flowers"));
        }
        if spec.sleeves.as_ref().is_some_and(|s| s.value == SleeveKind::None) {
            for e in st.edits.iter().filter(|e| e.part == Some(Part::Sleeves)) {
                diags.push(Diag::new(format!("\"{}\": the dress is sleeveless", e.words)));
            }
        }
    } else {
        if let Some(l) = &spec.dress_length {
            diags.push(Diag::new(format!("\"{}\": only dresses have a dress length", l.words)));
        }
        if let Some(s) = spec.sleeves.as_ref().filter(|s| s.value == SleeveKind::None) {
            diags.push(Diag::new(format!("\"{}\": in v0 only dresses can be sleeveless", s.words)));
        }
        for p in spec.pins.iter().filter(|p| matches!(p.rule, "skirt" | "closure" | "armhole_finish") || (p.rule == "neck_finish" && p.variants == ["facing"])) {
            diags.push(Diag::new(format!("\"{}\" is for dresses", p.words)));
        }
    }

    if !g.is_top() {
        for (what, set) in [("sleeves", spec.sleeves.as_ref().map(|w| &w.words)), ("a hood", spec.hood.as_ref().map(|w| &w.words)), ("a pocket", spec.pocket.as_ref().map(|w| &w.words))] {
            if let Some(words) = set {
                diags.push(Diag::new(format!("\"{words}\": pants have no {what} in v0")));
            }
        }
    }
    for p in &spec.pins {
        let top_rule = !matches!(p.rule, "waist");
        if top_rule != g.is_top() {
            diags.push(Diag::new(format!("\"{}\" doesn't apply to {}", p.words, g.name())));
        }
    }

    // Amounts shared across clauses of one verb group.
    let n = st.edits.len();
    for i in 0..n {
        if st.edits[i].mm.is_none() {
            let group = st.edits[i].group;
            if let Some(mm) = st.edits[i + 1..].iter().take_while(|e| e.group == group).find_map(|e| e.mm) {
                st.edits[i].mm = Some(mm);
            }
        }
    }
    for e in &st.edits {
        match resolve(e, g, cfg) {
            Ok(edit) => spec.edits.push(edit),
            Err(d) => diags.push(d),
        }
    }
    if diags.is_empty() {
        Ok(spec)
    } else {
        Err(diags)
    }
}

fn clause_ends_sentence(all: &[Token], clause: &[Token]) -> bool {
    // The separator after `clause` is the token following its last element.
    let Some(last) = clause.last() else { return false };
    let idx = all.iter().position(|t| std::ptr::eq(t, last)).unwrap_or(0);
    matches!(all.get(idx + 1).map(|t| t.tok), Some(Tok::Stop | Tok::Then))
}

fn words(clause: &[Token]) -> String {
    clause.iter().map(|t| t.text.as_str()).collect::<Vec<_>>().join(" ")
}

impl State {
    fn clause(&mut self, clause: &[Token], cfg: &Config) -> Result<(), Diag> {
        let text = words(clause);
        let mut verb: Option<Verb> = None;
        let mut make = false;
        let mut parts: Vec<Part> = Vec::new();
        let mut garment: Option<Garment> = None;
        let mut sleeve_adj: Option<SleeveKind> = None;
        let mut neg = false;
        let mut with = false;
        let mut pins: Vec<PinDef> = Vec::new();
        let amount = amount(clause)?;
        let colored = self.colors(clause, &text, cfg)?;

        for t in clause {
            match t.tok {
                Tok::Verb(Verb::Make) => make = true,
                Tok::Verb(v) | Tok::Cmp(v) => {
                    if verb.is_some_and(|old| old != v) {
                        return Err(Diag::new(format!("\"{text}\" asks for two changes")).hint("give each change its own clause: \"widen the collar and lengthen the sleeves\""));
                    }
                    verb = Some(v);
                }
                Tok::Garment(g) => garment = Some(g),
                Tok::Part(p) => parts.push(p),
                Tok::Fit(f) => self.fit = Some(Worded { value: f, words: t.text.clone() }),
                Tok::Mod(m) => self.mods.push(m),
                Tok::Size(s) => self.size = Some(s),
                Tok::Sleeves(k) => sleeve_adj = Some(k),
                Tok::DressLen(l) => self.dress_length = Some(Worded { value: l, words: text.clone() }),
                Tok::Pin(p) => pins.push(p),
                Tok::With => with = true,
                Tok::Without | Tok::No => {
                    with = true;
                    neg = true;
                }
                _ => {}
            }
        }

        if let Some(g) = garment {
            match &self.garment {
                Some(old) if old.value != g => {
                    return Err(Diag::new(format!("two garments: \"{}\" and \"{}\"", old.words, t_words(clause, |t| matches!(t, Tok::Garment(_)))))
                        .hint("Aras drafts one garment per sentence"));
                }
                Some(_) => {}
                None => self.garment = Some(Worded { value: g, words: text.clone() }),
            }
        }
        if let Some(k) = sleeve_adj {
            self.sleeves = Some(Worded { value: k, words: text.clone() });
            parts.retain(|p| *p != Part::Sleeves);
        }
        for p in &pins {
            // "without a zip", "no zip": the dress pulls on
            let variants: &'static [&'static str] = if neg && p.rule == "closure" { &["pull_on"] } else { p.variants };
            self.pins.push(Pin { rule: p.rule, variants, words: text.clone() });
            if p.implies == Some(Part::Hood) {
                self.hood = Some(Worded { value: true, words: text.clone() });
            }
            // finishing the armholes means there are no sleeves
            if p.rule == "armhole_finish" {
                self.sleeves = Some(Worded { value: SleeveKind::None, words: text.clone() });
            }
        }

        // What does this clause do?
        let verb = match verb {
            Some(v) => {
                self.listing = None;
                self.group += 1;
                self.last = Some(v);
                Some(v)
            }
            None if with => {
                self.listing = Some(neg);
                None
            }
            None if make || garment.is_some() => {
                self.last = Some(Verb::Make);
                None
            }
            None => self.last,
        };
        let feature_neg = match (verb, self.listing) {
            (Some(Verb::Add | Verb::Make), _) => Some(false),
            (Some(Verb::Remove), _) => Some(true),
            (None, Some(n)) => Some(n || neg),
            _ if with => Some(neg),
            _ => None,
        };

        if let Some(v) = verb.filter(|v| v.is_edit()) {
            let refers = clause.iter().any(|t| t.text == "it" || t.text == "them");
            if parts.is_empty() && garment.is_none() && !refers {
                return Err(Diag::new(format!("\"{text}\": {} what?", verb_word(v))).hint("name a part: sleeves, collar, body, legs, waist, chest or hips"));
            }
            let targets: Vec<Option<Part>> = if parts.is_empty() { vec![None] } else { parts.iter().copied().map(Some).collect() };
            for part in targets {
                self.edits.push(RawEdit { verb: v, part, mm: amount, words: text.clone(), group: self.group });
            }
            return Ok(());
        }
        if let Some(mm) = amount {
            return Err(Diag::new(format!("\"{text}\": nothing uses {}", fmt_mm(mm))).hint("amounts go with a change: \"lengthen the sleeves by 2 inches\""));
        }
        if let Some(n) = feature_neg {
            for p in parts {
                self.feature(p, n, &text)?;
            }
        } else if !parts.is_empty() && garment.is_none() && pins.is_empty() && !colored {
            return Err(Diag::new(format!("\"{text}\": what should happen to the {}?", parts[0].name())).hint("add, remove, lengthen, shorten, widen, narrow, lower or raise it"));
        }
        Ok(())
    }

    /// Colours and prints in a clause. A colour goes to the part right after
    /// it ("white sleeves"), else to the part before it ("sleeves in white");
    /// with a print word the loose colours are the print's; otherwise the
    /// first loose colour is the garment's. Returns whether a part got a colour.
    fn colors(&mut self, clause: &[Token], text: &str, cfg: &Config) -> Result<bool, Diag> {
        let mut shade = 0.0;
        let mut pending: Option<Color> = None;
        let mut free: Vec<Color> = Vec::new();
        let mut assigned: Vec<(Part, Color)> = Vec::new();
        let mut last_part: Option<Part> = None;
        let mut print = None;
        let mut scale = 1.0;
        for t in clause {
            match t.tok {
                Tok::Shade(s) => shade = s,
                Tok::Color(name) => {
                    let c = Color::named(name).expect("lexicon colour").shaded(shade * cfg.design["shade"]);
                    shade = 0.0;
                    if let Some(old) = pending.replace(c) {
                        free.push(old);
                    }
                }
                Tok::Part(p) => {
                    if let Some(c) = pending.take() {
                        assigned.push((p, c));
                    }
                    last_part = Some(p);
                }
                // A print or garment word takes the colours before it:
                // "black and white striped", "a navy hoodie".
                Tok::Print(k) => {
                    // "stripes from the hem into the sleeves": the sweep wins.
                    if print != Some(PrintKind::Sweep) {
                        print = Some(k);
                    }
                    free.extend(pending.take());
                }
                Tok::Garment(_) => free.extend(pending.take()),
                Tok::Coverage(c) => self.design.coverage = Some(c),
                Tok::Scale(s) => scale = if s > 0.0 { cfg.design["thick"] } else { cfg.design["thin"] },
                _ => {}
            }
        }
        if shade != 0.0 {
            return Err(Diag::new(format!("\"{text}\": light or dark what?")).hint("put a colour after it: \"dark green\""));
        }
        if let Some(c) = pending {
            match last_part {
                Some(p) if print.is_none() && !assigned.iter().any(|(q, _)| *q == p) => assigned.push((p, c)),
                _ => free.push(c),
            }
        }
        match print {
            Some(kind) => self.design.print = Some(PrintReq { kind, colors: free, scale, words: text.to_string() }),
            None => {
                if free.len() > 1 {
                    return Err(Diag::new(format!("\"{text}\" has {} colours for one garment", free.len()))
                        .hint("say which part gets which (\"a black hoodie with white sleeves\") or name a print (\"black and white striped\")"));
                }
                if let Some(c) = free.pop() {
                    self.design.base = Some(Worded { value: c, words: text.to_string() });
                }
            }
        }
        let colored = !assigned.is_empty();
        for (p, c) in assigned {
            let zone = match p {
                Part::Body | Part::Legs | Part::Chest | Part::Seat => {
                    self.design.base = Some(Worded { value: c, words: text.to_string() });
                    continue;
                }
                Part::Sleeves => Zone::Sleeves,
                Part::Collar => Zone::Collar,
                Part::Hood => Zone::Hood,
                Part::Pocket => Zone::Pocket,
                Part::Cuffs => Zone::Cuffs,
                Part::Waist => Zone::Waistband,
                Part::Lining => Zone::HoodLining,
                Part::Drawstring => Zone::Drawstring,
                Part::Buttons => Zone::Buttons,
                Part::Stitching => Zone::Stitching,
                Part::Rib => Zone::Rib,
            };
            self.design.zones.push((zone, Worded { value: c, words: text.to_string() }));
        }
        Ok(colored)
    }

    fn feature(&mut self, p: Part, neg: bool, text: &str) -> Result<(), Diag> {
        let w = |v| Some(Worded { value: v, words: text.to_string() });
        match p {
            Part::Hood => self.hood = w(!neg),
            Part::Pocket => self.pocket = w(!neg),
            Part::Cuffs => {
                let variants: &'static [&'static str] = if neg { &["hemmed", "hemmed_wide"] } else { &["rib_cuff", "barrel_cuff"] };
                self.pins.push(Pin { rule: "sleeve_finish", variants, words: text.to_string() });
            }
            // checked against the garment once it is known: only dresses
            Part::Sleeves if neg => self.sleeves = Some(Worded { value: SleeveKind::None, words: text.to_string() }),
            Part::Lining if !neg => {
                self.pins.push(Pin { rule: "hood_lining", variants: &["lined"], words: text.to_string() });
                self.hood = w(true);
            }
            Part::Drawstring | Part::Buttons | Part::Stitching | Part::Rib if !neg => {}
            Part::Sleeves | Part::Collar | Part::Body | Part::Legs | Part::Waist if !neg => {}
            _ => return Err(Diag::new(format!("\"{text}\": the {} can't be {}", p.name(), if neg { "removed" } else { "added" }))),
        }
        Ok(())
    }
}

fn t_words(clause: &[Token], pred: impl Fn(&Tok) -> bool) -> String {
    clause.iter().filter(|t| pred(&t.tok)).map(|t| t.text.as_str()).collect::<Vec<_>>().join(" ")
}

fn verb_word(v: Verb) -> &'static str {
    match v {
        Verb::Lengthen => "lengthen",
        Verb::Shorten => "shorten",
        Verb::Crop => "crop",
        Verb::Widen => "widen",
        Verb::Narrow => "narrow",
        Verb::Lower => "lower",
        Verb::Raise => "raise",
        Verb::Make => "make",
        Verb::Add => "add",
        Verb::Remove => "remove",
    }
}

pub fn fmt_mm(mm: f64) -> String {
    format!("{:.1} cm", mm / 10.0)
}

/// The clause's amount in mm: "2 inches", "an inch", "two and a half cm",
/// "half an inch". A number with no unit is an error.
fn amount(clause: &[Token]) -> Result<Option<f64>, Diag> {
    let text = words(clause);
    let mut num: Option<f64> = None;
    let mut found: Option<f64> = None;
    let mut prev: Option<Tok> = None;
    for t in clause {
        match t.tok {
            Tok::Num(n) => num = Some(num.unwrap_or(0.0) + n),
            Tok::Half => num = Some(num.unwrap_or(0.0) + 0.5),
            Tok::Unit(per) => {
                let n = match (num.take(), prev) {
                    (Some(n), _) => n,
                    (None, Some(Tok::Article)) => 1.0,
                    _ => return Err(Diag::new(format!("\"{text}\": how many {}?", t.text))),
                };
                if found.is_some() {
                    return Err(Diag::new(format!("\"{text}\" has two amounts")));
                }
                found = Some(n * per);
            }
            Tok::Article | Tok::Filler => {}
            _ => {
                if let Some(n) = num.take() {
                    return Err(Diag::new(format!("\"{text}\": {n} what?")).hint("give a unit: inches, cm or mm"));
                }
            }
        }
        prev = Some(t.tok);
    }
    if let Some(n) = num {
        return Err(Diag::new(format!("\"{text}\": {n} what?")).hint("give a unit: inches, cm or mm"));
    }
    Ok(found)
}

/// Verb + part + garment -> a concrete edit.
fn resolve(e: &RawEdit, g: Garment, cfg: &Config) -> Result<Edit, Diag> {
    use EditKind as K;
    let top = g.is_top();
    let part = e.part.unwrap_or(if top { Part::Body } else { Part::Legs });
    let bad = || {
        Diag::new(format!("\"{}\": can't {} the {} of {}", e.words, verb_word(e.verb), part.name(), g.name()))
    };
    let (kind, sign) = match (e.verb, part) {
        (Verb::Lengthen | Verb::Shorten, Part::Sleeves) if top => (K::SleeveLength, 1.0),
        (Verb::Lengthen | Verb::Shorten, Part::Body) if top => (K::BodyLength, 1.0),
        (Verb::Lengthen | Verb::Shorten, Part::Legs | Part::Body) if !top => (K::LegLength, 1.0),
        (Verb::Crop, Part::Body) if top => (K::BodyLength, -1.0),
        (Verb::Crop, Part::Legs | Part::Body) if !top => (K::LegLength, -1.0),
        (Verb::Widen | Verb::Narrow, Part::Collar) if top => (K::NeckWidth, 1.0),
        (Verb::Widen | Verb::Narrow, Part::Sleeves) if top => (K::SleeveWidth, 1.0),
        (Verb::Widen | Verb::Narrow, Part::Body | Part::Chest) if top => (K::ChestWidth, 1.0),
        (Verb::Widen | Verb::Narrow, Part::Legs) if !top => (K::LegWidth, 1.0),
        (Verb::Widen | Verb::Narrow, Part::Seat | Part::Body) if !top => (K::SeatWidth, 1.0),
        (Verb::Lower | Verb::Raise, Part::Collar) if top => (K::NeckDepth, 1.0),
        (Verb::Lower | Verb::Raise, Part::Waist) if !top => (K::Rise, -1.0),
        _ => {
            let hint = match (top, part) {
                (true, Part::Legs | Part::Waist | Part::Seat) => format!("{} has no {}", g.name(), part.name()),
                (false, Part::Sleeves | Part::Collar | Part::Chest | Part::Hood | Part::Cuffs) => format!("pants have no {}", part.name()),
                _ => "v0 changes: sleeve length/width, collar width/depth, body length/width, leg length/width, seat width, waist height".into(),
            };
            return Err(bad().hint(hint));
        }
    };
    // Direction of the verb: shorten, narrow, raise (a neckline) and lower (a waist) are negative.
    let dir = match e.verb {
        Verb::Shorten | Verb::Narrow | Verb::Raise => -1.0,
        _ => 1.0,
    };
    let default_key = if e.verb == Verb::Crop { "crop" } else { kind.key() };
    let (mm, given) = match e.mm {
        Some(mm) => (mm, true),
        None => (cfg.edits[default_key], false),
    };
    Ok(Edit { kind, mm: mm * sign * dir, given, words: e.words.clone() })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexicon::tokenize;

    fn spec(s: &str) -> Spec {
        let cfg = Config::builtin();
        parse(s, tokenize(s), &cfg, &ParseOptions::default()).unwrap_or_else(|d| panic!("{s}: {d:?}"))
    }

    fn err(s: &str) -> String {
        let cfg = Config::builtin();
        parse(s, tokenize(s), &cfg, &ParseOptions::default()).unwrap_err()[0].message.clone()
    }

    #[test]
    fn features_after_with() {
        let s = spec("make a t-shirt with long sleeves and a hood");
        assert_eq!(s.g(), Garment::Tshirt);
        assert_eq!(s.sleeves.unwrap().value, SleeveKind::Long);
        assert!(s.hood.unwrap().value);
        assert!(s.edits.is_empty());
    }

    #[test]
    fn amount_belongs_to_its_clause() {
        let s = spec("make a hoodie, widen the collar and lengthen the sleeves by two inches");
        assert_eq!(s.edits.len(), 2);
        assert_eq!(s.edits[0].kind, EditKind::NeckWidth);
        assert!(!s.edits[0].given);
        assert_eq!(s.edits[1].kind, EditKind::SleeveLength);
        assert!((s.edits[1].mm - 50.8).abs() < 1e-9);
    }

    #[test]
    fn elided_verb_shares_the_amount() {
        let s = spec("make a shirt and lengthen the sleeves and the body by 3 cm");
        assert_eq!(s.edits.iter().map(|e| (e.kind, e.mm)).collect::<Vec<_>>(), vec![(EditKind::SleeveLength, 30.0), (EditKind::BodyLength, 30.0)]);
    }

    #[test]
    fn comparatives_and_signs() {
        let s = spec("baggy pants, make the legs shorter by an inch and raise the waist 1 cm");
        assert_eq!(s.fit.unwrap().value, Fit::Loose);
        assert_eq!(s.edits[0].kind, EditKind::LegLength);
        assert!((s.edits[0].mm + 25.4).abs() < 1e-9);
        assert_eq!((s.edits[1].kind, s.edits[1].mm), (EditKind::Rise, 10.0));
    }

    #[test]
    fn long_sleeve_shirt() {
        let s = spec("a long sleeve shirt in size L");
        assert_eq!((s.g(), s.size), (Garment::Shirt, Size::L));
        assert_eq!(s.sleeves.unwrap().value, SleeveKind::Long);
    }

    #[test]
    fn pins_narrow_variants() {
        let s = spec("a hoodie with rib cuffs and a lined hood, no pocket");
        assert_eq!(s.pin("sleeve_finish").unwrap().variants, &["rib_cuff"]);
        assert_eq!(s.pin("hood_lining").unwrap().variants, &["lined"]);
        assert!(!s.pocket.unwrap().value);
    }

    #[test]
    fn it_means_the_garment() {
        let s = spec("make a boxy t-shirt and crop it by 3 inches");
        assert_eq!(s.edits[0].kind, EditKind::BodyLength);
        assert!((s.edits[0].mm + 76.2).abs() < 1e-9);
    }

    #[test]
    fn colours_go_to_their_parts() {
        let s = spec("make a navy hoodie with white drawstrings and a grey hood");
        assert_eq!(s.design.base.unwrap().value.name, "navy");
        let z: Vec<_> = s.design.zones.iter().map(|(z, c)| (*z, c.value.name.clone())).collect();
        assert_eq!(z, vec![(Zone::Drawstring, "white".to_string()), (Zone::Hood, "grey".to_string())]);
    }

    #[test]
    fn two_colours_and_a_print() {
        let s = spec("make a black and white striped t-shirt");
        let p = s.design.print.unwrap();
        assert_eq!(p.colors.iter().map(|c| c.name.as_str()).collect::<Vec<_>>(), vec!["black", "white"]);
        assert!(s.design.base.is_none());
    }

    #[test]
    fn print_colours_are_not_stolen_by_later_parts() {
        let s = spec("make a black and white striped t-shirt with long sleeves");
        assert_eq!(s.design.print.unwrap().colors.len(), 2);
        assert!(s.design.zones.is_empty());
        let s = spec("make a navy hoodie with a grey hood");
        assert_eq!(s.design.base.unwrap().value.name, "navy");
    }

    #[test]
    fn sweep_with_a_colour_list() {
        let s = spec("make a white t-shirt with long sleeves and wheat, dark blue and forest green stripes from the hem into the sleeves");
        assert_eq!(s.design.base.unwrap().value.name, "white");
        let p = s.design.print.unwrap();
        assert_eq!(p.kind, PrintKind::Sweep);
        assert_eq!(p.colors.iter().map(|c| c.name.as_str()).collect::<Vec<_>>(), vec!["wheat", "dark blue", "forest green"]);
    }

    #[test]
    fn coverage_does_not_name_a_garment() {
        let s = spec("make a white t-shirt with wheat and navy stripes from the hem into the sleeves, covering almost the whole shirt");
        assert_eq!(s.g(), Garment::Tshirt);
        assert_eq!(s.design.coverage, Some(0.85));
    }

    #[test]
    fn shades_and_trailing_colours() {
        let s = spec("make pants in dark green");
        assert_eq!(s.design.base.unwrap().value.name, "dark green");
        let s = spec("make a t-shirt with sleeves in red");
        assert_eq!(s.design.zones[0].0, Zone::Sleeves);
    }

    #[test]
    fn errors() {
        assert!(err("make a t-shirt and lengthen the sleeves by 2").contains("2 what"));
        assert!(err("widen the collar").contains("no garment"));
        assert!(err("make pants and lengthen the sleeves").contains("sleeves"));
        assert!(err("make a hoodie with a hood by 2 cm").contains("nothing uses"));
        assert!(err("make a tshirt and a hoodie").contains("two garments"));
        assert!(err("make a tshirt with long sleves").contains("sleves"));
        assert!(err("make a black white hoodie").contains("colours"));
    }
}
