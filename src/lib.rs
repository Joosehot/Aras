//! Aras: descriptive English in, deterministic sewing patterns and products out.
//!
//! sentence -> lexicon (tokens) -> parser (garment + design spec) -> search
//! (beam over construction rules, finalists drafted and laid out) -> design
//! (colourway resolved per piece) -> SVG: a product sheet, a cutting layout
//! or a printable pattern. No model in the loop: the same sentence and the
//! same rules.toml always give byte-identical output.

pub mod blocks;
pub mod config;
pub mod design;
pub mod draft;
pub mod explain;
pub mod geom;
pub mod layout;
pub mod lexicon;
pub mod model;
pub mod parser;
pub mod pattern;
pub mod product;
pub mod rules;
pub mod search;
pub mod svg;

use config::Config;
use design::Look;
use layout::Marker;
use model::{Garment, Size, Spec};
use parser::{Diag, ParseOptions};
use search::Outcome;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SheetKind {
    /// Product sheet: flats in colour, colourway, bill of materials (default).
    #[default]
    Product,
    /// Cutting layout on fabric at 1:1, pieces printed in their colourway.
    Marker,
    /// Printable pattern: halves on the fold, test square.
    Pattern,
}

#[derive(Clone, Debug, Default)]
pub struct Options {
    /// Ignore words the lexicon doesn't know instead of failing.
    pub lenient: bool,
    /// Garment to change when the sentence names none.
    pub base: Option<Garment>,
    /// Size override.
    pub size: Option<Size>,
    pub sheet: SheetKind,
}

pub struct Compiled {
    pub spec: Spec,
    pub outcome: Outcome,
    pub look: Look,
    /// Cutting layout per fabric and colourway.
    pub marker: Marker,
    pub svg: String,
}

pub fn compile(sentence: &str, cfg: &Config, opts: &Options) -> Result<Compiled, Vec<Diag>> {
    let tokens = lexicon::tokenize(sentence);
    let popts = ParseOptions { lenient: opts.lenient, base: opts.base, size: opts.size };
    let spec = parser::parse(sentence, tokens, cfg, &popts)?;
    let outcome = search::search(&spec, cfg)?;
    let look = design::resolve(&spec, &outcome.pattern, cfg);
    let marker = layout::marker_by(&outcome.pattern, cfg, |i, p| look.piece_label(i, p));
    let svg = match opts.sheet {
        SheetKind::Product => product::sheet(&spec, &outcome, &look, &marker, cfg),
        SheetKind::Marker => svg::marker(&outcome.pattern, &outcome, &marker, Some(&look), &spec.sentence, cfg),
        SheetKind::Pattern => {
            let sheet = layout::pattern_sheet(&outcome.pattern, cfg);
            svg::pattern_sheet(&outcome.pattern, &outcome, &sheet, &spec.sentence, cfg)
        }
    };
    Ok(Compiled { spec, outcome, look, marker, svg })
}
